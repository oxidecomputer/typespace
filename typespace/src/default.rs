// Copyright 2026 Oxide Computer Company

use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use log::debug;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    TypespaceBuilder, TypespaceRenderer,
    build::{self, StructProperty, StructPropertySerde, StructPropertyState, Type, VariantDetails},
    error::Error,
    settings::{OptionalNullable, Settings, Std},
};

/// Prefix shared by the `::std::num::NonZero*` type paths.
///
/// A default value for one of these is built through `new()` rather
/// than written as a literal, since there is no literal form for them.
const STD_NUM_NONZERO_PREFIX: &str = "::std::num::NonZero";

/// A function in the generated `defaults` module that properties share.
///
/// A boolean or integer default value needs no code of its own: one
/// generic function, parameterized by the value, produces the default
/// for every property whose default is a value of that kind. Rendering
/// records which of these it wants and emits one definition of each, so
/// that the module holds a single `default_u64` rather than one
/// function per property, all effectively identical.
///
/// The declaration order here is the order the definitions appear in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefaultHelper {
    /// `default_bool::<const V: bool>() -> bool`
    Boolean,
    /// `default_i64::<T, const V: i64>() -> T`
    I64,
    /// `default_u64::<T, const V: u64>() -> T`
    U64,
    /// `default_nzu64::<T, const V: u64>() -> T`
    NZU64,
}

impl DefaultHelper {
    /// The helper's definition, for the `defaults` module.
    pub(crate) fn definition(&self) -> TokenStream {
        match self {
            DefaultHelper::Boolean => quote! {
                pub(super) fn default_bool<const V: bool>() -> bool {
                    V
                }
            },
            DefaultHelper::I64 => quote! {
                pub(super) fn default_i64<T, const V: i64>() -> T
                where
                    T: ::std::convert::TryFrom<i64>,
                    <T as ::std::convert::TryFrom<i64>>::Error: ::std::fmt::Debug,
                {
                    T::try_from(V).unwrap()
                }
            },
            DefaultHelper::U64 => quote! {
                pub(super) fn default_u64<T, const V: u64>() -> T
                where
                    T: ::std::convert::TryFrom<u64>,
                    <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,
                {
                    T::try_from(V).unwrap()
                }
            },
            DefaultHelper::NZU64 => quote! {
                pub(super) fn default_nzu64<T, const V: u64>() -> T
                where
                    T: ::std::convert::TryFrom<::std::num::NonZeroU64>,
                    <T as ::std::convert::TryFrom<::std::num::NonZeroU64>>::Error:
                        ::std::fmt::Debug,
                {
                    T::try_from(::std::num::NonZeroU64::try_from(V).unwrap())
                        .unwrap()
                }
            },
        }
    }
}

/// A shared helper together with the path that instantiates it.
pub(crate) struct SharedDefaultFn {
    pub helper: DefaultHelper,
    /// How a property names the helper, as `defaults::name::<args>`.
    pub path: String,
}

/// Match a property's type and default value to a shared helper.
///
/// `None` means no helper produces this value, and the property needs a
/// function minted for it. Only the type's own node is considered: a
/// named type whose definition is an integer, or an `Option` of one,
/// takes a function of its own, since a helper would have to name the
/// value's type as well as produce it.
pub(crate) fn shared_default_fn<Id: Ord>(
    types: &BTreeMap<Id, Type<Id>>,
    id: &Id,
    value: &serde_json::Value,
) -> Option<SharedDefaultFn> {
    let (helper, path) = match types.get(id).expect("invalid type id") {
        Type::Boolean => {
            let value = value.as_bool()?;
            (
                DefaultHelper::Boolean,
                format!("defaults::default_bool::<{}>", value),
            )
        }

        // An unsigned value is produced from a u64 (or, for the
        // ::std::num::NonZero* types, from a NonZeroU64, which is what
        // they convert from); anything else that fits in an i64 is
        // produced from an i64.
        Type::Integer(itype) => match (value.as_u64(), value.as_i64()) {
            (Some(value), _) if itype.starts_with(STD_NUM_NONZERO_PREFIX) => (
                DefaultHelper::NZU64,
                format!("defaults::default_nzu64::<{}, {}>", itype, value),
            ),
            (Some(value), _) => (
                DefaultHelper::U64,
                format!("defaults::default_u64::<{}, {}>", itype, value),
            ),
            // ATTN REVIEWER: typify 1 checks for a NonZero type only
            // in the unsigned branch, so it routes a negative default
            // on a signed NonZero type to default_i64, which does not
            // compile: NonZeroI32 converts from NonZeroU64 and from
            // i32, but not from i64. No typify 1 golden covers it. The
            // per-property function builds the value with
            // `NonZeroI32::new(-5).unwrap()`, which compiles, so send
            // it there.
            (_, Some(_)) if itype.starts_with(STD_NUM_NONZERO_PREFIX) => return None,
            (_, Some(value)) => (
                DefaultHelper::I64,
                format!("defaults::default_i64::<{}, {}>", itype, value),
            ),
            // A value that is a number but neither a u64 nor an i64 is
            // a float, which no helper produces. Validation admits one
            // for an integer type, so this is reachable; the
            // per-property path reports it.
            (None, None) => return None,
        },

        _ => return None,
    };

    Some(SharedDefaultFn { helper, path })
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceBuilder<Id> {
    pub(crate) fn check_default(
        &self,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<(), Error<Id>> {
        let Self { types, settings } = self;
        check_default(types, settings, value, id)
    }
}

fn check_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    value: &serde_json::Value,
    id: &Id,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    debug!("checking default for {id}");
    let imp = DefaultImpl {
        types,
        settings,
        scope: None,
        mode: Mode::Check,
    };
    let mut expansion_set = Vec::new();
    imp.default_impl(&mut expansion_set, id, value).map(|_| ())
}

impl<'a, Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceRenderer<'a, Id> {
    pub(crate) fn generate_default(&self, value: &serde_json::Value, id: &Id) -> TokenStream {
        let Self { types, settings } = self;
        generate_default(types, settings, value, id)
    }

    pub(crate) fn generate_default_value_for_impl(
        &self,
        value: &serde_json::Value,
        id: &Id,
    ) -> TokenStream {
        let Self { types, settings } = self;
        let imp = DefaultImpl {
            types,
            settings,
            scope: None,
            mode: Mode::Generate,
        };
        let mut expansion_set = Vec::new();
        imp.default_impl(&mut expansion_set, id, value)
            .expect("an error should not be possible post-validation")
            .expect("a value should be generated with Mode::Generate")
    }

    pub(crate) fn generate_default_enum(&self, value: &serde_json::Value, id: &Id) -> EnumDefault {
        let Self { types, settings } = self;
        let imp = DefaultImpl {
            types,
            settings,
            scope: None,
            mode: Mode::Generate,
        };
        let mut expansion_set = Vec::new();

        let Some(Type::Enum(enum_info)) = types.get(&id) else {
            unreachable!("this should only be called on an enum type")
        };

        let enum_default = imp
            .default_impl_enum(&mut expansion_set, enum_info, value, id)
            .expect("an error should not be possible post-validation")
            .expect("a value should be generated with Mode::Generate");

        match (enum_default, &settings.typify_compat) {
            ((_, Some(variant_name)), false) => EnumDefault::Variant(variant_name),
            ((default_value, _), _) => EnumDefault::Value(default_value),
        }
    }
}

fn generate_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    value: &serde_json::Value,
    id: &Id,
) -> TokenStream
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    debug!("generating default for {id}");
    let imp = DefaultImpl {
        types,
        settings,
        scope: Some("super"),
        mode: Mode::Generate,
    };
    let mut expansion_set = Vec::new();
    imp.default_impl(&mut expansion_set, id, value)
        .expect("an error should not be possible post-validation")
        .expect("a value should be generated with Mode::Generate")
}

pub(crate) enum EnumDefault {
    Value(TokenStream),
    Variant(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Check,
    Generate,
}

/// Holds the shared context threaded through default-value processing.
struct DefaultImpl<'a, Id> {
    types: &'a BTreeMap<Id, Type<Id>>,
    settings: &'a Settings,
    /// Scope for reaching a type from the generated `defaults` module,
    /// typically `super::`.
    scope: Option<&'a str>,
    mode: Mode,
}

impl<Id> DefaultImpl<'_, Id>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    /// Produce `f`'s value in `Mode::Generate`; nothing in
    /// `Mode::Check`, which only validates a value's shape.
    fn generate<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce() -> T,
    {
        match self.mode {
            Mode::Check => None,
            Mode::Generate => Some(f()),
        }
    }

    /// Render `id`'s type, qualified for the walk's scope.
    fn render_ident(&self, id: &Id) -> TokenStream {
        TypespaceRenderer::new(self.types, self.settings).render_ident_with_scope(id, self.scope)
    }

    /// Render `id`'s type with its generic arguments left off.
    ///
    /// That is how the path to one of the type's variants or
    /// associated functions is spelled.
    fn render_base_type(&self, id: &Id) -> TokenStream {
        TypespaceRenderer::new(self.types, self.settings).render_raw_type(id)
    }

    /// Render one of `Option`'s variants.
    ///
    /// `Some` and `None` are in the prelude, so the unqualified spelling
    /// names the variant on its own.
    fn render_option_variant(&self, id: &Id, variant: &str) -> TokenStream {
        let variant = format_ident!("{}", variant);
        match self.settings.std {
            Std::Unqualified => quote! { #variant },
            Std::FullyQualified => {
                let option_type = self.render_base_type(id);
                quote! { #option_type::#variant }
            }
        }
    }

    fn render_option_variant2(&self, variant: &str) -> TokenStream {
        let variant = format_ident!("{}", variant);
        match self.settings.std {
            Std::Unqualified => quote! { #variant },
            Std::FullyQualified => {
                quote! { ::std::option::Option::#variant }
            }
        }
    }

    fn default_impl(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        id: &Id,
        value: &serde_json::Value,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let ty = self.types.get(&id).unwrap();
        match ty {
            Type::Enum(enum_info) => self
                .default_impl_enum(expansion_set, enum_info, value, id)
                .map(|ok| ok.map(|some| some.0)),
            Type::Struct(struct_info) => {
                self.default_impl_struct(expansion_set, struct_info, value, id)
            }
            Type::UnitStruct(unit_struct) => self.default_impl_unit_struct(unit_struct, value, id),
            Type::TupleStruct(tuple_struct) => {
                self.default_impl_tuple_struct(expansion_set, tuple_struct, value, id)
            }
            Type::NewtypeStruct(newtype_struct) => {
                // TODO 9/4/2026
                // if mode = validate we need to check the value against
                // constraints for the newtype

                // A newtype struct may contain itself (directly or
                // indirectly), so we need to take care not to recur without
                // narrowing the JSON value.
                let inner =
                    self.expansion_guard_default_impl(expansion_set, &newtype_struct.inner, value)?;
                Ok(inner.map(|inner| {
                    let ident = self.render_ident(&id);
                    quote! { #ident(#inner) }
                }))
            }
            Type::TypeAlias(type_alias) => {
                // A type alias may refer to itself (directly or indirectly),
                // so we need to take care not to recur without narrowing the
                // JSON value.
                self.expansion_guard_default_impl(expansion_set, &type_alias.target, value)
            }

            Type::Native(_) => {
                // A native type's value is whatever its own Deserialize
                // accepts, which we have no way to check here; a value that
                // doesn't fit fails the unwrap() in the generated code.

                // TODO 9/4/2026
                // expect rather than unwrap?
                let text = value.to_string();
                Ok(self.generate(|| {
                    let type_path = self.render_ident(&id);
                    quote! {
                        ::serde_json::from_str::<#type_path>(#text).unwrap()
                    }
                }))
            }

            // Note that this doesn't mean "optional"; it means that null is
            // also permitted as a value.
            Type::Option(type_id) => {
                if value.is_null() {
                    Ok(self.generate(|| self.render_option_variant(&id, "None")))
                } else {
                    // We're not narrowing the value so check for cycles. This
                    // really could only happen if someone were attempting
                    // self-harm: an anonymous Option that contained itself.
                    // But people are weird and terrible.
                    let inner = self.expansion_guard_default_impl(expansion_set, type_id, value)?;
                    Ok(inner.map(|inner| {
                        let some = self.render_option_variant(&id, "Some");
                        quote! { #some(#inner) }
                    }))
                }
            }
            Type::Box(type_id) => {
                // As above with Option, a deliberately self-harming
                // construction could cause infinite recursion without the
                // guard.
                let inner = self.expansion_guard_default_impl(expansion_set, type_id, value)?;
                // TODO 9/4/2026
                // We need Settings to know what to render here...
                Ok(inner.map(|inner| quote! { Box::new(#inner) }))
            }
            Type::Vec(elem_id) => {
                let arr = value.as_array().ok_or_else(|| Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected array".to_string(),
                })?;

                let elems = arr
                    .iter()
                    .map(|elem_value| self.default_impl(expansion_set, elem_id, elem_value))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.generate(|| {
                    let elems = elems
                        .into_iter()
                        .map(|elem| elem.expect("a value should be generated with Mode::Generate"));

                    // TODO 9/8/2026
                    // TYPIFY COMPAT: could rationalize this and Set below
                    quote! { vec![ #( #elems ),* ] }
                }))
            }
            Type::Map(key_id, value_id) => {
                let map = value.as_object().ok_or_else(|| Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected object".to_string(),
                })?;

                let entries = map
                    .iter()
                    .map(|(key, entry_value)| {
                        // A JSON object's keys are always strings, but the
                        // map's key type need not be: wrap the raw key text
                        // as a JSON string and let the key type's own walk
                        // make sense of it, exactly as it would any other
                        // string value.
                        let key_value = serde_json::Value::String(key.clone());
                        let key = self.default_impl(expansion_set, key_id, &key_value)?;
                        let entry_value =
                            self.default_impl(expansion_set, value_id, entry_value)?;
                        Ok((key, entry_value))
                    })
                    .collect::<Result<Vec<_>, Error<Id>>>()?;

                Ok(self.generate(|| {
                    let entries = entries.into_iter().map(|(key, entry_value)| {
                        let key = key.expect("a value should be generated with Mode::Generate");
                        let entry_value =
                            entry_value.expect("a value should be generated with Mode::Generate");
                        quote! { (#key, #entry_value) }
                    });
                    // This expression works whatever `map_type` renders as:
                    // every container it can be (a `BTreeMap`, a `HashMap`,
                    // or a consumer's own choice) implements `FromIterator`.
                    quote! { [ #( #entries ),* ].into_iter().collect() }
                }))
            }
            Type::Set(elem_id) => {
                let arr = value.as_array().ok_or_else(|| Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected a JSON array".to_string(),
                })?;

                // A set can't contain duplicates; `Value` has no `Ord`
                // impl, so this is the O(n^2) check.
                for (index, element) in arr.iter().enumerate() {
                    if arr[..index].contains(element) {
                        return Err(Error::InvalidDefault {
                            value: value.clone(),
                            id: id.clone(),
                            reason: format!("duplicate value in set default: {element}"),
                        });
                    }
                }

                let elems = arr
                    .iter()
                    .map(|elem_value| self.default_impl(expansion_set, elem_id, elem_value))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.generate(|| {
                    let elems = elems
                        .into_iter()
                        .map(|elem| elem.expect("a value should be generated with Mode::Generate"));
                    quote! { [ #( #elems ),* ].into_iter().collect() }
                }))
            }
            Type::Array(elem_id, len) => {
                let arr = value.as_array().ok_or_else(|| Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected a JSON array".to_string(),
                })?;
                if arr.len() != *len {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: format!("expected an array of length {len}"),
                    });
                }

                let elems = arr
                    .iter()
                    .map(|elem_value| self.default_impl(expansion_set, elem_id, elem_value))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.generate(|| {
                    let elems = elems
                        .into_iter()
                        .map(|elem| elem.expect("a value should be generated with Mode::Generate"));
                    quote! { [ #( #elems ),* ] }
                }))
            }
            Type::Tuple(items) => {
                let elems = self.default_impl_tuple_items(expansion_set, items, value, &id)?;
                Ok(elems.map(|elems| quote! { ( #( #elems ),* ) }))
            }

            Type::Unit => {
                if value.is_null() {
                    Ok(self.generate(|| quote! { () }))
                } else {
                    Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "unit type default value must be null".to_string(),
                    })
                }
            }

            Type::Boolean => {
                let Some(v) = value.as_bool() else {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected a boolean".to_string(),
                    });
                };
                Ok(self.generate(|| quote! { #v }))
            }
            Type::Integer(itype) => {
                let Some(_) = value.as_number() else {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected an integer".to_string(),
                    });
                };

                if itype.starts_with(STD_NUM_NONZERO_PREFIX) {
                    let type_path = syn::parse_str::<syn::TypePath>(itype).unwrap();
                    let num = proc_macro2::Literal::from_str(value.to_string().as_str()).unwrap();

                    Ok(self.generate(|| {
                        quote! {
                            #type_path::new(#num).unwrap()
                        }
                    }))
                } else {
                    let val = match proc_macro2::Literal::from_str(&format!("{}_{}", value, itype))
                    {
                        Ok(v) => v,
                        Err(_) => unreachable!(),
                    };
                    Ok(self.generate(|| TokenStream::from(proc_macro2::TokenTree::from(val))))
                }
            }
            Type::Float(ftype) => {
                let Some(_) = value.as_number() else {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected a number".to_string(),
                    });
                };

                let val = match proc_macro2::Literal::from_str(&format!("{}_{}", value, ftype)) {
                    Ok(v) => v,
                    Err(_) => unreachable!(),
                };
                Ok(self.generate(|| TokenStream::from(proc_macro2::TokenTree::from(val))))
            }
            Type::String => {
                let Some(s) = value.as_str() else {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected a string".to_string(),
                    });
                };
                Ok(self.generate(|| quote! { #s.to_string()}))
            }
            Type::JsonValue => {
                let text = value.to_string();
                Ok(self.generate(|| {
                    quote! {
                        ::serde_json::from_str::<::serde_json::Value>(#text).unwrap()
                    }
                }))
            }
            Type::Never => Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: "a never type may not have a value".to_string(),
            }),
        }
    }

    /// Validate and (in `Mode::Generate`) render a tuple-shaped value's
    /// components against `items`, in order.
    ///
    /// Shared by `Type::Tuple`, the enum variants whose payload is a
    /// tuple, and a tuple struct's fixed fields: all three check a JSON
    /// array's length against a fixed list of types and walk each
    /// component in turn.
    fn default_impl_tuple_items(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        items: &[Id],
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<Vec<TokenStream>>, Error<Id>> {
        let arr = value.as_array().ok_or_else(|| Error::InvalidDefault {
            value: value.clone(),
            id: id.clone(),
            reason: "expected a JSON array".to_string(),
        })?;
        if arr.len() != items.len() {
            return Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("expected a tuple of length {}", items.len()),
            });
        }

        let elems = items
            .iter()
            .zip(arr.iter())
            .map(|(item_id, item_value)| self.default_impl(expansion_set, item_id, item_value))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(self.generate(|| {
            elems
                .into_iter()
                .map(|elem| elem.expect("a value should be generated with Mode::Generate"))
                .collect()
        }))
    }

    /// Build the field initializers for a struct-shaped value: `f: expr`
    /// for each property, in declaration order.
    ///
    /// For structs, we wrap this in the struct's name; for struct enum
    /// variant's, we wrap it in the variant's name.
    fn default_impl_struct_props(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        properties: &[StructProperty<Id>],
        deny_unknown_fields: bool,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Vec<TokenStream>, Error<Id>> {
        let map = value.as_object().ok_or_else(|| Error::InvalidDefault {
            value: value.clone(),
            id: id.clone(),
            reason: "expected JSON object".to_string(),
        })?;

        let mut rendered_properties = Vec::new();
        let mut fields = BTreeSet::new();

        for prop_info in properties {
            let named = match &prop_info.json_name {
                StructPropertySerde::None => Some(&prop_info.rust_name),
                StructPropertySerde::Rename(rename) => Some(rename),
                StructPropertySerde::Flatten => None,
            };

            if let Some(prop_name) = named {
                fields.insert(prop_name);
                let prop_value = map.get(prop_name);
                match (&prop_info.state, prop_value) {
                    // Required property; no value.
                    (StructPropertyState::Required, None) => {
                        return Err(Error::InvalidDefault {
                            value: value.clone(),
                            id: id.clone(),
                            reason: format!("property {} is required", prop_info.rust_name),
                        });
                    }

                    // Optional or default; no value.
                    //
                    // Both produce Default::default(). For Optional, it means
                    // that the field is absent (this applies for all
                    // optional-nullable settings).
                    (StructPropertyState::Optional, None)
                    | (StructPropertyState::Default, None) => {
                        if self.mode == Mode::Generate {
                            // TODO 9/4/2026
                            // Qualify default Default
                            let prop_ident = format_ident!("{}", prop_info.rust_name);
                            rendered_properties.push(quote! {
                                #prop_ident: Default::default()
                            });
                        }
                    }

                    // Default with value; no value.
                    //
                    // Note that this is the only place in our recursive
                    // descent where we're *expanding* the input. This is
                    // particularly where we need to use the expansion guard.
                    (StructPropertyState::DefaultValue(prop_default_value), None) => {
                        let try_rendered_prop_value = self.expansion_guard_default_impl(
                            expansion_set,
                            &prop_info.type_id,
                            &prop_default_value.0,
                        );
                        if let Some(rendered_prop_value) = try_rendered_prop_value? {
                            let prop_ident = format_ident!("{}", prop_info.rust_name);
                            rendered_properties.push(quote! {
                                #prop_ident: #rendered_prop_value
                            })
                        }
                    }

                    // Optional field; value present.
                    (StructPropertyState::Optional, Some(prop_value)) => {
                        let prop_id = &prop_info.type_id;

                        let prop_ty = self.types.get(prop_id).unwrap();
                        let is_option = matches!(prop_ty, Type::Option(_));

                        let prop_default_value = if is_option {
                            match &self.settings.optional_nullable {
                                // A simple Option<T> is sufficient.
                                OptionalNullable::ConflateAsAbsent
                                | OptionalNullable::ConflateAsNull => {
                                    self.default_impl(expansion_set, prop_id, prop_value)?
                                }
                                // Nest the option in a `Some`.
                                OptionalNullable::DoubleOption => self
                                    .default_impl(expansion_set, prop_id, prop_value)?
                                    .map(|prop_value| {
                                        let some = self.render_option_variant2("Some");
                                        quote! { #some(#prop_value) }
                                    }),

                                // Construct the custom type
                                OptionalNullable::CustomType(type_name) => self
                                    .default_impl_custom_optional_nullable(
                                        expansion_set,
                                        prop_id,
                                        prop_value,
                                        type_name,
                                    )?,
                            }
                        } else {
                            self.default_impl(expansion_set, prop_id, prop_value)?.map(
                                |prop_value| {
                                    let some = self.render_option_variant2("Some");
                                    quote! { #some(#prop_value) }
                                },
                            )
                        };

                        let prop_default = prop_default_value.map(|value| {
                            let prop_ident = format_ident!("{}", prop_info.rust_name);
                            quote! { #prop_ident: #value}
                        });

                        if self.mode == Mode::Generate {
                            rendered_properties.push(
                                prop_default
                                    .expect("a value should be generated with Mode::Generate"),
                            );
                        }
                    }

                    // All other fields; value present.
                    (
                        StructPropertyState::Required
                        | StructPropertyState::Default
                        | StructPropertyState::DefaultValue(_),
                        Some(prop_value),
                    ) => {
                        let prop_id = &prop_info.type_id;

                        let prop_default_value =
                            self.default_impl(expansion_set, prop_id, prop_value)?;

                        let prop_default = prop_default_value.map(|value| {
                            let prop_ident = format_ident!("{}", prop_info.rust_name);
                            quote! { #prop_ident: #value}
                        });

                        if self.mode == Mode::Generate {
                            rendered_properties.push(
                                prop_default
                                    .expect("a value should be generated with Mode::Generate"),
                            );
                        }
                    }
                }
            } else {
                // `check_type_structure` rejects a struct that both
                // denies unknown fields and flattens a property, so a
                // flattened property here means the flag is clear.
                assert!(
                    !deny_unknown_fields,
                    "check_type_structure rejects deny_unknown_fields with a flattened property"
                );

                // We're flattening; take the full value and see if the
                // property's type can make something of it.

                let prop_keys = properties
                    .iter()
                    .filter_map(
                        |StructProperty {
                             rust_name,
                             json_name,
                             ..
                         }| match json_name {
                            StructPropertySerde::None => Some(rust_name),
                            StructPropertySerde::Rename(rename) => Some(rename),
                            StructPropertySerde::Flatten => None,
                        },
                    )
                    .collect::<BTreeSet<_>>();
                let new_value = serde_json::Value::Object(
                    map.clone()
                        .into_iter()
                        .filter(|(key, _)| !prop_keys.contains(key))
                        .collect(),
                );

                let prop_id = &prop_info.type_id;

                if prop_info.state == StructPropertyState::Optional {
                    // Note that we ignore errors for Optional flattened fields
                    // intentionally.
                    let prop_default = if let Some(prop_default) = self
                        .default_impl(expansion_set, prop_id, &new_value)
                        .ok()
                        .flatten()
                    {
                        let some = self.render_option_variant2("Some");
                        quote! {
                            #some(#prop_default)
                        }
                    } else {
                        // TODO 9/8/2026
                        // qualify Default
                        quote! { Default::default() }
                    };
                    if self.mode == Mode::Generate {
                        let prop_ident = format_ident!("{}", prop_info.rust_name);
                        rendered_properties.push(quote! {
                            #prop_ident: #prop_default
                        });
                    }
                } else {
                    if let Some(prop_default) =
                        self.default_impl(expansion_set, prop_id, &new_value)?
                    {
                        let prop_ident = format_ident!("{}", prop_info.rust_name);
                        rendered_properties.push(quote! {
                            #prop_ident: #prop_default
                        });
                    }
                }
            }
        }

        if deny_unknown_fields {
            let extra_keys = map
                .keys()
                .collect::<BTreeSet<_>>()
                .difference(&fields)
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            if !extra_keys.is_empty() {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!("extra properties in default: {}", extra_keys.join(",")),
                });
            }
        }

        Ok(rendered_properties)
    }

    fn default_impl_struct(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        struct_info: &build::Struct<Id>,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let rendered_properties = self.default_impl_struct_props(
            expansion_set,
            &struct_info.properties,
            struct_info.deny_unknown_fields,
            value,
            id,
        )?;

        Ok(self.generate(|| {
            let struct_ident = self.render_ident(&id);
            quote! {
                #struct_ident {
                    #( #rendered_properties, )*
                }
            }
        }))
    }

    fn default_impl_enum(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<(TokenStream, Option<String>)>, Error<Id>> {
        match enum_info.tag_type.as_ref().unwrap() {
            build::EnumTagType::External => {
                self.default_impl_enum_external(expansion_set, enum_info, value, id)
            }
            build::EnumTagType::Internal { tag } => {
                self.default_impl_enum_internal(expansion_set, enum_info, tag, value, id)
            }
            build::EnumTagType::Adjacent { tag, content } => {
                self.default_impl_enum_adjacent(expansion_set, enum_info, tag, content, value, id)
            }
            build::EnumTagType::Untagged => {
                self.default_impl_enum_untagged(expansion_set, enum_info, value, id)
            }
        }
    }

    /// An externally tagged enum uses a bare string to represent unit
    /// variants, and a one-item object with the variant name as the key
    /// for all other variant types.
    fn default_impl_enum_external(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<(TokenStream, Option<String>)>, Error<Id>> {
        if let Some(variant_name) = value.as_str() {
            let variant = enum_info
                .variants
                .iter()
                .find(|variant| {
                    // TODO 9/4/2026
                    // This is kind of wrong; we're going to need to represent the json
                    // serialization name and use that.
                    let name = variant.rename.as_ref().unwrap_or(&variant.rust_name);
                    variant_name == name
                })
                .ok_or_else(|| Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!("variant {} not found in enum", variant_name),
                })?;

            if variant.details != VariantDetails::Unit {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!("non-unit variant {} without its payload", variant_name),
                });
            }

            let var_ident = format_ident!("{}", variant.rust_name);
            let type_ident = self.render_ident(&id);
            Ok(self.generate(|| {
                (
                    quote! { #type_ident::#var_ident },
                    Some(variant.rust_name.clone()),
                )
            }))
        } else if let Some(map) = value.as_object() {
            if map.len() != 1 {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected an object with exactly one entry".to_string(),
                });
            }
            let (variant_name, var_value) = map.iter().next().unwrap();

            let Some(variant) = enum_info
                .variants
                .iter()
                .find(|variant| variant_name == variant.json_name())
            else {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!("no variant matching {}", variant_name),
                });
            };

            let var_ident = format_ident!("{}", variant.rust_name);
            let type_ident = self.render_ident(&id);

            match &variant.details {
                VariantDetails::Unit => Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!("unit variant {} carries no payload", variant_name),
                }),
                VariantDetails::Item(item_id) => {
                    let item = self.default_impl(expansion_set, item_id, var_value)?;
                    Ok(item.map(|item| (quote! { #type_ident::#var_ident(#item) }, None)))
                }
                VariantDetails::Tuple(items) => {
                    let elems =
                        self.default_impl_tuple_items(expansion_set, items, var_value, &id)?;
                    Ok(elems
                        .map(|elems| (quote! { #type_ident::#var_ident( #( #elems ),* ) }, None)))
                }
                VariantDetails::Struct(props) => {
                    let rendered = self.default_impl_struct_props(
                        expansion_set,
                        props,
                        enum_info.deny_unknown_fields,
                        var_value,
                        id,
                    )?;
                    Ok(self.generate(|| {
                        (
                            quote! { #type_ident::#var_ident { #( #rendered, )* } },
                            None,
                        )
                    }))
                }
            }
        } else {
            // A variant carrying a payload serializes as a single-entry
            // object, keyed by the variant's serialized name.
            Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: "expected a string or a single-entry object".to_string(),
            })
        }
    }

    fn default_impl_enum_internal(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        enum_info: &build::Enum<Id>,
        tag: &str,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<(TokenStream, Option<String>)>, Error<Id>> {
        let Some(map) = value.as_object() else {
            return Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: "expected a JSON object".to_string(),
            });
        };

        let Some(tag_value) = map.get(tag).and_then(serde_json::Value::as_str) else {
            return Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("expected a string tag property {tag:?}"),
            });
        };

        let Some(variant) = enum_info
            .variants
            .iter()
            .find(|variant| tag_value == variant.json_name())
        else {
            return Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("variant {} not found in enum", tag_value),
            });
        };

        let var_ident = format_ident!("{}", variant.rust_name);
        let type_ident = self.render_ident(&id);

        // Everything but the tag belongs to the variant's own payload.
        let inner_value = serde_json::Value::Object(
            map.iter()
                .filter(|(name, _)| name.as_str() != tag)
                .map(|(name, prop_value)| (name.clone(), prop_value.clone()))
                .collect(),
        );

        match &variant.details {
            VariantDetails::Unit => Ok(self.generate(|| {
                (
                    quote! { #type_ident::#var_ident },
                    Some(variant.rust_name.clone()),
                )
            })),
            // Serde accepts an internally-tagged newtype variant as long
            // as its payload serializes as a map, so the tag can sit
            // alongside the payload's own keys; walk the payload's type
            // against the tag-stripped object exactly as Struct does.
            VariantDetails::Item(item_id) => {
                let item = self.default_impl(expansion_set, item_id, &inner_value)?;
                Ok(item.map(|item| (quote! { #type_ident::#var_ident(#item) }, None)))
            }
            VariantDetails::Struct(props) => {
                let rendered = self.default_impl_struct_props(
                    expansion_set,
                    props,
                    enum_info.deny_unknown_fields,
                    &inner_value,
                    id,
                )?;
                Ok(self.generate(|| {
                    (
                        quote! { #type_ident::#var_ident { #( #rendered, )* } },
                        None,
                    )
                }))
            }
            // Serde rejects an internally-tagged tuple variant outright: a
            // tuple's payload has no keys to merge the tag alongside.
            VariantDetails::Tuple(_) => Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!(
                    "variant {tag_value} carries a tuple payload, which \
                     internal tagging cannot represent"
                ),
            }),
        }
    }

    fn default_impl_enum_adjacent(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        enum_info: &build::Enum<Id>,
        tag: &str,
        content: &str,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<(TokenStream, Option<String>)>, Error<Id>> {
        let map = value.as_object().ok_or_else(|| Error::InvalidDefault {
            value: value.clone(),
            id: id.clone(),
            reason: "expected a JSON object".to_string(),
        })?;

        let tag_value = map.get(tag).and_then(serde_json::Value::as_str);
        let content_value = map.get(content);

        let (tag_value, content_value) = match (map.len(), tag_value, content_value) {
            (1, Some(tag_value), None) => (tag_value, None),
            (2, Some(tag_value), content_value @ Some(_)) => (tag_value, content_value),
            _ => {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: format!(
                        "expected an object with a {tag:?} tag property and, if the variant \
                         carries a payload, a {content:?} content property"
                    ),
                });
            }
        };

        let variant = enum_info
            .variants
            .iter()
            .find(|variant| tag_value == variant.json_name())
            .ok_or_else(|| Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("variant {} not found in enum", tag_value),
            })?;

        let var_ident = format_ident!("{}", variant.rust_name);
        let type_ident = self.render_ident(&id);

        match (&variant.details, content_value) {
            (VariantDetails::Unit, None) => Ok(self.generate(|| {
                (
                    quote! { #type_ident::#var_ident },
                    Some(variant.rust_name.clone()),
                )
            })),
            (VariantDetails::Item(item_id), Some(content_value)) => {
                let item = self.default_impl(expansion_set, item_id, content_value)?;
                Ok(item.map(|item| (quote! { #type_ident::#var_ident(#item) }, None)))
            }
            (VariantDetails::Tuple(items), Some(content_value)) => {
                let elems =
                    self.default_impl_tuple_items(expansion_set, items, content_value, &id)?;
                Ok(elems.map(|elems| (quote! { #type_ident::#var_ident( #( #elems ),* ) }, None)))
            }
            (VariantDetails::Struct(props), Some(content_value)) => {
                let rendered = self.default_impl_struct_props(
                    expansion_set,
                    props,
                    enum_info.deny_unknown_fields,
                    content_value,
                    id,
                )?;
                Ok(self.generate(|| {
                    (
                        quote! { #type_ident::#var_ident { #( #rendered, )* } },
                        None,
                    )
                }))
            }
            _ => Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("variant {} does not accept this payload", tag_value),
            }),
        }
    }

    fn default_impl_enum_untagged(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<(TokenStream, Option<String>)>, Error<Id>> {
        let type_ident = self.render_ident(&id);

        // Untagged deserialization tries each variant in declaration
        // order and keeps the first whose shape fits; a default value
        // follows the same rule, so the first variant this value fits is
        // the one used to build it. A variant that always fits (a native
        // type, say, which accepts anything) makes every variant behind
        // it unreachable here, exactly as it would at deserialization
        // time.
        enum_info
            .variants
            .iter()
            .find_map(|variant| {
                let var_ident = format_ident!("{}", variant.rust_name);
                match &variant.details {
                    // TODO 9/6/2026
                    // Need to consider unit variants with non-null
                    // serializations.
                    VariantDetails::Unit => value.is_null().then(|| {
                        self.generate(|| {
                            (
                                quote! { #type_ident::#var_ident },
                                Some(variant.rust_name.clone()),
                            )
                        })
                    }),

                    // Note that in this case we don't reduce the size of the
                    // value so this opens the door for infinite recursion if
                    // we don't add the guard.
                    VariantDetails::Item(item_id) => self
                        .expansion_guard_default_impl(expansion_set, item_id, value)
                        .ok()
                        .map(|item| {
                            item.map(|item| (quote! { #type_ident::#var_ident(#item) }, None))
                        }),
                    VariantDetails::Tuple(items) => self
                        .default_impl_tuple_items(expansion_set, items, value, &id)
                        .ok()
                        .map(|elems| {
                            elems.map(|elems| {
                                (quote! { #type_ident::#var_ident( #( #elems ),* ) }, None)
                            })
                        }),
                    VariantDetails::Struct(props) => self
                        .default_impl_struct_props(
                            expansion_set,
                            props,
                            enum_info.deny_unknown_fields,
                            value,
                            id,
                        )
                        .ok()
                        .map(|rendered| {
                            self.generate(|| {
                                (
                                    quote! { #type_ident::#var_ident { #( #rendered, )* } },
                                    None,
                                )
                            })
                        }),
                }
            })
            .ok_or_else(|| Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: "no variant of the untagged enum accepts this value".to_string(),
            })
    }

    fn default_impl_tuple_struct(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        tuple_struct: &build::TupleStruct<Id>,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let arr = value.as_array().ok_or_else(|| Error::InvalidDefault {
            value: value.clone(),
            id: id.clone(),
            reason: "expected a JSON array".to_string(),
        })?;

        let field_count = tuple_struct.fields.len();
        let has_enough = match tuple_struct.rest {
            Some(_) => arr.len() >= field_count,
            None => arr.len() == field_count,
        };
        if !has_enough {
            return Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: match tuple_struct.rest {
                    Some(_) => format!("expected an array of at least length {field_count}"),
                    None => format!("expected an array of length {field_count}"),
                },
            });
        }

        let (head, tail) = arr.split_at(field_count);
        let field_values = self.default_impl_tuple_items(
            expansion_set,
            &tuple_struct.fields,
            &serde_json::Value::Array(head.to_vec()),
            &id,
        )?;

        // Anything past the fixed fields belongs to `rest` as a whole,
        // walked as a value of its own array-shaped type.
        let rest_value = tuple_struct
            .rest
            .as_ref()
            .map(|rest_id| {
                self.default_impl(
                    expansion_set,
                    rest_id,
                    &serde_json::Value::Array(tail.to_vec()),
                )
            })
            .transpose()?;

        Ok(self.generate(|| {
            let field_values = field_values
                .expect("a value should be generated with Mode::Generate")
                .into_iter();
            let rest_value = rest_value
                .map(|value| value.expect("a value should be generated with Mode::Generate"));
            let struct_ident = self.render_ident(&id);
            quote! { #struct_ident( #( #field_values, )* #rest_value ) }
        }))
    }

    fn default_impl_unit_struct(
        &self,
        unit_struct: &build::UnitStruct,
        value: &serde_json::Value,
        id: &Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        if value == &unit_struct.repr {
            Ok(self.generate(|| self.render_ident(&id)))
        } else {
            Err(Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("unit struct default value must be {}", unit_struct.repr),
            })
        }
    }

    fn default_impl_custom_optional_nullable(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        id: &Id,
        value: &serde_json::Value,
        _type_name: &str,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        if value.is_null() {
            Ok(self.generate(|| {
                quote! {
                    // TODO 9/4/2026
                    // Create the null value.
                    todo!()
                }
            }))
        } else {
            Ok(self
                .default_impl(expansion_set, id, value)?
                .map(|_value_stream| {
                    quote! {
                        // TODO 9/4/2026
                        // Create the typed value
                        todo!()
                    }
                }))
        }
    }

    /// Guard against cycles for situations where default generation may
    /// expand--or insufficiently narrow--the input value.
    fn expansion_guard_default_impl(
        &self,
        expansion_set: &mut Vec<(Id, serde_json::Value)>,
        id: &Id,
        value: &serde_json::Value,
    ) -> Result<Option<TokenStream>, Error<Id>>
    where
        Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    {
        let key = (id.clone(), value.clone());
        if expansion_set.contains(&key) {
            let (id, value) = key;
            return Err(Error::InvalidDefault {
                value,
                id,
                reason: "property default value is recursive".to_string(),
            });
        }
        expansion_set.push(key);
        let result = self.default_impl(expansion_set, id, value);
        expansion_set.pop();

        result
    }
}

#[cfg(test)]
mod tests {

    use quote::ToTokens;
    use typespace_test_macro::typespace_builder;

    use super::*;
    use crate::settings::Std;

    /// Re-tokenize an expression so that a `>>` written in source and
    /// the two `>` a generated stream carries compare equal.
    fn reparse(tokens: &str) -> String {
        syn::parse_str::<syn::Expr>(tokens)
            .unwrap()
            .into_token_stream()
            .to_string()
    }

    /// Unparse a helper definition the way the snapshots render it.
    fn unparse(helper: DefaultHelper) -> String {
        let file = syn::parse2::<syn::File>(helper.definition()).unwrap();
        prettyplease::unparse(&file)
    }

    /// Run both halves of the walk, returning the generated tokens.
    fn walk(
        types: &BTreeMap<String, Type<String>>,
        settings: &Settings,
        id: &str,
        value: &serde_json::Value,
    ) -> String {
        check_default(types, settings, value, &id.to_string()).unwrap();
        generate_default(types, settings, value, &id.to_string()).to_string()
    }

    /// Run only the check half, returning the error it produced.
    fn reject(
        types: &BTreeMap<String, Type<String>>,
        settings: &Settings,
        id: &str,
        value: &serde_json::Value,
    ) -> Error<String> {
        check_default(types, settings, value, &id.to_string()).unwrap_err()
    }

    #[test]
    fn test_null() {
        let mut types = BTreeMap::new();
        let id = "unit".to_string();
        types.insert(id.clone(), Type::<String>::Unit);

        let value = serde_json::json! { null };
        let settings = Settings::minimal();

        check_default(&types, &settings, &value, &id).unwrap();
        let code = generate_default(&types, &settings, &value, &id);

        assert_eq!(code.to_string(), "()");
    }

    /// A native type's value goes through `from_str`, not `from_value`.
    #[test]
    fn test_native() {
        let mut types = BTreeMap::new();
        types.insert(
            "date".to_string(),
            Type::Native(build::Native::new_string_like("::chrono::NaiveDate")),
        );
        let settings = Settings::minimal();
        let value = serde_json::json! { "1970-01-01" };

        assert_eq!(
            walk(&types, &settings, "date", &value),
            quote! {
                ::serde_json::from_str::<::chrono::NaiveDate>("\"1970-01-01\"").unwrap()
            }
            .to_string()
        );
    }

    /// A native type accepts any value; only the generated code can tell.
    #[test]
    fn test_native_accepts_anything() {
        let mut types = BTreeMap::new();
        types.insert(
            "date".to_string(),
            Type::Native(build::Native::new_string_like("::chrono::NaiveDate")),
        );
        let settings = Settings::minimal();
        let value = serde_json::json! { { "not": "a date" } };

        assert_eq!(
            walk(&types, &settings, "date", &value),
            quote! {
                ::serde_json::from_str::<::chrono::NaiveDate>("{\"not\":\"a date\"}").unwrap()
            }
            .to_string()
        );
    }

    /// A JSON value's default is its compact text, deserialized.
    #[test]
    fn test_json_value() {
        let mut types = BTreeMap::new();
        types.insert("any".to_string(), Type::<String>::JsonValue);
        let settings = Settings::minimal();
        let value = serde_json::json! { [8, 6, 7] };

        assert_eq!(
            walk(&types, &settings, "any", &value),
            quote! {
                ::serde_json::from_str::<::serde_json::Value>("[8,6,7]").unwrap()
            }
            .to_string()
        );
    }

    /// A float's default is a suffixed numeric literal.
    #[test]
    fn test_float() {
        let mut types = BTreeMap::new();
        types.insert("f".to_string(), Type::<String>::Float("f64".to_string()));
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "f", &serde_json::json! { 1.5 }),
            quote! { 1.5_f64 }.to_string()
        );
        assert_eq!(
            walk(&types, &settings, "f", &serde_json::json! { 7 }),
            quote! { 7_f64 }.to_string()
        );
    }

    /// A float rejects a value that is not a number.
    #[test]
    fn test_float_rejects_non_number() {
        let mut types = BTreeMap::new();
        types.insert("f".to_string(), Type::<String>::Float("f64".to_string()));
        let settings = Settings::minimal();

        assert!(matches!(
            reject(&types, &settings, "f", &serde_json::json! { "1.5" }),
            Error::InvalidDefault { .. }
        ));
    }

    /// A `NonZero` integer is built through `new`, not written as a literal.
    #[test]
    fn test_integer_non_zero() {
        let mut types = BTreeMap::new();
        types.insert(
            "nz".to_string(),
            Type::<String>::Integer("::std::num::NonZeroU64".to_string()),
        );
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "nz", &serde_json::json! { 1 }),
            quote! { ::std::num::NonZeroU64::new(1).unwrap() }.to_string()
        );
    }

    /// A type alias contributes nothing; its target answers.
    #[test]
    fn test_type_alias() {
        let mut types = BTreeMap::new();
        types.insert(
            "u32".to_string(),
            Type::<String>::Integer("u32".to_string()),
        );
        types.insert(
            "Count".to_string(),
            build::TypeAlias::new("u32".to_string())
                .name("Count")
                .build()
                .unwrap(),
        );
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "Count", &serde_json::json! { 7 }),
            quote! { 7_u32 }.to_string()
        );
        assert!(matches!(
            reject(&types, &settings, "Count", &serde_json::json! { "7" }),
            Error::InvalidDefault { .. }
        ));
    }

    /// A newtype struct wraps its inner value in its own scoped name.
    #[test]
    fn test_newtype_struct() {
        let mut types = BTreeMap::new();
        types.insert(
            "i64".to_string(),
            Type::<String>::Integer("i64".to_string()),
        );
        types.insert(
            "UInt".to_string(),
            build::NewtypeStruct::new("i64".to_string())
                .name("UInt")
                .build()
                .unwrap(),
        );
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "UInt", &serde_json::json! { 1 }),
            quote! { super::UInt(1_i64) }.to_string()
        );
        assert!(matches!(
            reject(&types, &settings, "UInt", &serde_json::json! { "1" }),
            Error::InvalidDefault { .. }
        ));
    }

    /// A unit struct's value expression is its own scoped name.
    #[test]
    fn test_unit_struct() {
        let mut types = BTreeMap::new();
        types.insert(
            "Marker".to_string(),
            build::UnitStruct::new(serde_json::json! { "<<+>>" })
                .name("Marker")
                .build()
                .unwrap(),
        );
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "Marker", &serde_json::json! { "<<+>>" }),
            quote! { super::Marker }.to_string()
        );
        assert!(matches!(
            reject(&types, &settings, "Marker", &serde_json::json! { "other" }),
            Error::InvalidDefault { .. }
        ));
    }

    /// `Option` follows the configured `Std` syntax in both cases.
    #[test]
    fn test_option_qualification() {
        let mut types = BTreeMap::new();
        types.insert(
            "u32".to_string(),
            Type::<String>::Integer("u32".to_string()),
        );
        types.insert(
            "MaybeU32".to_string(),
            Type::<String>::Option("u32".to_string()),
        );

        let settings = Settings::minimal();
        assert_eq!(
            walk(&types, &settings, "MaybeU32", &serde_json::json! { 1 }),
            quote! { ::std::option::Option::Some(1_u32) }.to_string()
        );
        assert_eq!(
            walk(&types, &settings, "MaybeU32", &serde_json::json! { null }),
            quote! { ::std::option::Option::None }.to_string()
        );

        let settings = Settings::minimal().with_std(Std::Unqualified);
        assert_eq!(
            walk(&types, &settings, "MaybeU32", &serde_json::json! { 1 }),
            quote! { Some(1_u32) }.to_string()
        );
        assert_eq!(
            walk(&types, &settings, "MaybeU32", &serde_json::json! { null }),
            quote! { None }.to_string()
        );
    }

    /// An optional `NonZero` default, the shape typify 1's goldens show.
    #[test]
    fn test_option_of_non_zero() {
        let mut types = BTreeMap::new();
        types.insert(
            "nz".to_string(),
            Type::<String>::Integer("::std::num::NonZeroU64".to_string()),
        );
        types.insert(
            "MaybeNz".to_string(),
            Type::<String>::Option("nz".to_string()),
        );
        let settings = Settings::minimal();

        assert_eq!(
            walk(&types, &settings, "MaybeNz", &serde_json::json! { 1 }),
            quote! {
                ::std::option::Option::Some(::std::num::NonZeroU64::new(1).unwrap())
            }
            .to_string()
        );
    }

    /// A named type inside a native type's parameters is scoped too.
    #[test]
    fn test_scope_reaches_native_parameters() {
        let mut types = BTreeMap::new();
        types.insert(
            "i64".to_string(),
            Type::<String>::Integer("i64".to_string()),
        );
        types.insert(
            "UInt".to_string(),
            build::NewtypeStruct::new("i64".to_string())
                .name("UInt")
                .build()
                .unwrap(),
        );
        types.insert(
            "Wrapper".to_string(),
            Type::Native(build::Native::new(
                "::foo::Wrapper",
                Default::default(),
                vec!["UInt".to_string()],
            )),
        );
        let settings = Settings::minimal();

        assert_eq!(
            reparse(&walk(
                &types,
                &settings,
                "Wrapper",
                &serde_json::json! { 1 }
            )),
            reparse(
                &quote! {
                    ::serde_json::from_str::<::foo::Wrapper<super::UInt>>("1").unwrap()
                }
                .to_string()
            )
        );
    }

    #[test]
    fn test_struct_default_with_optional_field() {
        let builder = typespace_builder!(Settings::minimal(), {
            #[default = { a: 1, b: 2}]
            struct Test {
                a: u32,
                b: Optional<u32>,
                c: Optional<u32>,
            }
        });

        let types = builder.types;

        let settings = Settings::minimal().with_std(Std::Unqualified);

        assert_eq!(
            reparse(&walk(
                &types,
                &settings,
                "Test",
                &serde_json::json! { { "a": 1, "b": 2}}
            )),
            reparse(
                &quote! {
                   super::Test {
                       a: 1_u32,
                       b: Some(2_u32),
                       c: Default::default(),
                   }
                }
                .to_string()
            )
        );
    }

    /// Types to route default values against, one per node kind that
    /// the routing distinguishes.
    fn routing_types() -> BTreeMap<String, Type<String>> {
        [
            ("bool".to_string(), Type::Boolean),
            ("u32".to_string(), Type::Integer("u32".to_string())),
            ("i32".to_string(), Type::Integer("i32".to_string())),
            (
                "nzu8".to_string(),
                Type::Integer("::std::num::NonZeroU8".to_string()),
            ),
            (
                "nzi32".to_string(),
                Type::Integer("::std::num::NonZeroI32".to_string()),
            ),
            ("f64".to_string(), Type::Float("f64".to_string())),
            ("String".to_string(), Type::String),
            ("json".to_string(), Type::JsonValue),
            ("unit".to_string(), Type::Unit),
            ("opt".to_string(), Type::Option("u32".to_string())),
        ]
        .into_iter()
        .collect()
    }

    /// The path that instantiates the shared function for `value` at
    /// `id`, or `None` where the property needs a function of its own.
    fn shared_path(
        types: &BTreeMap<String, Type<String>>,
        id: &str,
        value: serde_json::Value,
    ) -> Option<String> {
        shared_default_fn(types, &id.to_string(), &value).map(|shared| shared.path)
    }

    /// Which default values a shared function produces, matching what
    /// typify 1's `default_fn` (typify-impl/src/defaults.rs) selects.
    ///
    /// Only the property type's own node is consulted, so a named type
    /// over an integer takes a function of its own; the `count: Count`
    /// property of the `test_default_value_property_kinds` render test
    /// covers that.
    #[test]
    fn test_shared_default_fn_routing() {
        let types = routing_types();

        // A boolean is produced by default_bool, whichever value it is.
        // typify 1 reaches the helper only for `true`, since it treats
        // a `false` default as the intrinsic Default; typespace leaves
        // that choice to its consumer and renders whatever state the
        // property carries.
        assert_eq!(
            shared_path(&types, "bool", serde_json::json!(true)).as_deref(),
            Some("defaults::default_bool::<true>"),
        );
        assert_eq!(
            shared_path(&types, "bool", serde_json::json!(false)).as_deref(),
            Some("defaults::default_bool::<false>"),
        );

        // An integer is produced from a u64 when the value is one, and
        // from an i64 otherwise. Zero is no exception here: typify 1
        // treats a zero default as the intrinsic Default before it gets
        // this far.
        assert_eq!(
            shared_path(&types, "u32", serde_json::json!(7)).as_deref(),
            Some("defaults::default_u64::<u32, 7>"),
        );
        assert_eq!(
            shared_path(&types, "u32", serde_json::json!(0)).as_deref(),
            Some("defaults::default_u64::<u32, 0>"),
        );
        assert_eq!(
            shared_path(&types, "i32", serde_json::json!(-3)).as_deref(),
            Some("defaults::default_i64::<i32, -3>"),
        );

        // A NonZero type converts from a NonZeroU64.
        assert_eq!(
            shared_path(&types, "nzu8", serde_json::json!(2)).as_deref(),
            Some("defaults::default_nzu64::<::std::num::NonZeroU8, 2>"),
        );

        // A negative value for a signed NonZero type has no shared
        // function: nothing converts a NonZero from an i64.
        assert_eq!(shared_path(&types, "nzi32", serde_json::json!(-3)), None);

        // A number that is neither a u64 nor an i64 has none either,
        // even for an integer type, which the walk admits.
        assert_eq!(shared_path(&types, "u32", serde_json::json!(1.5)), None);

        // No other type has one.
        assert_eq!(shared_path(&types, "f64", serde_json::json!(1.5)), None);
        assert_eq!(shared_path(&types, "String", serde_json::json!("hi")), None);
        assert_eq!(shared_path(&types, "json", serde_json::json!({})), None);
        assert_eq!(shared_path(&types, "unit", serde_json::json!(null)), None);
        assert_eq!(shared_path(&types, "opt", serde_json::json!(5)), None);
    }

    /// The shared helpers reproduce typify 1's, whose output they have
    /// to match: `default_bool` appears in typify 1's github.out and
    /// vega.out goldens, `default_i64` in vega.out, and `default_nzu64`
    /// in its types-with-defaults.rs golden. No typify 1 golden
    /// contains `default_u64`, so that one is pinned against the source
    /// of `impl From<&DefaultImpl> for TokenStream` in typify 1's
    /// typify-impl/src/defaults.rs.
    #[test]
    fn test_shared_default_fn_definitions() {
        assert_eq!(
            unparse(DefaultHelper::Boolean),
            "pub(super) fn default_bool<const V: bool>() -> bool {\n    V\n}\n",
        );
        assert_eq!(
            unparse(DefaultHelper::I64),
            concat!(
                "pub(super) fn default_i64<T, const V: i64>() -> T\n",
                "where\n",
                "    T: ::std::convert::TryFrom<i64>,\n",
                "    <T as ::std::convert::TryFrom<i64>>::Error: ::std::fmt::Debug,\n",
                "{\n",
                "    T::try_from(V).unwrap()\n",
                "}\n",
            ),
        );
        assert_eq!(
            unparse(DefaultHelper::U64),
            concat!(
                "pub(super) fn default_u64<T, const V: u64>() -> T\n",
                "where\n",
                "    T: ::std::convert::TryFrom<u64>,\n",
                "    <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,\n",
                "{\n",
                "    T::try_from(V).unwrap()\n",
                "}\n",
            ),
        );
        assert_eq!(
            unparse(DefaultHelper::NZU64),
            concat!(
                "pub(super) fn default_nzu64<T, const V: u64>() -> T\n",
                "where\n",
                "    T: ::std::convert::TryFrom<::std::num::NonZeroU64>,\n",
                "    <T as ::std::convert::TryFrom<::std::num::NonZeroU64>>::Error: ::std::fmt::Debug,\n",
                "{\n",
                "    T::try_from(::std::num::NonZeroU64::try_from(V).unwrap()).unwrap()\n",
                "}\n",
            ),
        );
    }
}

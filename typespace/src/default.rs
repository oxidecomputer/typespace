// Copyright 2026 Oxide Computer Company

use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    TypespaceRenderer,
    build::{self, StructPropertySerde, StructPropertyState, Type},
    error::Error,
    settings::{OptionalNullable, Settings, Std},
};

/// Prefix shared by the `::std::num::NonZero*` type paths.
///
/// A default value for one of these is built through `new()` rather
/// than written as a literal, since there is no literal form for them.
const STD_NUM_NONZERO_PREFIX: &str = "::std::num::NonZero";

pub(crate) fn check_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    value: &serde_json::Value,
    id: Id,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let imp = DefaultImpl {
        types,
        settings,
        scope: None,
        mode: Mode::Check,
    };
    imp.default_impl(id, value).map(|_| ())
}

pub(crate) fn generate_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    value: &serde_json::Value,
    id: Id,
) -> TokenStream
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let imp = DefaultImpl {
        types,
        settings,
        scope: Some("super"),
        mode: Mode::Generate,
    };
    imp.default_impl(id, value)
        .expect("an error should not be possible post-validation")
        .expect("a value should be generated with Mode::Generate")
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Check,
    Generate,
}
impl Mode {
    fn then<F>(&self, generate: F) -> Option<TokenStream>
    where
        F: FnOnce() -> TokenStream,
    {
        match self {
            Mode::Check => None,
            Mode::Generate => Some(generate()),
        }
    }
}

/// Holds the shared context threaded through default-value processing.
struct DefaultImpl<'a, Id> {
    types: &'a BTreeMap<Id, Type<Id>>,
    settings: &'a Settings,
    /// Scope to find generated types. Generated default functions live in the
    /// `defaults` module; to reach generated types it typically needs to
    /// prepend `super::`.
    scope: Option<&'a str>,
    mode: Mode,
}

impl<Id> DefaultImpl<'_, Id>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
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
        id: Id,
        value: &serde_json::Value,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let ty = self.types.get(&id).unwrap();
        match ty {
            Type::Enum(enum_info) => self.default_impl_enum(enum_info, value, id),
            Type::Struct(struct_info) => self.default_impl_struct(struct_info, value, id),
            Type::UnitStruct(unit_struct) => self.default_impl_unit_struct(unit_struct, value, id),
            Type::TupleStruct(tuple_struct) => {
                self.default_impl_tuple_struct(tuple_struct, value, id)
            }
            Type::NewtypeStruct(newtype_struct) => {
                // TODO 9/4/2026
                // if mode = validate we need to check the value against
                // constraints for the newtype

                let inner = self.default_impl(newtype_struct.inner.clone(), value)?;
                Ok(inner.map(|inner| {
                    let ident = self.render_ident(&id);
                    quote! { #ident(#inner) }
                }))
            }
            Type::TypeAlias(type_alias) => self.default_impl(type_alias.target.clone(), value),
            Type::Native(_) => {
                // A native type's value is whatever its own Deserialize
                // accepts, which we have no way to check here; a value that
                // doesn't fit fails the unwrap() in the generated code.

                // TODO 9/4/2026
                // expect rather than unwrap?
                let text = value.to_string();
                Ok(self.mode.then(|| {
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
                    Ok(self.mode.then(|| self.render_option_variant(&id, "None")))
                } else {
                    let inner = self.default_impl(type_id.clone(), value)?;
                    Ok(inner.map(|inner| {
                        let some = self.render_option_variant(&id, "Some");
                        quote! { #some(#inner) }
                    }))
                }
            }
            Type::Box(type_id) => {
                let inner = self.default_impl(type_id.clone(), value)?;
                // TODO 9/4/2026
                // We need Settings to know what to render here...
                Ok(inner.map(|inner| quote! { Box::new(#inner) }))
            }
            Type::Vec(_) => todo!(),
            Type::Map(_, _) => todo!(),
            Type::Set(_) => todo!(),
            Type::Array(_, _) => todo!(),
            Type::Tuple(items) => todo!(),

            Type::Unit => {
                if value.is_null() {
                    Ok(self.mode.then(|| quote! { () }))
                } else {
                    Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "unit type default value must be null".to_string(),
                    })
                }
            }

            Type::Boolean => {
                if let Some(v) = value.as_bool() {
                    Ok(self.mode.then(|| quote! { #v }))
                } else {
                    Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected a boolean".to_string(),
                    })
                }
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

                    Ok(self.mode.then(|| {
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
                    Ok(self
                        .mode
                        .then(|| TokenStream::from(proc_macro2::TokenTree::from(val))))
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
                Ok(self
                    .mode
                    .then(|| TokenStream::from(proc_macro2::TokenTree::from(val))))
            }
            Type::String => {
                if let Some(s) = value.as_str() {
                    Ok(self.mode.then(|| quote! { #s.to_string()}))
                } else {
                    Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: "expected a string".to_string(),
                    })
                }
            }
            Type::JsonValue => {
                let text = value.to_string();
                Ok(self.mode.then(|| {
                    quote! {
                        ::serde_json::from_str::<::serde_json::Value>(#text).unwrap()
                    }
                }))
            }
            Type::Never => todo!(),
        }
    }

    fn default_impl_struct(
        &self,
        struct_info: &build::Struct<Id>,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let map = value.as_object().ok_or_else(|| Error::InvalidDefault {
            value: value.clone(),
            id: id.clone(),
            reason: "expected JSON object".to_string(),
        })?;

        // Examine local (non-flattened) properties first, then descend into
        // flattened properties with the value. If deny_unknown_fields is set,
        // things may be weird.

        let (local_properties, flattened_properties): (Vec<_>, Vec<_>) = struct_info
            .properties
            .iter()
            .partition(|prop| !matches!(prop.json_name, StructPropertySerde::Flatten));

        let local_properties = local_properties
            .into_iter()
            .map(|prop| {
                let name = match &prop.json_name {
                    StructPropertySerde::None => prop.rust_name.as_str(),
                    StructPropertySerde::Rename(rename) => rename.as_str(),
                    StructPropertySerde::Flatten => unreachable!(),
                };
                (name, prop)
            })
            .collect::<BTreeMap<_, _>>();

        let keys = local_properties
            .keys()
            .map(|k| *k)
            .chain(map.keys().map(|k| k.as_str()))
            .collect::<BTreeSet<_>>();

        let mut extra_keys = Vec::new();
        let mut rendered_properties = Vec::new();

        for key in keys {
            let prop_info = local_properties.get(key);
            let prop_value = map.get(key);

            match (prop_info, prop_value) {
                (None, None) => unreachable!(),
                (None, Some(_)) => extra_keys.push(key),
                (Some(prop_info), None) => {
                    if prop_info.state != StructPropertyState::Optional {
                        return Err(Error::InvalidDefault {
                            value: value.clone(),
                            id: id.clone(),
                            reason: format!("missing required property {}", key),
                        });
                    } else if self.mode == Mode::Generate {
                        // TODO 9/4/2026
                        // Qualify default
                        let prop_ident = format_ident!("{}", prop_info.rust_name);
                        rendered_properties.push(quote! {
                            #prop_ident: Default::default()
                        });
                    }
                }
                (Some(prop_info), Some(prop_value)) => {
                    let prop_id = &prop_info.type_id;

                    let prop_ty = self.types.get(prop_id).unwrap();
                    let is_option = matches!(prop_ty, Type::Option(_));

                    let prop_default_value = match (&prop_info.state, is_option) {
                        // Optional field with an Option type.
                        (StructPropertyState::Optional, true) => {
                            match &self.settings.optional_nullable {
                                // A simple Option<T> is sufficient.
                                OptionalNullable::ConflateAsAbsent
                                | OptionalNullable::ConflateAsNull => {
                                    self.default_impl(prop_id.clone(), prop_value)?
                                }
                                // Nest the option in a `Some`.
                                OptionalNullable::DoubleOption => self
                                    .default_impl(prop_id.clone(), prop_value)?
                                    .map(|prop_value| {
                                        let some = self.render_option_variant2("Some");
                                        quote! { #some(#prop_value) }
                                    }),

                                // Construct the custom type
                                OptionalNullable::CustomType(type_name) => self
                                    .default_impl_custom_optional_nullable(
                                        prop_id, prop_value, type_name,
                                    )?,
                            }
                        }

                        // Optional field with a non-Option type.
                        (StructPropertyState::Optional, false) => self
                            .default_impl(prop_id.clone(), prop_value)?
                            .map(|prop_value| {
                                let some = self.render_option_variant2("Some");
                                quote! { #some(#prop_value) }
                            }),

                        // Non-optional field, and we don't care about the
                        // type.
                        _ => self.default_impl(prop_id.clone(), prop_value)?,
                    };

                    let prop_default = prop_default_value.map(|value| {
                        let prop_ident = format_ident!("{}", prop_info.rust_name);
                        quote! { #prop_ident: #value}
                    });

                    if self.mode == Mode::Generate {
                        rendered_properties.push(
                            prop_default.expect("a value should be generated with Mode::Generate"),
                        );
                    }
                }
            }
        }

        assert!(extra_keys.is_empty());
        assert!(flattened_properties.is_empty());

        Ok(self.mode.then(|| {
            let struct_ident = format_ident!("{}", struct_info.common.name.as_ref().unwrap());
            quote! {
                #struct_ident {
                    #( #rendered_properties, )*
                }
            }
        }))
    }

    fn default_impl_enum(
        &self,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        let type_name = enum_info.common.name.as_ref().unwrap();
        match enum_info.tag_type.as_ref().unwrap() {
            build::EnumTagType::External => {
                self.default_impl_enum_external(type_name, enum_info, value, id)
            }
            build::EnumTagType::Internal { tag } => {
                self.default_impl_enum_internal(enum_info, tag, value, id)
            }
            build::EnumTagType::Adjacent { tag, content } => {
                self.default_impl_enum_adjacent(enum_info, tag, content, value, id)
            }
            build::EnumTagType::Untagged => self.default_impl_enum_untagged(enum_info, value, id),
        }
    }

    fn default_impl_enum_external(
        &self,
        type_name: &str,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
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

            let var_ident = format_ident!("{}", variant.rust_name);
            let type_ident = format_ident!("{}", type_name);
            Ok(self.mode.then(|| quote! { #type_ident::#var_ident }))
        } else {
            todo!()
        }
    }

    fn default_impl_enum_internal(
        &self,
        enum_info: &build::Enum<Id>,
        tag: &str,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        todo!()
    }

    fn default_impl_enum_adjacent(
        &self,
        enum_info: &build::Enum<Id>,
        tag: &str,
        content: &str,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        todo!()
    }

    fn default_impl_enum_untagged(
        &self,
        enum_info: &build::Enum<Id>,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        todo!()
    }

    fn default_impl_tuple_struct(
        &self,
        tuple_struct: &build::TupleStruct<Id>,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        todo!()
    }

    fn default_impl_unit_struct(
        &self,
        unit_struct: &build::UnitStruct,
        value: &serde_json::Value,
        id: Id,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        if value == &unit_struct.repr {
            Ok(self.mode.then(|| self.render_ident(&id)))
        } else {
            Err(Error::InvalidDefault {
                value: value.clone(),
                id,
                reason: format!("unit struct default value must be {}", unit_struct.repr),
            })
        }
    }

    fn default_impl_custom_optional_nullable(
        &self,
        id: &Id,
        value: &serde_json::Value,
        _type_name: &str,
    ) -> Result<Option<TokenStream>, Error<Id>> {
        if value.is_null() {
            Ok(self.mode.then(|| {
                quote! {
                    // TODO 9/4/2026
                    // Create the null value.
                    todo!()
                }
            }))
        } else {
            Ok(self.default_impl(id.clone(), value)?.map(|_value_stream| {
                quote! {
                    // TODO 9/4/2026
                    // Create the typed value
                    todo!()
                }
            }))
        }
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

    /// Run both halves of the walk, returning the generated tokens.
    fn walk(
        types: &BTreeMap<String, Type<String>>,
        settings: &Settings,
        id: &str,
        value: &serde_json::Value,
    ) -> String {
        check_default(types, settings, value, id.to_string()).unwrap();
        generate_default(types, settings, value, id.to_string()).to_string()
    }

    /// Run only the check half, returning the error it produced.
    fn reject(
        types: &BTreeMap<String, Type<String>>,
        settings: &Settings,
        id: &str,
        value: &serde_json::Value,
    ) -> Error<String> {
        check_default(types, settings, value, id.to_string()).unwrap_err()
    }

    #[test]
    fn test_null() {
        let mut types = BTreeMap::new();
        let id = "unit".to_string();
        types.insert(id.clone(), Type::<String>::Unit);

        let value = serde_json::json! { null };
        let settings = Settings::minimal();

        check_default(&types, &settings, &value, id.clone()).unwrap();
        let code = generate_default(&types, &settings, &value, id);

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
                   Test {
                       a: 1_u32,
                       b: Some(2_u32),
                       c: Default::default(),
                   }
                }
                .to_string()
            )
        );
    }
}

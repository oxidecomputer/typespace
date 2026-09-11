// Copyright 2026 Oxide Computer Company

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::build::{JsonValue, Type, TypeCommon, TypeCommonBuilt, validate_ident};
use crate::error::{Error, NameAxis};
use crate::output::Outputspace;
use crate::serde_attrs::SerdeDerives;
use crate::{
    DefaultConstructor, RenderedStructProperty, TypespaceBuilder, TypespaceRenderer, TypespaceTrait,
};

/// A struct with named fields.
///
/// A `Struct` is its own builder: [`Struct::new`] starts one under
/// construction, the fluent methods fill it in ([`Struct::name`] may
/// come at any point), and [`Struct::build`] validates it and produces
/// the finished [`Type::Struct`] value.
#[derive(Debug, Clone)]
pub struct Struct<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) properties: Vec<StructProperty<Id>>,
    pub(crate) deny_unknown_fields: bool,
}

impl<Id> Default for Struct<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> Struct<Id> {
    /// Start a struct under construction.
    pub fn new() -> Self {
        Self {
            common: Default::default(),
            properties: Vec::new(),
            deny_unknown_fields: false,
        }
    }

    /// Set the struct's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.common.name = Some(name.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.common.description = Some(description.into());
        self
    }

    /// Set the default value.
    pub fn default(mut self, default: impl Into<JsonValue>) -> Self {
        self.common.default = Some(default.into());
        self
    }

    /// Add opaque derive paths applied to this type alone.
    ///
    /// These are additional to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive);
    /// a type's derive attribute names both sets. Each path is emitted
    /// verbatim, with the same caveats `with_derive` documents.
    pub fn extra_derives(mut self, derives: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_derives
            .extend(derives.into_iter().map(Into::into));
        self
    }

    /// Add opaque attributes applied to this type alone.
    ///
    /// These are additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn extra_attrs(mut self, attrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_attrs
            .extend(attrs.into_iter().map(Into::into));
        self
    }

    /// Append properties.
    pub fn properties(mut self, properties: impl IntoIterator<Item = StructProperty<Id>>) -> Self {
        self.properties.extend(properties);
        self
    }

    /// Make deserialization reject unknown fields.
    pub fn deny_unknown_fields(mut self) -> Self {
        self.deny_unknown_fields = true;
        self
    }

    /// Validate the struct and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier, if any
    /// property name is not a valid identifier, or if two properties
    /// collide on either name axis (see
    /// [`Error::DuplicateItemName`]).
    pub fn build(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::Struct(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("struct")?;
        check_properties(self.common.built_name(), &self.properties)
    }

    pub(crate) fn check_field_defaults(
        &self,
        typespace: &TypespaceBuilder<Id>,
    ) -> Result<BTreeSet<Id>, Error<Id>>
    where
        Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    {
        self.properties
            .iter()
            .try_fold(BTreeSet::new(), |mut natives, prop| {
                natives.extend(prop.check_defaults(typespace)?);
                Ok(natives)
            })
    }

    /// The struct's name, if one has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn get_description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn get_default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The opaque derive paths applied to this type alone, additional
    /// to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive).
    pub fn get_extra_derives(&self) -> &[String] {
        self.common.extra_derives()
    }

    /// The opaque attributes applied to this type alone, additional to
    /// the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn get_extra_attrs(&self) -> &[String] {
        self.common.extra_attrs()
    }

    /// The struct's properties, in declaration order.
    pub fn get_properties(&self) -> &[StructProperty<Id>] {
        &self.properties
    }

    /// Whether deserialization rejects unknown fields.
    pub fn get_deny_unknown_fields(&self) -> bool {
        self.deny_unknown_fields
    }
}

/// Check property names for validity and uniqueness.
///
/// Every Rust name must be a valid identifier, and names must be unique
/// on both axes: the Rust name and the wire name (the serialized name
/// after any rename). Flattened properties have no wire name of their
/// own and are exempt from the wire axis.
pub(crate) fn check_properties<Id>(
    type_name: &str,
    properties: &[StructProperty<Id>],
) -> Result<(), Error<Id>>
where
    Id: std::fmt::Debug + std::fmt::Display,
{
    let mut rust_names = BTreeSet::new();
    let mut wire_names = BTreeSet::new();
    for property in properties {
        let rust_name = property.rust_name.clone();
        validate_ident("property", &rust_name)?;
        let wire_name = property.wire_name().map(str::to_string);
        if !rust_names.insert(rust_name.clone()) {
            return Err(Error::DuplicateItemName {
                kind: "property",
                type_name: type_name.to_string(),
                name: rust_name,
                axis: NameAxis::Rust,
            });
        }
        if let Some(wire_name) = wire_name
            && !wire_names.insert(wire_name.clone())
        {
            return Err(Error::DuplicateItemName {
                kind: "property",
                type_name: type_name.to_string(),
                name: wire_name,
                axis: NameAxis::Wire,
            });
        }
    }
    Ok(())
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Struct<Id> {
    pub(crate) fn render(
        &self,
        id: &Id,
        typespace: &TypespaceRenderer<'_, Id>,
        out: &mut Outputspace,
    ) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default,
                    built:
                        Some(TypeCommonBuilt {
                            traits,
                            from_string_irrefutable: _,
                        }),
                    extra_derives,
                    extra_attrs,
                },
            properties,
            deny_unknown_fields,
        } = self
        else {
            unreachable!()
        };
        let name = name.as_deref().expect("validated type has a name");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });
        let name_ident = format_ident!("{name}");

        let mut traits = traits.clone();
        let serde_derives = SerdeDerives::new(&traits);

        let rendered_properties = properties
            .iter()
            .map(|prop| typespace.render_struct_property(prop, serde_derives, true, name, out))
            .collect::<Vec<_>>();

        if typespace.has_builder(id) {
            // TODO 9/1/2026
            // for compat: some of these are unqualified and some are fully
            // qualified; resolve.

            let prop_ident = rendered_properties
                .iter()
                .map(
                    |RenderedStructProperty {
                         rust_name_ident, ..
                     }| rust_name_ident,
                )
                .collect::<Vec<_>>();
            let prop_error = rendered_properties.iter().map(
                |RenderedStructProperty {
                     rust_name_ident, ..
                 }| {
                    format!(
                        "error converting supplied value for {}: {{e}}",
                        rust_name_ident
                    )
                },
            );
            let prop_ty_ident_scoped = rendered_properties
                .iter()
                .map(
                    |RenderedStructProperty {
                         prop_ty_ident_scoped,
                         ..
                     }| prop_ty_ident_scoped,
                )
                .collect::<Vec<_>>();
            let prop_default_value = rendered_properties.iter().map(
                |RenderedStructProperty {
                     rust_name_ident,
                     default,
                     ..
                 }| match default {
                    DefaultConstructor::None => {
                        let msg = format!("no value supplied for {}", rust_name_ident);
                        quote! {
                            Err(#msg.to_string())
                        }
                    }
                    DefaultConstructor::Default => quote! { Ok(Default::default()) },
                    DefaultConstructor::Generated(default_expr) => {
                        quote! { Ok(super::#default_expr) }
                    }
                },
            );

            // The builder mod is a separate application of the canonical
            // item order (see build::mod) under its own item key: decl,
            // Default, setters, TryFrom<Builder> for Type, From<Type>
            // for Builder.
            let value_ident = if prop_ident.is_empty() {
                quote! { _value }
            } else {
                quote! { value }
            };

            let builder = quote! {
                #[derive(Clone, Debug)]
                pub struct #name_ident {
                    #(
                        #prop_ident: ::std::result::Result<
                            #prop_ty_ident_scoped,
                            ::std::string::String,
                        >,
                    )*
                }

                impl ::std::default::Default for #name_ident {
                    fn default() -> Self {
                        Self {
                            #(
                                #prop_ident: #prop_default_value,
                            )*
                        }
                    }
                }

                impl #name_ident {
                    #(
                        pub fn #prop_ident<T>(mut self, value: T) -> Self
                        where
                            T: ::std::convert::TryInto<#prop_ty_ident_scoped>,
                            T::Error: ::std::fmt::Display,
                        {
                            self.#prop_ident = value.try_into()
                                .map_err(|e| format!(#prop_error));
                            self
                        }
                    )*
                }

                impl ::std::convert::TryFrom<#name_ident>
                    for super::#name_ident
                {
                    type Error = super::error::ConversionError;

                    fn try_from(#value_ident: #name_ident)
                        -> ::std::result::Result<Self, super::error::ConversionError>
                    {
                        Ok(Self {
                            #(
                                #prop_ident: value.#prop_ident?,
                            )*
                        })
                    }
                }

                impl ::std::convert::From<super::#name_ident> for #name_ident {
                    fn from(#value_ident: super::#name_ident) -> Self {
                        Self {
                            #(
                                #prop_ident: Ok(value.#prop_ident),
                            )*
                        }
                    }
                }
            };

            out.cs()
                .get_root_mod()
                .get_mod("builder")
                .add_item(name, builder);
            typespace.add_error_mod(out);
        }

        let builder_impl = typespace
            .render_builder_ident(id, None)
            .map(|builder_ident| {
                quote! {
                    impl #name_ident {
                        pub fn builder() -> #builder_ident {
                            // TODO 9/1/2026
                            // TYPIFY COMPAT: Add std scope
                            Default::default()
                        }
                    }
                }
            });

        let default_impl = traits.contains(&TypespaceTrait::Default).then(|| {
            // If there's no whole-type default value and every property's
            // default is the intrinsic `Default::default()`, the hand-written
            // `impl Default` would be exactly what `#[derive(Default)]`
            // produces (and would trip clippy's `derivable_impls` lint
            // downstream). In that case we derive `Default` rather than
            // emitting the manual impl below.
            //
            // TODO 9/4/2026
            // Default... or if it's optional? Not sure how typify handles
            // this.
            if default.is_none()
                && rendered_properties
                    .iter()
                    .all(|prop| matches!(&prop.default, DefaultConstructor::Default))
            {
                return Default::default();
            }

            traits.remove(TypespaceTrait::Default);

            if let Some(JsonValue(default_value)) = default {
                let body = typespace.generate_default_value_for_impl(default_value, id);
                quote! {
                    impl ::std::default::Default for #name_ident {
                        fn default() -> Self {
                            #body
                        }
                    }
                }
            } else {
                let default_props = rendered_properties.iter().map(
                    |RenderedStructProperty {
                         rust_name_ident,
                         default,
                         ..
                     }| {
                        let default_value = match default {
                            DefaultConstructor::None => unreachable!(),
                            DefaultConstructor::Default => quote! { Default::default() },
                            DefaultConstructor::Generated(default_fn) => default_fn.clone(),
                        };
                        quote! {
                            #rust_name_ident: #default_value
                        }
                    },
                );

                quote! {
                    impl ::std::default::Default for #name_ident {
                        fn default() -> Self {
                            Self {
                                #( #default_props, )*
                            }
                        }
                    }
                }
            }
        });

        // An ordinary struct is neither of typify's comparison-derive
        // exceptions, so it is never exempt.
        let derive_attr = typespace.render_derives(&traits, extra_derives, false);
        let attrs = typespace.render_attrs(extra_attrs);

        let mut serde = serde_derives.attrs();
        // An unknown field is a deserialization concern, so this one is
        // held back from a Serialize-only type rather than left inert.
        if serde_derives.deserialize() && *deny_unknown_fields {
            serde.push(quote! { deny_unknown_fields });
        }

        // Canonical item order: see build::mod.
        quote! {
            #description
            #( #attrs )*
            #derive_attr
            #serde
            pub struct #name_ident {
                #( #rendered_properties, )*
            }

            #default_impl

            #builder_impl
        }
    }

    pub(crate) fn children(&self) -> Vec<Id> {
        self.properties
            .iter()
            .map(|StructProperty { type_id, .. }| type_id.clone())
            .collect()
    }
}

/// One named field of a [`Struct`] (or of a struct-shaped enum
/// variant).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct StructProperty<Id> {
    pub(crate) rust_name: String,
    pub(crate) json_name: StructPropertySerde,
    pub(crate) state: StructPropertyState,
    pub(crate) description: Option<String>,
    pub(crate) type_id: Id,
}

impl<Id> StructProperty<Id> {
    /// Create a property named `rust_name` whose type is `type_id`.
    ///
    /// The property starts [`StructPropertyState::Required`], with no
    /// serde renaming and no description; adjust with the `with_`
    /// methods. The name is validated--as a plain, non-keyword, non-raw
    /// Rust identifier--when the containing shape is built.
    pub fn new(rust_name: impl Into<String>, type_id: Id) -> Self {
        Self {
            rust_name: rust_name.into(),
            json_name: StructPropertySerde::None,
            state: StructPropertyState::Required,
            description: None,
            type_id,
        }
    }

    /// Set the property's volitionality.
    pub fn with_state(mut self, state: StructPropertyState) -> Self {
        self.state = state;
        self
    }

    /// Set the serde treatment of the property's name.
    pub fn with_json_name(mut self, json_name: StructPropertySerde) -> Self {
        self.json_name = json_name;
        self
    }

    /// Set the description (doc comment source).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The Rust field name.
    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    /// The serde treatment of the property's name.
    pub fn json_name(&self) -> &StructPropertySerde {
        &self.json_name
    }

    /// The name the property serializes under.
    ///
    /// The serde rename when there is one and the Rust name otherwise.
    /// A flattened property has no wire name of its own: its fields are
    /// spliced into the containing type's wire form.
    pub fn wire_name(&self) -> Option<&str> {
        match &self.json_name {
            StructPropertySerde::None => Some(self.rust_name.as_str()),
            StructPropertySerde::Rename(rename) => Some(rename.as_str()),
            StructPropertySerde::Flatten => None,
        }
    }

    /// The property's volitionality.
    pub fn state(&self) -> &StructPropertyState {
        &self.state
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The ID of the property's type.
    pub fn type_id(&self) -> &Id {
        &self.type_id
    }

    pub(crate) fn check_defaults(
        &self,
        typespace: &TypespaceBuilder<Id>,
    ) -> Result<BTreeSet<Id>, Error<Id>>
    where
        Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    {
        let StructPropertyState::DefaultValue(JsonValue(value)) = &self.state else {
            return Ok(BTreeSet::new());
        };

        typespace.check_default(value, &self.type_id)
    }
}

/// The serde treatment of a struct property's name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructPropertySerde {
    /// The property serializes under its Rust name.
    None,
    /// The property serializes under the given name instead.
    Rename(String),
    /// The property's own fields are flattened into the containing type's
    /// serialized form; see [the serde
    /// docs](https://serde.rs/attr-flatten.html).
    Flatten,
}

/// The volitionality of a struct property.
///
/// Only `Optional` translates into an `Option<T>` type; the others are
/// required in Rust. Conversely, only `Required` must be present during
/// deserialization; the others may be omitted. Note that the rendering of an
/// `Optional` property whose type is [`Type::Option`] is dictated by the
/// value of
/// [`Settings::optional_nullable`](crate::settings::Settings::optional_nullable).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructPropertyState {
    /// The field must be present.
    Required,
    /// The field may be omitted.
    Optional,
    /// The field may be omitted; if it is, its value comes from the type's
    /// intrinsic default. For built-in types, serialization of the default
    /// will be omitted.
    Default,
    /// The field may be omitted; if it is, its value comes from the provided
    /// JSON value. This applies only to deserialization; serialization
    /// will always emit the field.
    DefaultValue(JsonValue),
}

impl StructPropertyState {
    /// Whether the property is in the optional state.
    pub(crate) fn is_optional(&self) -> bool {
        matches!(self, StructPropertyState::Optional)
    }
}

/// A fieldless struct with a fixed JSON representation.
///
/// A `UnitStruct` is its own builder: [`UnitStruct::new`] starts one
/// under construction around its required representation, the fluent
/// methods fill it in, and [`UnitStruct::build`] validates it and
/// produces the finished [`Type::UnitStruct`] value.
#[derive(Debug, Clone)]
pub struct UnitStruct {
    pub(crate) common: TypeCommon,

    pub(crate) repr: serde_json::Value,
}
impl UnitStruct {
    /// Start a unit struct that serializes as `repr`.
    pub fn new(repr: serde_json::Value) -> Self {
        Self {
            common: Default::default(),
            repr,
        }
    }

    /// Set the unit struct's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.common.name = Some(name.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.common.description = Some(description.into());
        self
    }

    /// Add opaque derive paths applied to this type alone.
    ///
    /// These are additional to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive);
    /// a type's derive attribute names both sets. Each path is emitted
    /// verbatim, with the same caveats `with_derive` documents.
    pub fn extra_derives(mut self, derives: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_derives
            .extend(derives.into_iter().map(Into::into));
        self
    }

    /// Add opaque attributes applied to this type alone.
    ///
    /// These are additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn extra_attrs(mut self, attrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_attrs
            .extend(attrs.into_iter().map(Into::into));
        self
    }

    /// Validate the unit struct and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier.
    pub fn build<Id>(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::UnitStruct(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate<Id>(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("unit struct")
    }

    /// The unit struct's name, if one has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn get_description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The opaque derive paths applied to this type alone, additional
    /// to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive).
    pub fn get_extra_derives(&self) -> &[String] {
        self.common.extra_derives()
    }

    /// The opaque attributes applied to this type alone, additional to
    /// the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn get_extra_attrs(&self) -> &[String] {
        self.common.extra_attrs()
    }

    /// The fixed JSON value the unit struct serializes to and
    /// deserializes from.
    pub fn get_repr(&self) -> &serde_json::Value {
        &self.repr
    }

    pub(crate) fn render<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display>(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
    ) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    built:
                        Some(TypeCommonBuilt {
                            traits,
                            from_string_irrefutable: _,
                        }),
                    default: _,
                    extra_derives,
                    extra_attrs,
                },
            repr,
        } = self
        else {
            unreachable!()
        };
        let name = name.as_deref().expect("validated type has a name");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        let repr_tokens = crate::value_tokens::value_tokens(repr);
        let repr_string = serde_json::to_string(repr).unwrap();

        let mut traits = traits.clone();
        let serialize_impl = traits.remove(TypespaceTrait::Serialize).then(|| {
            quote! {
                impl ::serde::Serialize for #name_ident {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: ::serde::Serializer,
                    {
                        #repr_tokens.serialize(serializer)
                    }
                }
            }
        });

        let deserialize_impl = traits.remove(TypespaceTrait::Deserialize).then(|| {
            quote! {
                impl<'de> ::serde::Deserialize<'de> for #name_ident {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        let expected = #repr_tokens;
                        let value: ::serde_json::Value =
                            ::serde::Deserialize::deserialize(deserializer)?;
                        if value != expected {
                            return Err(::serde::de::Error::custom(format!(
                                "expected unit struct value {}, found {}",
                                #repr_string,
                                ::serde_json::to_string(&value).unwrap())));
                        }
                        Ok(#name_ident)
                    }
                }
            }
        });

        // A unit struct is neither of typify's comparison-derive
        // exceptions, so it is never exempt.
        let derive_attr = typespace.render_derives(&traits, extra_derives, false);
        let attrs = typespace.render_attrs(extra_attrs);

        // Canonical item order: see build::mod.
        quote! {
            #description
            #( #attrs )*
            #derive_attr
            pub struct #name_ident;

            #serialize_impl
            #deserialize_impl
        }
    }
}

/// A struct with unnamed, positional fields.
///
/// A `TupleStruct` is its own builder: [`TupleStruct::new`] starts one
/// under construction, the fluent methods fill it in, and
/// [`TupleStruct::build`] validates it and produces the finished
/// [`Type::TupleStruct`] value.
#[derive(Debug, Clone)]
pub struct TupleStruct<Id> {
    pub(crate) common: TypeCommon,
    /// Fields of the tuple.
    pub(crate) fields: Vec<Id>,

    /// Optional type, which must be represented as an array, that stores
    /// items beyond those in `fields`.
    pub(crate) rest: Option<Id>,
}

impl<Id> Default for TupleStruct<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> TupleStruct<Id> {
    /// Start a tuple struct under construction.
    pub fn new() -> Self {
        Self {
            common: Default::default(),
            fields: Vec::new(),
            rest: None,
        }
    }

    /// Set the tuple struct's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.common.name = Some(name.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.common.description = Some(description.into());
        self
    }

    /// Set the default value.
    pub fn default(mut self, default: impl Into<JsonValue>) -> Self {
        self.common.default = Some(default.into());
        self
    }

    /// Add opaque derive paths applied to this type alone.
    ///
    /// These are additional to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive);
    /// a type's derive attribute names both sets. Each path is emitted
    /// verbatim, with the same caveats `with_derive` documents.
    pub fn extra_derives(mut self, derives: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_derives
            .extend(derives.into_iter().map(Into::into));
        self
    }

    /// Add opaque attributes applied to this type alone.
    ///
    /// These are additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn extra_attrs(mut self, attrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_attrs
            .extend(attrs.into_iter().map(Into::into));
        self
    }

    /// Append positional fields.
    pub fn fields(mut self, fields: impl IntoIterator<Item = Id>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set the type, necessarily array-shaped, that stores items beyond
    /// the positional fields.
    pub fn rest(mut self, rest: Id) -> Self {
        self.rest = Some(rest);
        self
    }

    /// Validate the tuple struct and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier, or if no
    /// fields were added (see [`Error::FieldlessTupleStruct`]).
    pub fn build(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::TupleStruct(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("tuple struct")?;
        if self.fields.is_empty() {
            let alternative = match self.rest {
                Some(_) => "`NewtypeStruct` over the sequence type",
                None => "`UnitStruct`",
            };
            return Err(Error::FieldlessTupleStruct {
                name: self.common.built_name().to_string(),
                alternative,
            });
        }
        Ok(())
    }

    /// The tuple struct's name, if one has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn get_description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn get_default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The opaque derive paths applied to this type alone, additional
    /// to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive).
    pub fn get_extra_derives(&self) -> &[String] {
        self.common.extra_derives()
    }

    /// The opaque attributes applied to this type alone, additional to
    /// the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn get_extra_attrs(&self) -> &[String] {
        self.common.extra_attrs()
    }

    /// The fields of the tuple, in order.
    pub fn get_fields(&self) -> &[Id] {
        &self.fields
    }

    /// The type, necessarily array-shaped, holding items beyond the
    /// positional fields, if any.
    pub fn get_rest(&self) -> Option<&Id> {
        self.rest.as_ref()
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TupleStruct<Id> {
    pub(crate) fn render(
        &self,
        id: &Id,
        typespace: &TypespaceRenderer<'_, Id>,
    ) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default,
                    built:
                        Some(TypeCommonBuilt {
                            traits,
                            from_string_irrefutable: _,
                        }),
                    extra_derives,
                    extra_attrs,
                },
            fields,
            rest,
        } = self
        else {
            unreachable!()
        };
        let name = name.as_deref().expect("validated type has a name");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });

        let name_ident = format_ident!("{name}");

        let field_ident = fields
            .iter()
            .map(|field_id| typespace.render_ident(field_id))
            .collect::<Vec<_>>();
        let rest_ident = rest.as_ref().map(|rest_id| typespace.render_ident(rest_id));

        let field_index = (0..fields.len()).map(syn::Index::from);
        let rest_index = rest
            .as_ref()
            .map(|_| syn::Index::from(fields.len()))
            .into_iter();

        let field_var = (0..fields.len())
            .map(|ii| format_ident!("field_{ii}"))
            .collect::<Vec<_>>();
        let field_int = (0..fields.len()).collect::<Vec<_>>();
        let rest_var = rest
            .as_ref()
            .map(|_| format_ident!("rest"))
            .into_iter()
            .collect::<Vec<_>>();
        let expected = format!("a tuple of size {} or more", fields.len());

        let mut traits = traits.clone();
        let serialize_impl = traits.remove(TypespaceTrait::Serialize).then(|| {
            quote! {
                impl ::serde::Serialize for #name_ident {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: ::serde::Serializer,
                    {
                        use ::serde::ser::SerializeSeq;
                        let mut seq = serializer.serialize_seq(None)?;
                        #(
                            seq.serialize_element(&self.#field_index)?;
                        )*
                        #(
                            self.#rest_index.serialize(
                                ::json_serde::FlattenedSequenceSerializer::new(&mut seq)
                            )?;
                        )*
                        seq.end()
                    }
                }
            }
        });
        let deserialize_impl = traits.remove(TypespaceTrait::Deserialize).then(|| {
            quote! {
                impl<'de> ::serde::Deserialize<'de> for #name_ident {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        struct Visitor;

                        impl<'de> ::serde::de::Visitor<'de> for Visitor {
                            type Value = #name_ident;

                            fn expecting(&self, formatter: &mut ::std::fmt::Formatter)
                                -> ::std::fmt::Result
                            {
                                // TODO could we specify the type here?
                                formatter.write_str("a sequence")
                            }

                            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                            where
                                A: ::serde::de::SeqAccess<'de>,
                            {
                                // Strictly speaking, we don't need to store
                                // each tuple element in a variable, but as a
                                // practical matter, it makes the generated
                                // code much easier to follow and less deeply
                                // indented.
                                #(
                                    let #field_var = seq
                                        .next_element()?
                                        .ok_or_else(|| ::serde::de::Error::invalid_length(
                                            #field_int,
                                            &#expected
                                        ))?;
                                )*
                                #(
                                    let #rest_var = ::serde::Deserialize::deserialize(
                                        ::json_serde::FlattenedSequenceDeserializer::new(&mut seq)
                                    )?;
                                )*
                                Ok(#name_ident(
                                    #( #field_var, )*
                                    #( #rest_var, )*
                                ))
                            }
                        }

                        deserializer.deserialize_seq(Visitor)
                    }
                }
            }
        });

        // TODO 9/10/2026
        // Do we only want to do this if `rest` is Some?
        let json_schema_impl = traits.remove(TypespaceTrait::JsonSchema).then(|| {
            let (additional_items, min_items, max_items) = if let Some(rest_id) = rest.as_ref() {
                assert!(rest_ident.is_some());
                let additional_items = quote! {
                    additional_items: Some(::std::boxed::Box::new(
                        g.subschema_for::<#rest_ident>()
                    )),
                };

                let array_bounds = typespace.array_bounds(rest_id);
                let len = fields.len();

                let min = match array_bounds {
                    Some((min, _)) => len + min,
                    _ => len,
                } as u32;
                let max = match array_bounds {
                    Some((_, Some(max))) => Some((len + max) as u32),
                    _ => None,
                };

                let max_items = if let Some(max) = max {
                    quote! { max_items: Some(#max), }
                } else {
                    TokenStream::new()
                };

                (
                    additional_items,
                    quote! { min_items: Some(#min), },
                    max_items,
                )
            } else {
                let len = fields.len() as u32;
                (
                    TokenStream::new(),
                    quote! { min_items: Some(#len), },
                    quote! { max_items: Some(#len), },
                )
            };

            let description = description.as_ref().map(|d| {
                quote! {
                    description: Some(#d.to_string()),
                }
            });

            let default = default.as_ref().map(|JsonValue(value)| {
                let as_str = value.to_string();
                quote! { default: Some(::serde_json::from_str(#as_str).unwrap()), }
            });

            quote! {
                impl ::schemars::JsonSchema for #name_ident {
                    fn schema_name() -> ::std::string::String {
                        #name.to_string()
                    }

                    fn json_schema(
                        g: &mut ::schemars::r#gen::SchemaGenerator,
                    ) -> ::schemars::schema::Schema {
                        let fields = [
                            #(
                                g.subschema_for::<#field_ident>(),
                            )*
                        ]
                            .into_iter()
                            .collect();
                        ::schemars::schema::SchemaObject {
                            metadata: Some(::std::boxed::Box::new(
                                ::schemars::schema::Metadata {
                                    title: Some(#name.to_string()),
                                    #description
                                    #default
                                    ..::std::default::Default::default()
                                }
                            )),
                            instance_type: Some(
                                ::schemars::schema::SingleOrVec::Single(
                                    ::std::boxed::Box::new(
                                        ::schemars::schema::InstanceType::Array,
                                    )
                                )
                            ),
                            array: Some(::std::boxed::Box::new(
                                ::schemars::schema::ArrayValidation {
                                    items: Some(
                                        ::schemars::schema::SingleOrVec::Vec(fields)
                                    ),
                                    #additional_items
                                    #max_items
                                    #min_items
                                    ..::std::default::Default::default()
                                }
                            )),
                            ..::std::default::Default::default()
                        }
                        .into()
                    }
                }
            }
        });

        let default_impl = if let Some(JsonValue(value)) = default
            && traits.remove(TypespaceTrait::Default)
        {
            let default_value = typespace.generate_default_value_for_impl(value, id);
            quote! {
                impl ::std::default::Default for #name_ident {
                    fn default() -> Self {
                        #default_value
                    }
                }
            }
        } else {
            TokenStream::new()
        };

        // A tuple struct is neither of typify's comparison-derive
        // exceptions, so it is never exempt.
        let derive_attr = typespace.render_derives(&traits, extra_derives, false);
        let attrs = typespace.render_attrs(extra_attrs);

        let rest_ident_iter = rest_ident.into_iter();

        // Canonical item order: see build::mod.
        quote! {
            #description
            #( #attrs )*
            #derive_attr
            pub struct #name_ident(
                #( pub #field_ident, )*
                #( pub #rest_ident_iter, )*
            );

            #default_impl
            #serialize_impl
            #deserialize_impl
            #json_schema_impl
        }
    }

    pub(crate) fn children(&self) -> Vec<Id> {
        let mut children = self.fields.clone();
        if let Some(rest) = &self.rest {
            children.push(rest.clone());
        }

        children
    }

    pub(crate) fn contained_children_mut(&mut self) -> Vec<&mut Id> {
        let mut children = self.fields.iter_mut().collect::<Vec<&mut Id>>();

        if let Some(rest) = &mut self.rest {
            children.push(rest);
        }

        children
    }
}

/// A single-field wrapper struct.
///
/// A `NewtypeStruct` is its own builder: [`NewtypeStruct::new`] starts
/// one under construction around its required inner type, the fluent
/// methods fill it in, and [`NewtypeStruct::build`] validates it and
/// produces the finished [`Type::NewtypeStruct`] value.
#[derive(Debug, Clone)]
pub struct NewtypeStruct<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) inner: Id,
    pub(crate) constraints: NewtypeConstraints,
}

impl<Id> NewtypeStruct<Id> {
    /// Start a newtype struct wrapping the type `inner`.
    pub fn new(inner: Id) -> Self {
        Self {
            common: Default::default(),
            inner,
            constraints: NewtypeConstraints::None,
        }
    }

    /// Set the newtype's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.common.name = Some(name.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.common.description = Some(description.into());
        self
    }

    /// Set the default value.
    pub fn default(mut self, default: impl Into<JsonValue>) -> Self {
        self.common.default = Some(default.into());
        self
    }

    /// Add opaque derive paths applied to this type alone.
    ///
    /// These are additional to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive);
    /// a type's derive attribute names both sets. Each path is emitted
    /// verbatim, with the same caveats `with_derive` documents.
    pub fn extra_derives(mut self, derives: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_derives
            .extend(derives.into_iter().map(Into::into));
        self
    }

    /// Add opaque attributes applied to this type alone.
    ///
    /// These are additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn extra_attrs(mut self, attrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_attrs
            .extend(attrs.into_iter().map(Into::into));
        self
    }

    /// Set the constraints on the wrapped value; see
    /// [`NewtypeConstraints`].
    pub fn constraints(mut self, constraints: NewtypeConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Validate the newtype struct and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier.
    pub fn build(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::NewtypeStruct(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("newtype struct")?;

        // Constraints with nothing in them say nothing
        // NewtypeConstraints::None does not already say, and an
        // unconstrained newtype is the one written the short way.
        // Rejecting them is also what keeps "syntactically constrained"
        // and "stores its input verbatim" from disagreeing.
        let vacuous = match &self.constraints {
            NewtypeConstraints::String {
                min: None,
                max: None,
                patterns,
            } if patterns.is_empty() => Some("string"),
            NewtypeConstraints::AllowList(values) if values.is_empty() => Some("allow list"),
            NewtypeConstraints::DenyList(values) if values.is_empty() => Some("deny list"),
            _ => None,
        };

        match vacuous {
            Some(kind) => Err(Error::VacuousConstraints {
                name: self.common.built_name().to_string(),
                kind,
            }),
            None => Ok(()),
        }
    }

    /// The newtype's name, if one has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn get_description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn get_default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The opaque derive paths applied to this type alone, additional
    /// to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive).
    pub fn get_extra_derives(&self) -> &[String] {
        self.common.extra_derives()
    }

    /// The opaque attributes applied to this type alone, additional to
    /// the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn get_extra_attrs(&self) -> &[String] {
        self.common.extra_attrs()
    }

    /// The ID of the wrapped type.
    pub fn get_inner(&self) -> &Id {
        &self.inner
    }

    /// The constraints on the wrapped value.
    pub fn get_constraints(&self) -> &NewtypeConstraints {
        &self.constraints
    }
}

// TODO 3/7/2026
// I'm ambivalent as to whether the constrained form of a newtype should be
// its own, fundamentally distinct entity. However for now I'm going to just
// shove it into the existing newtype representation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NewtypeConstraints {
    None,
    AllowList(Vec<JsonValue>),
    DenyList(Vec<JsonValue>),
    String {
        min: Option<usize>,
        max: Option<usize>,
        patterns: Vec<String>,
    },
    Array {
        min: Option<usize>,
        max: Option<usize>,
        // TODO 3/7/2026
        // I'm quite unsure of how to model the contains keyword. It also
        // occurs to me that the constraints below don't suffice--we need
        // an array of structures.
        // As a side-note, as I recall the interaction between `contains` and
        // `unevaluatedItems` is quite baroque i.e satisfying a `contains`
        // constraint counts as evaluation. I suppose this also means that
        // there's an important distinction between `items` being absent vs.
        // having the value of `true`.
        // min_contains: Option<usize>,
        // max_contains: Option<usize>,
        // contains: (),
    },

    /// Fallback constraint
    ///
    /// Verify data against the given JSON schema (using the crate
    /// `jsonschema` for runtime validation).
    JsonSchema(JsonValue),
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> NewtypeStruct<Id> {
    pub(crate) fn children(&self) -> Vec<Id> {
        vec![self.inner.clone()]
    }

    pub(crate) fn contained_children_mut(&mut self) -> Vec<&mut Id> {
        vec![&mut self.inner]
    }

    pub(crate) fn render(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        out: &mut Outputspace,
    ) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default: _,
                    built:
                        Some(TypeCommonBuilt {
                            traits,
                            from_string_irrefutable: _,
                        }),
                    extra_derives,
                    extra_attrs,
                },
            inner,
            constraints,
        } = self
        else {
            unreachable!()
        };

        let mut traits = traits.clone();

        let name = name.as_deref().expect("validated type has a name");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        let inner_ident = typespace.render_ident(inner);

        // A newtype wrapping `String` directly is typify's other
        // comparison-derive exception.
        // TYPIFY COMPAT: read by render_derives' exemption and by the
        // unconstrained newtype's FromStr just below.
        let wraps_string = matches!(typespace.types.get(inner), Some(Type::String));

        let vis = matches!(constraints, NewtypeConstraints::None).then(|| quote! { pub });

        let constraint_impl = self.render_constraint_impl(typespace, out, &mut traits);

        let derive_attr = typespace.render_derives(&traits, extra_derives, wraps_string);
        let attrs = typespace.render_attrs(extra_attrs);

        // A newtype struct is its inner value on the wire.
        let mut serde_attr = SerdeDerives::new(&traits).attrs();
        serde_attr.push(quote! { transparent });

        // Canonical item order: see build::mod.
        quote! {
            #description
            #( #attrs )*
            #derive_attr
            #serde_attr
            pub struct #name_ident(#vis #inner_ident);

            impl ::std::ops::Deref for #name_ident {
                type Target = #inner_ident;
                // TODO: typespace compat
                // fn deref(&self) -> &Self::Target {
                fn deref(&self) -> & #inner_ident {
                    &self.0
                }
            }

            impl ::std::convert::From<#name_ident> for #inner_ident {
                fn from(value: #name_ident) -> Self {
                    value.0
                }
            }

            #constraint_impl
        }
    }

    fn render_constraint_impl(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        out: &mut Outputspace,
        traits: &mut crate::TypespaceTraitSet,
    ) -> TokenStream
    where
        Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    {
        let Self {
            common: TypeCommon {
                name: Some(name), ..
            },
            inner,
            constraints,
        } = self
        else {
            unreachable!();
        };

        let name_ident = format_ident!("{name}");
        let inner_ident = typespace.render_ident(inner);

        match constraints {
            NewtypeConstraints::None => {
                // An unconstrained newtype parses and prints according to its inner
                // value.

                // does: `feasibility` grants both traits with an obligation on
                // the inner type, so both impls forward to it. A constrained
                // newtype takes both traits out of the set inside
                // `render_constraint_impl`, so `remove` answers false here and
                // neither impl is written twice.
                //
                // TYPIFY COMPAT: a newtype directly over `String` takes the
                // value verbatim, so its FromStr cannot fail and it gets no
                // TryFrom impls. Every other inner type parses, so FromStr
                // forwards to the inner type's and the two TryFrom impls
                // forward to that.
                let from_str_impl = traits.remove(TypespaceTrait::FromStr).then(|| {
                    let wraps_string = matches!(typespace.types.get(inner), Some(Type::String));
                    if wraps_string {
                        quote! {
                            impl ::std::str::FromStr for #name_ident {
                                type Err = ::std::convert::Infallible;
                                fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                                    Ok(Self(value.to_string()))
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl ::std::str::FromStr for #name_ident {
                                type Err = <#inner_ident as ::std::str::FromStr>::Err;
                                fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                                    Ok(Self(value.parse()?))
                                }
                            }

                        }
                    }
                });
                let display_impl = traits.remove(TypespaceTrait::Display).then(|| {
                    quote! {
                        impl ::std::fmt::Display for #name_ident {
                            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                                self.0.fmt(f)
                            }
                        }
                    }
                });

                quote! {
                    impl ::std::convert::From<#inner_ident> for #name_ident {
                        fn from(value: #inner_ident) -> Self {
                            Self(value)
                        }
                    }

                    #display_impl
                    #from_str_impl
                }
            }

            NewtypeConstraints::AllowList(values) | NewtypeConstraints::DenyList(values) => {
                let value_output = values
                    .iter()
                    .map(|value| typespace.generate_default(&value.0, inner));

                let value_string = values
                    .iter()
                    .map(|value| serde_json::to_string(&value.0).unwrap());

                let deserialize_impl = traits.remove(TypespaceTrait::Deserialize).then(|| {
                    quote! {
                        impl<'de> ::serde::Deserialize<'de> for #name_ident {
                            fn deserialize<D>(
                                deserializer: D,
                            ) -> ::std::result::Result<Self, D::Error>
                            where
                                D: ::serde::Deserializer<'de>,
                            {
                                Self::try_from(
                                    <#inner_ident>::deserialize(deserializer)?,
                                )
                                .map_err(|e| {
                                    <D::Error as ::serde::de::Error>::custom(
                                        e.to_string(),
                                    )
                                })
                            }
                        }
                    }
                });

                // As with Deserialize, serde::JsonSchema requires a custom
                // impl. If it's present in the set of derives, remove it and
                // generate something that accurately models the type.
                let json_schema_impl = traits.remove(TypespaceTrait::JsonSchema).then(|| {
                    // TODO 9/7/2026
                    // I'm really not sure why typify 1 did this `from_str` stuff
                    // when we--I think--already have serde_json::Value tokens
                    // ready to go... but we can look into that later.
                    let enum_values = quote! {
                        ::std::option::Option::Some([
                            #( ::serde_json::from_str(#value_string).unwrap(), )*
                        ].into_iter().collect())
                    };

                    let body = match constraints {
                        NewtypeConstraints::AllowList(_) => quote! {
                            schema.enum_values = #enum_values;
                        },
                        NewtypeConstraints::DenyList(_) => quote! {
                            let not = ::schemars::schema::SchemaObject {
                                enum_values: #enum_values,
                                ..::std::default::Default::default()
                            };
                            schema.subschemas().not = Some(
                                ::std::boxed::Box::new(not.into())
                            );
                        },
                        _ => unreachable!(),
                    };
                    quote! {
                        impl ::schemars::JsonSchema for #name_ident {
                            fn schema_name() -> ::std::string::String {
                                #name.to_string()
                            }

                            fn json_schema(
                                g: &mut ::schemars::r#gen::SchemaGenerator
                            ) -> ::schemars::schema::Schema {
                                let mut schema =
                                    <#inner_ident as ::schemars::JsonSchema>
                                        ::json_schema(g)
                                        .into_object();
                                #body
                                schema.into()
                            }
                        }
                    }
                });

                // TODO if the sub_type is a string we could probably impl
                // TryFrom<&str> as well and FromStr.

                let from_str_impl = traits.remove(TypespaceTrait::FromStr).then(|| quote! {});
                let display_impl = traits.remove(TypespaceTrait::Display).then(|| quote! {});

                let not =
                    matches!(constraints, NewtypeConstraints::AllowList(_)).then(|| quote! { ! });
                typespace.add_error_mod(out);

                quote! {
                        // This is effectively the constructor for this type.
                        impl ::std::convert::TryFrom<#inner_ident> for #name_ident {
                            type Error = self::error::ConversionError;

                            fn try_from(
                                value: #inner_ident
                            ) -> ::std::result::Result<Self, self::error::ConversionError>
                            {
                                if #not [
                                    #(#value_output,)*
                                ].contains(&value) {
                                    Err("invalid value".into())
                                } else {
                                    Ok(Self(value))
                                }
                            }
                        }


                        #display_impl
                        #from_str_impl
                        #deserialize_impl
                        #json_schema_impl

                }
            }

            NewtypeConstraints::String { min, max, patterns } => {
                typespace.add_error_mod(out);
                let max = max.map(|v| {
                    let err = format!("longer than {} characters", v);
                    quote! {
                        if value.chars().count() > #v {
                            return Err(#err.into());
                        }
                    }
                });
                let min = min.map(|v| {
                    let err = format!("shorter than {} characters", v);
                    quote! {
                        if value.chars().count() < #v {
                            return Err(#err.into());
                        }
                    }
                });

                let pat = patterns.iter().map(|p| {
                let err = format!("doesn't match pattern \"{}\"", p);
                quote! {
                    static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(|| {
                        ::regress::Regex::new(#p).unwrap()
                    });
                    if PATTERN.find(value).is_none() {
                        return Err(#err.into());
                    }
                }
            }).collect::<Vec<_>>();

                // TYPIFY 1 COMPAT: pull out a lone pat
                let pat = match &pat[..] {
                    [] => TokenStream::new(),
                    [solo] => solo.clone(),
                    many => quote! {
                        #(
                            {
                                #many
                            }
                        )*
                    },
                };

                let deserialize_impl = traits.remove(TypespaceTrait::Deserialize).then(|| {
                    quote! {
                        impl<'de> ::serde::Deserialize<'de> for #name_ident {
                            fn deserialize<D>(
                                deserializer: D,
                            ) -> ::std::result::Result<Self, D::Error>
                            where
                                D: ::serde::Deserializer<'de>,
                            {
                                ::std::convert::TryFrom::try_from(
                                    ::std::string::String::deserialize(
                                        deserializer
                                    )?
                                )
                                .map_err(|e: self::error::ConversionError| {
                                    <D::Error as ::serde::de::Error>::custom(
                                        e.to_string(),
                                    )
                                })
                            }
                        }
                    }
                });

                let from_str_impl = traits.remove(TypespaceTrait::FromStr).then(|| {
                quote! {
                    impl ::std::str::FromStr for #name_ident {
                        type Err = self::error::ConversionError;

                        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
                            ::std::convert::TryFrom::try_from(value)
                        }
                    }

                }
            });
                // TYPIFY COMPAT: typify 1 writes no Display for a
                // constrained string newtype.
                let display_impl = (traits.remove(TypespaceTrait::Display)
                && !typespace.settings.typify_compat)
                .then(|| {
                    quote! {
                        impl ::std::fmt::Display for #name_ident {
                            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                                self.0.fmt(f)
                            }
                        }
                    }
                });

                quote! {
                    #display_impl
                    #from_str_impl

                    impl ::std::convert::TryFrom<&str> for #name_ident {
                        type Error = self::error::ConversionError;

                        fn try_from(value: &str) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            #max
                            #min
                            #pat
                            Ok(Self(value.to_string()))
                        }
                    }
                    impl ::std::convert::TryFrom<::std::string::String> for #name_ident {
                        type Error = self::error::ConversionError;

                        fn try_from(value: ::std::string::String) ->
                            ::std::result::Result<Self, self::error::ConversionError>
                        {
                            ::std::convert::TryFrom::try_from(value.as_str())
                        }
                    }

                    #deserialize_impl
                }
            }
            NewtypeConstraints::Array { .. } => todo!(),
            NewtypeConstraints::JsonSchema(_json_value) => todo!(),
        }
    }
}

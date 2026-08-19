// Copyright 2026 Oxide Computer Company

use log::debug;
use quote::{format_ident, quote};
use syn::Ident;

use crate::build::{CommonBuilder, JsonValue, Type, TypeCommon, TypeCommonBuilt};
use crate::{TypespaceError, TypespaceRenderer, TypespaceTrait};

/// A struct with named fields; construct one with [`Struct::builder`].
#[derive(Debug, Clone)]
pub struct Struct<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) properties: Vec<StructProperty<Id>>,
    pub(crate) deny_unknown_fields: bool,
}

impl<Id> Struct<Id> {
    /// Start building a struct; see [`StructBuilder`].
    pub fn builder() -> StructBuilder<Id> {
        StructBuilder {
            common: CommonBuilder::default(),
            properties: Vec::new(),
            deny_unknown_fields: false,
        }
    }

    /// The struct's name, always nonempty.
    pub fn name(&self) -> &str {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The struct's properties, in declaration order.
    pub fn properties(&self) -> &[StructProperty<Id>] {
        &self.properties
    }

    /// Whether deserialization rejects unknown fields.
    pub fn deny_unknown_fields(&self) -> bool {
        self.deny_unknown_fields
    }
}

/// Assembles a [`Struct`]; created by [`Struct::builder`].
///
/// The name is the one required ingredient and may be supplied at any
/// point before [`StructBuilder::build`], which produces the finished
/// [`Type::Struct`] value.
#[derive(Debug, Clone)]
pub struct StructBuilder<Id> {
    common: CommonBuilder,
    properties: Vec<StructProperty<Id>>,
    deny_unknown_fields: bool,
}

impl<Id> StructBuilder<Id> {
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

    /// Append one property.
    pub fn property(mut self, property: StructProperty<Id>) -> Self {
        self.properties.push(property);
        self
    }

    /// Append any number of properties.
    pub fn properties(mut self, properties: impl IntoIterator<Item = StructProperty<Id>>) -> Self {
        self.properties.extend(properties);
        self
    }

    /// Make deserialization reject unknown fields.
    pub fn deny_unknown_fields(mut self) -> Self {
        self.deny_unknown_fields = true;
        self
    }

    /// Produce the struct as a [`Type`] value.
    ///
    /// Fails with [`TypespaceError::MissingTypeName`] unless a nonempty
    /// name was provided.
    pub fn build(self) -> Result<Type<Id>, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self {
            common,
            properties,
            deny_unknown_fields,
        } = self;
        Ok(Type::Struct(Struct {
            common: common.build("struct")?,
            properties,
            deny_unknown_fields,
        }))
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Struct<Id> {
    pub(crate) fn render(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        cs: &mut codespace::Codespace,
    ) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default: _,
                    built: Some(TypeCommonBuilt { traits }),
                },
            properties,
            deny_unknown_fields: _,
        } = self
        else {
            unreachable!()
        };
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });
        let name_ident = format_ident!("{name}");
        let snake_name = heck::AsSnakeCase(name.as_str()).to_string();

        let mut rendered_properties = Vec::new();
        for prop in properties {
            rendered_properties.push(typespace.render_struct_property(prop, true, &snake_name, cs));
        }

        // Serialize and Deserialize are always derived; propagated traits
        // are realized as additional derives.
        let derive_attr = typespace.render_derives(
            &[
                quote! { ::serde::Deserialize },
                quote! { ::serde::Serialize },
            ],
            traits,
            &[TypespaceTrait::Serialize, TypespaceTrait::Deserialize],
        );

        quote! {
            #description
            #derive_attr
            pub struct #name_ident {
                #( #rendered_properties, )*
            }
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
    pub(crate) rust_name: Ident,
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
    /// methods.
    pub fn new(rust_name: Ident, type_id: Id) -> Self {
        Self {
            rust_name,
            json_name: StructPropertySerde::None,
            state: StructPropertyState::Required,
            description: None,
            type_id,
        }
    }

    /// Set the property's volitionality; see [`StructPropertyState`].
    pub fn with_state(mut self, state: StructPropertyState) -> Self {
        self.state = state;
        self
    }

    /// Set the serde treatment of the property's name; see
    /// [`StructPropertySerde`].
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
    pub fn rust_name(&self) -> &Ident {
        &self.rust_name
    }

    /// The serde treatment of the property's name.
    pub fn json_name(&self) -> &StructPropertySerde {
        &self.json_name
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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructPropertySerde {
    None,
    Rename(String),
    Flatten,
}

/// The volitionality of a struct property. Only `Optional` will translate into
/// an `Option<T>` type; the others will be required in Rust. Conversely, only
/// `Required` must be present during deserialization; the others may be
/// omitted.
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
    /// JSON value. Note that this applies only to deserialization;
    /// serialization will always emit the field.
    DefaultValue(JsonValue),
}

/// A fieldless struct with a fixed JSON representation; construct one
/// with [`UnitStruct::builder`].
#[derive(Debug, Clone)]
pub struct UnitStruct {
    pub(crate) common: TypeCommon,

    pub(crate) repr: serde_json::Value,
}
impl UnitStruct {
    /// Start building a unit struct that serializes as `repr`; see
    /// [`UnitStructBuilder`].
    pub fn builder(repr: serde_json::Value) -> UnitStructBuilder {
        UnitStructBuilder {
            common: CommonBuilder::default(),
            repr,
        }
    }

    /// The unit struct's name, always nonempty.
    pub fn name(&self) -> &str {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The fixed JSON value the unit struct serializes to and
    /// deserializes from.
    pub fn repr(&self) -> &serde_json::Value {
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
                    built: Some(TypeCommonBuilt { traits }),
                    default: _,
                },
            repr,
        } = self
        else {
            unreachable!()
        };
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        // Clone and Debug are always derived; Serialize and Deserialize are
        // hand-written impls below, so they never appear as derives.
        let derive_attr = typespace.render_derives(
            &[quote! { ::std::clone::Clone }, quote! { ::std::fmt::Debug }],
            traits,
            &[
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
            ],
        );

        let repr_tokens = crate::value_tokens::value_tokens(repr);
        let repr_string = serde_json::to_string(repr).unwrap();
        quote! {
            #description
            #derive_attr
            pub struct #name_ident;

            impl ::serde::Serialize for #name_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    #repr_tokens.serialize(serializer)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #name_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    let expected = #repr_tokens;
                    let value: serde_json::Value =
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
    }
}

/// Assembles a [`UnitStruct`]; created by [`UnitStruct::builder`].
///
/// The name is the one required ingredient and may be supplied at any
/// point before [`UnitStructBuilder::build`], which produces the
/// finished [`Type::UnitStruct`] value.
#[derive(Debug, Clone)]
pub struct UnitStructBuilder {
    common: CommonBuilder,
    repr: serde_json::Value,
}

impl UnitStructBuilder {
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

    /// Set the default value.
    pub fn default(mut self, default: impl Into<JsonValue>) -> Self {
        self.common.default = Some(default.into());
        self
    }

    /// Produce the unit struct as a [`Type`] value.
    ///
    /// Fails with [`TypespaceError::MissingTypeName`] unless a nonempty
    /// name was provided.
    pub fn build<Id>(self) -> Result<Type<Id>, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self { common, repr } = self;
        Ok(Type::UnitStruct(UnitStruct {
            common: common.build("unit struct")?,
            repr,
        }))
    }
}

/// A struct with unnamed, positional fields; construct one with
/// [`TupleStruct::builder`].
#[derive(Debug, Clone)]
pub struct TupleStruct<Id> {
    pub(crate) common: TypeCommon,
    /// Fields of the tuple.
    pub(crate) fields: Vec<Id>,

    /// Optional type, which must be represented as an array, that stores
    /// items beyond those in `fields`.
    pub(crate) rest: Option<Id>,
}
impl<Id> TupleStruct<Id> {
    /// Start building a tuple struct; see [`TupleStructBuilder`].
    pub fn builder() -> TupleStructBuilder<Id> {
        TupleStructBuilder {
            common: CommonBuilder::default(),
            fields: Vec::new(),
            rest: None,
        }
    }

    /// The tuple struct's name, always nonempty.
    pub fn name(&self) -> &str {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The fields of the tuple, in order.
    pub fn fields(&self) -> &[Id] {
        &self.fields
    }

    /// The type, necessarily array-shaped, holding items beyond those
    /// in [`TupleStruct::fields`], if any.
    pub fn rest(&self) -> Option<&Id> {
        self.rest.as_ref()
    }
}

/// Assembles a [`TupleStruct`]; created by [`TupleStruct::builder`].
///
/// The name is the one required ingredient and may be supplied at any
/// point before [`TupleStructBuilder::build`], which produces the
/// finished [`Type::TupleStruct`] value.
#[derive(Debug, Clone)]
pub struct TupleStructBuilder<Id> {
    common: CommonBuilder,
    fields: Vec<Id>,
    rest: Option<Id>,
}

impl<Id> TupleStructBuilder<Id> {
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

    /// Append one positional field.
    pub fn field(mut self, field: Id) -> Self {
        self.fields.push(field);
        self
    }

    /// Append any number of positional fields.
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

    /// Produce the tuple struct as a [`Type`] value.
    ///
    /// Fails with [`TypespaceError::MissingTypeName`] unless a nonempty
    /// name was provided.
    pub fn build(self) -> Result<Type<Id>, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self {
            common,
            fields,
            rest,
        } = self;
        Ok(Type::TupleStruct(TupleStruct {
            common: common.build("tuple struct")?,
            fields,
            rest,
        }))
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TupleStruct<Id> {
    pub(crate) fn render(&self, typespace: &TypespaceRenderer<'_, Id>) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default: _,
                    built: Some(TypeCommonBuilt { traits }),
                },
            fields,
            rest,
        } = self
        else {
            unreachable!()
        };
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });

        let name_ident = format_ident!("{name}");

        // Clone and Debug are always derived; Serialize and Deserialize are
        // hand-written impls below, so they never appear as derives.
        let derive_attr = typespace.render_derives(
            &[quote! { ::std::clone::Clone }, quote! { ::std::fmt::Debug }],
            traits,
            &[
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
            ],
        );

        // The flattened-sequence helpers come from the json-serde crate,
        // whose path is configurable.
        let json_serde = syn::parse_str::<syn::Path>(typespace.settings.json_serde_crate())
            .expect("invalid json-serde crate path");

        let field_ident = fields
            .iter()
            .map(|field_id| typespace.render_ident(field_id));
        let rest_ident = rest
            .as_ref()
            .map(|rest_id| typespace.render_ident(rest_id))
            .into_iter();

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

        quote! {
            #description
            #derive_attr
            pub struct #name_ident(
                #( pub #field_ident, )*
                #( pub #rest_ident, )*
            );

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
                            #json_serde::FlattenedSequenceSerializer::new(&mut seq)
                        )?;
                    )*
                    seq.end()
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #name_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    struct Visitor;

                    impl<'de> ::serde::de::Visitor<'de> for Visitor {
                        type Value = #name_ident;

                        fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            // TODO could we specify the type here?
                            formatter.write_str("a sequence")
                        }

                        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                        where
                            A: ::serde::de::SeqAccess<'de>,
                        {
                            // Strictly speaking, we don't need to
                            // store each tuple element in a
                            // variable, but as a practical matter,
                            // it makes the generated code much
                            // easier to follow and less indented.
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
                                    #json_serde::FlattenedSequenceDeserializer::new(&mut seq)
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

/// A single-field wrapper struct; construct one with
/// [`NewtypeStruct::builder`].
#[derive(Debug, Clone)]
pub struct NewtypeStruct<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) inner: Id,
    pub(crate) constraints: NewtypeConstraints,
}

impl<Id> NewtypeStruct<Id> {
    /// Start building a newtype struct wrapping the type `inner`; see
    /// [`NewtypeStructBuilder`].
    pub fn builder(inner: Id) -> NewtypeStructBuilder<Id> {
        NewtypeStructBuilder {
            common: CommonBuilder::default(),
            inner,
            constraints: NewtypeConstraints::None,
        }
    }

    /// The newtype's name, always nonempty.
    pub fn name(&self) -> &str {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.common.default()
    }

    /// The ID of the wrapped type.
    pub fn inner(&self) -> &Id {
        &self.inner
    }

    /// The constraints on the wrapped value.
    pub fn constraints(&self) -> &NewtypeConstraints {
        &self.constraints
    }
}

/// Assembles a [`NewtypeStruct`]; created by [`NewtypeStruct::builder`].
///
/// The name is the one required ingredient and may be supplied at any
/// point before [`NewtypeStructBuilder::build`], which produces the
/// finished [`Type::NewtypeStruct`] value.
#[derive(Debug, Clone)]
pub struct NewtypeStructBuilder<Id> {
    common: CommonBuilder,
    inner: Id,
    constraints: NewtypeConstraints,
}

impl<Id> NewtypeStructBuilder<Id> {
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

    /// Set the constraints on the wrapped value; see
    /// [`NewtypeConstraints`].
    pub fn constraints(mut self, constraints: NewtypeConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Produce the newtype struct as a [`Type`] value.
    ///
    /// Fails with [`TypespaceError::MissingTypeName`] unless a nonempty
    /// name was provided.
    pub fn build(self) -> Result<Type<Id>, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self {
            common,
            inner,
            constraints,
        } = self;
        Ok(Type::NewtypeStruct(NewtypeStruct {
            common: common.build("newtype struct")?,
            inner,
            constraints,
        }))
    }
}

// TODO 3/7/2026
// I'm ambivalent as to whether the constrained form of a newtype should be
// it's own, fundamentally distinct entity. However for now I'm going to just
// shove it into the existing newtype representation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NewtypeConstraints {
    None,
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
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> NewtypeStruct<Id> {
    pub(crate) fn children(&self) -> Vec<Id> {
        vec![self.inner.clone()]
    }

    pub(crate) fn contained_children_mut(&mut self) -> Vec<&mut Id> {
        vec![&mut self.inner]
    }

    pub(crate) fn render(&self, typespace: &TypespaceRenderer<'_, Id>) -> proc_macro2::TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default: _,
                    built: Some(TypeCommonBuilt { traits }),
                },
            inner,
            constraints,
        } = self
        else {
            unreachable!()
        };

        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        let inner_ident = typespace.render_ident(inner);

        // Serialize and Deserialize are hand-written impls below, so they
        // never appear as derives; other propagated traits are derived.
        let derive_attr = typespace.render_derives(
            &[],
            traits,
            &[TypespaceTrait::Serialize, TypespaceTrait::Deserialize],
        );

        debug!("constraints: {constraints:#?}");

        quote! {
            #description
            #derive_attr
            pub struct #name_ident(pub #inner_ident);

            impl ::std::ops::Deref for #name_ident {
                type Target = #inner_ident;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::convert::From<#name_ident> for #inner_ident {
                fn from(value: #name_ident) -> Self {
                    value.0
                }
            }

            impl ::serde::Serialize for #name_ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    self.0.serialize(serializer)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #name_ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    Ok(Self(::serde::Deserialize::deserialize(deserializer)?))
                }
            }
        }
    }
}

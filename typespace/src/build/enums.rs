// Copyright 2026 Oxide Computer Company

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::build::{
    check_properties, validate_ident, JsonValue, StructProperty, Type, TypeCommon, TypeCommonBuilt,
};
use crate::error::{Error, NameAxis};
use crate::{TypespaceRenderer, TypespaceTrait};

/// An enum.
///
/// An `Enum` is its own builder: [`Enum::new`] starts one under
/// construction, the fluent methods fill it in ([`Enum::name`] and
/// [`Enum::tag_type`] are both required and may come at any point), and
/// [`Enum::build`] validates it and produces the finished
/// [`Type::Enum`] value.
#[derive(Debug, Clone)]
pub struct Enum<Id> {
    pub(crate) common: TypeCommon,

    pub(crate) tag_type: Option<EnumTagType>,
    pub(crate) variants: Vec<EnumVariant<Id>>,
    pub(crate) deny_unknown_fields: bool,
}

impl<Id> Default for Enum<Id> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id> Enum<Id> {
    /// Start an enum under construction.
    pub fn new() -> Self {
        Self {
            common: Default::default(),
            tag_type: None,
            variants: Vec::new(),
            deny_unknown_fields: false,
        }
    }

    /// Set the enum's name.
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

    /// Set the serde tagging scheme; see [`EnumTagType`].
    ///
    /// There is no presumed default: an enum cannot be built until its
    /// tagging scheme has been chosen.
    pub fn tag_type(mut self, tag_type: EnumTagType) -> Self {
        self.tag_type = Some(tag_type);
        self
    }

    /// Append variants.
    pub fn variants(mut self, variants: impl IntoIterator<Item = EnumVariant<Id>>) -> Self {
        self.variants.extend(variants);
        self
    }

    /// Make deserialization reject unknown fields.
    pub fn deny_unknown_fields(mut self) -> Self {
        self.deny_unknown_fields = true;
        self
    }

    /// Validate the enum and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier, if no
    /// tag type was set, if any variant or variant-field name is not a
    /// valid identifier, or if names collide on either axis (see
    /// [`Error::DuplicateItemName`]).
    pub fn build(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::Enum(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("enum")?;
        let type_name = self.common.built_name();
        if self.tag_type.is_none() {
            return Err(Error::MissingTagType {
                name: type_name.to_string(),
            });
        }

        // Variant names must be valid identifiers and unique on both
        // the Rust and wire axes; a struct-shaped variant's fields are
        // held to the same property rules as a struct's.
        let mut rust_names = BTreeSet::new();
        let mut wire_names = BTreeSet::new();
        for variant in &self.variants {
            validate_ident("variant", &variant.rust_name)?;
            if !rust_names.insert(variant.rust_name.clone()) {
                return Err(Error::DuplicateItemName {
                    kind: "variant",
                    type_name: type_name.to_string(),
                    name: variant.rust_name.clone(),
                    axis: NameAxis::Rust,
                });
            }
            let wire_name = variant
                .rename
                .clone()
                .unwrap_or_else(|| variant.rust_name.clone());
            if !wire_names.insert(wire_name.clone()) {
                return Err(Error::DuplicateItemName {
                    kind: "variant",
                    type_name: type_name.to_string(),
                    name: wire_name,
                    axis: NameAxis::Wire,
                });
            }
            if let VariantDetails::Struct(properties) = &variant.details {
                check_properties(&format!("{type_name}::{}", variant.rust_name), properties)?;
            }
        }
        Ok(())
    }

    /// The enum's name, if one has been set.
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

    /// The serde tagging scheme, if one has been set.
    pub fn get_tag_type(&self) -> Option<&EnumTagType> {
        self.tag_type.as_ref()
    }

    /// The enum's variants, in declaration order.
    pub fn get_variants(&self) -> &[EnumVariant<Id>] {
        &self.variants
    }

    /// Whether deserialization rejects unknown fields.
    pub fn get_deny_unknown_fields(&self) -> bool {
        self.deny_unknown_fields
    }

    /// Whether every variant is a unit variant (and the enum is
    /// nonempty and neither untagged nor missing its tag type).
    ///
    /// Such enums are value-like: they can derive `Copy`, `Eq`, `Ord`,
    /// and `Hash`, and admit bespoke `Display` and `FromStr` impls that
    /// map variants to and from their serialized names.
    pub fn all_simple_variants(&self) -> bool {
        self.tag_type
            .as_ref()
            .is_some_and(|tag_type| *tag_type != EnumTagType::Untagged)
            && !self.variants.is_empty()
            && self
                .variants
                .iter()
                .all(|variant| matches!(variant.details, VariantDetails::Unit))
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Enum<Id> {
    pub(crate) fn children(&self) -> Vec<Id> {
        self.variants
            .iter()
            .flat_map(|variant| variant.children())
            .collect()
    }

    pub(crate) fn render(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        cs: &mut codespace::Codespace,
    ) -> TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    default: _,
                    built: Some(TypeCommonBuilt { traits }),
                },
            tag_type,
            variants,
            deny_unknown_fields: _,
        } = self
        else {
            unreachable!()
        };
        let name = name.as_deref().expect("validated type has a name");
        let tag_type = tag_type.as_ref().expect("validated enum has a tag type");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });
        let serde = match tag_type {
            EnumTagType::External => TokenStream::new(),
            EnumTagType::Internal { tag } => quote! { #[serde(tag = #tag)] },
            EnumTagType::Adjacent { tag, content } => {
                quote! { #[serde(tag = #tag, content = #content)] }
            }
            EnumTagType::Untagged => quote! { #[serde(untagged)] },
        };

        let variants = variants.iter().map(|variant| {
            let EnumVariant {
                rust_name,
                rename,
                description,
                details,
            } = variant;
            let name = format_ident!("{}", rust_name);
            let variant_serde = rename.as_ref().map(|n| quote! { #[serde(rename = #n)] });
            let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });

            let data = match details {
                VariantDetails::Unit => TokenStream::new(),
                VariantDetails::Item(item) => {
                    let item_ident = typespace.render_ident(item);
                    quote! { (#item_ident) }
                }
                VariantDetails::Tuple(items) => {
                    let item_idents = items.iter().map(|item| typespace.render_ident(item));
                    quote! { ( #( #item_idents, )* ) }
                }
                VariantDetails::Struct(properties) => {
                    let properties = properties.iter().map(|prop| {
                        typespace.render_struct_property(
                            prop,
                            false,
                            &format!("{}{}", name, rust_name),
                            cs,
                        )
                    });
                    quote! { { #( #properties, )* } }
                }
            };

            quote! {
                #description
                #variant_serde
                #name #data
            }
        });

        let name_ident = format_ident!("{name}");

        // Serialize and Deserialize are always derived; propagated traits
        // are realized as additional derives. Default is excluded: its
        // derive form requires a #[default] variant attribute, so a
        // required Default on an enum awaits a hand-written impl.
        let derive_attr = typespace.render_derives(
            &[
                quote! { ::serde::Deserialize },
                quote! { ::serde::Serialize },
            ],
            traits,
            &[
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
                TypespaceTrait::Default,
            ],
        );

        quote! {
            // TODO I want to have the original unique id available
            #description
            #derive_attr
            #serde
            pub enum #name_ident {
                #( #variants, )*
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnumTagType {
    /// serde external tagging (serde's default)
    External,
    /// serde internal tagging
    Internal { tag: String },
    /// serde adjacent tagging
    Adjacent { tag: String, content: String },
    /// serde untagged
    Untagged,
}

// TODO 6/24/2025
// Do I want the variants to have tagging? I mean we could support the variant
// tagging for untagged if we wanted. Also how would we support more custom
// enums ala typify#811
// 6/28/2025
// Answer: No. Recall that the untagged variant markers need to be at the end
// of the type which makes it kind of a pain in the neck.
// TODO 10/7/2025
// How would we deal with an enum value that isn't a string--a number or
// boolean value for example? { "enum": [true, 1, "on"] } First we need to be
// able to represent this and then we need to be able to generate custom
// serialize/deserialize impls.

// TODO 2/27/2026
// I'm starting to think that all of the serde impls should be generated by
// the typespace module. Why? Well the benefits of using serde derive and the
// associated annotations are as follows:
//
// 1. Familiar notation for users to understand the serialized form
// 2. Less code for us to generate
//
// The first of these falls apart quickly--serde annotations are not part of
// the docs so you'd really have to be looking closely at the code to draw
// any inferences. The second is only true if we don't spend just as much
// effort organizing our structures to emit the appropriate annotations. In
// addition, we know that there are enumerations that serde simply can't
// represent, such as if the serialized values of unit variants are not
// strings. The only serde-supported serialization scheme that actually
// requires variants to have string names is external tagging. It makes sense
// that this is the default for serde since it's the most efficient to
// deserialize.
//
// TODO 3/1/2026
// I'd like to be able to represent enums in this version that the previous
// version couldn't, in particular simple enums with a variety of serialized
// data-types as noted above ({ "enum": [true, 1, "on"] }). Rather than
// thinking about these as special, I think I'd rather generate the serde
// implementations for all enums (or maybe all types).

/// One variant of an [`Enum`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct EnumVariant<Id> {
    pub(crate) rust_name: String,
    pub(crate) rename: Option<String>,
    // TODO need a name for serialization?
    // pub json_name: String,
    pub(crate) description: Option<String>,
    pub(crate) details: VariantDetails<Id>,
}

impl<Id> EnumVariant<Id> {
    /// Create an enum variant named `rust_name` with the given shape of
    /// associated data.
    ///
    /// The variant serializes under its Rust name and has no
    /// description; adjust with the `with_` methods.
    pub fn new(rust_name: impl Into<String>, details: VariantDetails<Id>) -> Self {
        Self {
            rust_name: rust_name.into(),
            rename: None,
            description: None,
            details,
        }
    }

    /// Serialize the variant as `rename` instead of its Rust name.
    pub fn with_rename(mut self, rename: impl Into<String>) -> Self {
        self.rename = Some(rename.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The Rust name of the variant.
    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    /// The serde rename, if the serialized name differs from the Rust
    /// name.
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    /// The serialized (JSON) name of the variant.
    ///
    /// The rename if one is present, the Rust name otherwise; anything
    /// that checks serialized data against the enum (validating a
    /// schema-supplied default value, say) needs this name back.
    pub fn json_name(&self) -> &str {
        self.rename.as_deref().unwrap_or(&self.rust_name)
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The shape of the variant's associated data.
    pub fn details(&self) -> &VariantDetails<Id> {
        &self.details
    }
}

impl<Id: Clone> EnumVariant<Id> {
    fn children(&self) -> Vec<Id> {
        match &self.details {
            VariantDetails::Unit => Vec::new(),
            VariantDetails::Item(id) => vec![id.clone()],
            VariantDetails::Tuple(items) => items.clone(),
            VariantDetails::Struct(items) => {
                items.iter().map(|prop| prop.type_id.clone()).collect()
            }
        }
    }

    pub(crate) fn contained_children(&self) -> Vec<Id> {
        self.children()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariantDetails<Id> {
    Unit,
    Item(Id),
    Tuple(Vec<Id>),
    Struct(Vec<StructProperty<Id>>),
}

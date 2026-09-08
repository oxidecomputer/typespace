// Copyright 2026 Oxide Computer Company

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::build::{
    JsonValue, StructProperty, Type, TypeCommon, TypeCommonBuilt, check_properties, validate_ident,
};
use crate::default::{EnumDefault, generate_default_enum};
use crate::error::{Error, NameAxis};
use crate::serde_attrs::SerdeDerives;
use crate::{TypespaceRenderer, TypespaceTrait, TypespaceTraitSet};

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
    pub fn all_tagged_unit_variants(&self) -> bool {
        self.tag_type
            .as_ref()
            .is_some_and(|tag_type| *tag_type != EnumTagType::Untagged)
            && !self.variants.is_empty()
            && self
                .variants
                .iter()
                .all(|variant| matches!(variant.details, VariantDetails::Unit))
    }

    /// Whether the enum is untagged and every variant carries exactly
    /// one payload (and the enum is nonempty).
    ///
    /// Such an enum's serialized form is exactly one variant payload's
    /// serialized form, so it admits bespoke `Display` and `FromStr`
    /// impls that forward to the payload types. A variant carrying no
    /// payload, several payloads, or named fields has no single form
    /// to forward to.
    pub fn all_untagged_item_variants(&self) -> bool {
        self.tag_type
            .as_ref()
            .is_some_and(|tag_type| *tag_type == EnumTagType::Untagged)
            && !self.variants.is_empty()
            && self
                .variants
                .iter()
                .all(|variant| matches!(variant.details, VariantDetails::Item(_)))
    }

    pub(crate) fn check_field_defaults(
        &self,
        types: &BTreeMap<Id, Type<Id>>,
        settings: &crate::settings::Settings,
    ) -> Result<(), Error<Id>>
    where
        Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    {
        self.variants
            .iter()
            .try_for_each(|variant| match &variant.details {
                VariantDetails::Struct(items) => items
                    .iter()
                    .try_for_each(|prop| prop.check_defaults(types, settings)),
                _ => Ok(()),
            })
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
        id: &Id,
        typespace: &TypespaceRenderer<'_, Id>,
        cs: &mut codespace::Codespace,
    ) -> TokenStream {
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
            tag_type,
            variants,
            deny_unknown_fields,
        } = self
        else {
            unreachable!()
        };
        let name = name.as_deref().expect("validated type has a name");
        let tag_type = tag_type.as_ref().expect("validated enum has a tag type");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });

        let name_ident = format_ident!("{name}");

        // Display and FromStr have no derive. An enum that implements
        // them does so through the impls below; each is removed from
        // the trait set as it is rendered so that render_derives, which
        // rejects both, sees only derivable traits.
        let mut derived_traits = traits.clone();
        let all_unit_variants = self.all_tagged_unit_variants();
        let all_item_variants = self.all_untagged_item_variants();

        let special_impls = match (all_unit_variants, all_item_variants) {
            (true, true) => unreachable!(),
            (true, false) => self.render_tagged_unit_variant_impls(
                typespace,
                cs,
                &name_ident,
                &mut derived_traits,
            ),
            (false, true) => self.render_untagged_item_variant_impls(
                typespace,
                cs,
                &name_ident,
                &mut derived_traits,
            ),
            (false, false) => TokenStream::new(),
        };

        // typify's comparison-derive exception checks only that every
        // variant is a unit variant, which an empty variant list
        // satisfies vacuously; it asks nothing about a tag type. That
        // is broader than all_unit_variants, which excludes an empty
        // or untagged enum--exclusions that serve the Display/FromStr
        // impls above, not the derive list.
        // TYPIFY COMPAT: read only by render_derives' exemption.
        let every_variant_is_unit = variants
            .iter()
            .all(|variant| matches!(variant.details, VariantDetails::Unit));

        let serde_derives = SerdeDerives::new(&derived_traits);
        let mut serde = serde_derives.attrs();
        serde.extend(match tag_type {
            EnumTagType::External => Vec::new(),
            EnumTagType::Internal { tag } => vec![quote! { tag = #tag }],
            EnumTagType::Adjacent { tag, content } => {
                vec![quote! { tag = #tag }, quote! { content = #content }]
            }
            EnumTagType::Untagged => vec![quote! { untagged }],
        });

        let variant_from = self.render_variant_from(typespace, &name_ident);

        let (default_impl, unit_default_value) =
            if derived_traits.contains(&TypespaceTrait::Default) {
                let default_value = &default
                    .as_ref()
                    .expect("validated type with Default among its impls must have a valid default")
                    .0;
                let generated_default = generate_default_enum(
                    typespace.types,
                    typespace.settings,
                    &default_value,
                    id.clone(),
                );

                match generated_default {
                    EnumDefault::Value(default_value) => {
                        let default_impl = quote! {
                            impl ::std::default::Default for #name_ident {
                                fn default() -> Self {
                                    #default_value
                                }
                            }
                        };
                        derived_traits.remove(TypespaceTrait::Default);
                        (default_impl, None)
                    }
                    EnumDefault::Variant(variant_name) => (TokenStream::new(), Some(variant_name)),
                }
            } else {
                (TokenStream::new(), None)
            };

        let rendered_variants = variants.iter().map(|variant| {
            let EnumVariant {
                rust_name,
                rename,
                description,
                details,
            } = variant;
            let name = format_ident!("{}", rust_name);
            let mut variant_serde = serde_derives.attrs();
            variant_serde.extend(rename.as_ref().map(|n| quote! { rename = #n }));
            let description = description.as_ref().map(|desc| quote! { #[doc = #desc] });

            let default_attr = (unit_default_value.as_ref() == Some(rust_name)).then(|| {
                quote! { #[default] }
            });

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
                            serde_derives,
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
                #default_attr
                #name #data
            }
        });

        // An unknown field is a deserialization concern, so this one is
        // held back from a Serialize-only type rather than left inert.
        if serde_derives.deserialize() && *deny_unknown_fields {
            serde.push(quote! { deny_unknown_fields });
        }

        let derives_attr =
            typespace.render_derives(&derived_traits, extra_derives, every_variant_is_unit);
        let attrs = typespace.render_attrs(extra_attrs);

        quote! {
            // TODO I want to have the original Id available
            #description
            #( #attrs )*
            #derives_attr
            #serde
            pub enum #name_ident {
                #( #rendered_variants, )*
            }

            #default_impl

            #special_impls
            #( #variant_from )*
        }
    }

    /// Render an all-unit-variant enum's `Display` and `FromStr`.
    ///
    /// These traits **must** be manually implemented.
    fn render_tagged_unit_variant_impls(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        cs: &mut codespace::Codespace,
        name_ident: &Ident,
        derived_traits: &mut TypespaceTraitSet,
    ) -> TokenStream {
        // Both impls map the whole enum to and from a bare string, so
        // every variant has to be payload-free for them to be writable
        // at all.
        assert!(
            self.all_tagged_unit_variants(),
            "{} is not an all-unit-variant enum",
            self.common.built_name(),
        );

        let (variant_idents, variant_names): (Vec<_>, Vec<_>) = self
            .variants
            .iter()
            .map(|variant| (format_ident!("{}", variant.rust_name), variant.json_name()))
            .unzip();

        let display_impl = derived_traits.remove(TypespaceTrait::Display).then(|| {
            // Display each variant as its serialized name.
            quote! {
                impl ::std::fmt::Display for #name_ident {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                        -> ::std::fmt::Result
                    {
                        match *self {
                            #( Self::#variant_idents => f.write_str(#variant_names), )*
                        }
                    }
                }
            }
        });

        let from_str_impl = derived_traits.remove(TypespaceTrait::FromStr).then(|| {
            // Parse each variant from its serialized name.
            typespace.add_error_mod(cs);
            let string_type = typespace.render_std_string();
            quote! {
                impl ::std::str::FromStr for #name_ident {
                    type Err = self::error::ConversionError;

                    fn from_str(value: &str)
                        -> ::std::result::Result<Self, self::error::ConversionError>
                    {
                        match value {
                            #( #variant_names => Ok(Self::#variant_idents), )*
                            _ => Err("invalid value".into()),
                        }
                    }
                }
                impl ::std::convert::TryFrom<&str> for #name_ident {
                    type Error = self::error::ConversionError;

                    fn try_from(value: &str)
                        -> ::std::result::Result<Self, self::error::ConversionError>
                    {
                        value.parse()
                    }
                }
                impl ::std::convert::TryFrom<#string_type> for #name_ident {
                    type Error = self::error::ConversionError;

                    fn try_from(value: #string_type)
                        -> ::std::result::Result<Self, self::error::ConversionError>
                    {
                        value.parse()
                    }
                }
            }
        });

        quote! {
            #display_impl
            #from_str_impl
        }
    }

    fn render_untagged_item_variant_impls(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        cs: &mut codespace::Codespace,
        name_ident: &Ident,
        derived_traits: &mut TypespaceTraitSet,
    ) -> TokenStream {
        let variant_idents = self
            .variants
            .iter()
            .map(|variant| format_ident!("{}", variant.rust_name))
            .collect::<Vec<_>>();

        let from_str_impl = derived_traits.remove(TypespaceTrait::FromStr).then(|| {
            typespace.add_error_mod(cs);
            quote! {
                impl ::std::str::FromStr for #name_ident {
                    type Err = self::error::ConversionError;

                    fn from_str(value: &str) ->
                        ::std::result::Result<Self, self::error::ConversionError>
                    {
                        #(
                            // Try to parse() into each variant.
                            if let Ok(v) = value.parse() {
                                Ok(Self::#variant_idents(v))
                            } else
                        )*
                        {
                            Err("string conversion failed for all variants".into())
                        }
                    }
                }
                impl ::std::convert::TryFrom<&str> for #name_ident {
                    type Error = self::error::ConversionError;

                    fn try_from(value: &str) ->
                        ::std::result::Result<Self, self::error::ConversionError>
                    {
                        value.parse()
                    }
                }
                impl ::std::convert::TryFrom<::std::string::String> for #name_ident {
                    type Error = self::error::ConversionError;

                    fn try_from(value: ::std::string::String) ->
                        ::std::result::Result<Self, self::error::ConversionError>
                    {
                        value.parse()
                    }
                }
            }
        });
        let display_impl = derived_traits.remove(TypespaceTrait::Display).then(|| {
            quote! {
                impl ::std::fmt::Display for #name_ident {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        match self {
                            #(Self::#variant_idents(x) => x.fmt(f),)*
                        }
                    }
                }
            }
        });

        quote! {
            #display_impl
            #from_str_impl
        }
    }

    /// Render a `From<Payload>` impl per `Item` and `Tuple` variant.
    ///
    /// Each converts a payload value into that variant, and the impls
    /// come out in variant declaration order. A `Unit` variant carries
    /// no payload and a `Struct` variant's payload has no type of its
    /// own, so neither gets a From impl.
    fn render_variant_from(
        &self,
        typespace: &TypespaceRenderer<'_, Id>,
        name_ident: &Ident,
    ) -> Vec<TokenStream> {
        // Key each Item and Tuple variant by the rendered form of the types
        // it carries. A key carried by more than one variant yields no impl
        // for any of them, since two `From<Foo> for E` impls would not
        // compile.
        //
        // The key is the rendered form rather than ids because typespace gives
        // each anonymous type its own node, so two variants can carry
        // different ids that render as the same Rust type. It is the rendered
        // form rather than a structural summary because the question is
        // exactly whether two payloads render alike, and any structure
        // faithful enough to answer that is the rendering written a second
        // way, which then has to be kept in agreement with the first.
        // Rendering settles it directly: a set and a vec that share a path, a
        // unit and an empty tuple, a one-element tuple and its element, and a
        // native declared as a container's path all compare equal without a
        // rule for each.
        //
        // TODO the key is the list of payload types, but what decides
        // whether two impls collide is the type each one converts FROM: an
        // Item variant gives `From<X>` and a Tuple gives `From<(X, Y)>`.
        // Those come apart at one element. `Item(X)` and `Tuple([X])` give
        // `From<X>` and `From<(X,)>`, which are different impls wanting
        // different bodies (`Self::V(value)` against `Self::V(value.0)`),
        // yet they key alike here and suppress each other. Keying on the
        // converted-from type would let both render, and the match below
        // already writes the right body for each kind.
        //
        // The reverse collision cannot happen yet: an Item carrying a
        // one-element tuple type also converts from `(Y,)`, so it would
        // clash with a Tuple over that element, but `render_ident_impl`
        // joins tuple elements with a separator and never emits a trailing
        // comma, so typespace cannot render `(Y,)` at all.
        let unique_variants =
            self.variants
                .iter()
                .enumerate()
                .fold(BTreeMap::new(), |mut map, (index, variant)| {
                    let key = match &variant.details {
                        VariantDetails::Item(id) => {
                            vec![typespace.render_ident(id).to_string()]
                        }
                        VariantDetails::Tuple(ids) => ids
                            .iter()
                            .map(|id| typespace.render_ident(id).to_string())
                            .collect::<Vec<_>>(),
                        VariantDetails::Unit | VariantDetails::Struct(_) => return map,
                    };
                    map.entry(key)
                        .and_modify(|seen| *seen = None)
                        .or_insert(Some((index, variant)));
                    map
                });

        // Drop the collisions, then re-key by the variant's index so
        // that the impls follow declaration order.
        unique_variants
            .into_values()
            .flatten()
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .filter_map(|variant| {
                let variant_ident = format_ident!("{}", variant.rust_name);
                match &variant.details {
                    // A bare String payload gets no impl. Core's
                    // blanket `impl<T, U: Into<T>> TryFrom<U> for T`
                    // turns `From<String> for E` into `TryFrom<String>
                    // for E`, which collides with a hand-written
                    // `TryFrom<String>` on the same enum. Excluding
                    // every String payload stands in for asking whether
                    // this enum writes that impl, which is exactly
                    // whether FromStr is in its trait set; checking
                    // that instead would leave the From impl in place
                    // for an enum with a String payload and no FromStr.
                    VariantDetails::Item(id)
                        if matches!(typespace.types.get(id), Some(Type::String)) =>
                    {
                        None
                    }
                    VariantDetails::Item(id) => {
                        let payload = typespace.render_ident(id);
                        Some(quote! {
                            impl ::std::convert::From<#payload> for #name_ident {
                                fn from(value: #payload) -> Self {
                                    Self::#variant_ident(value)
                                }
                            }
                        })
                    }
                    VariantDetails::Tuple(ids) => {
                        let payloads = ids
                            .iter()
                            .map(|id| typespace.render_ident(id))
                            .collect::<Vec<_>>();
                        // A one-element tuple type needs its trailing comma to
                        // be a tuple at all.
                        let payload = match ids.len() {
                            1 => quote! { ( #( #payloads, )* ) },
                            _ => quote! { ( #( #payloads ),* ) },
                        };
                        let field = (0..ids.len()).map(syn::Index::from);
                        Some(quote! {
                            impl ::std::convert::From<#payload> for #name_ident {
                                fn from(value: #payload) -> Self {
                                    Self::#variant_ident( #( value.#field, )* )
                                }
                            }
                        })
                    }
                    VariantDetails::Unit | VariantDetails::Struct(_) => None,
                }
            })
            .collect::<Vec<_>>()
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

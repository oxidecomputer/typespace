// Copyright 2026 Oxide Computer Company

//! Semantic model of Rust types for code generation.
//!
//! The crate is organized around the type lifecycle: consumers assemble
//! types from the [`build`] module's vocabulary, insert them into a
//! [`TypespaceBuilder`], and call [`TypespaceBuilder::finalize`] with
//! [`settings::Settings`] to produce a [`Typespace`]. A finalized
//! typespace renders code via [`Typespace::to_codespace`] and answers
//! queries through the [`view`] module's types.
//!
//! # Dependencies of generated code
//!
//! Rendered code can reference crates that typespace itself does not
//! depend on. Cargo cannot surface these requirements; the crate that
//! contains the generated code must declare them. Which crates are
//! needed depends on the constructs in the output:
//!
//! - [serde](https://crates.io/crates/serde), with the `derive`
//!   feature: required by every generated struct, enum, newtype
//!   struct, unit struct, and tuple struct; each is emitted with serde
//!   derives or hand-written `Serialize`/`Deserialize` impls. Only
//!   output consisting solely of type aliases avoids it.
//! - [serde_json](https://crates.io/crates/serde_json): required if
//!   the output contains a [`build::Type::JsonValue`] (rendered as
//!   `::serde_json::Value`), a property with
//!   [`build::StructPropertyState::DefaultValue`] (the generated default
//!   function calls `::serde_json::from_value`), or a
//!   [`build::UnitStruct`] (its `Deserialize` impl compares input
//!   against the fixed JSON representation).
//! - [json-serde](https://crates.io/crates/json-serde): required if
//!   the output contains any of:
//!   - a property with [`build::StructPropertyState::Optional`] whose
//!     type is not an `Option` (deserialized with
//!     `::json_serde::deserialize_some`, which distinguishes an absent
//!     field from a present one and rejects `null`);
//!   - a property with [`build::StructPropertyState::Optional`] whose
//!     type is an `Option`, when
//!     [`settings::OptionalNullable::DoubleOption`] is selected
//!     (also `::json_serde::deserialize_some`);
//!   - a [`build::TupleStruct`] with a `rest` field (its serde impls use
//!     `::json_serde::FlattenedSequenceSerializer` and
//!     `::json_serde::FlattenedSequenceDeserializer`).
//!
//! Generated code also reproduces, verbatim, every type path the
//! consumer supplies: the `name` of a [`build::Native`] (a converter
//! might inject `uuid::Uuid` or `chrono` types for string formats or
//! `x-rust-type` extensions, as typify does) and the wrapper named by
//! [`settings::OptionalNullable::CustomType`]. The crates
//! behind those paths are dependencies chosen by the consumer that
//! builds the typespace, not by typespace, and the consumer should
//! document them the way typify documents `uuid`, `chrono`, and
//! `regress`. typespace itself emits no reference to `regress` today;
//! that changes when constraint validation rendering lands.

pub mod build;
mod error;
pub mod settings;
pub(crate) mod value_tokens;
pub mod view;

pub use error::TypespaceError;

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::build::{
    Enum, JsonValue, Native, NewtypeStruct, Struct, StructProperty, StructPropertySerde,
    StructPropertyState, TupleStruct, Type, TypeAlias, TypeCommonBuilt, UnitStruct,
};
use crate::settings::{OptionalNullable, Settings, Std};

// 6/25/2025
// I think I need a builder form e.g. of an enum or struct and then the
// finalized form which probably is basically what typify shows today in its
// public interface.

// 7/11/2025
// Thinking through some options on this one. At first I really wanted this to
// be a generic interface that I might be able to use separate from typify. But
// as I got into it, it was kind of a pain in the neck, and hard to keep
// everything straight. So I decided to have it use numeric IDs for the types
// and just map to and from the SchemaRef.
//
// That also kind of sucks because I lose the context of the SchemaRef e.g. if
// I need to report errors. As much as I hate it, I think I should just embed
// SchemaRef everywhere, get all the way through it, and then figure out if I
// can clean up the boundaries.
//
// At a minimum it seems like I need several different forms of a type:
// - Builder -- used to create *de novo* types. It would seem convenient to be
//   able to express these in terms of SchemaRef only. A builder type should be
//   able to (generically) tell you its dependencies. It's not really meant for
//   user interaction beyond that.
// - Internal -- used both before and after finalization; opaque to external
//   consumers. It's where we might incrementally build the thing. (TODO and
//   probably requires a bunch more figuring out)
// - External -- for external consumers of the typify crate e.g. progenitor.
//   This should only work (probably?) for finalized types. But there might be
//   situations where we need to know a little about types before finalization.
//   Something else to consider.

/// A trait that typespace tracks for generated and native types.
///
/// Uses of a type impose trait requirements that
/// [`TypespaceBuilder::finalize`] propagates through the graph: a type
/// used as a map key must implement `Eq`, `PartialEq`, `Ord`, and
/// `PartialOrd`, and so must every type it contains. Generated types
/// absorb propagated requirements and emit the corresponding derives;
/// a [`build::Native`] type must already declare the required traits
/// among its `impls`. A requirement that a type cannot satisfy--`Ord`
/// on a float, say--is a [`TypespaceError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TypespaceTrait {
    Clone,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
    Display,
    FromStr,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
}

impl TypespaceTrait {
    pub(crate) fn render(&self, settings: &Settings) -> proc_macro2::TokenStream {
        if settings.std == Std::FullyQualified {
            match self {
                TypespaceTrait::Clone => quote! { ::std::clone::Clone },
                TypespaceTrait::Debug => quote! { ::std::fmt::Debug },
                TypespaceTrait::Serialize => quote! { ::serde::Serialize },
                TypespaceTrait::Deserialize => quote! { ::serde::Deserialize },
                TypespaceTrait::JsonSchema => quote! { ::schemars::JsonSchema },
                TypespaceTrait::Ord => quote! { ::std::cmp::Ord },
                TypespaceTrait::PartialOrd => quote! { ::std::cmp::PartialOrd },
                TypespaceTrait::Eq => quote! { ::std::cmp::Eq },
                TypespaceTrait::PartialEq => quote! { ::std::cmp::PartialEq },
                TypespaceTrait::Hash => quote! { ::std::hash::Hash },
                TypespaceTrait::Display => quote! { ::std::fmt::Display },
                TypespaceTrait::FromStr => quote! { ::std::str::FromStr },
            }
        } else {
            match self {
                TypespaceTrait::Clone => quote! { Clone },
                TypespaceTrait::Debug => quote! { Debug },
                TypespaceTrait::Serialize => quote! { ::serde::Serialize },
                TypespaceTrait::Deserialize => quote! { ::serde::Deserialize },
                TypespaceTrait::JsonSchema => quote! { ::schemars::JsonSchema },
                TypespaceTrait::Ord => quote! { Ord },
                TypespaceTrait::PartialOrd => quote! { PartialOrd },
                TypespaceTrait::Eq => quote! { Eq },
                TypespaceTrait::PartialEq => quote! { PartialEq },
                TypespaceTrait::Hash => quote! { Hash },
                TypespaceTrait::Display => quote! { Display },
                TypespaceTrait::FromStr => quote! { FromStr },
            }
        }
    }
}

/// An unordered collection of [`TypespaceTrait`] values.
///
/// Used, for example, for the traits a [`build::Native`] type declares
/// that it implements. Build one with [`TypespaceTraitSet::empty`] and
/// [`TypespaceTraitSet::add`], or collect from an iterator of traits.
#[derive(Debug, Clone)]
pub struct TypespaceTraitSet(BTreeSet<TypespaceTrait>);

impl FromIterator<TypespaceTrait> for TypespaceTraitSet {
    fn from_iter<T: IntoIterator<Item = TypespaceTrait>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for TypespaceTraitSet {
    type Item = TypespaceTrait;
    type IntoIter = std::collections::btree_set::IntoIter<TypespaceTrait>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl TypespaceTraitSet {
    pub fn empty() -> Self {
        Self(Default::default())
    }

    pub fn contains(&self, tt: &TypespaceTrait) -> bool {
        self.0.contains(tt)
    }
    pub fn add(&mut self, tt: TypespaceTrait) {
        self.0.insert(tt);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &TypespaceTrait> {
        self.0.iter()
    }

    pub fn difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = &'a TypespaceTrait> + 'a {
        self.0.difference(&other.0)
    }
}

/// Identifies a trait implementation that typespace is aware of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeSpaceImpl {
    Display,
    FromStr,
}

/// Accumulates the type graph prior to finalization.
///
/// Insert every type--each named type along with every built-in and
/// container type it references--under a caller-chosen ID with
/// [`TypespaceBuilder::insert`], then call
/// [`TypespaceBuilder::finalize`] to validate the graph and produce a
/// [`Typespace`].
pub struct TypespaceBuilder<Id> {
    types: BTreeMap<Id, Type<Id>>,
}

impl<Id> Default for TypespaceBuilder<Id> {
    fn default() -> Self {
        Self {
            types: Default::default(),
        }
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceBuilder<Id> {
    /// Add a type under the given ID.
    ///
    /// The IDs that `typ` refers to need not be present yet, but each
    /// must be inserted before [`finalize`](Self::finalize) is called.
    /// Fails with [`TypespaceError::DuplicateTypeId`] if a type with
    /// this ID was already inserted, and with
    /// [`TypespaceError::EmptyTypeName`] if `typ` is a named type whose
    /// name is empty.
    pub fn insert(&mut self, id: Id, typ: Type<Id>) -> Result<(), TypespaceError<Id>> {
        // Rendering interpolates the name of every named type into an
        // identifier; an empty name would panic there, so reject it here
        // where we can name the offending ID.
        if let Some(common) = typ.common() {
            if common.name.is_empty() {
                return Err(TypespaceError::EmptyTypeName { type_id: id });
            }
        }
        match self.types.entry(id) {
            Entry::Vacant(e) => {
                e.insert(typ);
                Ok(())
            }
            Entry::Occupied(e) => {
                // Duplicate insertions are a caller error.
                Err(TypespaceError::DuplicateTypeId {
                    type_id: e.key().clone(),
                })
            }
        }
    }

    /// Whether a type has already been inserted under the given ID.
    pub fn contains_type(&self, id: &Id) -> bool {
        self.types.contains_key(id)
    }

    /// Finalize the typespace.
    ///
    /// Verifies that every ID referenced by a type names an inserted
    /// type (a dangling reference is a
    /// [`TypespaceError::UnknownTypeId`]), breaks containment cycles by
    /// inserting `Box` types, and propagates trait requirements through
    /// the graph--a type used as a map key must be `Ord`, and so must
    /// everything it contains. A trait requirement that a type cannot
    /// satisfy is an error naming the offending ID.
    ///
    /// `make_box_id` is called to generate a fresh ID for each `Box<T>`
    /// wrapper inserted to break a containment cycle. The argument is the ID
    /// of the inner type being wrapped. Pass [`no_cycles`] to assert
    /// that the graph contains no containment cycles.
    pub fn finalize<F>(
        self,
        settings: Settings,
        make_box_id: F,
    ) -> Result<Typespace<Id>, TypespaceError<Id>>
    where
        F: FnMut(&Id) -> Id,
    {
        // Basic steps:
        // 1. Break containment cycles with Box types
        // 2. Propagate trait impls
        // 3. Type-specific finalization

        let Self { mut types } = self;

        // Verify that every type ID referenced by another type is actually
        // present; subsequent steps rely on lookups of child IDs succeeding.
        for (type_id, typ) in &types {
            for child_id in typ.children() {
                if !types.contains_key(&child_id) {
                    return Err(TypespaceError::UnknownTypeId {
                        type_id: type_id.clone(),
                        child_id,
                    });
                }
            }
        }

        build_commons(&mut types);
        break_cycles(&mut types, make_box_id);
        push_traits(&mut types)?;

        Ok(Typespace { types, settings })
    }
}

/// A `make_box_id` argument for [`TypespaceBuilder::finalize`] that
/// asserts the type graph contains no containment cycles: it panics if
/// finalization ever needs to insert a `Box`.
pub fn no_cycles<Id>(_: &Id) -> Id {
    panic!("unexpected cycle in typespace")
}

/// A finalized, validated collection of types.
///
/// Produced by [`TypespaceBuilder::finalize`]. Render every named type
/// with [`Typespace::to_codespace`], or inspect individual types
/// without rendering via [`Typespace::get_type`] and
/// [`Typespace::iter_types`].
pub struct Typespace<Id> {
    pub(crate) types: BTreeMap<Id, Type<Id>>,
    /// The settings supplied at finalization, which govern rendering.
    pub settings: Settings,
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Typespace<Id> {
    /// Look up a type by its id.
    ///
    /// # Panics
    ///
    /// Panics if `id` does not name a type in the typespace; every ID
    /// accepted at insert time (plus the box IDs generated during
    /// finalization) is valid.
    pub fn get_type(&self, id: &Id) -> view::Type<'_, Id> {
        let (id, typ) = self.types.get_key_value(id).expect("invalid type id");
        view::Type {
            typespace: self,
            id,
            typ,
        }
    }

    /// Iterate over all types in the typespace.
    pub fn iter_types(&self) -> impl Iterator<Item = view::Type<'_, Id>> {
        self.types.iter().map(|(id, typ)| view::Type {
            typespace: self,
            id,
            typ,
        })
    }

    /// Render every named type into a [`codespace::Codespace`].
    ///
    /// Each named type becomes one item keyed by its name; generated
    /// helper functions (serde default functions, for example) are
    /// routed to their own modules. Output is deterministic and
    /// unformatted; turning the codespace into a token stream or files
    /// is the caller's job from here.
    pub fn to_codespace(&self) -> codespace::Codespace {
        TypespaceRenderer {
            types: &self.types,
            settings: &self.settings,
        }
        .render()
    }
}

pub(crate) struct TypespaceRenderer<'a, Id> {
    pub(crate) types: &'a BTreeMap<Id, Type<Id>>,
    pub(crate) settings: &'a Settings,
}

impl<'a, Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceRenderer<'a, Id> {
    fn render(&self) -> codespace::Codespace {
        let mut cs = codespace::Codespace::default();

        for typ in self.types.values() {
            match typ {
                Type::Struct(s) => {
                    let name = s.common.name.clone();
                    let tokens = s.render(self, &mut cs);
                    cs.add_item(name, tokens);
                }
                Type::Enum(e) => {
                    let name = e.common.name.clone();
                    let tokens = e.render(self, &mut cs);
                    cs.add_item(name, tokens);
                }
                Type::UnitStruct(u) => {
                    let name = u.common.name.clone();
                    cs.add_item(name, u.render());
                }
                Type::TupleStruct(t) => {
                    let name = t.common.name.clone();
                    cs.add_item(name, t.render(self));
                }
                Type::NewtypeStruct(n) => {
                    let name = n.common.name.clone();
                    cs.add_item(name, n.render(self));
                }
                Type::TypeAlias(a) => {
                    let name = a.common.name.clone();
                    cs.add_item(name, a.render(self));
                }
                _ => {}
            }
        }

        cs
    }

    pub(crate) fn render_ident(&self, id: &Id) -> TokenStream {
        self.render_ident_impl(id, None, false)
    }

    pub(crate) fn render_ident_with_scope(&self, id: &Id, scope: Option<&str>) -> TokenStream {
        self.render_ident_impl(id, scope, false)
    }

    pub(crate) fn render_raw_type(&self, id: &Id) -> TokenStream {
        self.render_ident_impl(id, None, true)
    }

    pub(crate) fn render_ident_impl(
        &self,
        id: &Id,
        scope: Option<&str>,
        base_type: bool,
    ) -> TokenStream {
        let ty = self.types.get(id).unwrap();
        match ty {
            Type::Enum(Enum { common, .. })
            | Type::Struct(Struct { common, .. })
            | Type::UnitStruct(UnitStruct { common, .. })
            | Type::TupleStruct(TupleStruct { common, .. })
            | Type::NewtypeStruct(NewtypeStruct { common, .. })
            | Type::TypeAlias(TypeAlias { common, .. }) => {
                let name = &common.name;
                let name_ident = format_ident!("{name}");

                if let Some(scope) = scope {
                    let scope_ident = format_ident!("{scope}");
                    quote! { #scope_ident::#name_ident }
                } else {
                    name_ident.into_token_stream()
                }
            }

            Type::Native(Native {
                name, parameters, ..
            }) => {
                let name_ident = syn::parse_str::<syn::TypePath>(name).unwrap();
                let parameters = (!base_type && !parameters.is_empty()).then(|| {
                    let parameter_idents = parameters
                        .iter()
                        .map(|param_id| self.render_ident_with_scope(param_id, scope));
                    quote! {
                        < #( #parameter_idents ),* >
                    }
                });
                quote! {
                    #name_ident #parameters
                }
            }

            Type::Array(schema_ref, n) => {
                let inner_ident = self.render_ident_with_scope(schema_ref, scope);
                quote! {
                    [#inner_ident; #n]
                }
            }
            Type::Tuple(schema_refs) => {
                let inner_idents = schema_refs
                    .iter()
                    .map(|id| self.render_ident_with_scope(id, scope));
                quote! {
                    ( #( #inner_idents ),* )
                }
            }

            Type::Option(option_id) => {
                let option_type = match &self.settings.std {
                    Std::FullyQualified => quote! { ::std::option::Option },
                    Std::Unqualified => quote! { Option },
                };
                if base_type {
                    option_type
                } else {
                    let option_ident = self.render_ident_with_scope(option_id, scope);
                    quote! {
                        #option_type<#option_ident>
                    }
                }
            }
            Type::Box(boxed_id) => {
                let box_type = match &self.settings.std {
                    Std::FullyQualified => quote! { ::std::boxed::Box },
                    Std::Unqualified => quote! { Box },
                };
                if base_type {
                    box_type
                } else {
                    let boxed_ident = self.render_ident_with_scope(boxed_id, scope);
                    quote! {
                        #box_type<#boxed_ident>
                    }
                }
            }
            Type::Set(inner_id) => {
                // TODO 3/25/2026
                // Replace with set type
                let vec_type = match &self.settings.std {
                    Std::FullyQualified => quote! { ::std::vec::Vec },
                    Std::Unqualified => quote! { Vec },
                };
                if base_type {
                    vec_type
                } else {
                    let inner_ident = self.render_ident_with_scope(inner_id, scope);
                    quote! {
                        #vec_type<#inner_ident>
                    }
                }
            }
            Type::Vec(inner_id) => {
                // TODO 3/25/2026
                // Make configurable?
                let vec_type = match &self.settings.std {
                    Std::FullyQualified => quote! { ::std::vec::Vec },
                    Std::Unqualified => quote! { Vec },
                };
                if base_type {
                    vec_type
                } else {
                    let inner_ident = self.render_ident_with_scope(inner_id, scope);
                    quote! {
                        #vec_type<#inner_ident>
                    }
                }
            }
            Type::Map(key_id, value_id) => {
                // TODO 3/25/2026
                // Configurable like typify 1
                let map_type = quote! { ::std::collections::BTreeMap };
                if base_type {
                    map_type
                } else {
                    let key_ident = self.render_ident_with_scope(key_id, scope);
                    let value_ident = self.render_ident_with_scope(value_id, scope);
                    quote! {
                        #map_type<#key_ident, #value_ident>
                    }
                }
            }
            Type::Boolean => quote! { bool },
            Type::Integer(name) | Type::Float(name) => syn::parse_str::<syn::TypePath>(name)
                .unwrap()
                .to_token_stream(),
            Type::String => match &self.settings.std {
                Std::FullyQualified => quote! { ::std::string::String },
                Std::Unqualified => quote! { String },
            },
            Type::JsonValue => quote! { ::serde_json::Value },
            Type::Unit => quote! { () },
        }
    }

    pub(crate) fn render_struct_property(
        &self,
        StructProperty {
            rust_name,
            json_name,
            state,
            description,
            type_id,
        }: &StructProperty<Id>,
        vis_pub: bool,
        context: &str,
        cs: &mut codespace::Codespace,
    ) -> TokenStream {
        let description = description.as_ref().map(|text| {
            quote! {
                #[doc = #text]
            }
        });

        let mut serde_options = Vec::new();

        match json_name {
            StructPropertySerde::None => {}
            StructPropertySerde::Rename(s) => {
                serde_options.push(quote! {
                    rename = #s
                });
            }
            StructPropertySerde::Flatten => {
                serde_options.push(quote! {
                    flatten
                });
            }
        };

        let ty = self.types.get(type_id).unwrap();

        // If the type is itself an Option (i.e. may be null), let's save the
        // alternative (i.e. non-null) type, which we may use i.e. if the field
        // may be absent and the consumer has specified a custom type for that
        // situation. In other cases, we need to know if the type is an Option--
        // even if we don't need to know the identity of the inner type.
        let maybe_option_type = if let Type::Option(id) = ty {
            Some(id)
        } else {
            None
        };

        let ty_ident = self.render_ident(type_id);

        let std_opt_type = match &self.settings.std {
            Std::FullyQualified => quote! { ::std::option::Option },
            Std::Unqualified => quote! { Option },
        };
        let std_opt_is_none = format!("{std_opt_type}::is_none");

        let prop_ty_ident = match (state, maybe_option_type) {
            // A required field needs no serde annotations.
            (StructPropertyState::Required, None) => ty_ident,

            // A required field that is an Option<T> needs a custom
            // deserializer so that the field is mandatory, but may be null;
            // without this attribute, the default handling is to permit
            // either.
            (StructPropertyState::Required, Some(_)) => {
                let opt_deserialize = format!("{std_opt_type}::deserialize");
                // TODO schemars schema_with?
                serde_options.push(quote! { deserialize_with = #opt_deserialize });
                ty_ident
            }

            // An optional field that is not an Option<T> may not be null; we
            // use the json::serde::deserialize_some function to enforce this.
            (StructPropertyState::Optional, None) => {
                serde_options.push(quote! { default });
                serde_options.push(quote! {
                    deserialize_with = "::json_serde::deserialize_some"
                });
                serde_options.push(quote! { skip_serializing_if = #std_opt_is_none });
                // TODO schemars schema_with

                quote! {
                    #std_opt_type<#ty_ident>
                }
            }

            // An optional field that is also an Option<T> may be the type
            // value, null, or absent. Customizable settings determine the
            // handling of this.
            (StructPropertyState::Optional, Some(inner_id)) => {
                match &self.settings.optional_nullable {
                    OptionalNullable::ConflateAsAbsent => {
                        serde_options.push(quote! { skip_serializing_if = #std_opt_is_none });
                        ty_ident
                    }
                    OptionalNullable::ConflateAsNull => {
                        // We always serialize--including `None` as `null`--so
                        // no serde options are necessary.
                        ty_ident
                    }
                    OptionalNullable::DoubleOption => {
                        serde_options.push(quote! { default });
                        serde_options.push(quote! {
                            deserialize_with = "::json_serde::deserialize_some"
                        });
                        serde_options.push(quote! {
                            skip_serializing_if = #std_opt_is_none
                        });

                        quote! {
                            #std_opt_type<#ty_ident>
                        }
                    }
                    OptionalNullable::CustomType(custom_type_name) => {
                        let custom_type_path =
                            syn::parse_str::<syn::TypePath>(custom_type_name).unwrap();
                        serde_options.push(quote! { default });
                        let custom_is_absent = format!("{}::is_absent", custom_type_name);
                        serde_options.push(quote! { skip_serializing_if = #custom_is_absent });

                        let inner_ident = self.render_ident(inner_id);

                        quote! {
                            #custom_type_path<#inner_ident>
                        }
                    }
                }
            }
            (StructPropertyState::Default, _) => {
                serde_options.push(quote! { default });
                self.render_struct_property_add_skip(
                    &mut serde_options,
                    type_id,
                    ty,
                    std_opt_is_none,
                );

                ty_ident
            }
            (StructPropertyState::DefaultValue(JsonValue(value)), _) => {
                let fn_name_str = format!("{}__{}", context, rust_name);
                let fn_name_ident = format_ident!("{}", fn_name_str);
                let serde_path = format!("defaults::{fn_name_str}");
                serde_options.push(quote! { default = #serde_path });

                let ty_for_fn = self.render_ident_with_scope(type_id, Some("super"));
                let value_tokens = crate::value_tokens::value_tokens(value);
                cs.get_root_mod().get_mod("defaults").add_item(
                    &fn_name_str,
                    quote! {
                        pub fn #fn_name_ident() -> #ty_for_fn {
                            ::serde_json::from_value(#value_tokens)
                                .expect("invalid default value")
                        }
                    },
                );

                ty_ident
            }
        };

        let serde = (!serde_options.is_empty()).then(|| {
            quote! {
                #[serde(
                    #( #serde_options ),*
                )]
            }
        });
        let vis_pub = vis_pub.then(|| quote! { pub });

        quote! {
            #description
            #serde
            #vis_pub #rust_name: #prop_ty_ident
        }
    }

    fn render_struct_property_add_skip(
        &self,
        serde_options: &mut Vec<TokenStream>,
        ty_id: &Id,
        ty: &Type<Id>,
        std_opt_is_none: String,
    ) {
        match ty {
            // Here we assume that the generated type for the field has a
            // implementation of Default. There isn't a simple "is_default()"
            // that we can presume... so we'll just leave it.
            Type::Enum(_)
            | Type::Struct(_)
            | Type::UnitStruct(_)
            | Type::TupleStruct(_)
            | Type::NewtypeStruct(_)
            | Type::TypeAlias(_) => {}

            // The same applies to external types.
            Type::Native(_) => {}

            Type::Option(_) => {
                // We have a property whose type is an Option meaning that it
                // may have a value of null. The "Default" state means that it
                // takes on its "intrinsic" default value if the field is
                // absent. This means we can ignore any of the
                // optional/nullable settings as they don't particularly apply
                // here.
                //
                // Note that #[serde(default)] is a no-op for Option<T>.
                serde_options.push(quote! { skip_serializing_if = #std_opt_is_none });
            }
            Type::Box(boxed_id) => {
                let boxed_ty = self.types.get(boxed_id).unwrap();
                self.render_struct_property_add_skip(
                    serde_options,
                    boxed_id,
                    boxed_ty,
                    std_opt_is_none,
                );
            }

            Type::Vec(_) | Type::Map(_, _) | Type::Set(_) | Type::String => {
                let ty_raw_ident = self.render_raw_type(ty_id);
                let is_empty = format!("{ty_raw_ident}::is_empty");
                serde_options.push(quote! { skip_serializing_if = #is_empty });
            }

            // As above, sure--there might be a Default impl--but we don't have
            // a way to check if the value matches that value, so... whatever.
            Type::Array(_, _) | Type::Tuple(_) => {}

            Type::Unit => {
                // There's only one value for the unit type, so I guess we can
                // unconditionally skip serializing it.
                serde_options.push(quote! { skip });
            }

            Type::Boolean => {
                // Congratulation! You found an external expression of my
                // insanity. I am genuinely curious if anyone will ever
                // encounter this via a generated type. Note that this will
                // cause invalid code to be generated e.g. if the type is
                // Box<bool>, and I'm fine with that.

                serde_options.push(quote! {
                    skip_serializing_if = "std::ops::Not::not"
                });
            }

            // There isn't an "is_zero()" so... we'll just leave it be.
            Type::Integer(_) | Type::Float(_) => {}

            // This isn't a runtime error that could be handled; it's a
            // programming error.
            Type::JsonValue => panic!("Default value for JsonValue is not supported"),
        }
    }
}

/// Initialize `TypeCommonBuilt` for every named type before trait propagation.
fn build_commons<Id: Clone>(types: &mut BTreeMap<Id, Type<Id>>) {
    for typ in types.values_mut() {
        if let Some(common) = typ.common_mut() {
            common.built = Some(TypeCommonBuilt {
                traits: TypespaceTraitSet::empty(),
            });
        }
    }
}

fn break_cycles<Id, F>(types: &mut BTreeMap<Id, Type<Id>>, mut make_box_id: F)
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
    F: FnMut(&Id) -> Id,
{
    enum Node<Id> {
        Start { type_id: Id },
        Processing { type_id: Id, children_ids: Vec<Id> },
    }

    let mut visited = BTreeSet::<Id>::new();

    for type_id in types.keys().cloned().collect::<Vec<_>>() {
        if visited.contains(&type_id) {
            continue;
        }

        let mut active = BTreeSet::<Id>::new();
        let mut stack = Vec::<Node<Id>>::new();

        active.insert(type_id.clone());
        stack.push(Node::Start { type_id });

        while let Some(top) = stack.last_mut() {
            match top {
                // Skip right to the end since we've already seen this type.
                Node::Start { type_id } if visited.contains(type_id) => {
                    assert!(active.contains(type_id));

                    let type_id = type_id.clone();
                    *top = Node::Processing {
                        type_id,
                        children_ids: Vec::new(),
                    };
                }

                // Break any immediate cycles and queue up this type for
                // descent into its child types.
                Node::Start { type_id } => {
                    assert!(active.contains(type_id));

                    visited.insert(type_id.clone());

                    // Determine which child types form cycles--and
                    // therefore need to be snipped--and the rest--into
                    // which we should descend. We make this its own block
                    // to clarify the lifetime of the exclusive reference
                    // to the type. We don't really *need* to have an
                    // exclusive reference here, but there's no point in
                    // writing `get_child_ids` again for shared references.
                    let (snip, descend) = {
                        let typ = types.get_mut(type_id).unwrap();

                        let child_ids = typ
                            .contained_children_mut()
                            .into_iter()
                            .map(|child_id| child_id.clone());

                        // If the child type is in active then we've found
                        // a cycle (otherwise we'll descend).
                        child_ids.partition::<Vec<_>, _>(|child_id| active.contains(child_id))
                    };

                    // Note that while `snip` might contain duplicates,
                    // `id_to_box` is idempotent insofar as the same input
                    // TypeId will result in the same output TypeId. Ergo
                    // the resulting pairs from which we construct the
                    // mapping would contain exact duplicates; it would not
                    // contain two values associated with the same key.
                    let replace = snip
                        .into_iter()
                        .map(|type_id| {
                            let box_id = make_box_id(&type_id);
                            let box_typ = Type::Box(type_id.clone());
                            types.insert(box_id.clone(), box_typ);

                            (type_id, box_id)
                        })
                        .collect::<BTreeMap<Id, Id>>();

                    // Break any cycles by reassigning the child type to a box.
                    let typ = types.get_mut(type_id).unwrap();

                    let child_ids = typ.contained_children_mut();
                    for child_id in child_ids {
                        if let Some(replace_id) = replace.get(child_id) {
                            *child_id = replace_id.clone();
                        }
                    }

                    // Descend into child types.
                    let node = Node::Processing {
                        type_id: type_id.clone(),
                        children_ids: descend,
                    };
                    *top = node;
                }
                Node::Processing {
                    type_id,
                    children_ids: children,
                } => {
                    if let Some(child) = children.pop() {
                        active.insert(child.clone());
                        stack.push(Node::Start { type_id: child });
                    } else {
                        let type_id = type_id.clone();
                        active.remove(&type_id);
                        stack.pop();
                    }
                }
            }
        }
    }
}

fn push_traits<Id>(types: &mut BTreeMap<Id, Type<Id>>) -> Result<(), TypespaceError<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    // First, look through all types to determine what traits are required of
    // various children.
    let mut work = types
        .values()
        .filter_map(|ty| match ty {
            // TODO 3/31/2026
            // need to check map settings
            Type::Map(key_schema_ref, _) => Some((
                key_schema_ref.clone(),
                [
                    TypespaceTrait::Eq,
                    TypespaceTrait::PartialEq,
                    TypespaceTrait::Ord,
                    TypespaceTrait::PartialOrd,
                ]
                .into_iter()
                .collect::<TypespaceTraitSet>(),
            )),
            // TODO 3/31/2026
            // This is going to depend on what specific type we're using for a
            // set.
            // Type::Set(_) => todo!(),
            _ => None,
        })
        .collect::<VecDeque<_>>();

    // In each iteration, we need to assert the set of required traits to the
    // current type. If the current type is generated, that means adding the
    // traits and pushing children. If the type is **not** generated (native or
    // otherwise external to our control), we need to check that is implements
    // (or is capable of implementing) the required traits; if it doesn't (or
    // can't), we'll produce an error. We don't stop on the first failure, but
    // want to identify as many, distinct failures as is reasonable and as
    // would be useful for a consumer.
    while let Some((schema_ref, traits)) = work.pop_front() {
        let ty = types.get_mut(&schema_ref).unwrap();

        let common_built = match ty {
            Type::NewtypeStruct(NewtypeStruct { common, .. })
            | Type::Enum(Enum { common, .. })
            | Type::Struct(Struct { common, .. }) => Some(common.built.as_mut().unwrap()),
            Type::UnitStruct(_) => todo!(),
            Type::TupleStruct(_) => todo!(),
            Type::TypeAlias(_) => todo!(),

            _ => None,
        };

        if let Some(common) = common_built {
            let built_traits = &mut common.traits;
            // Collect the traits that this type doesn't already have.
            let mut new_traits = TypespaceTraitSet::empty();

            for trait_name in traits {
                if !built_traits.contains(&trait_name) {
                    built_traits.add(trait_name);
                    new_traits.add(trait_name);
                }
            }

            if !new_traits.is_empty() {
                for child_id in ty.contained_children_mut() {
                    work.push_back((child_id.clone(), new_traits.clone()));
                }
            }
        } else {
            match ty {
                Type::Enum(_)
                | Type::Struct(_)
                | Type::UnitStruct(_)
                | Type::TupleStruct(_)
                | Type::NewtypeStruct(_)
                | Type::TypeAlias(_) => unreachable!(),

                Type::Native(Native { name, impls, .. }) => {
                    let missing_traits = traits
                        .difference(impls)
                        .cloned()
                        .collect::<TypespaceTraitSet>();
                    if !missing_traits.is_empty() {
                        todo!(
                            "missing traits {:#?} for native type {name}",
                            missing_traits,
                        );
                    }
                }

                // Pass the buck...
                Type::Option(schema_ref) | Type::Box(schema_ref) => {
                    work.push_back((schema_ref.clone(), traits));
                }

                // Vec<T> and arrays impl everything we care about--except for
                // Display and FromStr--as long as T implemented them.
                Type::Vec(schema_ref) | Type::Array(schema_ref, _) => {
                    if traits.contains(&TypespaceTrait::Display)
                        || traits.contains(&TypespaceTrait::FromStr)
                    {
                        todo!();
                    }
                    work.push_back((schema_ref.clone(), traits));
                }
                // Tuples implement everything except for Display and FromStr
                // as long as all their component types do as well.
                Type::Tuple(schema_refs) => {
                    if traits.contains(&TypespaceTrait::Display)
                        || traits.contains(&TypespaceTrait::FromStr)
                    {
                        todo!();
                    }
                    for schema_ref in schema_refs {
                        work.push_back((schema_ref.clone(), traits.clone()));
                    }
                }

                Type::Map(_, _) | Type::Set(_) => todo!("wtf {schema_ref} {:#?}", ty),

                // TODO 3/31/2026
                // Comment and do better
                Type::Float(name) => {
                    let missing = [
                        TypespaceTrait::Ord,
                        TypespaceTrait::Eq,
                        TypespaceTrait::Hash,
                    ]
                    .into_iter()
                    .filter(|tt| traits.contains(tt))
                    .collect::<Vec<_>>();
                    if !missing.is_empty() {
                        return Err(TypespaceError::FloatTraits {
                            type_id: schema_ref,
                            name: name.clone(),
                            missing,
                        });
                    }
                }

                // These all implement all the traits we care about so there's
                // nothing to do.
                Type::Unit | Type::Boolean | Type::Integer(_) | Type::String => (),

                // JsonValue implements everything except for Eq, Ord,
                // PartialOrd, and Hash.
                Type::JsonValue => {
                    let missing = [
                        TypespaceTrait::Eq,
                        TypespaceTrait::Ord,
                        TypespaceTrait::PartialOrd,
                        TypespaceTrait::Hash,
                    ]
                    .into_iter()
                    .filter(|tt| traits.contains(tt))
                    .collect::<Vec<_>>();
                    if !missing.is_empty() {
                        return Err(TypespaceError::JsonValueTraits {
                            type_id: schema_ref,
                            missing,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

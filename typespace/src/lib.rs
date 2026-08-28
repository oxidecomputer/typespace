// Copyright 2026 Oxide Computer Company

//! Semantic model of Rust types for code generation.
//!
//! The crate is organized around the type lifecycle: consumers create a
//! [`TypespaceBuilder`] from [`settings::Settings`], assemble types
//! from the [`build`] module's vocabulary, insert them, and call
//! [`TypespaceBuilder::finalize`] to produce a [`Typespace`]. A finalized
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
//!     `::json_serde::FlattenedSequenceDeserializer`);
//!   - a [`build::Type::Never`] (rendered as `::json_serde::Absent`).
//!
//!   The `::json_serde` path itself follows
//!   [`settings::Settings::with_json_serde_crate`], for consumers that
//!   re-export the crate under another name.
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
pub mod error;
pub mod settings;
pub(crate) mod trait_resolution;
pub(crate) mod value_tokens;
pub mod view;

// Binds the name `typespace` to this crate itself, so the absolute
// `::typespace::...` paths that `typespace_builder!` emits (see
// typespace-test-macro's builder module) resolve from any module in
// this crate, including nested `#[cfg(test)]` modules, exactly as
// they would from an external crate depending on `typespace`.
extern crate self as typespace;

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::build::{
    Enum, JsonValue, Native, NewtypeStruct, Struct, StructProperty, StructPropertySerde,
    StructPropertyState, TupleStruct, Type, TypeAlias, TypeCommonBuilt, UnitStruct,
};
use crate::error::Error;
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
/// on a float, say--is a [`error::Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
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
    Default,
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
                TypespaceTrait::Default => quote! { ::std::default::Default },
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
                TypespaceTrait::Default => quote! { Default },
            }
        }
    }
}

impl std::fmt::Display for TypespaceTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            TypespaceTrait::Clone => "Clone",
            TypespaceTrait::Debug => "Debug",
            TypespaceTrait::Serialize => "Serialize",
            TypespaceTrait::Deserialize => "Deserialize",
            TypespaceTrait::JsonSchema => "JsonSchema",
            TypespaceTrait::Display => "Display",
            TypespaceTrait::FromStr => "FromStr",
            TypespaceTrait::Eq => "Eq",
            TypespaceTrait::PartialEq => "PartialEq",
            TypespaceTrait::Ord => "Ord",
            TypespaceTrait::PartialOrd => "PartialOrd",
            TypespaceTrait::Hash => "Hash",
            TypespaceTrait::Default => "Default",
        };
        f.write_str(name)
    }
}

/// An unordered collection of [`TypespaceTrait`] values.
///
/// Used, for example, for the traits a [`build::Native`] type declares
/// that it implements. Build one with [`TypespaceTraitSet::empty`] and
/// [`TypespaceTraitSet::add`], or collect from an iterator of traits.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
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

    pub fn remove(&mut self, tt: TypespaceTrait) -> bool {
        self.0.remove(&tt)
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
///
/// This is the query vocabulary for
/// [`view::Type::has_impl`](crate::view::Type::has_impl). The
/// comparison and hashing entries let a consumer ask whether a type is
/// usable as a map key--and let converters declare that capability for
/// native types (via [`build::Native`] impls) so that a native type
/// used as a map key without declaring `Ord` is a reportable conflict
/// rather than a mystery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeSpaceImpl {
    Display,
    FromStr,
    Eq,
    Ord,
    Hash,
}

/// Accumulates the type graph prior to finalization.
///
/// Create one from [`settings::Settings`] with
/// [`TypespaceBuilder::new`] (or [`TypespaceBuilder::default`] for
/// default settings), insert every type--each named type along with
/// every built-in and container type it references--under a
/// caller-chosen ID with [`TypespaceBuilder::insert`], then call
/// [`TypespaceBuilder::finalize`] to validate the graph and produce a
/// [`Typespace`].
pub struct TypespaceBuilder<Id> {
    types: BTreeMap<Id, Type<Id>>,
    settings: Settings,
}

impl<Id> Default for TypespaceBuilder<Id> {
    /// A builder with default [`settings::Settings`].
    fn default() -> Self {
        Self::new(Settings::typical())
    }
}

impl<Id> TypespaceBuilder<Id> {
    /// Create a builder whose finalization and rendering are governed
    /// by `settings`.
    pub fn new(settings: Settings) -> Self {
        Self {
            types: Default::default(),
            settings,
        }
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceBuilder<Id> {
    /// Add a type under the given ID.
    ///
    /// The IDs that `typ` refers to need not be present yet, but each
    /// must be inserted before [`finalize`](Self::finalize) is called.
    /// Fails with [`error::Error::DuplicateTypeId`] if a type with
    /// this ID was already inserted, and re-runs the shape checks that
    /// `build()` applies (rejecting, for example, a shape value smuggled
    /// into a [`Type`] variant without being built).
    pub fn insert(&mut self, id: Id, typ: Type<Id>) -> Result<(), Error<Id>> {
        // The shapes' build() methods validate names, but Type's
        // variants are not sealed against direct construction; re-run
        // the checks here so no unvalidated shape can enter the
        // typespace.
        typ.validate_built()?;
        match self.types.entry(id) {
            Entry::Vacant(e) => {
                e.insert(typ);
                Ok(())
            }
            Entry::Occupied(e) => {
                // Duplicate insertions are a caller error.
                Err(Error::DuplicateTypeId {
                    type_id: e.key().clone(),
                })
            }
        }
    }

    /// Whether a type has already been inserted under the given ID.
    pub fn contains_type(&self, id: &Id) -> bool {
        self.types.contains_key(id)
    }

    /// Render the Rust identifier of an inserted type before
    /// finalization.
    ///
    /// Rendering honors the builder's settings (container overrides,
    /// `std` spelling). Finalization-only effects are necessarily
    /// absent: no cycle-breaking boxes exist yet.
    ///
    /// # Panics
    ///
    /// Panics if `id`--or any type ID it references transitively
    /// through container types--has not been inserted.
    pub fn ident(&self, id: &Id) -> TokenStream {
        self.renderer().render_ident(id)
    }

    /// Like [`TypespaceBuilder::ident`], with named types qualified by
    /// the module `scope`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`TypespaceBuilder::ident`].
    pub fn ident_in(&self, id: &Id, scope: &str) -> TokenStream {
        self.renderer().render_ident_with_scope(id, Some(scope))
    }

    /// Render the identifier of an inserted type as a function
    /// parameter type, before finalization.
    ///
    /// Complex owned types are prefixed with `&`; simple types
    /// (primitives and options) are unchanged.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`TypespaceBuilder::ident`].
    pub fn parameter_ident(&self, id: &Id) -> TokenStream {
        self.parameter(id, self.ident(id))
    }

    /// Like [`TypespaceBuilder::parameter_ident`], with named types
    /// qualified by the module `scope`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`TypespaceBuilder::ident`].
    pub fn parameter_ident_in(&self, id: &Id, scope: &str) -> TokenStream {
        self.parameter(id, self.ident_in(id, scope))
    }

    fn parameter(&self, id: &Id, ident: TokenStream) -> TokenStream {
        let typ = self.types.get(id).expect("invalid type id");
        if typ.is_simple() {
            ident
        } else {
            quote! { &#ident }
        }
    }

    fn renderer(&self) -> TypespaceRenderer<'_, Id> {
        TypespaceRenderer {
            types: &self.types,
            settings: &self.settings,
        }
    }

    /// Reject unparseable extra derives so that rendering--which is
    /// infallible--can rely on them parsing.
    fn check_derives(&self) -> Result<(), Error<Id>> {
        for derive in &self.settings.extra_derives {
            if let Err(err) = syn::parse_str::<syn::Path>(derive) {
                return Err(Error::InvalidDerive {
                    derive: derive.clone(),
                    message: err.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Verify that every type ID referenced by a type is actually
    /// present; later steps rely on lookups of child IDs succeeding.
    fn check_references(&self) -> Result<(), Error<Id>> {
        for (type_id, typ) in &self.types {
            for child_id in typ.children() {
                if !self.types.contains_key(&child_id) {
                    return Err(Error::UnknownTypeId {
                        type_id: type_id.clone(),
                        child_id,
                    });
                }
            }
        }
        Ok(())
    }

    /// Verify that no two named types share a name.
    ///
    /// Names come from the consumer, which owns collision-free naming;
    /// this backstops converter naming bugs. Typespace never renames.
    fn check_type_names(&self) -> Result<(), Error<Id>> {
        let mut names = BTreeMap::<&str, &Id>::new();
        for (type_id, typ) in &self.types {
            if let Some(common) = typ.common() {
                let name = common.built_name();
                if let Some(first) = names.insert(name, type_id) {
                    return Err(Error::DuplicateTypeName {
                        name: name.to_string(),
                        first: first.clone(),
                        second: type_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Reject `Type::Never` used as the payload of a transparent
    /// wrapper: a `Box`, a type alias, or a `#[serde(transparent)]`
    /// newtype struct.
    ///
    /// Each of these wrappers is transparent on the wire, so wrapping
    /// `Never` in one produces a field that is wire-identical to a bare
    /// `Never` property but escapes the property-side skip logic, which
    /// only recognizes a property whose immediate type is `Type::Never`.
    /// None of the three wrappers add expressive power over a bare
    /// `Never`--each is just another name for "nothing"--so this rejects
    /// them outright rather than teaching rendering to see through them.
    ///
    /// Only the immediate inner or target type is checked; there is no
    /// recursion through chains of wrappers. None is needed: a chain
    /// such as `type B = A` where `type A = !` bottoms out at a wrapper
    /// that directly contains `Never` (`A`), and that wrapper alone
    /// fails this check, which fails validation for the whole graph.
    fn check_never_wrappers(&self) -> Result<(), Error<Id>> {
        for (type_id, typ) in &self.types {
            match typ {
                Type::Box(inner) if matches!(self.types.get(inner), Some(Type::Never)) => {
                    return Err(Error::NeverInTransparentWrapper {
                        wrapper: "Box",
                        type_id: type_id.clone(),
                    });
                }
                Type::TypeAlias(TypeAlias { target, .. })
                    if matches!(self.types.get(target), Some(Type::Never)) =>
                {
                    return Err(Error::NeverInTransparentWrapper {
                        wrapper: "type alias",
                        type_id: type_id.clone(),
                    });
                }
                Type::NewtypeStruct(NewtypeStruct { inner, .. })
                    if matches!(self.types.get(inner), Some(Type::Never)) =>
                {
                    return Err(Error::NeverInTransparentWrapper {
                        wrapper: "newtype struct",
                        type_id: type_id.clone(),
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Finalize the typespace.
    ///
    /// Verifies that every ID referenced by a type names an inserted
    /// type (a dangling reference is a
    /// [`error::Error::UnknownTypeId`]), breaks containment cycles by
    /// inserting `Box` types, and propagates trait requirements through
    /// the graph--a type used as a map key must be `Ord`, and so must
    /// everything it contains. Trait requirements that types cannot
    /// satisfy are collected--all of them, not just the first--into
    /// [`error::Error::TraitConflicts`].
    ///
    /// `make_box_id` is called to generate a fresh ID for each `Box<T>`
    /// wrapper inserted to break a containment cycle. The argument is the ID
    /// of the inner type being wrapped. Pass [`no_cycles`] to assert
    /// that the graph contains no containment cycles.
    pub fn finalize<F>(self, make_box_id: F) -> Result<Typespace<Id>, Error<Id>>
    where
        F: FnMut(&Id) -> Id,
    {
        // Basic steps:
        // 1. Break containment cycles with Box types
        // 2. Propagate trait impls
        // 3. Type-specific finalization

        self.check_derives()?;
        self.check_references()?;
        self.check_type_names()?;
        self.check_never_wrappers()?;

        let Self {
            mut types,
            settings,
        } = self;

        build_commons(&mut types);
        break_cycles(&mut types, make_box_id);
        trait_resolution::resolve_traits(&mut types, &settings)?;

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
                    let name = s.common.built_name().to_string();
                    let tokens = s.render(self, &mut cs);
                    cs.add_item(name, tokens);
                }
                Type::Enum(e) => {
                    let name = e.common.built_name().to_string();
                    let tokens = e.render(self, &mut cs);
                    cs.add_item(name, tokens);
                }
                Type::UnitStruct(u) => {
                    let name = u.common.built_name().to_string();
                    cs.add_item(name, u.render(self));
                }
                Type::TupleStruct(t) => {
                    let name = t.common.built_name().to_string();
                    cs.add_item(name, t.render(self));
                }
                Type::NewtypeStruct(n) => {
                    let name = n.common.built_name().to_string();
                    cs.add_item(name, n.render(self));
                }
                Type::TypeAlias(a) => {
                    let name = a.common.built_name().to_string();
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

    /// Render the derive attribute given the computed traits for a type and
    /// the extra derives from settings.
    pub(crate) fn render_derives(&self, traits: &TypespaceTraitSet) -> Option<TokenStream> {
        // Verify that traits that require manual implementation aren't
        // included as derives. If this happens it indicates that either the
        // finalize step didn't detect an unsatisfiable situation, or that the
        // caller (a renderer for a type) neglected to implement (and remove)
        // one of these traits.
        [TypespaceTrait::Display, TypespaceTrait::FromStr]
            .into_iter()
            .for_each(|manual_trait| {
                if traits.contains(&manual_trait) {
                    panic!(
                        "trying to derive {manual_trait} which requires a \
                        manual implementation; this is a bug",
                    )
                }
            });

        let mut derives = traits
            .iter()
            .map(|tt| tt.render(self.settings))
            .collect::<Vec<_>>();
        // TODO 8/20/2026
        // I think that we should validate (and maybe render) these extra
        // derives from settings during finalization and store them in the
        // TypespaceRenderer.
        derives.extend(self.settings.extra_derives.iter().map(|derive| {
            syn::parse_str::<syn::Path>(derive)
                .expect("invalid derive path")
                .to_token_stream()
        }));
        (!derives.is_empty()).then(|| {
            quote! {
                #[derive( #( #derives ),* )]
            }
        })
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
                let name = common.built_name();
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
                let set_type = match &self.settings.set_type {
                    Some(settings::ContainerType(path)) => quote! { #path },
                    // Without an override, a set renders as a Vec:
                    // deduplication is not enforced, but no trait demands
                    // are made of the element type either.
                    None => match &self.settings.std {
                        Std::FullyQualified => quote! { ::std::vec::Vec },
                        Std::Unqualified => quote! { Vec },
                    },
                };
                if base_type {
                    set_type
                } else {
                    let inner_ident = self.render_ident_with_scope(inner_id, scope);
                    quote! {
                        #set_type<#inner_ident>
                    }
                }
            }
            Type::Vec(inner_id) => {
                let vec_type = match &self.settings.vec_type {
                    Some(settings::ContainerType(path)) => quote! { #path },
                    None => match &self.settings.std {
                        Std::FullyQualified => quote! { ::std::vec::Vec },
                        Std::Unqualified => quote! { Vec },
                    },
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
                // A string-to-JSON-value map renders as ::serde_json::Map
                // regardless of the configured map type, matching the map
                // type inside ::serde_json::Value itself.
                let key_ty = self.types.get(key_id).unwrap();
                let value_ty = self.types.get(value_id).unwrap();
                let map_type =
                    if matches!(key_ty, Type::String) && matches!(value_ty, Type::JsonValue) {
                        quote! { ::serde_json::Map }
                    } else {
                        match &self.settings.map_type {
                            Some(settings::ContainerType(path)) => quote! { #path },
                            None => quote! { ::std::collections::BTreeMap },
                        }
                    };
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
            Type::Never => quote! { ::json_serde::Absent },
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

        enum TypeOfInterest<Id> {
            // If the type is itself an Option (i.e. may be null), let's save
            // the inner  type, which we may use i.e. if the field may be
            // absent and the consumer has specified a custom type for that
            // situation. In other cases, we need to know if the type is an
            // Option to add the appropriate serde annotations.
            Option(Id),
            // A Never property that's non-required turns into the
            // ::json_serde::Absent type.
            Never,
            // Other types don't require special handling.
            Other,
        }

        let type_of_interest = match ty {
            Type::Option(id) => TypeOfInterest::Option(id),
            Type::Never => TypeOfInterest::Never,
            _ => TypeOfInterest::Other,
        };

        let ty_ident = self.render_ident(type_id);

        let std_opt_type = match &self.settings.std {
            Std::FullyQualified => quote! { ::std::option::Option },
            Std::Unqualified => quote! { Option },
        };
        let std_opt_is_none = format!("{std_opt_type}::is_none");

        let prop_ty_ident = match (state, type_of_interest) {
            // A required field needs no serde annotations.
            (StructPropertyState::Required, TypeOfInterest::Other) => ty_ident,

            // A required field that is an Option<T> needs a custom
            // deserializer so that the field is mandatory, but may be null;
            // without this attribute, the default handling is to permit
            // either.
            (StructPropertyState::Required, TypeOfInterest::Option(_)) => {
                let opt_deserialize = format!("{std_opt_type}::deserialize");
                // TODO schemars schema_with?
                serde_options.push(quote! { deserialize_with = #opt_deserialize });
                ty_ident
            }

            // An optional field that is not an Option<T> may not be null; we
            // use the json::serde::deserialize_some function to enforce this.
            (StructPropertyState::Optional, TypeOfInterest::Other) => {
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
            (StructPropertyState::Optional, TypeOfInterest::Option(inner_id)) => {
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
            (StructPropertyState::Default, TypeOfInterest::Option(_) | TypeOfInterest::Other) => {
                serde_options.push(quote! { default });
                self.render_struct_property_add_skip(
                    &mut serde_options,
                    type_id,
                    ty,
                    std_opt_is_none,
                );

                ty_ident
            }
            (
                StructPropertyState::DefaultValue(JsonValue(value)),
                TypeOfInterest::Option(_) | TypeOfInterest::Other,
            ) => {
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

            (StructPropertyState::Optional, TypeOfInterest::Never) => {
                // Convert to the ::json_serde::Absent type. It must have
                // `default` since it cannot be deserialized, and
                // `skip_serializing_if = "::json_serde::always"` because it
                // cannot be serialized (and to work around schemars bugs in
                // all versions).
                serde_options.push(quote! { default });
                serde_options.push(quote! {
                    skip_serializing_if = "::json_serde::always"
                });

                quote! { ::json_serde::Absent }
            }
            (_, TypeOfInterest::Never) => {
                // TODO 8/28/2026
                // I think I want this to be unreachable; I'd like to make sure
                // people aren't doing this because it's a dumb thing to do.
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
        let rust_name_ident = format_ident!("{rust_name}");

        quote! {
            #description
            #serde
            #vis_pub #rust_name_ident: #prop_ty_ident
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

            // render_struct_property short-circuits Never properties.
            Type::Never => unreachable!("Never properties are skipped before state handling"),
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

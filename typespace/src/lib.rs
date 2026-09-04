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
pub(crate) mod cycles;
mod default;
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

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};

use crate::build::{
    Enum, JsonValue, Native, NewtypeStruct, Struct, StructProperty, StructPropertySerde,
    StructPropertyState, TupleStruct, Type, TypeAlias, TypeCommonBuilt, UnitStruct, VariantDetails,
};
use crate::default::check_default;
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
/// among its `impls`, or leave them unknown. A requirement that a type cannot satisfy--`Ord`
/// on a float, say--is a [`error::Error`].

// TODO 9/3/2026
// The order of these turns out to be the output order; that's probably wrong
// or we want to sort these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TypespaceTrait {
    Deserialize,
    Serialize,
    Clone,
    Debug,
    JsonSchema,
    Display,
    FromStr,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Default,
}

impl TypespaceTrait {
    pub(crate) fn render(&self, settings: &Settings) -> proc_macro2::TokenStream {
        if settings.std == Std::FullyQualified {
            match self {
                // TypespaceTrait::Clone => quote! { ::std::clone::Clone },
                // TypespaceTrait::Debug => quote! { ::std::fmt::Debug },
                TypespaceTrait::Clone => quote! { Clone },
                TypespaceTrait::Debug => quote! { Debug },
                TypespaceTrait::Serialize => quote! { ::serde::Serialize },
                TypespaceTrait::Deserialize => quote! { ::serde::Deserialize },
                TypespaceTrait::JsonSchema => quote! { ::schemars::JsonSchema },
                // TypespaceTrait::Eq => quote! { ::std::cmp::Eq },
                // TypespaceTrait::PartialEq => quote! { ::std::cmp::PartialEq },
                // TypespaceTrait::Hash => quote! { ::std::hash::Hash },
                // TypespaceTrait::Ord => quote! { ::std::cmp::Ord },
                // TypespaceTrait::PartialOrd => quote! { ::std::cmp::PartialOrd },
                TypespaceTrait::Ord => quote! { Ord },
                TypespaceTrait::PartialOrd => quote! { PartialOrd },
                TypespaceTrait::Eq => quote! { Eq },
                TypespaceTrait::PartialEq => quote! { PartialEq },
                TypespaceTrait::Hash => quote! { Hash },
                TypespaceTrait::Display => quote! { ::std::fmt::Display },
                TypespaceTrait::FromStr => quote! { ::std::str::FromStr },
                // TypespaceTrait::Default => quote! { ::std::default::Default },
                TypespaceTrait::Default => quote! { Default },
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

// REVIEW: this seems like it's likely going to fall out of date when we add a new variants.
/// Every trait typespace tracks, in declaration order.
pub(crate) const ALL_TRAITS: [TypespaceTrait; 13] = [
    TypespaceTrait::Clone,
    TypespaceTrait::Debug,
    TypespaceTrait::Serialize,
    TypespaceTrait::Deserialize,
    TypespaceTrait::JsonSchema,
    TypespaceTrait::Display,
    TypespaceTrait::FromStr,
    TypespaceTrait::Eq,
    TypespaceTrait::PartialEq,
    TypespaceTrait::Ord,
    TypespaceTrait::PartialOrd,
    TypespaceTrait::Hash,
    TypespaceTrait::Default,
];

/// What a [`build::Native`] says about one trait.
///
/// A declarer that knows the answer states it; one that does not
/// leaves the trait [`Unknown`](TraitDisposition::Unknown). The two
/// resolution phases read an unknown trait in opposite directions: a
/// requirement for it passes, because refusing to generate for a valid
/// schema is worse than a compile error naming the real missing impl,
/// and a desired trait is never granted from it, because granting one
/// on a guess emits a derive nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitDisposition {
    /// The type implements the trait.
    Yes,
    /// The type does not implement the trait.
    No,
    /// The declaration cannot answer either way.
    Unknown,
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
    /// `std` syntax). Finalization-only effects are necessarily
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
        TypespaceRenderer::new(&self.types, &self.settings)
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

    /// Verify that each configured container declares what it demands
    /// of every type parameter the position it renders supplies.
    fn check_containers(&self) -> Result<(), Error<Id>> {
        [
            ("map", &self.settings.map_type, 2),
            ("set", &self.settings.set_type, 1),
            ("vec", &self.settings.vec_type, 1),
        ]
        .into_iter()
        .try_for_each(|(position, container, parameters)| {
            match container.obligations().len() {
                declared if declared == parameters => Ok(()),
                declared => Err(Error::ContainerParameterCount {
                    position,
                    path: container.path().to_token_stream().to_string(),
                    declared,
                    parameters,
                }),
            }
        })
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

    fn check_type_defaults(&self) -> Result<(), Error<Id>> {
        for (type_id, typ) in &self.types {
            if let Some(common) = typ.common()
                && let Some(default) = &common.default
            {
                check_default(&self.types, &self.settings, &default.0, type_id.clone())?;
            }
        }
        Ok(())
    }

    /// Reject `Type::Never` in any position that requires a value.
    ///
    /// `Never` renders as `::json_serde::Absent`, a type that can be
    /// neither serialized nor deserialized, so it says something only
    /// where the construct holding it can leave it out: a struct
    /// property that may be absent
    /// ([`StructPropertyState::Optional`]), including a struct-shaped
    /// enum variant's field; either side of a map; and the element of a
    /// vec, a set, or a zero-length array, each of which may be empty.
    /// An `Option<Never>` is a value of its own--`None`--and so is
    /// legal wherever a value is required.
    ///
    /// Every other position demands a value that can never be produced,
    /// which makes the type holding it a type with no values at all: a
    /// property that must be present or fall back to a default, a
    /// tuple component, the element of a non-empty fixed-size array, a
    /// tuple struct field, and an enum variant's item or tuple payload.
    /// These report [`Error::NeverInValuePosition`].
    ///
    /// A transparent wrapper--a `Box`, a type alias, or a
    /// `#[serde(transparent)]` newtype struct--requires a value as
    /// well, and reports [`Error::NeverInTransparentWrapper`] instead.
    /// Each of these wrappers is transparent on the wire, so wrapping
    /// `Never` in one produces a field that is wire-identical to a bare
    /// `Never` property but escapes the property-side skip logic, which
    /// only recognizes a property whose immediate type is
    /// `Type::Never`. None of the three wrappers add expressive power
    /// over a bare `Never`--each is just another name for
    /// "nothing"--and the dedicated error says so rather than teaching
    /// rendering to see through them.
    ///
    /// Only the immediate type in each position is checked; there is no
    /// recursion. None is needed: every type in the graph is checked
    /// here, so every position in the graph is checked. A chain such as
    /// `type B = A` where `type A = !` bottoms out at a wrapper that
    /// directly contains `Never` (`A`), and that wrapper alone fails
    /// this check, which fails validation for the whole graph.
    fn check_never_positions(&self) -> Result<(), Error<Id>> {
        match self
            .types
            .iter()
            .find_map(|(type_id, typ)| self.never_position(type_id, typ))
        {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// The error for a position of `typ` that requires a value and whose
    /// type is `Type::Never`, or `None` if `typ` has no such position.
    /// `type_id` is the id of `typ`, reported as the type that holds the
    /// position. See `check_never_positions` for the rule this applies.
    fn never_position(&self, type_id: &Id, typ: &Type<Id>) -> Option<Error<Id>> {
        let is_never = |id: &Id| matches!(self.types.get(id), Some(Type::Never));
        let value_position = |position: &'static str, name: String| Error::NeverInValuePosition {
            position,
            name,
            type_id: type_id.clone(),
        };
        let transparent_wrapper = |wrapper: &'static str| Error::NeverInTransparentWrapper {
            wrapper,
            type_id: type_id.clone(),
        };
        // The Rust name of the first property that requires a value--any
        // state but Optional--and whose type is Never.
        let never_property = |properties: &[StructProperty<Id>]| {
            properties
                .iter()
                .find(|prop| {
                    !matches!(prop.state, StructPropertyState::Optional) && is_never(&prop.type_id)
                })
                .map(|prop| prop.rust_name.clone())
        };
        // The index of the first component that is Never.
        let never_component = |components: &[Id]| {
            components
                .iter()
                .position(is_never)
                .map(|index| index.to_string())
        };

        match typ {
            Type::Struct(Struct { properties, .. }) => {
                never_property(properties).map(|name| value_position("property", name))
            }

            Type::Enum(Enum { variants, .. }) => variants.iter().find_map(|variant| {
                let variant_name = &variant.rust_name;
                match &variant.details {
                    VariantDetails::Unit => None,
                    VariantDetails::Item(id) => is_never(id)
                        .then(|| value_position("variant payload", variant_name.clone())),
                    VariantDetails::Tuple(components) => never_component(components).map(|index| {
                        value_position(
                            "variant payload component",
                            format!("{variant_name}.{index}"),
                        )
                    }),
                    VariantDetails::Struct(properties) => never_property(properties).map(|name| {
                        value_position("variant property", format!("{variant_name}.{name}"))
                    }),
                }
            }),

            // The rest type holds the items beyond the positional
            // fields; it is required exactly as they are, so it counts
            // as the field one past the last.
            Type::TupleStruct(TupleStruct { fields, rest, .. }) => never_component(fields)
                .or_else(|| {
                    rest.as_ref()
                        .filter(|id| is_never(id))
                        .map(|_| fields.len().to_string())
                })
                .map(|index| value_position("tuple struct field", index)),

            Type::Tuple(components) => {
                never_component(components).map(|index| value_position("tuple component", index))
            }

            // An array of length zero holds no element, so the empty
            // array is its one value; any other length demands elements.
            Type::Array(id, length) => (*length > 0 && is_never(id))
                .then(|| value_position("array element", "item".to_string())),

            Type::Box(inner) => is_never(inner).then(|| transparent_wrapper("Box")),
            Type::TypeAlias(TypeAlias { target, .. }) => {
                is_never(target).then(|| transparent_wrapper("type alias"))
            }
            Type::NewtypeStruct(NewtypeStruct { inner, .. }) => {
                is_never(inner).then(|| transparent_wrapper("newtype struct"))
            }

            // The positions that absorb a Never: an Option of it has the
            // value None, and a vec, a set, or a map of it may be empty.
            Type::Option(_) | Type::Vec(_) | Type::Set(_) | Type::Map(_, _) => None,

            // A native type's parameters are the consumer's to
            // interpret; typespace cannot tell whether one absorbs a
            // Never the way a vec does.
            Type::Native(_) => None,

            // Types with no position that could hold a Never.
            Type::UnitStruct(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer(_)
            | Type::Float(_)
            | Type::String
            | Type::JsonValue
            | Type::Never => None,
        }
    }

    /// Finalize the typespace.
    ///
    /// Verifies that every ID referenced by a type names an inserted
    /// type (a dangling reference is a
    /// [`error::Error::UnknownTypeId`]), breaks containment cycles by
    /// inserting `Box` types, verifies that no representation cycles remain
    /// ([`error::Error::AnonymousCycle`]), and propagates trait requirements
    /// through the graph--a type used as a map key must be `Ord`, and so must
    /// everything it contains. Trait requirements that types cannot satisfy
    /// are collected--all of them, not just the first--into
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
        // TODO 9/1/2026
        // We've lost sight of this comment vvvvvvv and it's order; fix.

        // Basic steps:
        // 1. Break containment cycles with Box types
        // 2. Propagate trait impls
        // 3. Type-specific finalization

        // Validate that derives are parseable as Rust paths.
        // TODO 9/1/2026
        // We should cache this and save it in the finalized Typespace rather
        // than saving the raw settings.
        self.check_derives()?;

        // Validate that each container declares an obligation for every
        // type parameter it is rendered with.
        self.check_containers()?;

        // Ensure that every referenced type ID has been initialized.
        self.check_references()?;

        // Check the uniqueness of type names.
        self.check_type_names()?;

        // Check type defaults
        self.check_type_defaults()?;

        // Disallow never (!) from being used in positions where a value would
        // be required.
        // TODO 9/1/2026 I hate this; I think we should be doing general type
        // validation for which this is one kind of validation. There may be
        // multiple passes: per-type and then intra-type.
        self.check_never_positions()?;

        let Self {
            mut types,
            settings,
        } = self;

        build_commons(&mut types);
        cycles::break_cycles(&mut types, make_box_id);
        cycles::check_anonymous_cycles(&types)?;
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
        TypespaceRenderer::new(&self.types, &self.settings).render()
    }
}

pub(crate) struct TypespaceRenderer<'a, Id> {
    pub(crate) types: &'a BTreeMap<Id, Type<Id>>,
    pub(crate) settings: &'a Settings,
}

impl<'a, Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypespaceRenderer<'a, Id> {
    pub(crate) fn new(types: &'a BTreeMap<Id, Type<Id>>, settings: &'a Settings) -> Self {
        Self { types, settings }
    }

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

    pub(crate) fn add_error_mod(&self, cs: &mut codespace::Codespace) {
        // We only need the error mod once and we carefully control its
        // contents.
        if !cs.get_root_mod().has_mod("error") {
            let mut error_mod = codespace::Mod::default();
            error_mod.add_docs(" Error types.");
            error_mod.add_item(
                "",
                quote! {
                    /// Error from a `TryFrom` or `FromStr` implementation.
                    pub struct ConversionError(::std::borrow::Cow<'static, str>);

                    impl ::std::error::Error for ConversionError {}
                    impl ::std::fmt::Display for ConversionError {
                        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                            -> Result<(), ::std::fmt::Error>
                        {
                            ::std::fmt::Display::fmt(&self.0, f)
                        }
                    }

                    impl ::std::fmt::Debug for ConversionError {
                        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                            -> Result<(), ::std::fmt::Error>
                        {
                            ::std::fmt::Debug::fmt(&self.0, f)
                        }
                    }
                    impl From<&'static str> for ConversionError {
                        fn from(value: &'static str) -> Self {
                            Self(value.into())
                        }
                    }
                    impl From<String> for ConversionError {
                        fn from(value: String) -> Self {
                            Self(value.into())
                        }
                    }
                },
            );
            let _ = cs.get_root_mod().replace_mod("error", error_mod);
        }
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

    /// Render `String` per the configured [`Std`] syntax.
    ///
    /// Bespoke impls that need the `String` don't have an ID they can use to
    /// render it.
    pub(crate) fn render_std_string(&self) -> TokenStream {
        match &self.settings.std {
            Std::FullyQualified => quote! { ::std::string::String },
            Std::Unqualified => quote! { String },
        }
    }

    /// Render the derive attribute given the computed traits for a type and
    /// the extra derives from settings.
    pub(crate) fn render_derives(
        &self,
        traits: &TypespaceTraitSet,
        extra_derives: &[String],
    ) -> Option<TokenStream> {
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
        derives.extend(
            self.settings
                .extra_derives
                .iter()
                .chain(extra_derives.iter())
                .map(|derive| {
                    syn::parse_str::<syn::Path>(derive)
                        .expect("invalid derive path")
                        .to_token_stream()
                }),
        );
        (!derives.is_empty()).then(|| {
            quote! {
                #[derive( #( #derives ),* )]
            }
        })
    }

    pub(crate) fn render_attrs<'b>(
        &'b self,
        extra_attrs: &'b [String],
    ) -> impl Iterator<Item = TokenStream> + 'b {
        self.settings
            .extra_attrs
            .iter()
            .chain(extra_attrs.iter())
            .map(|attr| attr.parse().unwrap())
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
                // Without an override, a set renders as a Vec:
                // deduplication is not enforced, but no trait demands
                // are made of the element type either.
                let set_type = self.settings.set_type.rendered_path(&self.settings.std);
                if base_type {
                    quote! { #set_type }
                } else {
                    let inner_ident = self.render_ident_with_scope(inner_id, scope);
                    quote! {
                        #set_type<#inner_ident>
                    }
                }
            }
            Type::Vec(inner_id) => {
                let vec_type = self.settings.vec_type.rendered_path(&self.settings.std);
                if base_type {
                    quote! { #vec_type }
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
                        let path = self.settings.map_type.rendered_path(&self.settings.std);
                        quote! { #path }
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
    ) -> RenderedStructProperty {
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
        let ty_ident_scoped = self.render_ident_with_scope(type_id, Some("super"));

        let std_opt_type = match &self.settings.std {
            Std::FullyQualified => quote! { ::std::option::Option },
            Std::Unqualified => quote! { Option },
        };
        let std_opt_type_str = std_opt_type.clone().token_print();
        let std_opt_is_none = format!("{std_opt_type_str}::is_none");

        let (prop_ty_ident, prop_ty_ident_scoped) = match (state, type_of_interest) {
            // A required field needs no serde annotations.
            (StructPropertyState::Required, TypeOfInterest::Other) => (ty_ident, ty_ident_scoped),

            // A required field that is an Option<T> needs a custom
            // deserializer so that the field is mandatory, but may be null;
            // without this attribute, the default handling is to permit
            // either.
            (StructPropertyState::Required, TypeOfInterest::Option(_)) => {
                let opt_deserialize = format!("{std_opt_type_str}::deserialize");
                // TODO schemars schema_with?
                serde_options.push(quote! { deserialize_with = #opt_deserialize });
                (ty_ident, ty_ident_scoped)
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

                (
                    quote! { #std_opt_type<#ty_ident> },
                    quote! {#std_opt_type<#ty_ident_scoped>},
                )
            }

            // An optional field that is also an Option<T> may be the type
            // value, null, or absent. Customizable settings determine the
            // handling of this.
            (StructPropertyState::Optional, TypeOfInterest::Option(inner_id)) => {
                match &self.settings.optional_nullable {
                    OptionalNullable::ConflateAsAbsent => {
                        serde_options.push(quote! {
                            skip_serializing_if = #std_opt_is_none
                        });
                        (ty_ident, ty_ident_scoped)
                    }
                    OptionalNullable::ConflateAsNull => {
                        // We always serialize--including `None` as `null`--so
                        // no serde options are necessary.
                        (ty_ident, ty_ident_scoped)
                    }
                    OptionalNullable::DoubleOption => {
                        serde_options.push(quote! { default });
                        serde_options.push(quote! {
                            deserialize_with = "::json_serde::deserialize_some"
                        });
                        serde_options.push(quote! {
                            skip_serializing_if = #std_opt_is_none
                        });

                        (
                            quote! { #std_opt_type<#ty_ident> },
                            quote! { #std_opt_type<#ty_ident_scoped> },
                        )
                    }
                    OptionalNullable::CustomType(custom_type_name) => {
                        let custom_type_path =
                            syn::parse_str::<syn::TypePath>(custom_type_name).unwrap();
                        serde_options.push(quote! { default });
                        let custom_is_absent = format!("{}::is_absent", custom_type_name);
                        serde_options.push(quote! { skip_serializing_if = #custom_is_absent });

                        let inner_ident = self.render_ident(inner_id);
                        let inner_ident_scoped =
                            self.render_ident_with_scope(inner_id, Some("super"));

                        (
                            quote! { #custom_type_path<#inner_ident> },
                            quote! { #custom_type_path<#inner_ident_scoped> },
                        )
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

                (ty_ident, ty_ident_scoped)
            }
            (
                StructPropertyState::DefaultValue(JsonValue(value)),
                TypeOfInterest::Option(_) | TypeOfInterest::Other,
            ) => {
                // TODO 9/3/2026
                // I don't love that the door is open to name collisions here,
                // but this is what typify 1 does so we'll hold the line for
                // now.
                let fn_name_str = format!("{}_{}", context, rust_name);
                let fn_name_ident = format_ident!("{}", fn_name_str);
                let serde_path = format!("defaults::{fn_name_str}");
                serde_options.push(quote! { default = #serde_path });

                let ty_for_fn = self.render_ident_with_scope(type_id, Some("super"));
                let value_tokens = crate::value_tokens::value_tokens(value);
                cs.get_root_mod().get_mod("defaults").add_item(
                    &fn_name_str,
                    quote! {
                        pub fn #fn_name_ident() -> #ty_for_fn {
                            // TODO 9/1/2026
                            // I don't love this use of serde_json here...
                            ::serde_json::from_value(#value_tokens)
                                .expect("invalid default value")
                        }
                    },
                );

                (ty_ident, ty_ident_scoped)
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

                (
                    quote! { ::json_serde::Absent },
                    quote! { ::json_serde::Absent },
                )
            }
            (
                StructPropertyState::Required
                | StructPropertyState::Default
                | StructPropertyState::DefaultValue(_),
                TypeOfInterest::Never,
            ) => unreachable!("finalization rejects a Never property that requires a value"),
        };

        let default = match state {
            StructPropertyState::Required => DefaultConstructor::None,
            StructPropertyState::Optional | StructPropertyState::Default => {
                DefaultConstructor::Default
            }
            StructPropertyState::DefaultValue(_) => {
                // TODO 9/1/2026
                // we should dedup this code
                let fn_name_str = format!("{}_{}", context, rust_name);
                let fn_name_ident = format_ident!("{}", fn_name_str);
                DefaultConstructor::Generated(quote! { defaults::#fn_name_ident() })
            }
        };

        let serde = (!serde_options.is_empty()).then(|| {
            quote! {
                #[serde(
                    #( #serde_options ),*
                )]
            }
        });
        let rust_name_ident = format_ident!("{rust_name}");

        RenderedStructProperty {
            description,
            serde,
            vis_pub,
            rust_name_ident,
            prop_ty_ident,
            prop_ty_ident_scoped,
            default,
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
                let is_empty = format!("{}::is_empty", ty_raw_ident.token_print());
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
                // encounter this via a generated type. Note that this may
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

            // A Never property reaches this function in no state:
            // finalization rejects every state that would, and the
            // Optional state renders without a skip of this kind.
            Type::Never => unreachable!("Never properties add no skip attribute"),
        }
    }
}

pub(crate) enum DefaultConstructor {
    None,
    Default,
    Generated(TokenStream),
}

pub(crate) struct RenderedStructProperty {
    pub description: Option<TokenStream>,
    pub serde: Option<TokenStream>,
    pub vis_pub: bool,
    pub rust_name_ident: syn::Ident,
    pub prop_ty_ident: TokenStream,
    pub prop_ty_ident_scoped: TokenStream,
    pub default: DefaultConstructor,
}

impl ToTokens for RenderedStructProperty {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            description,
            serde,
            vis_pub,
            rust_name_ident,
            prop_ty_ident,
            prop_ty_ident_scoped: _,
            default: _,
        } = self;
        let vis_pub = vis_pub.then(|| quote! { pub });
        tokens.extend(quote! {
            #description
            #serde
            #vis_pub #rust_name_ident: #prop_ty_ident
        });
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

trait TokenPrint {
    fn token_print(self) -> String;
}

impl TokenPrint for proc_macro2::TokenStream {
    fn token_print(self) -> String {
        self.into_iter()
            .map(|tt| tt.to_string())
            .collect::<String>()
    }
}

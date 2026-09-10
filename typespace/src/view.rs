// Copyright 2026 Oxide Computer Company

//! Query-side views of a finalized [`Typespace`].
//!
//! [`Typespace::get_type`] and [`Typespace::iter_types`] hand out
//! [`Type`] values: borrowed views that answer questions about a type
//! (its name, identifier tokens, structural details, trait impls)
//! without exposing the underlying construction data. Names mirror the
//! [`build`] module: [`build::EnumVariant`] is the construction form
//! and [`EnumVariant`] the finalized-view form of the same concept.

use std::borrow::Cow;
use std::collections::BTreeSet;

use proc_macro2::TokenStream;

use crate::{Typespace, TypespaceRenderer, TypespaceTrait, build};

/// A view of a type in a finalized [`Typespace`].
pub struct Type<'a, Id> {
    pub(crate) typespace: &'a Typespace<Id>,
    pub(crate) id: &'a Id,
    pub(crate) typ: &'a build::Type<Id>,
}

impl<'a, Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Type<'a, Id> {
    /// The name of this type, or its rendered token representation for unnamed
    /// types.
    ///
    /// A named type borrows the name it was built with. Every other
    /// type has no name of its own, so its identifier is rendered and
    /// the resulting string is owned.
    pub fn name(&self) -> Cow<'a, str> {
        match self.typ {
            build::Type::Enum(e) => Cow::Borrowed(e.common.built_name()),
            build::Type::Struct(s) => Cow::Borrowed(s.common.built_name()),
            build::Type::UnitStruct(u) => Cow::Borrowed(u.common.built_name()),
            build::Type::TupleStruct(t) => Cow::Borrowed(t.common.built_name()),
            build::Type::NewtypeStruct(n) => Cow::Borrowed(n.common.built_name()),
            build::Type::TypeAlias(a) => Cow::Borrowed(a.common.built_name()),
            _ => Cow::Owned(self.ident().to_string()),
        }
    }

    /// The Rust identifier for this type as a [`TokenStream`].
    pub fn ident(&self) -> TokenStream {
        TypespaceRenderer::new(&self.typespace.types, &self.typespace.settings)
            .render_ident(self.id)
    }

    /// The Rust identifier for this type qualified by the module
    /// `scope`.
    ///
    /// Named types render as `scope::Name`; container and built-in
    /// types thread the scope through to any named types they mention.
    /// Rendering honors the typespace's settings (container overrides,
    /// `std` syntax).
    pub fn ident_in(&self, scope: &str) -> TokenStream {
        TypespaceRenderer::new(&self.typespace.types, &self.typespace.settings)
            .render_ident_with_scope(self.id, Some(scope))
    }

    /// The Rust identifier suitable for use as a function parameter type.
    ///
    /// A caller passes what it owns cheaply and borrows the rest: a
    /// primitive, the unit type, and an enum whose variants are all
    /// unit variants go by value; a `String` becomes a `&str`; and
    /// every other owned type (a struct, a newtype, a `Vec`, a `Map`,
    /// a JSON value) is prefixed with `&`. An `Option` and a tuple
    /// keep their own syntax and apply the rule to what they hold, so
    /// an `Option<String>` reads as `Option<&str>`.
    ///
    /// A `scope` qualifies named types by that module, as
    /// [`Type::ident_in`] does. A `lifetime` is named on every
    /// reference the parameter introduces, and only those, so a
    /// `String` reads as `&'a str` while a `bool` is unchanged. The
    /// two are independent: pass either, neither, or both.
    pub fn parameter_ident(&self, scope: Option<&str>, lifetime: Option<&str>) -> TokenStream {
        self.renderer()
            .render_parameter_ident(self.id, scope, lifetime)
    }

    /// The identifier of this type's generated builder, if it has
    /// one.
    ///
    /// A builder is generated for a struct when
    /// [`Settings::with_struct_builder`](crate::settings::Settings::with_struct_builder)
    /// is set. Builders live in `mod builder`, so the identifier is
    /// `builder::TypeName`, and `scope` qualifies it the way
    /// [`Type::ident_in`] qualifies a type.
    pub fn builder_ident(&self, scope: Option<&str>) -> Option<TokenStream> {
        self.renderer().render_builder_ident(self.id, scope)
    }

    fn renderer(&self) -> TypespaceRenderer<'_, Id> {
        TypespaceRenderer::new(&self.typespace.types, &self.typespace.settings)
    }

    /// The description (doc comment source) for this type, if any.
    pub fn description(&self) -> Option<&str> {
        let common = match self.typ {
            build::Type::Enum(e) => &e.common,
            build::Type::Struct(s) => &s.common,
            build::Type::UnitStruct(u) => &u.common,
            build::Type::TupleStruct(t) => &t.common,
            build::Type::NewtypeStruct(n) => &n.common,
            build::Type::TypeAlias(a) => &a.common,
            _ => return None,
        };
        common.description.as_deref()
    }

    /// Structural details of this type.
    pub fn details(&self) -> TypeDetails<'a, Id> {
        match self.typ {
            build::Type::Enum(e) => TypeDetails::Enum(Enum { inner: e }),
            build::Type::Struct(s) => TypeDetails::Struct(Struct { inner: s }),
            build::Type::NewtypeStruct(n) => TypeDetails::Newtype(NewtypeStruct { inner: n }),

            build::Type::Option(id) => TypeDetails::Option(id.clone()),
            build::Type::Vec(id) => TypeDetails::Vec(id.clone()),
            build::Type::Map(k, v) => TypeDetails::Map(k.clone(), v.clone()),
            build::Type::Set(id) => TypeDetails::Set(id.clone()),
            build::Type::Box(id) => TypeDetails::Box(id.clone()),
            build::Type::Array(id, n) => TypeDetails::Array(id.clone(), *n),
            build::Type::Tuple(ids) => TypeDetails::Tuple(Box::new(ids.clone().into_iter())),

            build::Type::Unit => TypeDetails::Unit,
            build::Type::String => TypeDetails::String,
            build::Type::Boolean => TypeDetails::Builtin("bool"),
            build::Type::Integer(s) => TypeDetails::Builtin(s.as_str()),
            build::Type::Float(s) => TypeDetails::Builtin(s.as_str()),
            build::Type::JsonValue => TypeDetails::Builtin("::serde_json::Value"),
            build::Type::Never => TypeDetails::Builtin("::json_serde::Absent"),
            build::Type::Native(n) => TypeDetails::Builtin(n.name.as_str()),

            // Treat these less-common named types as opaque to callers.
            build::Type::UnitStruct(_)
            | build::Type::TupleStruct(_)
            | build::Type::TypeAlias(_) => TypeDetails::Builtin(self.name_str()),
        }
    }

    fn name_str(&self) -> &'a str {
        match self.typ {
            build::Type::UnitStruct(build::UnitStruct { common, .. })
            | build::Type::TupleStruct(build::TupleStruct { common, .. }) => common.built_name(),
            build::Type::TypeAlias(build::TypeAlias { common, .. }) => common.built_name(),
            _ => "",
        }
    }

    /// Whether this type implements the given trait.
    ///
    /// A native type answers from what it is known to implement, so a trait
    /// its declaration cannot answer for returns `false`. A container's answer
    /// (`Vec`, `Option`, `Map`, `Set`, `Box`, an array, a tuple) depends on
    /// its children and, for the configurable containers, from the declared
    /// container tables trait resolution consults, descending into each child
    /// through this same method; a type alias forwards to its target.
    pub fn has_impl(&self, trait_: TypespaceTrait) -> bool {
        self.has_trait(trait_, &mut BTreeSet::new())
    }

    /// The recursive worker behind [`Type::has_impl`]; `seen` holds
    /// the ids on the walk's current path, and a revisit answers
    /// `false`.
    fn has_trait(&self, trait_: TypespaceTrait, seen: &mut BTreeSet<Id>) -> bool {
        if !seen.insert(self.id.clone()) {
            return false;
        }
        let answer = match self.typ {
            build::Type::Native(n) => n.impls.contains(&trait_),
            build::Type::Enum(e) => e
                .common
                .built
                .as_ref()
                .is_some_and(|b| b.traits.contains(&trait_)),
            build::Type::Struct(s) => s
                .common
                .built
                .as_ref()
                .is_some_and(|b| b.traits.contains(&trait_)),
            build::Type::NewtypeStruct(n) => n
                .common
                .built
                .as_ref()
                .is_some_and(|b| b.traits.contains(&trait_)),
            build::Type::UnitStruct(u) => u
                .common
                .built
                .as_ref()
                .is_some_and(|b| b.traits.contains(&trait_)),
            build::Type::TupleStruct(t) => t
                .common
                .built
                .as_ref()
                .is_some_and(|b| b.traits.contains(&trait_)),
            // A type alias has no impl site of its own; its answer is
            // entirely its target's, exactly as required resolution
            // treats it (see `Feasibility::Forward`).
            build::Type::TypeAlias(a) => self.typespace.get_type(&a.target).has_trait(trait_, seen),
            typ => crate::trait_resolution::unnamed_provides(
                typ,
                trait_,
                &self.typespace.settings,
                &mut |child_id| self.typespace.get_type(child_id).has_trait(trait_, seen),
            ),
        };
        seen.remove(self.id);
        answer
    }
}

/// Structural details of a type, as reported by [`Type::details`].
#[non_exhaustive]
pub enum TypeDetails<'a, Id> {
    Enum(Enum<'a, Id>),
    Struct(Struct<'a, Id>),
    Newtype(NewtypeStruct<'a, Id>),
    Option(Id),
    Vec(Id),
    Map(Id, Id),
    Set(Id),
    Box(Id),
    Tuple(Box<dyn Iterator<Item = Id> + 'a>),
    Array(Id, usize),
    Builtin(&'a str),
    Unit,
    String,
}

// -- Struct view --------------------------------------------------------------

/// A view of a struct type's properties.
pub struct Struct<'a, Id> {
    inner: &'a build::Struct<Id>,
}

impl<'a, Id: Clone> Struct<'a, Id> {
    /// Iterate over `(property_name, type_id)` pairs.
    pub fn properties(&self) -> impl Iterator<Item = (&'a str, Id)> + 'a {
        self.inner
            .properties
            .iter()
            .map(|p| (p.rust_name.as_str(), p.type_id.clone()))
    }

    /// Iterate over full property information.
    pub fn properties_info(&self) -> impl Iterator<Item = StructProperty<'a, Id>> + 'a {
        self.inner.properties.iter().map(|p| StructProperty {
            name: p.rust_name.as_str(),
            description: p.description.as_deref(),
            required: matches!(p.state, build::StructPropertyState::Required),
            type_id: p.type_id.clone(),
        })
    }
}

/// Information about a single struct property.
pub struct StructProperty<'a, Id> {
    /// The Rust field name.
    pub name: &'a str,
    /// The description (doc comment source) for the property, if any.
    pub description: Option<&'a str>,
    /// `true` if the field must be present in the serialized form.
    pub required: bool,
    /// The ID of the property's type.
    pub type_id: Id,
}

// -- Enum view ----------------------------------------------------------------

/// A view of an enum type's variants.
pub struct Enum<'a, Id> {
    inner: &'a build::Enum<Id>,
}

impl<'a, Id: Clone> Enum<'a, Id> {
    /// Iterate over `(variant_name, variant_details)` pairs.
    pub fn variants(&'a self) -> impl Iterator<Item = (&'a str, VariantDetails<Id>)> {
        self.inner
            .variants
            .iter()
            .map(|v| (v.rust_name.as_str(), variant_details_to_info(&v.details)))
    }

    /// Iterate over full variant information.
    pub fn variants_info(&'a self) -> impl Iterator<Item = EnumVariant<'a, Id>> {
        self.inner.variants.iter().map(|v| EnumVariant {
            name: v.rust_name.as_str(),
            description: v.description.as_deref(),
            details: variant_details_to_info(&v.details),
        })
    }
}

fn variant_details_to_info<Id: Clone>(details: &build::VariantDetails<Id>) -> VariantDetails<Id> {
    match details {
        build::VariantDetails::Unit => VariantDetails::Unit,
        build::VariantDetails::Item(id) => VariantDetails::Tuple(vec![id.clone()]),
        build::VariantDetails::Tuple(ids) => VariantDetails::Tuple(ids.clone()),
        build::VariantDetails::Struct(props) => VariantDetails::Struct(
            props
                .iter()
                .map(|p| (p.rust_name.to_string(), p.type_id.clone()))
                .collect(),
        ),
    }
}

/// Full information about a single enum variant.
pub struct EnumVariant<'a, Id> {
    /// The Rust name of the variant.
    pub name: &'a str,
    /// The description (doc comment source) for the variant, if any.
    pub description: Option<&'a str>,
    /// The variant's associated data.
    pub details: VariantDetails<Id>,
}

/// The associated data of an enum variant.
#[non_exhaustive]
pub enum VariantDetails<Id> {
    /// A unit variant with no associated data.
    Unit,
    /// A variant with one or more unnamed values of the given types.
    Tuple(Vec<Id>),
    /// A struct-like variant with named fields.
    Struct(Vec<(String, Id)>),
}

// -- Newtype view -------------------------------------------------------------

/// A view of a newtype struct's inner type.
pub struct NewtypeStruct<'a, Id> {
    inner: &'a build::NewtypeStruct<Id>,
}

impl<'a, Id: Clone> NewtypeStruct<'a, Id> {
    /// The inner type wrapped by this newtype.
    pub fn inner(&self) -> Id {
        self.inner.inner.clone()
    }
}

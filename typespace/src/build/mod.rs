// Copyright 2026 Oxide Computer Company

//! Construction-side vocabulary for assembling a typespace.
//!
//! These are the types a consumer assembles and inserts into a
//! [`TypespaceBuilder`](crate::TypespaceBuilder).
//! [`Type`] is the sum of every kind of type a typespace can hold; the
//! shape types ([`Struct`], [`Enum`], [`NewtypeStruct`], and friends)
//! describe named types in detail. Types refer to one another by ID,
//! never by containment. The finalized, queryable counterparts of these
//! types live in [`view`](crate::view); names are mirrored across the
//! two modules (for example [`EnumVariant`] here and
//! [`view::EnumVariant`](crate::view::EnumVariant) are the construction
//! and finalized-view forms of the same concept).
//!
//! # Canonical item order
//!
//! Each named type renders as a group of items under one type name.
//! Within that group, every `render` function places its pieces in
//! this order (a one-line "Canonical item order: see build::mod"
//! comment at each render site points back here instead of repeating
//! the list):
//!
//! 1. The declaration: doc comment, then custom (extra) attributes,
//!    then the derive attribute, then the serde attribute, then the
//!    `pub struct` / `pub enum` / `pub type` itself.
//! 2. `Deref`, then `From<Self> for Inner`, then `From<Inner> for
//!    Self` (newtype only).
//! 3. `Display`, then `FromStr`, then `TryFrom<&str>`, then
//!    `TryFrom<String>`.
//! 4. `TryFrom<Inner> for Self`, the constrained-newtype constructor
//!    (allow-list and deny-list newtypes; a string-constrained
//!    newtype's constructor is its `TryFrom<&str>` from bucket 3, so
//!    this bucket is empty for it).
//! 5. `Default`.
//! 6. Enum per-variant payload conversions, `From<Payload> for Self`,
//!    in variant declaration order.
//! 7. The inherent `impl Type { pub fn builder() }`.
//! 8. `Deserialize`, then `JsonSchema`.
//!
//! This is the order typify 1 already emits across its fixtures, so
//! the two generators agree during the typespace integration without
//! churning typify's fixtures.
//!
//! Two further points are part of the rule rather than exceptions to
//! it:
//!
//! - The `builder` mod carries its own application of this order,
//!   under its own item key: the builder struct declaration, then its
//!   `Default`, then its inherent setters impl, then `TryFrom<Builder>
//!   for Type`, then `From<Type> for Builder`.
//! - The `error` mod is a fixed, hand-authored literal and is exempt
//!   from this order entirely.
//!
//! # Intended order
//!
//! Once the typespace integration lands and typify 1's renderer is
//! deleted, nothing needs to match its emission order any longer, and
//! the plan is to adopt the order below instead:
//!
//! A. Definition: doc comment, custom attributes, derive attribute,
//!    serde attribute, declaration.
//! B. Type-specific impls: for a struct, the builder; for a newtype,
//!    `Deref`, then the conversions out of `Self`, then the
//!    conversions into `Self`; for an enum, the per-variant payload
//!    conversions.
//! C. String conversions: `TryFrom<&str>`, then `TryFrom<String>`.
//! D. Custom impls: `Default`, `Display`, `FromStr`, `Serialize`,
//!    `Deserialize`, `JsonSchema`.
//!
//! `TryFrom<&str>` and `TryFrom<String>` assert what the type *is*: a
//! string that is not any old string. That belongs high, near the
//! declaration, in bucket C. `FromStr` is interpretation--"one could
//! read this string as meaning this"--which is closer to
//! `Deserialize`, so it belongs with the hand-written impls in bucket
//! D instead.
//!
//! The order in force above carries a concrete wart this order fixes:
//! for a string-constrained newtype, `FromStr` delegates to
//! `TryFrom<&str>`, so the order in force places a caller (`FromStr`,
//! in bucket 3) above the implementation it calls (`TryFrom<&str>`,
//! also in bucket 3, but `FromStr` is emitted first). The intended
//! order puts `TryFrom<&str>` in bucket C, ahead of `FromStr` in
//! bucket D, so the callee always precedes its caller.
//!
//! The two orders also disagree about `Default` and an enum's
//! per-variant payload conversions, and that disagreement is
//! deliberate rather than a typo in either list: typify 1 emits
//! `Default` first, so the order in force does too, while the intended
//! order groups the variant conversions with the other type-specific
//! conversions in bucket B and leaves `Default` among the hand-written
//! impls in bucket D.

mod alias;
mod common;
mod enums;
mod native;
mod structs;

pub use alias::*;
pub use common::*;
pub use enums::*;
pub use native::*;
pub use structs::*;

// 9.15.2025
// Little bit of a random thought: "Native" is actually kind of a catch-all for
// which things like boolean, integer, unit, etc. could apply. I think we'll
// eventually want more of a builder interface to construct types and then
// a finished interface to inspect them. I could imagine--for example--
// "native" being used for any non-constructed type (so anything except for
// generated structs, generated enums, and compound types such as tuples and
// arrays). Could these also have type parameters and therefore be inclusive of
// maps and vecs? Maybe? Something to noodle on as we think about Typespace as
// an interface.

/// A type in a typespace.
///
/// A type refers to other types by ID, never by containment; every ID
/// used here must have its own entry in the
/// [`TypespaceBuilder`](crate::TypespaceBuilder). Named types (see
/// [`Type::is_named`]) render as items; the remaining variants are
/// built-in and container types that appear where other types reference
/// them.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Type<Id> {
    Enum(Enum<Id>),
    Struct(Struct<Id>),
    UnitStruct(UnitStruct),
    TupleStruct(TupleStruct<Id>),
    NewtypeStruct(NewtypeStruct<Id>),
    TypeAlias(TypeAlias<Id>),

    Native(Native<Id>),
    Option(Id),
    Box(Id),
    Vec(Id),
    Map(Id, Id),
    Set(Id),
    Array(Id, usize),
    Tuple(Vec<Id>),
    Unit,
    Boolean,
    Integer(String),
    Float(String),
    String,
    JsonValue,
    Never,
}

macro_rules! all_named_types {
    ($common:pat) => {
        $crate::build::Type::Enum($crate::build::Enum {
            common: $common,
            ..
        }) | $crate::build::Type::Struct($crate::build::Struct {
            common: $common,
            ..
        }) | $crate::build::Type::UnitStruct($crate::build::UnitStruct {
            common: $common,
            ..
        }) | $crate::build::Type::TupleStruct($crate::build::TupleStruct {
            common: $common,
            ..
        }) | $crate::build::Type::NewtypeStruct($crate::build::NewtypeStruct {
            common: $common,
            ..
        }) | $crate::build::Type::TypeAlias($crate::build::TypeAlias {
            common: $common,
            ..
        })
    };
}

pub(crate) use all_named_types;

impl<Id> Type<Id> {
    /// The name of this type, if it has one.
    ///
    /// Named types (see [`Type::is_named`]) that have passed `build()`
    /// or been inserted into a
    /// [`TypespaceBuilder`](crate::TypespaceBuilder) always report
    /// `Some`; built-in and container types have no name.
    pub fn name(&self) -> Option<&str> {
        self.common().and_then(|common| common.name())
    }

    /// The type's default value, if any.
    ///
    /// Always `None` for built-in and container types, which carry no
    /// default slot.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.common().and_then(|common| common.default())
    }

    /// Set or clear the type's default value after construction.
    ///
    /// Consumers often learn a type's default after building it (from
    /// the metadata of a schema that references the type, say); this is
    /// the post-construction counterpart of the shapes' fluent
    /// `default` methods. Has no effect on built-in and container
    /// types, which carry no default slot.
    pub fn set_default(&mut self, default: Option<JsonValue>) {
        if let Some(common) = self.common_mut() {
            common.default = default;
        }
    }

    /// The opaque derive paths carried by this type alone.
    ///
    /// Additional to the crate-wide paths from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive).
    /// Always empty for built-in and container types, which carry no
    /// such slot.
    pub fn extra_derives(&self) -> &[String] {
        self.common()
            .map_or(&[] as &[String], |common| common.extra_derives())
    }

    /// The opaque attributes carried by this type alone.
    ///
    /// Additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    /// Always empty for built-in and container types, which carry no
    /// such slot.
    pub fn extra_attrs(&self) -> &[String] {
        self.common()
            .map_or(&[] as &[String], |common| common.extra_attrs())
    }

    /// The metadata common to named types: the name, description,
    /// default value, and per-type derives and attributes. Returns
    /// `Some` exactly when [`Type::is_named`] returns `true` (structs,
    /// enums, unit structs, tuple structs, newtype structs, and type
    /// aliases); `None` for built-in and container types, which have no
    /// caller-assigned name.
    pub(crate) fn common(&self) -> Option<&TypeCommon> {
        match self {
            all_named_types!(common) => Some(common),
            _ => None,
        }
    }

    /// Exclusive-reference form of [`Type::common`]: the metadata common
    /// to named types, mutably. Returns `Some` for exactly the same
    /// variants as `common`.
    pub(crate) fn common_mut(&mut self) -> Option<&mut TypeCommon> {
        match self {
            all_named_types!(common) => Some(common),
            _ => None,
        }
    }
}

/// A contained child of a type: the relation reaching it, its id, and
/// whether the edge is a struct-style property in the optional state.
///
/// Produced by [`Type::contained_children_related`]; trait resolution
/// classifies each against the settings to decide whether the declared
/// custom optional-nullable wrapper substitutes at that edge.
pub(crate) struct ContainedChild<Id> {
    pub(crate) relation: crate::error::Relation,
    pub(crate) id: Id,
    pub(crate) optional: bool,
}

impl<Id> ContainedChild<Id> {
    /// A required edge; not an optional struct-style property.
    fn required(relation: crate::error::Relation, id: Id) -> Self {
        Self {
            relation,
            id,
            optional: false,
        }
    }

    /// An optional struct-style property.
    fn optional(relation: crate::error::Relation, id: Id) -> Self {
        Self {
            relation,
            id,
            optional: true,
        }
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Type<Id> {
    /// The IDs of every type this type refers to directly. Each such ID
    /// must have a corresponding type inserted into the
    /// [`TypespaceBuilder`](crate::TypespaceBuilder) for finalization to
    /// succeed.
    ///
    /// A [`Native`]'s type parameters are children here, exactly like a
    /// container's: their IDs must resolve
    /// ([`check_references`](crate::TypespaceBuilder::insert)), a
    /// requirement the native forwards under
    /// [`TraitProvision::IfParameters`](crate::TraitProvision::IfParameters)
    /// reaches them, and a loss of a desired trait at a parameter
    /// reaches the native back through the referrer map desired
    /// resolution builds from this method.
    pub fn children(&self) -> Vec<Id> {
        match self {
            Type::Enum(type_enum) => type_enum.children(),
            Type::Struct(type_struct) => type_struct.children(),
            Type::UnitStruct(_) => Vec::new(),
            Type::TupleStruct(type_tuple_struct) => type_tuple_struct.children(),
            Type::NewtypeStruct(type_newtype_struct) => type_newtype_struct.children(),
            Type::TypeAlias(alias_info) => alias_info.children(),

            Type::Boolean => Vec::new(),
            Type::String => Vec::new(),
            Type::Native(native) => native.parameters().to_vec(),

            Type::Option(id)
            | Type::Box(id)
            | Type::Vec(id)
            | Type::Set(id)
            | Type::Array(id, _) => vec![id.clone()],

            Type::Map(key_id, value_id) => vec![key_id.clone(), value_id.clone()],
            Type::Tuple(items) => items.clone(),

            Type::Unit => Vec::new(),
            Type::Integer(_) => Vec::new(),
            Type::Float(_) => Vec::new(),
            Type::JsonValue => Vec::new(),
            Type::Never => Vec::new(),
        }
    }

    /// The IDs of every child type paired with its naming context.
    ///
    /// The context is the string a naming pass would use to name an
    /// anonymous child: a property name, a variant name (with `.field`
    /// or `.index` appended for variant payloads), a tuple index,
    /// `"inner"`, `"item"`, `"key"`, or `"value"`. Children reached
    /// with no context of their own (the contents of options and
    /// boxes) report an empty string.
    ///
    /// This may report fewer children than [`Type::children`]: a type
    /// alias and a native type's parameters confer no naming context
    /// and contribute nothing here.
    pub fn children_with_context(&self) -> Vec<(Id, String)> {
        match self {
            Type::Enum(type_enum) => type_enum
                .variants
                .iter()
                .flat_map(|variant| match &variant.details {
                    VariantDetails::Unit => Vec::new(),
                    VariantDetails::Item(id) => {
                        vec![(id.clone(), variant.rust_name.clone())]
                    }
                    VariantDetails::Tuple(items) => items
                        .iter()
                        .enumerate()
                        .map(|(ii, id)| (id.clone(), format!("{}.{}", variant.rust_name, ii)))
                        .collect::<Vec<_>>(),
                    VariantDetails::Struct(items) => items
                        .iter()
                        .map(|prop| {
                            (
                                prop.type_id.clone(),
                                format!("{}.{}", variant.rust_name, prop.rust_name),
                            )
                        })
                        .collect::<Vec<_>>(),
                })
                .collect::<Vec<_>>(),
            Type::Struct(type_struct) => type_struct
                .properties
                .iter()
                .map(
                    |StructProperty {
                         rust_name, type_id, ..
                     }| (type_id.clone(), rust_name.to_string()),
                )
                .collect::<Vec<_>>(),
            Type::UnitStruct(_) => Vec::new(),
            Type::TupleStruct(type_tuple_struct) => {
                let mut children = type_tuple_struct
                    .fields
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(ii, type_id)| (type_id, ii.to_string()))
                    .collect::<Vec<_>>();

                if let Some(rest) = &type_tuple_struct.rest {
                    children.push((rest.clone(), type_tuple_struct.fields.len().to_string()));
                }

                children
            }
            Type::NewtypeStruct(NewtypeStruct { inner, .. }) => {
                vec![(inner.clone(), "inner".to_string())]
            }

            // TODO 2/4/2026
            // I'm not really sure what to do here; the type alias shouldn't really
            // confer any context to the inner type. I think this is fine, but also
            // may mean that I want to change the name of this pass/fn/concept to
            // indicate that we should only express context insofar as we have it,
            // and that the cardinality of this fn may be different from that of
            // children().
            Type::TypeAlias(TypeAlias { .. }) => Vec::new(),

            Type::Native(_) => Vec::new(),
            Type::Option(id) | Type::Box(id) => vec![(id.clone(), "".to_string())],
            Type::Vec(id) | Type::Set(id) | Type::Array(id, _) => {
                vec![(id.clone(), "item".to_string())]
            }
            Type::Map(key_id, value_id) => vec![
                (key_id.clone(), "key".to_string()),
                (value_id.clone(), "value".to_string()),
            ],
            Type::Tuple(items) => items
                .iter()
                .enumerate()
                .map(|(ii, id)| (id.clone(), ii.to_string()))
                .collect::<Vec<_>>(),

            Type::Unit
            | Type::Boolean
            | Type::Integer(_)
            | Type::Float(_)
            | Type::String
            | Type::JsonValue
            | Type::Never => Vec::new(),
        }
    }

    /// The children that contribute to this type's size, as mutable
    /// references, for in-place cycle breaking.
    ///
    /// A cycle running only through these edges makes a type of
    /// infinite size, which is what `break_cycles` cuts with a `Box`.
    /// The heap-indirect containers (box, vec, map, set) already bound
    /// their contents and report nothing; a type alias reports its
    /// target, since the alias is that type under another name.
    pub fn contained_children_mut(&mut self) -> Vec<&mut Id> {
        match self {
            Type::Enum(Enum { variants, .. }) => {
                let mut out = Vec::new();
                for variant in variants {
                    match &mut variant.details {
                        VariantDetails::Unit => {}
                        VariantDetails::Item(schema_ref) => {
                            out.push(schema_ref);
                        }
                        VariantDetails::Tuple(schema_refs) => {
                            out.extend(schema_refs);
                        }
                        VariantDetails::Struct(props) => {
                            for StructProperty { type_id, .. } in props {
                                out.push(type_id);
                            }
                        }
                    }
                }
                out
            }
            Type::Struct(Struct { properties, .. }) => properties
                .iter_mut()
                .map(|prop| &mut prop.type_id)
                .collect(),

            Type::UnitStruct(_) => vec![],
            Type::TupleStruct(type_tuple_struct) => type_tuple_struct.contained_children_mut(),
            Type::NewtypeStruct(type_newtype_struct) => {
                type_newtype_struct.contained_children_mut()
            }

            // 2/4/2026
            // This is an interesting case. Let's say I have something like
            // this:
            // struct Foo{ foo: OptionString }
            // where OptionString is a type alias for Option<String>.
            // I guess we just want to return the target type... but we'll want
            // to make sure that doesn't turn this into an alias to a Box...
            // somehow?
            Type::TypeAlias(alias_info) => {
                vec![&mut alias_info.target]
            }

            Type::Option(id) => vec![id],
            Type::Array(id, _) => vec![id],
            Type::Tuple(items) => items.iter_mut().collect(),

            // TODO maybe native types could have children? Right now these are
            // just for self-contained types...
            //
            // Type::children() now reports a native's type parameters
            // (see its doc), but this method and contained_children
            // stay as they are: they exist to find containment cycles
            // that need a Box to stay finite-sized, and typespace has
            // no way to know whether a native holds its parameter by
            // value or behind its own indirection (a `Vec<T>`-like
            // native versus a `struct W<T>(T)`-like one), the same
            // reason Box, Vec, Map, and Set are absent below.
            Type::Native(_) => Default::default(),
            Type::Box(_)
            | Type::Vec(_)
            | Type::Map(_, _)
            | Type::Set(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer(_)
            | Type::Float(_)
            | Type::String
            | Type::JsonValue
            | Type::Never => Default::default(),
        }
    }

    /// The contained children, each with the relation reaching it and
    /// whether that edge is a struct-style property in the optional
    /// state--the one position a declared custom optional-nullable
    /// wrapper substitutes at.
    ///
    /// Mirrors [`Type::contained_children_mut`]--the same children, in
    /// the same order--labeled for trait-requirement propagation paths;
    /// keep the two functions in sync. Container types that trait
    /// propagation handles directly (box, vec, map, set) report no
    /// children here, exactly as `contained_children_mut` does.
    pub(crate) fn contained_children_related(&self) -> Vec<ContainedChild<Id>> {
        use crate::error::Relation;
        match self {
            Type::Enum(Enum { variants, .. }) => {
                let mut out = Vec::new();
                for variant in variants {
                    let relation = Relation::Variant(variant.rust_name.clone());
                    match &variant.details {
                        VariantDetails::Unit => {}
                        VariantDetails::Item(id) => {
                            out.push(ContainedChild::required(relation.clone(), id.clone()))
                        }
                        VariantDetails::Tuple(ids) => {
                            out.extend(
                                ids.iter().map(|id| {
                                    ContainedChild::required(relation.clone(), id.clone())
                                }),
                            );
                        }
                        VariantDetails::Struct(props) => {
                            out.extend(props.iter().map(|prop| {
                                if prop.state.is_optional() {
                                    ContainedChild::optional(relation.clone(), prop.type_id.clone())
                                } else {
                                    ContainedChild::required(relation.clone(), prop.type_id.clone())
                                }
                            }));
                        }
                    }
                }
                out
            }
            Type::Struct(Struct { properties, .. }) => properties
                .iter()
                .map(|prop| {
                    let relation = Relation::Field(prop.rust_name.to_string());
                    let id = prop.type_id.clone();
                    if prop.state.is_optional() {
                        ContainedChild::optional(relation, id)
                    } else {
                        ContainedChild::required(relation, id)
                    }
                })
                .collect(),

            Type::UnitStruct(_) => Vec::new(),
            Type::TupleStruct(TupleStruct { fields, rest, .. }) => {
                let mut out = fields
                    .iter()
                    .map(|id| ContainedChild::required(Relation::Element, id.clone()))
                    .collect::<Vec<_>>();
                if let Some(rest) = rest {
                    out.push(ContainedChild::required(Relation::Element, rest.clone()));
                }
                out
            }
            Type::NewtypeStruct(NewtypeStruct { inner, .. }) => {
                vec![ContainedChild::required(Relation::Inner, inner.clone())]
            }
            Type::TypeAlias(alias_info) => {
                vec![ContainedChild::required(
                    Relation::Target,
                    alias_info.target.clone(),
                )]
            }

            Type::Option(id) => vec![ContainedChild::required(Relation::Element, id.clone())],
            Type::Array(id, _) => vec![ContainedChild::required(Relation::Element, id.clone())],
            Type::Tuple(items) => items
                .iter()
                .map(|id| ContainedChild::required(Relation::Element, id.clone()))
                .collect(),

            Type::Native(_) => Vec::new(),
            Type::Box(_)
            | Type::Vec(_)
            | Type::Map(_, _)
            | Type::Set(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer(_)
            | Type::Float(_)
            | Type::String
            | Type::JsonValue
            | Type::Never => Vec::new(),
        }
    }

    /// Whether this is a named type--one that renders as its own item
    /// (struct, enum, unit struct, tuple struct, newtype struct, or type
    /// alias)--as opposed to a built-in or container type. Named types
    /// are exactly those carrying a name, description, default, and
    /// per-type derives and attributes.
    pub fn is_named(&self) -> bool {
        matches!(self, all_named_types!(_))
    }

    /// Re-run the shape checks that `build()` applies.
    ///
    /// The shapes' `build()` methods are the intended construction door,
    /// but `Type`'s variants are not sealed: `Type::Struct(Struct::new())`
    /// is expressible and would smuggle an unvalidated shape past
    /// `build()`. Insertion into the
    /// [`TypespaceBuilder`](crate::TypespaceBuilder) calls this as
    /// defense-in-depth so no unvalidated shape can enter a typespace.
    pub(crate) fn validate_built(&self) -> Result<(), crate::error::Error<Id>> {
        match self {
            Type::Enum(type_enum) => type_enum.validate(),
            Type::Struct(type_struct) => type_struct.validate(),
            Type::UnitStruct(unit_struct) => unit_struct.validate(),
            Type::TupleStruct(tuple_struct) => tuple_struct.validate(),
            Type::NewtypeStruct(newtype_struct) => newtype_struct.validate(),
            Type::TypeAlias(type_alias) => type_alias.validate(),
            Type::Native(native) => native.validate(),
            _ => Ok(()),
        }
    }
}

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
// eventually want more of a builder interface to construct types and and then
// a finished interface to inspect them. I could imagine--for example--
// "native" being used for any non-constructed type (so anything except for
// generated structs, generated enums, and compound types such as tuples and
// arrays). Could these also have type parameters and therefore be inclusive of
// maps and vecs? Maybe? Something to noodle on as we think about Typespace as
// an interface.

/// Represents a type in the Typespace.
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
}

impl<Id> Type<Id> {
    /// The metadata common to named types: the name, description, and
    /// default value. Returns `Some` exactly when [`Type::is_named`]
    /// returns `true` (structs, enums, unit structs, tuple structs,
    /// newtype structs, and type aliases); `None` for built-in and
    /// container types, which have no caller-assigned name.
    pub fn common(&self) -> Option<&TypeCommon> {
        match self {
            Type::Enum(Enum { common, .. })
            | Type::Struct(Struct { common, .. })
            | Type::UnitStruct(UnitStruct { common, .. })
            | Type::TupleStruct(TupleStruct { common, .. })
            | Type::NewtypeStruct(NewtypeStruct { common, .. })
            | Type::TypeAlias(TypeAlias { common, .. }) => Some(common),
            _ => None,
        }
    }

    /// Exclusive-reference form of [`Type::common`]: the metadata common
    /// to named types, mutably. Returns `Some` for exactly the same
    /// variants as `common`.
    pub fn common_mut(&mut self) -> Option<&mut TypeCommon> {
        match self {
            Type::Enum(Enum { common, .. })
            | Type::Struct(Struct { common, .. })
            | Type::UnitStruct(UnitStruct { common, .. })
            | Type::TupleStruct(TupleStruct { common, .. })
            | Type::NewtypeStruct(NewtypeStruct { common, .. })
            | Type::TypeAlias(TypeAlias { common, .. }) => Some(common),
            _ => None,
        }
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> Type<Id> {
    /// The IDs of every type this type refers to directly. Each such ID
    /// must have a corresponding type inserted into the
    /// [`TypespaceBuilder`](crate::TypespaceBuilder) for finalization to
    /// succeed.
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
            Type::Native(_) => Vec::new(),

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
        }
    }

    /// Children that this type "contains" (i.e. cycle-breaking candidates).
    pub fn contained_children(&self) -> Vec<Id> {
        match self {
            Type::TupleStruct(TupleStruct { fields, .. }) => fields.clone(),
            Type::NewtypeStruct(NewtypeStruct { inner, .. }) => vec![inner.clone()],
            Type::Option(id) | Type::Vec(id) | Type::Set(id) | Type::Array(id, _) => {
                vec![id.clone()]
            }
            Type::Map(k, v) => vec![k.clone(), v.clone()],
            Type::Tuple(ids) => ids.clone(),
            Type::Struct(s) => s.properties.iter().map(|p| p.type_id.clone()).collect(),
            Type::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| v.contained_children())
                .collect(),
            _ => vec![],
        }
    }

    /// Return the list of child types that are contained (i.e. contributed to
    /// the size of this type). This is used to consider containment cycles.
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
            | Type::JsonValue => Default::default(),
        }
    }

    /// The contained children paired with the relation reaching each.
    ///
    /// Mirrors [`Type::contained_children_mut`]--the same children, in
    /// the same order--labeled for trait-requirement propagation paths;
    /// keep the two functions in sync. Container types that trait
    /// propagation handles directly (box, vec, map, set) report no
    /// children here, exactly as `contained_children_mut` does.
    pub(crate) fn contained_children_related(&self) -> Vec<(crate::Relation, Id)> {
        use crate::Relation;
        match self {
            Type::Enum(Enum { variants, .. }) => {
                let mut out = Vec::new();
                for variant in variants {
                    let relation = || Relation::Variant(variant.rust_name.clone());
                    match &variant.details {
                        VariantDetails::Unit => {}
                        VariantDetails::Item(id) => out.push((relation(), id.clone())),
                        VariantDetails::Tuple(ids) => {
                            out.extend(ids.iter().map(|id| (relation(), id.clone())));
                        }
                        VariantDetails::Struct(props) => {
                            out.extend(props.iter().map(|prop| (relation(), prop.type_id.clone())));
                        }
                    }
                }
                out
            }
            Type::Struct(Struct { properties, .. }) => properties
                .iter()
                .map(|prop| {
                    (
                        Relation::Field(prop.rust_name.to_string()),
                        prop.type_id.clone(),
                    )
                })
                .collect(),

            Type::UnitStruct(_) => Vec::new(),
            Type::TupleStruct(TupleStruct { fields, rest, .. }) => {
                let mut out = fields
                    .iter()
                    .map(|id| (Relation::Element, id.clone()))
                    .collect::<Vec<_>>();
                if let Some(rest) = rest {
                    out.push((Relation::Element, rest.clone()));
                }
                out
            }
            Type::NewtypeStruct(NewtypeStruct { inner, .. }) => {
                vec![(Relation::Inner, inner.clone())]
            }
            Type::TypeAlias(alias_info) => {
                vec![(Relation::Target, alias_info.target.clone())]
            }

            Type::Option(id) => vec![(Relation::Element, id.clone())],
            Type::Array(id, _) => vec![(Relation::Element, id.clone())],
            Type::Tuple(items) => items
                .iter()
                .map(|id| (Relation::Element, id.clone()))
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
            | Type::JsonValue => Vec::new(),
        }
    }

    /// Whether this is a named type--one that renders as its own item
    /// (struct, enum, unit struct, tuple struct, newtype struct, or type
    /// alias)--as opposed to a built-in or container type. Named types
    /// are exactly those for which [`Type::common`] returns `Some`.
    pub fn is_named(&self) -> bool {
        matches!(
            self,
            Type::Enum(_)
                | Type::Struct(_)
                | Type::UnitStruct(_)
                | Type::TupleStruct(_)
                | Type::NewtypeStruct(_)
                | Type::TypeAlias(_)
        )
    }
}

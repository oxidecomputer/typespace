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
    Never,
}

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
    pub(crate) fn common_mut(&mut self) -> Option<&mut TypeCommon> {
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
    /// The cardinality may be smaller than [`Type::children`]: a type
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

    /// Exclusive-reference form of [`Type::contained_children`]: the same
    /// children, as mutable references, for in-place cycle breaking.
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
            | Type::JsonValue
            | Type::Never => Default::default(),
        }
    }

    /// The contained children paired with the relation reaching each.
    ///
    /// Mirrors [`Type::contained_children_mut`]--the same children, in
    /// the same order--labeled for trait-requirement propagation paths;
    /// keep the two functions in sync. Container types that trait
    /// propagation handles directly (box, vec, map, set) report no
    /// children here, exactly as `contained_children_mut` does.
    pub(crate) fn contained_children_related(&self) -> Vec<(crate::error::Relation, Id)> {
        use crate::error::Relation;
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

    /// Whether the type is cheap enough to pass by value as a function
    /// parameter (primitives and options); complex owned types take a
    /// reference instead.
    pub(crate) fn is_simple(&self) -> bool {
        matches!(
            self,
            Type::Boolean
                | Type::Integer(_)
                | Type::Float(_)
                | Type::Unit
                | Type::String
                | Type::Option(_)
        )
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
            _ => Ok(()),
        }
    }
}

// Copyright 2026 Oxide Computer Company

use crate::TypespaceTrait;

/// Errors that arise from an invalid type graph provided to the
/// [`TypespaceBuilder`](crate::TypespaceBuilder).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TypespaceError<Id>
where
    Id: std::fmt::Debug + std::fmt::Display,
{
    /// A type was inserted with an ID already in use by another type.
    #[error("a type with the id `{type_id}` has already been inserted")]
    DuplicateTypeId {
        /// The ID of the duplicate insertion.
        type_id: Id,
    },

    /// A named type was inserted with an empty name.
    ///
    /// Applies to structs, enums, newtype structs, unit structs, tuple
    /// structs, and type aliases. Names come from the caller; rendering
    /// interpolates them into identifiers and cannot tolerate an empty
    /// string.
    #[error("the type with id `{type_id}` has an empty name")]
    EmptyTypeName {
        /// The ID of the type whose name is empty.
        type_id: Id,
    },

    /// A derive path supplied via settings is not a valid Rust path.
    ///
    /// Extra derives come from
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive)
    /// and are emitted into every generated derive attribute; a path
    /// that does not parse would produce unbuildable code.
    #[error("the derive `{derive}` is not a valid Rust path: {message}")]
    InvalidDerive {
        /// The derive string as supplied.
        derive: String,
        /// The parse failure.
        message: String,
    },

    /// A type refers to a child type ID for which no type was inserted.
    #[error(
        "the type with id `{type_id}` references the id `{child_id}` \
         for which there is no type"
    )]
    UnknownTypeId {
        /// The ID of the type containing the dangling reference.
        type_id: Id,
        /// The referenced ID for which no type exists.
        child_id: Id,
    },

    /// Trait requirements that types in the graph cannot satisfy.
    ///
    /// Every conflict found during propagation is collected; the list
    /// is not deduplicated, so one root cause (a float inside a widely
    /// shared type, say) can appear once per requirement path that
    /// reaches it. Collapsing such cascades to their root cause is a
    /// planned improvement.
    #[error("{}", format_conflicts(conflicts))]
    TraitConflicts {
        /// The conflicts, in the order propagation found them.
        conflicts: Vec<TraitConflict<Id>>,
    },
}

fn format_conflicts<Id: std::fmt::Display>(conflicts: &[TraitConflict<Id>]) -> String {
    conflicts
        .iter()
        .map(|conflict| conflict.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A trait requirement that a type cannot satisfy.
///
/// Produced during finalization when a requirement--structural (a map
/// key must be `Ord`) or requested via settings--propagates to a type
/// that cannot provide the trait. The conflict records where the
/// requirement came from ([`RequirementOrigin`]), every propagation hop
/// it took ([`PathStep`]), the type that failed (`offender`), and why
/// it failed ([`OffenderReason`]). Its `Display` form renders the chain
/// one line per hop, innermost first.
#[derive(Debug)]
#[non_exhaustive]
pub struct TraitConflict<Id> {
    /// The trait that could not be satisfied.
    pub required: TypespaceTrait,
    /// Where the requirement originated.
    pub origin: RequirementOrigin<Id>,
    /// The propagation hops from the origin to the offender, in
    /// propagation order (the origin's requirement target first).
    pub path: Vec<PathStep<Id>>,
    /// The ID of the type that cannot satisfy the requirement.
    pub offender: Id,
    /// Why the offender cannot satisfy the requirement.
    pub reason: OffenderReason,
}

impl<Id: std::fmt::Display> std::fmt::Display for TraitConflict<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            required,
            origin,
            path,
            offender,
            reason,
        } = self;
        match reason {
            OffenderReason::Primitive { type_name } => write!(
                f,
                "type `{type_name}` (id `{offender}`) cannot implement \
                 the required trait `{required}`"
            )?,
            OffenderReason::NativeMissingImpl { type_name } => write!(
                f,
                "native type `{type_name}` (id `{offender}`) does not \
                 declare the required trait `{required}`"
            )?,
        }
        // Render the chain innermost first, rustc style: each hop names
        // the type that passed the requirement along and the relation it
        // used; the origin comes last.
        for PathStep { type_id, relation } in path.iter().rev() {
            write!(
                f,
                "\n    required because `{type_id}` passes the requirement \
                 to {relation}"
            )?;
        }
        match origin {
            RequirementOrigin::MapKey(id) => {
                write!(
                    f,
                    "\n    required because keys of map `{id}` must \
                     implement `{required}`"
                )
            }
            RequirementOrigin::SetElement(id) => {
                write!(
                    f,
                    "\n    required because elements of set `{id}` must \
                     implement `{required}`"
                )
            }
            RequirementOrigin::Requested => {
                write!(
                    f,
                    "\n    required because settings request `{required}` \
                     for all types"
                )
            }
        }
    }
}

/// Where a trait requirement originated.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RequirementOrigin<Id> {
    /// The requirement applies to the key type of the map with this ID.
    MapKey(Id),
    /// The requirement applies to the element type of the set with this
    /// ID.
    SetElement(Id),
    /// The requirement was requested for all named types via
    /// [`Settings::with_trait_impl`](crate::settings::Settings::with_trait_impl).
    Requested,
}

/// One hop in a trait requirement's propagation path.
///
/// Reads as: the type with `type_id` passed the requirement along to
/// the child reached via `relation`.
#[derive(Debug, Clone)]
pub struct PathStep<Id> {
    /// The type the requirement passed through.
    pub type_id: Id,
    /// How the requirement left that type.
    pub relation: Relation,
}

/// The relation by which a trait requirement moves from a type to one
/// of the types it contains.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Relation {
    /// A struct property with the given Rust name.
    Field(String),
    /// An enum variant with the given Rust name; covers the variant's
    /// payload whether it is an item, a tuple, or a struct-shaped set
    /// of fields.
    Variant(String),
    /// The element type of a vec, set, array, tuple, or option.
    Element,
    /// The key type of a map.
    Key,
    /// The value type of a map.
    Value,
    /// The type inside a box.
    Boxed,
    /// The type inside a newtype struct.
    Inner,
    /// The target of a type alias.
    Target,
}

impl std::fmt::Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Relation::Field(name) => write!(f, "its field `{name}`"),
            Relation::Variant(name) => write!(f, "its variant `{name}`"),
            Relation::Element => write!(f, "its element type"),
            Relation::Key => write!(f, "its key type"),
            Relation::Value => write!(f, "its value type"),
            Relation::Boxed => write!(f, "its boxed type"),
            Relation::Inner => write!(f, "its inner type"),
            Relation::Target => write!(f, "its target type"),
        }
    }
}

/// Why a type cannot satisfy a trait requirement.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum OffenderReason {
    /// A built-in type that is incapable of implementing the trait: a
    /// float required to be `Ord`, a JSON value required to be `Hash`,
    /// or a container required to be `Display`.
    Primitive {
        /// The rendered name of the built-in type (`f64`, say).
        type_name: String,
    },
    /// A native type that does not list the trait among its declared
    /// impls. Unlike [`OffenderReason::Primitive`], this is fixable:
    /// declare the impl on the [`Native`](crate::build::Native) if the
    /// underlying Rust type provides it.
    NativeMissingImpl {
        /// The Rust type path of the native type.
        type_name: String,
    },
}

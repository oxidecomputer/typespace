// Copyright 2026 Oxide Computer Company

//! The error type, and the detail types its messages are built from.
//!
//! Everything a consumer can get wrong reports through [`Error`], from
//! a `build()` call on a half-assembled type through insertion and on
//! to [`finalize`](crate::TypespaceBuilder::finalize). Most variants
//! name one thing and are done. Trait conflicts are the exception, and
//! they are worth knowing how to read.
//!
//! # Reading a trait conflict
//!
//! [`Error::TraitConflicts`] carries a [`TraitConflict`] per failure,
//! and each answers four questions.
//!
//! Where the requirement came from is [`RequirementOrigin`]. A trait
//! named in [`Settings`](crate::settings::Settings) applies to every
//! named type; a container demands traits of its parameters, a map key
//! needing `Ord` for instance; and a default value demands traits of
//! what it constructs.
//!
//! How it arrived is [`TraitConflict::path`], a run of [`PathStep`]s
//! from the origin to the offender. Each step names a type and the
//! [`Relation`] the requirement took out of it, and the `Display` impl
//! renders the run innermost first, rustc style, so the last line read
//! is where the demand started.
//!
//! Who could not satisfy it is [`TraitConflict::offender`], an id.
//!
//! Why is [`OffenderReason`], and it is the part that says what to do
//! about it. [`OffenderReason::Primitive`] is a built-in that cannot
//! implement the trait in any form, an `f64` asked for `Ord` say, so
//! the demand has to change rather than the type.
//! [`OffenderReason::NativeMissingImpl`] and
//! [`OffenderReason::ContainerMissingImpl`] are declarations that said
//! they do not provide the trait, so amending the declaration fixes
//! them. [`OffenderReason::TypeCannotImplement`] means rendering has no
//! implementation for that trait on that kind of type.
//! [`OffenderReason::IrrefutableVariantPayload`] is narrower: an
//! untagged enum could implement `FromStr`, but a payload that parses
//! every string would make every later variant unreachable, so the
//! trait is refused instead of emitted as a trap.
//!
//! One offending type reachable along several paths reports once per
//! path. Conflicts are not deduplicated to a root cause, so a single
//! missing impl low in the graph can produce a long list that says the
//! same thing many ways.

use crate::TypespaceTrait;

/// Errors that arise from an invalid shape, type graph, or settings.
///
/// Shape construction ([`build`](crate::build) `build()` methods),
/// [`TypespaceBuilder`](crate::TypespaceBuilder) insertion, validation,
/// and finalization all report through this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error<Id>
where
    Id: std::fmt::Debug + std::fmt::Display,
{
    /// A type was inserted with an ID already in use by another type.
    #[error("a type with the id `{type_id}` has already been inserted")]
    DuplicateTypeId {
        /// The ID of the duplicate insertion.
        type_id: Id,
    },

    /// A type reached `build()` without a name.
    ///
    /// Every named type (struct, enum, newtype struct, unit struct,
    /// tuple struct, or type alias) must have a name before `build()`
    /// can produce it; rendering interpolates names into identifiers.
    /// Set one with the shape's `name` method.
    #[error("cannot build the {kind}: no name was provided")]
    MissingTypeName {
        /// The kind of shape being built (`"struct"`, `"enum"`, ...).
        kind: &'static str,
    },

    /// A name is not usable as a Rust identifier.
    ///
    /// Applies to type names, property names, and variant names alike:
    /// each must parse as a plain (non-raw) Rust identifier and must
    /// not be a keyword.
    #[error("the {kind} name `{name}` {message}")]
    InvalidName {
        /// What the name names (`"struct"`, `"property"`, `"variant"`,
        /// ...).
        kind: &'static str,
        /// The offending name as supplied.
        name: String,
        /// What is wrong with it.
        message: &'static str,
    },

    /// An enum was built without a tag type.
    ///
    /// The serde tagging scheme has no presumed default; set one with
    /// the enum's `tag_type` method before `build()`.
    #[error("cannot build the enum `{name}`: no tag type was provided")]
    MissingTagType {
        /// The name of the enum being built.
        name: String,
    },

    /// A tuple struct was built with no fixed fields.
    ///
    /// With no rest, a `TupleStruct` with zero fields is a second way to
    /// write `UnitStruct`: both hold nothing and serialize the same
    /// way. With a rest, it is a second way to write `NewtypeStruct`
    /// over the rest type: flattening the rest through
    /// `FlattenedSequenceSerializer` behind an empty prefix produces the
    /// same bytes as serializing the rest type directly. Requiring at
    /// least one fixed field also keeps the default-value walk's
    /// recursion into the rest narrowing: with a field present, the
    /// tail handed to the next step is always a strict suffix of the
    /// input array, so it cannot recur forever on a self-referential or
    /// mutually referential rest.
    #[error(
        "cannot build the tuple struct `{name}`: it has no fixed fields, \
         which makes it redundant with {alternative}"
    )]
    FieldlessTupleStruct {
        /// The name of the tuple struct being built.
        name: String,
        /// What a fieldless tuple struct is redundant with: `UnitStruct` when
        /// there is no rest, or `NewtypeStruct` over the sequence type
        /// when there is.
        alternative: &'static str,
    },

    /// Two properties or two variants of one type share a name.
    ///
    /// Names must be unique on both axes: the Rust name (the identifier
    /// in generated code) and the wire name (the serialized name, after
    /// any rename). Flattened properties have no wire name of their
    /// own and are exempt from the wire axis.
    #[error(
        "in `{type_name}`, the {axis} name `{name}` is used by more \
         than one {kind}"
    )]
    DuplicateItemName {
        /// What collided: `"property"` or `"variant"`.
        kind: &'static str,
        /// The name of the type containing the collision.
        type_name: String,
        /// The colliding name.
        name: String,
        /// Which axis collided.
        axis: NameAxis,
    },

    /// A struct both denies unknown fields and flattens a property.
    ///
    /// serde cannot honor the combination: `deny_unknown_fields` is
    /// decided by the outer struct's deserializer, which sees a key
    /// the flattened type may claim and has no way to ask. serde
    /// documents the pair as unsupported. typespace refuses it rather
    /// than emitting code whose runtime behavior nobody can predict
    /// from reading it.
    #[error(
        "`{type_name}` denies unknown fields and flattens the property \
         `{property}`; serde does not support that combination"
    )]
    FlattenWithDenyUnknownFields {
        /// The name of the struct carrying both.
        type_name: String,
        /// The Rust name of one flattened property. A struct may
        /// flatten several; the first in declaration order is named.
        property: String,
    },

    /// Two types in the typespace share a name.
    ///
    /// Names come from the consumer, which is responsible for
    /// collision-free naming; typespace never renames. This check backs
    /// up converter naming logic at finalization and validation.
    #[error("the types with ids `{first}` and `{second}` are both named `{name}`")]
    DuplicateTypeName {
        /// The shared name.
        name: String,
        /// The ID of the first type encountered with the name.
        first: Id,
        /// The ID of the second type encountered with the name.
        second: Id,
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

    /// A container states obligations for the wrong number of type
    /// parameters.
    ///
    /// A map renders two; a set or a vec renders one.
    #[error(
        "the {position} container `{path}` states obligations for \
         {declared} type parameter(s), but the {position} position \
         renders {parameters}"
    )]
    ContainerParameterCount {
        /// The setting the container is configured for: `"map"`,
        /// `"set"`, or `"vec"`.
        position: &'static str,
        /// The container's path as configured.
        path: String,
        /// The number of parameter obligations the container states.
        declared: usize,
        /// The number of type parameters the position renders.
        parameters: usize,
    },

    /// A configured container answers
    /// [`TraitProvision::Unknown`](crate::TraitProvision::Unknown) for
    /// some trait.
    ///
    /// A container is hand-authored in
    /// [`Settings`](crate::settings::Settings), so its author is
    /// expected to know it completely; `Unknown` is only meaningful for
    /// a machine-authored [`Native`](crate::build::Native), which
    /// cannot always answer for the Rust type it names.
    #[error(
        "the {position} container `{path}` answers `unknown` for the \
         trait `{trait_}`, which only a native type may do"
    )]
    ContainerProvisionUnknown {
        /// The setting the container is configured for: `"map"`,
        /// `"set"`, `"vec"`, or `"optional-nullable"`.
        position: &'static str,
        /// The container's path as configured.
        path: String,
        /// The trait left unanswered.
        trait_: TypespaceTrait,
    },

    /// A native type states obligations for the wrong number of type
    /// parameters.
    #[error(
        "the native type `{path}` states obligations for {declared} \
         type parameter(s), but declares {parameters}"
    )]
    NativeParameterCount {
        /// The native's declared path.
        path: String,
        /// The number of parameter obligations the native states.
        declared: usize,
        /// The number of type parameters the native declares.
        parameters: usize,
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

    /// A transparent wrapper (`Box`, type alias, transparent newtype) may not
    /// have a Never type as its payload.
    ///
    /// Each of these wrappers is transparent on the wire, so a `Never` wrapped
    /// in one is wire-identical to a bare `Never` property; typespace
    /// rejects these redundant constructions rather than handling them.
    #[error(
        "the {wrapper} with id `{type_id}` wraps `Type::Never`, which \
         adds no meaning over `Type::Never` alone"
    )]
    NeverInTransparentWrapper {
        /// The kind of wrapper (`"Box"`, `"type alias"`, or `"newtype
        /// struct"`).
        wrapper: &'static str,
        /// The id of the wrapper type.
        type_id: Id,
    },

    /// A position that requires a value has `Type::Never` as its type.
    ///
    /// `Never` renders as `::json_serde::Never`, which has no values at
    /// all, so it says something only where the
    /// construct holding it can leave it out: a property that may be
    /// absent, either side of a map, or the element of a vec, a set, or
    /// a zero-length array. An `Option<Never>` is a value of its own,
    /// `None`, and is legal wherever a value is required.
    ///
    /// Every other position demands a value that can never be produced,
    /// which makes the type holding it a type with no values at all.
    /// typespace rejects the construction rather than generate such a
    /// type.
    #[error(
        "the {position} `{name}` of the type with id `{type_id}` is \
         `Type::Never`, which cannot provide the value the position \
         requires"
    )]
    NeverInValuePosition {
        /// The kind of position (`"property"`, `"tuple component"`,
        /// `"tuple struct field"`, `"array element"`, `"variant
        /// payload"`, `"variant payload component"`, or `"variant
        /// property"`).
        position: &'static str,
        /// Which position within the type: a property's Rust name, a
        /// field or component index, `"item"` for an array element, or a
        /// variant's Rust name with `.property` or `.index` appended for
        /// a struct-shaped or tuple variant.
        name: String,
        /// The id of the type that holds the position.
        type_id: Id,
    },

    /// A cycle in the type graph passes through no named type.
    ///
    /// [`TypespaceBuilder::finalize`](crate::TypespaceBuilder::finalize)
    /// breaks containment cycles by inserting `Box` types. This ensures
    /// that types can be compiled. It doesn't address cycles in code
    /// generation itself. Anonymous types may form cycles such that generating
    /// the code to represent them would be infinitely recursive.
    ///
    /// `type_id` and `child_id` form one edge of the offending cycle:
    /// `type_id` refers to `child_id`, and `child_id` is reachable from
    /// itself through anonymous types only.
    #[error(
        "the anonymous type with id `{type_id}` refers to `{child_id}`, \
         closing a cycle that passes through no named type"
    )]
    AnonymousCycle {
        /// The type whose reference closes the cycle.
        type_id: Id,
        /// The ancestor the cycle closes back to.
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

    /// A newtype struct states constraints with nothing in them.
    ///
    /// Three constructions say nothing that
    /// [`NewtypeConstraints::None`](crate::build::NewtypeConstraints::None)
    /// does not already say: a `String` constraint with no minimum, no
    /// maximum, and no patterns, an empty allow list, and an empty deny
    /// list. Each is a mistake at the source rather than a type worth
    /// generating, so each is rejected.
    #[error(
        "the newtype struct `{name}` states {kind} constraints with \
         nothing in them"
    )]
    VacuousConstraints {
        /// The name of the newtype struct.
        name: String,
        /// Which kind of constraint is empty: `"string"`, `"allow
        /// list"`, or `"deny list"`.
        kind: &'static str,
    },

    /// A default value is not valid for the type it is attached to.
    #[error("the value `{value}` does not fit the type `{id}`: {reason}")]
    InvalidDefault {
        /// The invalid value.
        value: serde_json::Value,
        /// The incompatible type.
        id: Id,
        /// Details on the incompatibility.
        reason: String,
    },
    /// A declared custom optional-nullable wrapper does not claim an
    /// unconditional `Default`.
    ///
    /// The wrapper substitutes for `Option` at every optional struct
    /// property whose type is itself an `Option`, and trait resolution
    /// exempts such a property from its own `Default` obligation
    /// without consulting the declaration. `Default: Always` is what
    /// makes that exemption sound, and a wrapper meeting its documented
    /// contract always has it to claim: the wrapper implements
    /// `json_serde::OptionalNullable`, which requires `Default` as a
    /// supertrait.
    #[error(
        "the optional-nullable wrapper `{path}` declares `Default: \
         {provision:?}`; it must declare `Default: Always`, since every \
         optional property rendering through it is exempted from its own \
         `Default` obligation on that assumption"
    )]
    OptionalNullableWrapperDefault {
        /// The wrapper's path as configured.
        path: String,
        /// The provision it declared for `Default`.
        provision: crate::TraitProvision,
    },
}

/// The axis on which a name collision occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameAxis {
    /// The Rust identifier in generated code.
    Rust,
    /// The serialized (wire) name, after any rename.
    Wire,
}

impl std::fmt::Display for NameAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameAxis::Rust => f.write_str("Rust"),
            NameAxis::Wire => f.write_str("wire"),
        }
    }
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
            OffenderReason::ContainerMissingImpl { type_name } => write!(
                f,
                "the configured container `{type_name}` (id \
                 `{offender}`) does not provide the required trait \
                 `{required}`"
            )?,
            OffenderReason::TypeCannotImplement { kind } => write!(
                f,
                "the generated {kind} with id `{offender}` cannot \
                 implement the required trait `{required}`"
            )?,
            OffenderReason::IrrefutableVariantPayload { variant } => write!(
                f,
                "the untagged enum with id `{offender}` cannot \
                 implement the required trait `{required}`: its \
                 `FromStr` would try the variants in order, and the \
                 payload of its variant `{variant}` parses every string"
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
            RequirementOrigin::ContainerParameter {
                container,
                relation,
            } => {
                write!(
                    f,
                    "\n    required because the container `{container}` \
                     requires `{required}` of {relation}"
                )
            }
            RequirementOrigin::PropertyDefault(id) => {
                write!(
                    f,
                    "\n    required because a property of `{id}` \
                     deserializes with `#[serde(default)]` and so must \
                     implement `{required}`"
                )
            }
            RequirementOrigin::DefaultValue(id) => {
                write!(
                    f,
                    "\n    required because the default value of `{id}` \
                    needs to construct it; and generated code does so \
                    by deserializing it"
                )
            }
            RequirementOrigin::GlobalSettings => {
                write!(
                    f,
                    "\n    required because global settings require \
                     `{required}` of all types"
                )
            }
        }
    }
}

/// Where a trait requirement originated.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RequirementOrigin<Id> {
    /// The requirement applies to one type parameter of the container
    /// or native type with this ID, because that type demands it of
    /// that parameter.
    ContainerParameter {
        /// The ID of the container or native type that makes the
        /// demand.
        container: Id,
        /// The parameter position the demand lands on:
        /// [`Relation::Key`] or [`Relation::Value`] for a map,
        /// [`Relation::Element`] for a set or a vec,
        /// [`Relation::Parameter`] for a native.
        relation: Relation,
    },
    /// The requirement applies to the type of a property of the type
    /// with this ID, because that property carries `#[serde(default)]`.
    PropertyDefault(Id),
    /// Generated code constructs native types within defaults by
    /// deserializing. This imposes the Deserialize requirement on a native
    /// type. The default value of the given ID has created this requirement.
    DefaultValue(Id),
    /// The requirement applies to every named type, via
    /// [`Settings::with_required_trait`](crate::settings::Settings::with_required_trait).
    GlobalSettings,
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
///
/// It names a hop in a [`PathStep`] and, in
/// [`RequirementOrigin::ContainerParameter`], the parameter position a
/// container's own demand lands on.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Relation {
    /// A struct property with the given Rust name.
    Field(String),
    /// An enum variant with the given Rust name.
    ///
    /// This names the variant's whole payload: an item, every element
    /// of a tuple, and every field of a struct-shaped variant all hop
    /// through this one relation. That is a limitation rather than the
    /// intent. A [`PathStep`] pairs a relation with the id of the type
    /// it left, and a variant has no id of its own, so a struct-shaped
    /// variant's field cannot be reached in a second step and there is
    /// no relation that names a variant and a field together. A path
    /// through such a variant therefore identifies the field only by
    /// the type it arrives at, which is ambiguous when two fields of
    /// the variant share a type.
    Variant(String),
    /// The element type of a vec, set, array, tuple, or option.
    Element,
    /// The key type of a map.
    Key,
    /// The value type of a map.
    Value,
    /// The type parameter of a [`Native`](crate::build::Native) at the
    /// given index, in declaration order.
    Parameter(usize),
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
            Relation::Parameter(index) => write!(f, "its type parameter {index}"),
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
    /// A native type that is known not to implement the trait. Unlike
    /// [`OffenderReason::Primitive`], this is fixable: declare the impl
    /// on the [`Native`](crate::build::Native) if the underlying Rust
    /// type provides it, or mark the trait unknown if the declaration
    /// cannot answer for it.
    NativeMissingImpl {
        /// The Rust type path of the native type.
        type_name: String,
    },
    /// The configured vec, set, or map type declares that it never
    /// provides the trait. Fixable the same way
    /// [`OffenderReason::NativeMissingImpl`] is: state the provision
    /// differently with
    /// [`with_provisions`](crate::settings::ContainerType::with_provisions),
    /// or configure a container that provides the trait.
    ContainerMissingImpl {
        /// The Rust type path the container was configured with,
        /// which is the name the consumer named it by rather than the
        /// prelude shortcut generated code may render.
        type_name: String,
    },
    /// A generated type that cannot implement the trait in any form:
    /// no derive exists and rendering has no manual implementation for
    /// the combination--`Display` on a struct, say.
    TypeCannotImplement {
        /// The kind of type (`"struct"`, `"enum"`, ...).
        kind: &'static str,
    },
    /// An untagged enum carries a variant whose payload has an
    /// irrefutable `FromStr`: one that returns `Ok` for every `&str`.
    ///
    /// The generated `FromStr` tries the variants in order and takes
    /// the first that parses, so an irrefutable payload always wins and
    /// every later variant is unreachable. `Display` and `FromStr` also
    /// stop round-tripping, since a value built from a later variant
    /// parses back as this one. The enum could implement `FromStr`, but
    /// the impl would be meaningless, so the trait is refused instead.
    IrrefutableVariantPayload {
        /// The Rust name of the first such variant, in declaration
        /// order. The message could name every qualifying variant
        /// instead; one reason per offending type is the granularity
        /// trait resolution reports at everywhere else.
        variant: String,
    },
}

// Copyright 2026 Oxide Computer Company

//! Trait resolution: determine the trait set for every type.
//!
//! A typespace holds a graph of types that refer to one another by ID.
//! Whether a type can implement a trait depends on what its children
//! implement, so no type can be finalized independently. This module
//! propagates trait information to each type with the results landing in each
//! named type's `TypeCommonBuilt`. Rendering emits code according to that set
//! of traits (either via `derive` or a custom `impl`).
//! [`Type::has_impl`](crate::view::Type::has_impl) answers consumer queries
//! from it (generated code and the answers about it cannot disagree). Unnamed
//! types (built-ins, Box, Map, etc.) don't store a trait set directly; a query
//! (re-)computes the answer (but with the significant benefit of the trait
//! sets stored in named types).
//!
//! Consumers express traits in two main ways. A REQUIRED trait must be
//! available for all named types (that is, all generated types excepting
//! anonymous types such as anonymous tuples). Failing to apply one results in
//! [`Error::TraitConflicts`]. A DESIRED trait is applied where it is
//! compatible and dropped--without error--where it is not. A consumer might,
//! for example, REQUIRE `serde::Serialize` and `serde::Deserialize` of all
//! types, but DESIRE convenience traits such as `Hash` and `Eq` where they can
//! be generated. Each gets its own traversal of the type graph.
//!
//! In addition, trait requirements may spawn from container types. For example,
//! a `BTreeSet` creates a requirement of the `Ord` trait on its type parameter
//! (which in turn creates requirements for `PartialOrd`, `Eq`, and
//! `PartialEq`; see "Supertrait dependencies" below).
//!
//! ## Phase 1: required traits flow down to children
//!
//! [`required_resolution`] initializes its traversal with every place a
//! REQUIREMENT originates:
//!
//! - [`Settings::with_required_trait`] at every named type.
//! - container obligations--what Map, Set, Vec, and a declared
//!   optional-nullable wrapper require of their type parameters (if
//!   anything).
//! - properties rendered with `#[serde(default)]`; these are populated via a
//!   call to `Default::default()` so their types must implement `Default`.
//! - native types inside an explicit default value; generated code constructs
//!   these native types by deserializing, so the value demands `Deserialize`.
//!
//! We descend the type graph with a work queue rather than recursion. Each
//! item in the work queue represents a required work set, a target type; it
//! also includes the origin of the REQUIREMENT and the path walked so far for
//! coherent error reporting. Traversal flows from a type to the children it
//! contains.
//!
//! - A named type consults [`feasibility`] for each trait. It applies the
//!   traits that are compatible. Any conflict is an error.
//! - Each container effectively has a disposition for each trait: always
//!   provided, never provided, or provided if all type parameters also provide
//!   the trait. Configurable containers encode this in [`TraitProvision`];
//!   fixed containers (Option, Box, arrays, tuples) hard-code these.
//! - A leaf type (a type with no children) satisfies the demand or conflicts,
//!   per [`leaf_provides`].
//! - Each native type declares a similar table of trait capabilities
//!   (supported, unsupported, unknown). It conflicts on traits known to be
//!   unsupported; a trait whose support is unknown is given the benefit of the
//!   doubt. If this turns out to be wrong, the generated code may result in a
//!   compilation-time failure that states both the type and trait requirement.
//!   TODO can native and container types be unified?
//!
//! Conflicts accumulate instead of stopping the pass; a consumer
//! fixing a schema wants every failure at once.
//!
//! ```text
//!     BTreeMap<Color, _>   origin: the map demands Ord of its key
//!            | Ord
//!            v
//!    struct Color { .. }   named: absorbs Ord, re-pushes to its fields
//!            | Ord
//!            v
//!        Vec<f64>          container: declared Ord follows its type parameter
//!            | Ord
//!            v
//!           f64            leaf: no Ord; the conflict names the
//!                          origin, the path, f64, and the reason
//! ```
//!
//! ## Phase 2: desired trait incompatibilities poison parent types
//!
//! Rather than propagating *requirements* from parent to child, for DESIRED
//! traits, we propagate *incompatibilities* from child to parent.
//! [`desired_resolution`] grants every desired trait to every type, then
//! initializes a work queue with the types that are known to be incompatible
//! with any of the DESIRED traits (e.g. `f64` and the `Eq` trait). Each
//! incompatibility poisons the types that refer to it (e.g. a `struct` with an
//! `f64` field). Each newly discovered incompatibility spawns a new work queue
//! item; a type can only be poisoned for a trait once (losing the given
//! trait). This walks the graph, but in reverse:
//!
//! ```text
//!           f64             no Ord: poisoning starts at the leaf
//!            ^
//!            |
//!        Vec<f64>           loses Ord: the container forwards it
//!            ^
//!            |
//!   struct Color { .. }     loses Ord: required of every field
//! ```
//!
//! The chain of poisoning stops when a type already omits the trait or when
//! it provides the trait on its own, without requiring the child to implement
//! it. For example, a `Vec<T>` implements `Default` even if `T` does not, so
//! poisoning of the `Default` trait would not continue from there.
//!
//! The two passes are mirror images: phase 1 walks children and asks
//! "can you also satisfy this?", phase 2 walks referrers and asks "can
//! you implement this without your child doing so?".
//!
//! Traits granted in Phase 1 are never poisoned in Phase 2. Phase 1 proves
//! that each type can support the REQUIRED traits so Phase 2 cannot "discover"
//! some new incompatibility. The ordering of the phases is necessary and
//! deliberate.
//!
//! ## Supertrait dependencies
//!
//! `Ord` requires both `PartialOrd` and `Eq`. Both `Eq` and `PartialOrd`
//! require `PartialEq` (and transitively, `Ord` requires `PartialEq`). Each
//! requirement is expanded through [`close_supertraits`] during traversal
//! initialization. Poisoning is expanded through [`strip_dependents`] in
//! reverse e.g. removing `PartialEq` also removes `Eq`, `PartialOrd`, and
//! `Ord`.
//!
//! TODO could close_supertraits and strip_dependents have names that have more
//! similarity? Could they (do they) share a source of truth?
//!
//! Desired traits carry one extra rule: a supertrait granted by
//! [`close_supertraits`] is not wanted for its own sake, so phase 2
//! grants each desired trait together with its supertraits or grants none of
//! them. If `Eq` was desired, but not satisfiable, we don't leave behind, say,
//! `PartialEq` if it is not also specifically desired.
//!
//! ## Custom optional `Option` wrappers
//!
//! A struct field that can be absent, null, or some value is represented as
//! [`StructPropertyState::Optional`] with a [`Type::Option`]. By default this
//! is rendered as `Option<T>`, but consumers may change this to
//! `Option<Option<T>>` to represent the three states, or a custom type (see
//! [`OptionalNullable::CustomType`]). Trait flow through an "optional option"
//! heeds this container type and its trait properties (as with any other
//! container).
//!
//! ## Does type X implement trait Y?
//!
//! For a named type, [`feasibility`] encodes whether and how a trait can be
//! had: derived, forwarded (an alias), realized by a hand-written impl with
//! its own obligation list, or impossible. Phase 1 absorbs and pushes from
//! that answer; phase 2's [`provides`] turns the same answer into yes or no
//! given what the children still hold.
//!
//! For an unnamed type the rules live twice: [`required_resolution`]'s
//! per-variant arms split requirement sets and record conflicts, while
//! [`unnamed_provides`] answers one trait as a boolean, with
//! [`leaf_provides`] and [`container_provides`] as its slices. The two
//! encodings must agree. `unnamed_provides` is shared with
//! [`Type::has_impl`](crate::view::Type::has_impl), so a query about
//! an unnamed type recomputes from the rules the passes used.
//!
//! TODO can we simplify the redundancy above?
//!
//! ## Preconditions
//!
//! By the time [`resolve_traits`] runs: `build_commons` has given every named
//! type a `TypeCommonBuilt` to write into; `break_cycles` has finalized the
//! ids, boxing containment cycles; `check_anonymous_cycles` verifies that
//! every type cycle contains a named type (see below); and
//! [`resolve_from_string_irrefutable`] has cached its answers, which
//! [`feasibility`] reads to decide FromStr on untagged enums. Within
//! [`resolve_traits`], required precedes desired because desired resolution
//! treats required grants as untouchable.
//!
//! ## Traversal termination
//!
//! Both phases are designed to make monotonic progress:
//!
//! - Phase 1: a named type's built set only grows; only newly added traits
//!   propagate. Unnamed types re-push unconditionally, but
//!   `check_anonymous_cycles` has rejected every cycle made only of unnamed
//!   types, so each cycle includes a named type that absorbs a requirement the
//!   first time it comes around so that the walk terminates on the next
//!   encounter.
//! - Phase 2: the optimistic grant only shrinks; each type loses each
//!   trait at most once. When we encounter a type that already lacks the
//!   poisoned trait, we don't need to poison further.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use log::debug;

use crate::build::{
    ContainedChild, Enum, NewtypeConstraints, NewtypeStruct, StructProperty, StructPropertyState,
    TupleStruct, Type, TypeAlias, VariantDetails,
};
use crate::error::{Error, OffenderReason, PathStep, Relation, RequirementOrigin, TraitConflict};
use crate::settings::{ContainerType, OptionalNullable, Settings, path_text};
use crate::{TraitProvision, TypespaceTrait, TypespaceTraitSet};

/// A contained child edge classified against the settings.
///
/// An edge rendering wraps with the declared custom optional-nullable
/// wrapper is `Wrapped`, carrying the declaration, the `Option` node at
/// the property, and the option's value type; every other edge is
/// `Plain`. The wrapping condition--the optional property state, an
/// `Option` type, the `CustomType` setting--mirrors
/// `render_struct_property`'s optional-`Option` arm; keep the two in
/// sync.
enum Edge<'a, Id> {
    Plain(Relation, Id),
    Wrapped {
        relation: Relation,
        option_id: Id,
        container: &'a ContainerType,
        value_id: Id,
    },
}

/// Classify every contained child of `ty` for trait propagation.
fn classify_edges<'a, Id: Clone + Ord + std::fmt::Debug + std::fmt::Display>(
    ty: &Type<Id>,
    types: &BTreeMap<Id, Type<Id>>,
    settings: &'a Settings,
) -> Vec<Edge<'a, Id>> {
    ty.contained_children_related()
        .into_iter()
        .map(|child| {
            match (
                &settings.optional_nullable,
                child.optional,
                types.get(&child.id).unwrap(),
            ) {
                (OptionalNullable::CustomType(container), true, Type::Option(value_id)) => {
                    Edge::Wrapped {
                        relation: child.relation,
                        option_id: child.id,
                        container,
                        value_id: value_id.clone(),
                    }
                }
                _ => Edge::Plain(child.relation, child.id),
            }
        })
        .collect()
}

/// Resolve the trait set for every named type in the graph.
///
/// On success, each named type's built trait set holds exactly the traits its
/// generated code should implement (either derived or with a generated impl).
/// On failure, [`Error::TraitConflicts`] lists every required trait that some
/// type cannot satisfy.
pub(crate) fn resolve_traits<Id>(
    types: &mut BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    deserialized_defaults: &BTreeMap<Id, Id>,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    // First propagate required traits. A failure to satisfy a required trait
    // is an error.
    required_resolution(types, settings, deserialized_defaults)?;

    // Then propagate desired traits to the types that support them.
    desired_resolution(types, settings);

    Ok(())
}

/// Record, for every named type, whether its `FromStr` is irrefutable.
///
/// Runs after `break_cycles`, so the ids it walks are the final ones,
/// and before [`resolve_traits`], which reads the answers through
/// [`from_string_irrefutable`] as it consults [`feasibility`].
///
/// One chain walk per named type: the walk is the expensive part, and
/// the cache is what keeps `feasibility`, called once per type per
/// trait, from repeating it.
pub(crate) fn resolve_from_string_irrefutable<Id>(types: &mut BTreeMap<Id, Type<Id>>)
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let answers = types
        .iter()
        .filter(|(_, ty)| ty.is_named())
        .map(|(type_id, _)| (type_id.clone(), walk_wrapper_chain(types, type_id)))
        .collect::<Vec<_>>();

    for (type_id, answer) in answers {
        let common = types
            .get_mut(&type_id)
            .expect("the id came from this map")
            .common_mut()
            .expect("a named type has common metadata");
        common
            .built
            .as_mut()
            .expect("build_commons ran before this pass")
            .from_string_irrefutable = answer;
    }
}

/// Follow the wrapper chain from `id` to the type that decides whether
/// the chain stores its input verbatim.
///
/// An unconstrained newtype struct hands the question to its inner type
/// and a type alias to its target, so the chain is a run of single
/// links ending at the first type that answers for itself:
/// `Type::String` stores the input verbatim and everything else does
/// not. A constrained newtype answers for itself too, since its
/// `FromStr` validates what it parsed.
///
/// A chain that revisits a type has no such end and answers false. That
/// guard is what bounds the walk; `break_cycles` also puts a `Box` in
/// every cycle before this pass runs, and a `Box` ends a chain on its
/// own account.
fn walk_wrapper_chain<Id>(types: &BTreeMap<Id, Type<Id>>, id: &Id) -> bool
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let mut seen = BTreeSet::new();
    let mut here = id.clone();
    loop {
        if !seen.insert(here.clone()) {
            return false;
        }
        match types.get(&here).expect("every id names a type") {
            Type::String => return true,
            Type::NewtypeStruct(NewtypeStruct {
                inner,
                constraints: NewtypeConstraints::None,
                ..
            }) => here = inner.clone(),
            Type::TypeAlias(TypeAlias { target, .. }) => here = target.clone(),
            _ => return false,
        }
    }
}

/// Whether the `FromStr` of the type with `id` returns `Ok` for every
/// `&str`.
///
/// True when the value is stored verbatim, with no validation step
/// between the `&str` and the constructed value: `Type::String`, an
/// unconstrained newtype struct over an irrefutable type, and a type
/// alias to one. The property is syntactic, not semantic: a constraint
/// counts as validation even where it accepts every string, so a
/// `String` newtype whose only pattern is `".*"` is refutable.
/// Everything else, `Type::Native` included, is refutable.
///
/// `Type::String` is answered by matching the variant, which is why it
/// is not cached; the two recursive answers are read from the cache
/// [`resolve_from_string_irrefutable`] fills in, so this is a lookup
/// rather than a second walk.
///
/// # Panics
///
/// Panics if `id` names no type in `types`. Every id survives
/// finalization's reference check, so a panic here is a typespace bug.
pub(crate) fn from_string_irrefutable<Id>(types: &BTreeMap<Id, Type<Id>>, id: &Id) -> bool
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    match types.get(id).expect("every id names a type") {
        Type::String => true,
        ty => ty.common().is_some_and(|common| {
            common
                .built
                .as_ref()
                .expect("build_commons ran before trait resolution")
                .from_string_irrefutable
        }),
    }
}

/// The first variant of `enum_info`, in declaration order, whose
/// payload has an irrefutable `FromStr`.
///
/// Only the first is reported: one conflict per offending type is the
/// granularity trait resolution works at everywhere else. The message
/// could name every qualifying variant instead.
fn irrefutable_variant<Id>(types: &BTreeMap<Id, Type<Id>>, enum_info: &Enum<Id>) -> Option<String>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    enum_info
        .variants
        .iter()
        .find(|variant| match &variant.details {
            VariantDetails::Item(id) => from_string_irrefutable(types, id),
            VariantDetails::Unit | VariantDetails::Tuple(_) | VariantDetails::Struct(_) => false,
        })
        .map(|variant| variant.rust_name.clone())
}

/// Expand a requirement set to include the supertraits its members
/// imply.
///
/// Applied to every initial requirement set as it is formed (the
/// settings-required set, container obligations, and the serde-default and
/// default-value seeds), so a requirement enters the queue with the
/// supertraits it needs; otherwise a lone `Ord` requirement would derive `Ord`
/// without the `Eq`/`PartialEq`/`PartialOrd` impls it needs, and the emitted
/// derive would not compile.
fn close_supertraits(mut traits: TypespaceTraitSet) -> TypespaceTraitSet {
    if traits.contains(&TypespaceTrait::Ord) {
        traits.add(TypespaceTrait::PartialOrd);
        traits.add(TypespaceTrait::Eq);
        traits.add(TypespaceTrait::PartialEq);
    }
    if traits.contains(&TypespaceTrait::Eq) {
        traits.add(TypespaceTrait::PartialEq);
    }
    if traits.contains(&TypespaceTrait::PartialOrd) {
        traits.add(TypespaceTrait::PartialEq);
    }
    if traits.contains(&TypespaceTrait::Copy) {
        traits.add(TypespaceTrait::Clone);
    }
    traits
}

// Traits no container can provide.
const CONTAINER_UNSUPPORTED: &[TypespaceTrait] =
    &[TypespaceTrait::Display, TypespaceTrait::FromStr];

/// What a child-free built-in type provides for one trait, or `None`
/// for a type whose answer involves children or resolution.
///
/// This is the leaf slice of [`provides`], shared with
/// [`view::Type::has_impl`](crate::view::Type::has_impl) so the two
/// answer from one table.
pub(crate) fn leaf_provides<Id>(
    ty: &Type<Id>,
    trait_name: TypespaceTrait,
    settings: &Settings,
) -> Option<bool> {
    match ty {
        // Integers and booleans implement every trait we track.
        Type::Integer(_) | Type::Boolean => Some(true),

        // String implements every trait we track except Copy: an
        // owned heap buffer can never be Copy.
        Type::String => Some(trait_name != TypespaceTrait::Copy),

        // The unit type and ::json_serde::Absent implement
        // everything except Display and FromStr.
        Type::Unit | Type::Never => Some(!CONTAINER_UNSUPPORTED.contains(&trait_name)),

        // Floating-point types have no total ordering, no equality
        // relation, and no hash.
        Type::Float(_) => Some(!matches!(
            trait_name,
            TypespaceTrait::Ord | TypespaceTrait::Eq | TypespaceTrait::Hash
        )),

        // JsonValue implements everything except for Ord,
        // PartialOrd, and Copy: it owns a String and a Vec.
        Type::JsonValue => Some(!matches!(
            (trait_name, settings.typify_compat),
            // Never Ord, PartialOrd, or Copy.
            (
                TypespaceTrait::Ord | TypespaceTrait::PartialOrd | TypespaceTrait::Copy,
                _
            ) |
            // Not FromStr or Display when under typify compat.
            (TypespaceTrait::FromStr | TypespaceTrait::Display, true)
        )),

        _ => None,
    }
}

/// What a named type can do about one required trait.
///
/// The end-state answer for `(kind of type, trait)`--consulted once
/// per required trait at every named type reached during required
/// resolution. Obligation lists carry the same trait onward to
/// specific, relation-labeled targets; `Derivable` and `Forward`
/// obligate every contained child (via
/// [`Type::contained_children_related`]) and are handled identically
/// by the caller, but are kept distinct here because they mean
/// different things to rendering: a derive attribute versus an alias
/// whose fate is entirely its target's.
enum Feasibility<Id> {
    /// The type derives the trait as long as every contained child
    /// does.
    Derivable,
    /// The type realizes the trait with a manual impl; the listed
    /// relation-labeled targets must also implement it. Often empty:
    /// an attached default value, say, needs nothing from anyone
    /// else.
    ManuallyRealizable(Vec<(Relation, Id)>),
    /// A type alias: the trait's fate is entirely its target's.
    Forward,
    /// No derive and no manual impl exists for this kind of type and
    /// trait; the reason is the one a [`TraitConflict`] reports if the
    /// trait was required.
    Impossible(OffenderReason),
}

/// The vocabulary word for [`OffenderReason::TypeCannotImplement`]'s
/// `kind` field, matching the `kind` strings `Error::MissingTypeName`
/// uses for the same types. Only called for named types.
fn type_kind<Id>(ty: &Type<Id>) -> &'static str {
    match ty {
        Type::Struct(_) => "struct",
        Type::Enum(_) => "enum",
        Type::NewtypeStruct(_) => "newtype struct",
        Type::UnitStruct(_) => "unit struct",
        Type::TupleStruct(_) => "tuple struct",
        _ => unreachable!("type_kind is only called for named, non-alias types"),
    }
}

/// Consult the end-state feasibility table for `trait_name` on `ty`.
///
/// Only called for named types ([`Type::is_named`]); the table is
/// documented in full in the trait propagation design plan. In brief:
/// `Display` and `FromStr` have no derive and are impossible on plain
/// structs and tuple structs and on unit structs (their serde impls
/// are already hand-written, but that says nothing about rendering
/// text); newtype structs forward both to their inner type (a
/// constrained newtype's `FromStr` validates instead, with no
/// obligation); enums realize both with a hand-written impl when every
/// variant is a simple unit variant, or by forwarding to variant
/// payloads when the enum is untagged, and are otherwise impossible.
/// The two part company at one place: an untagged enum whose `FromStr`
/// would be decided by a payload that parses every string goes without
/// `FromStr` while keeping `Display`.
/// `Default` needs every constituent to implement it, unless the type
/// carries an attached default value, in which case the manual impl
/// needs nothing further (an enum with no attached default value has
/// no derive and no invented `#[default]` variant, so it is
/// impossible). Every other trait derives normally--`Copy` included:
/// `Derivable` already means every contained child must have the
/// trait, which is exactly `derive(Copy)`'s own condition, so no
/// separate case is needed for it.
fn feasibility<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    ty: &Type<Id>,
    trait_name: TypespaceTrait,
    settings: &Settings,
) -> Feasibility<Id>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    // Every `Impossible` outside the untagged-enum `FromStr` case says
    // the same thing: this kind of type has no way to implement this
    // trait. `type_kind` panics for an alias, which never reaches here.
    let cannot_implement = || {
        Feasibility::Impossible(OffenderReason::TypeCannotImplement {
            kind: type_kind(ty),
        })
    };

    match ty {
        // An alias has no impl site of its own to realize anything
        // with; every trait's fate belongs entirely to its target.
        Type::TypeAlias(_) => Feasibility::Forward,
        Type::Struct(struct_info) => {
            match trait_name {
                // Neither has a derive, and neither has a sensible
                // manual rendering: a struct's fields have no implied
                // textual order or separator.
                TypespaceTrait::Display | TypespaceTrait::FromStr => cannot_implement(),
                TypespaceTrait::Default => {
                    if let Some(default) = struct_info.common.default() {
                        // The hand-written impl takes each property the
                        // default value names from that value, and
                        // takes the rest from whatever that property
                        // itself supplies: its own attached default
                        // value where it has one, and
                        // Default::default() otherwise. Only the
                        // second of those calls the property type's
                        // Default, so only those properties are
                        // obligations. A flattened property has no wire
                        // name to look for, and a default value that is
                        // not a JSON object names nothing at all;
                        // either way the property counts as unnamed. An
                        // optional property is exempt whether the value
                        // names it or not: it renders as an Option (or
                        // as the configured optional-nullable type),
                        // which is Default whatever the property's own
                        // type is.
                        let map = default.as_object();
                        let obligations = struct_info
                            .properties
                            .iter()
                            .filter(|prop| {
                                // Does the default value specify a value for
                                // this property?
                                let default_has_prop = matches!(
                                    (map, prop.wire_name()),
                                    (Some(named), Some(wire_name)) if named.contains_key(wire_name)
                                );
                                // Does the property have a given default value
                                // either by virtue of being Optional or by
                                // having a DefaultValue attached to the
                                // property?
                                let prop_has_default = matches!(
                                    &prop.state,
                                    StructPropertyState::Optional
                                        | StructPropertyState::DefaultValue(_)
                                );

                                // If both are false (i.e. the provided default
                                // value doesn't specify a value for this
                                // property AND the property doesn't provide a
                                // value), then we need the type of the
                                // property to provide an implementation for
                                // default.
                                !default_has_prop && !prop_has_default
                            })
                            .map(|prop| {
                                (
                                    Relation::Field(prop.rust_name.clone()),
                                    prop.type_id.clone(),
                                )
                            })
                            .collect();
                        Feasibility::ManuallyRealizable(obligations)
                    } else if struct_info
                        .properties
                        .iter()
                        .any(|prop| matches!(&prop.state, StructPropertyState::Required))
                    {
                        // If there's any required property, Default is not
                        // possible.
                        cannot_implement()
                    } else {
                        // A property in the Default state fills from
                        // Default::default(), so its type must implement
                        // it. serde_default_properties seeds the same
                        // requirement for serde's #[serde(default)]; this
                        // records it again because the impl calls it in
                        // its own right. An optional property is an
                        // Option, Default whatever it holds, and a
                        // property with its own default value fills from a
                        // generated function.
                        let obligations = struct_info
                            .properties
                            .iter()
                            .filter(|prop| matches!(prop.state, StructPropertyState::Default))
                            .map(|prop| {
                                (
                                    Relation::Field(prop.rust_name.clone()),
                                    prop.type_id.clone(),
                                )
                            })
                            .collect();
                        Feasibility::ManuallyRealizable(obligations)
                    }
                }
                _ => Feasibility::Derivable,
            }
        }

        Type::TupleStruct(TupleStruct { common, .. }) => {
            match trait_name {
                // Neither has a derive, and neither has a sensible
                // manual rendering: a struct's fields have no implied
                // textual order or separator.
                TypespaceTrait::Display | TypespaceTrait::FromStr => cannot_implement(),
                // TYPIFY COMPAT
                TypespaceTrait::Default if settings.typify_compat => cannot_implement(),
                TypespaceTrait::Default => {
                    if common.default().is_some() {
                        Feasibility::ManuallyRealizable(Vec::new())
                    } else {
                        Feasibility::Derivable
                    }
                }
                _ => Feasibility::Derivable,
            }
        }

        Type::UnitStruct(_) => match trait_name {
            // Same reasoning as Struct: there is no field to render
            // and no textual form to parse.
            TypespaceTrait::Display | TypespaceTrait::FromStr => cannot_implement(),
            // TYPIFY COMPAT
            TypespaceTrait::Default if settings.typify_compat => cannot_implement(),
            // No fields means no obligations either way.
            _ => Feasibility::Derivable,
        },

        Type::NewtypeStruct(NewtypeStruct {
            common,
            inner,
            constraints,
            ..
        }) => match trait_name {
            // A newtype's Display is always the inner value's Display.
            TypespaceTrait::Display => {
                Feasibility::ManuallyRealizable(vec![(Relation::Inner, inner.clone())])
            }
            TypespaceTrait::FromStr => {
                if matches!(constraints, NewtypeConstraints::None) {
                    Feasibility::ManuallyRealizable(vec![(Relation::Inner, inner.clone())])
                } else {
                    // A constrained newtype's FromStr parses the
                    // inner value and then validates it; the inner
                    // type need not implement FromStr on its own
                    // account (it may not: a constrained String
                    // newtype has no separate FromStr obligation for
                    // String).
                    Feasibility::ManuallyRealizable(Vec::new())
                }
            }
            // An attached default value needs a hand-written impl that
            // NewtypeStruct::render does not write. Claiming the trait
            // would derive one that ignores the value, or fail to
            // compile where the inner type has no Default.
            // TYPIFY COMPAT: the typify_compat half only; the attached
            // default value half stands on its own.
            TypespaceTrait::Default if settings.typify_compat || common.default().is_some() => {
                cannot_implement()
            }
            TypespaceTrait::Default => Feasibility::Derivable,
            _ => Feasibility::Derivable,
        },

        Type::Enum(e) => match trait_name {
            TypespaceTrait::Display => {
                if e.all_tagged_unit_variants() {
                    // A hand-written impl maps variants to and from
                    // their serialized names; no variant has payload
                    // types to forward to.
                    Feasibility::ManuallyRealizable(Vec::new())
                } else if e.all_untagged_item_variants() {
                    // An untagged enum's serialized form is exactly
                    // one variant's payload's serialized form, so
                    // Display forwards to whichever payload types the
                    // variants carry.
                    Feasibility::ManuallyRealizable(
                        ty.contained_children_related()
                            .into_iter()
                            .map(|ContainedChild { relation, id, .. }| (relation, id))
                            .collect(),
                    )
                } else {
                    cannot_implement()
                }
            }
            TypespaceTrait::FromStr => {
                if e.all_tagged_unit_variants() {
                    // The same hand-written impl as Display's, read
                    // backwards: a serialized name maps to one variant.
                    Feasibility::ManuallyRealizable(Vec::new())
                } else if !e.all_untagged_item_variants() {
                    cannot_implement()
                } else {
                    // An untagged enum's FromStr tries the payload
                    // types in declaration order and takes the first
                    // that parses. A payload whose own FromStr accepts
                    // every string always wins, which makes every later
                    // variant unreachable and stops Display and FromStr
                    // round-tripping, so the enum goes without FromStr
                    // wherever a payload is irrefutable. Position in the
                    // variant list does not enter into it.
                    match irrefutable_variant(types, e) {
                        Some(variant) => {
                            Feasibility::Impossible(OffenderReason::IrrefutableVariantPayload {
                                variant,
                            })
                        }
                        None => Feasibility::ManuallyRealizable(
                            ty.contained_children_related()
                                .into_iter()
                                .map(|ContainedChild { relation, id, .. }| (relation, id))
                                .collect(),
                        ),
                    }
                }
            }
            TypespaceTrait::Default => {
                if e.common.default().is_some() {
                    Feasibility::ManuallyRealizable(Vec::new())
                } else {
                    // No derive exists, and there is no invented
                    // #[default] variant.
                    cannot_implement()
                }
            }
            _ => Feasibility::Derivable,
        },

        _ => unreachable!("feasibility is only called for named types"),
    }
}

/// The properties of `ty` that render with `#[serde(default)]`.
///
/// A struct's own properties and those of an enum's struct-shaped
/// variants, which render through the same path.
fn serde_default_properties<Id>(ty: &Type<Id>) -> Vec<&StructProperty<Id>> {
    let is_default =
        |prop: &&StructProperty<Id>| matches!(prop.state, StructPropertyState::Default);
    match ty {
        Type::Struct(struct_info) => struct_info.properties.iter().filter(is_default).collect(),
        Type::Enum(enum_info) => enum_info
            .variants
            .iter()
            .flat_map(|variant| match &variant.details {
                VariantDetails::Struct(properties) => {
                    properties.iter().filter(is_default).collect()
                }
                VariantDetails::Unit | VariantDetails::Item(_) | VariantDetails::Tuple(_) => {
                    Vec::new()
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn required_resolution<Id>(
    types: &mut BTreeMap<Id, Type<Id>>,
    settings: &Settings,
    deserialized_defaults: &BTreeMap<Id, Id>,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    /// A pending trait requirement: `traits` are required of `target`,
    /// tracing back to `origin` along the hops in `path`.
    struct WorkItem<Id> {
        target: Id,
        traits: TypespaceTraitSet,
        origin: RequirementOrigin<Id>,
        path: Vec<PathStep<Id>>,
    }

    // A container's blanket obligations are what its own type needs of its
    // parameters to exist at all: BTreeMap needs Ord of its key, HashMap needs
    // Eq and Hash. They're necessary irrespective of other constraints.
    // Finalization has checked each container's type parameter count, so every
    // position is present. A native's obligations are the same idea, stated
    // by the declaration rather than fixed by which container it is;
    // validation at insertion has already checked its parameter count.
    let mut work = types
        .iter()
        .flat_map(|(type_id, ty)| match ty {
            Type::Map(key_schema_ref, value_schema_ref) => vec![
                WorkItem {
                    target: key_schema_ref.clone(),
                    traits: close_supertraits(settings.map_type.obligation(0).clone()),
                    origin: RequirementOrigin::ContainerParameter {
                        container: type_id.clone(),
                        relation: Relation::Key,
                    },
                    path: Vec::new(),
                },
                WorkItem {
                    target: value_schema_ref.clone(),
                    traits: close_supertraits(settings.map_type.obligation(1).clone()),
                    origin: RequirementOrigin::ContainerParameter {
                        container: type_id.clone(),
                        relation: Relation::Value,
                    },
                    path: Vec::new(),
                },
            ],
            Type::Set(element_schema_ref) => vec![WorkItem {
                target: element_schema_ref.clone(),
                traits: close_supertraits(settings.set_type.obligation(0).clone()),
                origin: RequirementOrigin::ContainerParameter {
                    container: type_id.clone(),
                    relation: Relation::Element,
                },
                path: Vec::new(),
            }],
            Type::Vec(element_schema_ref) => vec![WorkItem {
                target: element_schema_ref.clone(),
                traits: close_supertraits(settings.vec_type.obligation(0).clone()),
                origin: RequirementOrigin::ContainerParameter {
                    container: type_id.clone(),
                    relation: Relation::Element,
                },
                path: Vec::new(),
            }],
            // A declared custom optional-nullable wrapper is consulted at
            // exactly the positions rendering substitutes it: the optional
            // struct-style properties whose type is itself an Option. What the
            // declaration demands of the wrapper's value type seeds from each
            // such property, like a map key's demand; the same Option node
            // reached any other way is a std Option and demands nothing.
            Type::Struct(_) | Type::Enum(_) => classify_edges(ty, types, settings)
                .into_iter()
                .filter_map(|edge| match edge {
                    Edge::Wrapped {
                        option_id,
                        container,
                        value_id,
                        ..
                    } => Some(WorkItem {
                        target: value_id,
                        traits: close_supertraits(container.obligation(0).clone()),
                        origin: RequirementOrigin::ContainerParameter {
                            container: option_id,
                            relation: Relation::Element,
                        },
                        path: Vec::new(),
                    }),
                    Edge::Plain(..) => None,
                })
                .collect::<Vec<_>>(),
            Type::Native(native) => native
                .parameters()
                .iter()
                .zip(native.obligations())
                .enumerate()
                .map(|(index, (param_id, obligation))| WorkItem {
                    target: param_id.clone(),
                    traits: close_supertraits(obligation.clone()),
                    origin: RequirementOrigin::ContainerParameter {
                        container: type_id.clone(),
                        relation: Relation::Parameter(index),
                    },
                    path: Vec::new(),
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect::<VecDeque<_>>();

    // A property whose state is StructPropertyState::Default renders as
    // #[serde(default)], and serde's derive expands that into a call to
    // T::default() on the property's type, so that type must implement
    // Default. Seed the requirement for every such property: a struct's
    // own, and those of an enum's struct-shaped variants, which render
    // through the same path. The other states impose nothing here: an
    // optional property is an Option<T>, which is Default whatever T
    // is; a property with its own default value names a function rather
    // than Default::default(); and a required property emits no serde
    // default at all.
    let default_required = close_supertraits(
        [TypespaceTrait::Default]
            .into_iter()
            .collect::<TypespaceTraitSet>(),
    );
    work.extend(types.iter().flat_map(|(type_id, ty)| {
        serde_default_properties(ty)
            .into_iter()
            .map(|prop| WorkItem {
                target: prop.type_id.clone(),
                traits: default_required.clone(),
                origin: RequirementOrigin::PropertyDefault(type_id.clone()),
                path: vec![PathStep {
                    type_id: type_id.clone(),
                    relation: Relation::Field(prop.rust_name.clone()),
                }],
            })
            .collect::<Vec<_>>()
    }));

    // A native-typed position in a default value is constructed in
    // generated code by deserializing it (default.rs's Type::Native
    // arm), so the value requires Deserialize of the native. The
    // default-value walk collected each such native during
    // check_type_defaults, keyed by the type whose value first reached
    // it; seed the requirement so an undeclared native conflicts here
    // instead of failing in the consumer's build.
    let deserialize_required = close_supertraits(
        [TypespaceTrait::Deserialize]
            .into_iter()
            .collect::<TypespaceTraitSet>(),
    );
    work.extend(
        deserialized_defaults
            .iter()
            .map(|(native_id, owner_id)| WorkItem {
                target: native_id.clone(),
                traits: deserialize_required.clone(),
                origin: RequirementOrigin::DefaultValue(owner_id.clone()),
                path: Vec::new(),
            }),
    );

    // Traits required via Settings::with_required_trait seed the trait
    // set of every named type. We route the seeds through the normal
    // work queue rather than writing them into TypeCommonBuilt directly
    // so that they propagate to contained types--and are checked against
    // native and built-in leaf types--exactly like structural
    // requirements.
    if !settings.required_traits.is_empty() {
        let required = close_supertraits(settings.required_traits.clone());
        work.extend(
            types
                .iter()
                .filter(|(_, ty)| ty.is_named())
                .map(|(type_id, _)| WorkItem {
                    target: type_id.clone(),
                    traits: required.clone(),
                    origin: RequirementOrigin::GlobalSettings,
                    path: Vec::new(),
                }),
        );
    }

    let mut conflicts = Vec::<TraitConflict<Id>>::new();

    // Split `traits` into those present in `unsupported` and the rest.
    let split = |traits: &TypespaceTraitSet, unsupported: &[TypespaceTrait]| {
        let bad = traits
            .iter()
            .filter(|tt| unsupported.contains(tt))
            .copied()
            .collect::<Vec<_>>();
        let rest = traits
            .iter()
            .filter(|tt| !unsupported.contains(tt))
            .copied()
            .collect::<TypespaceTraitSet>();
        (bad, rest)
    };

    // Drop Default from a requirement set: used at containers (vec, map,
    // set, option) that implement Default regardless of their element
    // types.
    let strip_default = |traits: TypespaceTraitSet| {
        traits
            .iter()
            .filter(|tt| !matches!(tt, TypespaceTrait::Default))
            .copied()
            .collect::<TypespaceTraitSet>()
    };

    // Split `traits` at a configurable container according to what the
    // container declares it provides: the traits it never provides are
    // conflicts here, the traits it provides only when its parameters do
    // pass to those parameters, and the traits it provides
    // unconditionally are satisfied and go no further.
    let container_split = |declaration: &ContainerType, traits: &TypespaceTraitSet| {
        let bad = traits
            .iter()
            .filter(|tt| matches!(declaration.provision(**tt), TraitProvision::Never))
            .copied()
            .collect::<Vec<_>>();
        // Re-close the forwarded set: dropping a trait the container
        // provides unconditionally can leave a subtrait behind without
        // its supertraits, and a parameter that absorbed `Eq` with no
        // `PartialEq` derives code that does not compile.
        let pass = close_supertraits(
            traits
                .iter()
                .filter(|tt| matches!(declaration.provision(**tt), TraitProvision::IfParameters))
                .copied()
                .collect::<TypespaceTraitSet>(),
        );
        (bad, pass)
    };

    // In each iteration, we need to assert the set of required traits to the
    // current type. If the current type is generated, that means consulting
    // its feasibility for each newly-required trait, absorbing what
    // it can derive or manually realize, and pushing obligations onward. If
    // the type is **not** generated (native or otherwise external to our
    // control), we need to check that it implements (or is capable of
    // implementing) the required traits; if it doesn't (or can't), we'll
    // produce an error. We don't stop on the first failure, but want to
    // identify as many distinct failures as is reasonable and as would be
    // useful for a consumer.
    while let Some(WorkItem {
        target,
        traits,
        origin,
        path,
    }) = work.pop_front()
    {
        let ty = types.get(&target).unwrap();

        // Record one conflict per unsatisfiable trait at this type.
        let mut conflict = |bad: Vec<TypespaceTrait>, reason: OffenderReason| {
            conflicts.extend(bad.into_iter().map(|required| TraitConflict {
                required,
                origin: origin.clone(),
                path: path.clone(),
                offender: target.clone(),
                reason: reason.clone(),
            }));
        };

        // Extend the path with a hop leaving the current type.
        let hop = |relation: Relation| {
            let mut next = path.clone();
            next.push(PathStep {
                type_id: target.clone(),
                relation,
            });
            next
        };

        if ty.is_named() {
            // A named type absorbs conditionally: consult feasibility
            // per required trait. Derivable and forwarded (alias)
            // traits share one obligation set--every contained child,
            // via contained_children_related--so we batch them and push
            // once per child. Manually realized traits carry their own,
            // often-empty, obligation list, so each gets its own push.
            // Impossible traits become conflicts and are never
            // absorbed.
            let mut derivable_new = TypespaceTraitSet::empty();
            let mut manual_pushes = Vec::<(TypespaceTrait, Vec<(Relation, Id)>)>::new();

            // Work against a copy of the built trait set: feasibility
            // borrows the whole graph, so we cannot hold a live
            // reference into it across the loop. Written back below in
            // one shot, once every read of `ty` is done.
            let mut built = ty.common().unwrap().built.as_ref().unwrap().traits.clone();

            for trait_name in traits {
                if built.contains(&trait_name) {
                    continue;
                }

                match feasibility(types, ty, trait_name, settings) {
                    Feasibility::Derivable | Feasibility::Forward => {
                        built.add(trait_name);
                        derivable_new.add(trait_name);
                    }
                    Feasibility::ManuallyRealizable(obligations) => {
                        built.add(trait_name);
                        if !obligations.is_empty() {
                            manual_pushes.push((trait_name, obligations));
                        }
                    }
                    Feasibility::Impossible(reason) => {
                        conflict(vec![trait_name], reason);
                    }
                }
            }

            let children = classify_edges(ty, types, settings);

            types
                .get_mut(&target)
                .unwrap()
                .common_mut()
                .unwrap()
                .built
                .as_mut()
                .unwrap()
                .traits = built;

            // Push each derivable trait to every child: a derived impl
            // compiles only if each field or payload type also implements the
            // trait.
            //
            // A wrapped edge is the exception. There, the generated field's
            // type is the declared custom optional-nullable wrapper, not the
            // graph's Option node, so the requirement is put to the wrapper's
            // declaration and each trait takes the route its answer dictates:
            // a trait the declaration never provides is unsatisfiable at this
            // property and becomes a conflict naming the wrapper; a trait it
            // always provides is satisfied and stops here; a trait it
            // provides when its parameter does continues to the option's
            // value type, with the option node recorded as a hop on the path.
            if !derivable_new.is_empty() {
                for edge in children {
                    match edge {
                        Edge::Wrapped {
                            relation,
                            option_id,
                            container,
                            value_id,
                        } => {
                            let (bad, pass) = container_split(container, &derivable_new);
                            conflicts.extend(bad.into_iter().map(|required| TraitConflict {
                                required,
                                origin: origin.clone(),
                                path: hop(relation.clone()),
                                offender: option_id.clone(),
                                reason: OffenderReason::Primitive {
                                    type_name: path_text(container.path()),
                                },
                            }));
                            if !pass.is_empty() {
                                let mut path = hop(relation);
                                path.push(PathStep {
                                    type_id: option_id,
                                    relation: Relation::Element,
                                });
                                work.push_back(WorkItem {
                                    target: value_id,
                                    traits: pass,
                                    origin: origin.clone(),
                                    path,
                                });
                            }
                        }
                        Edge::Plain(relation, child_id) => work.push_back(WorkItem {
                            target: child_id,
                            traits: derivable_new.clone(),
                            origin: origin.clone(),
                            path: hop(relation),
                        }),
                    }
                }
            }
            // Manual obligations need no wrapper routing: feasibility builds
            // them only from default-state properties and from untagged item
            // variants, and the wrapper substitutes only at optional-state
            // properties, so no obligation ever names a wrapped edge.
            //
            // Pushing one bare trait with no supertrait closure is
            // sound only while every manually realizable trait
            // (Display, FromStr, Default) has no supertraits. If a
            // trait with supertraits ever becomes manually realizable,
            // expand each pushed set with its dependencies.
            for (trait_name, obligations) in manual_pushes {
                for (relation, child_id) in obligations {
                    work.push_back(WorkItem {
                        target: child_id,
                        traits: [trait_name].into_iter().collect::<TypespaceTraitSet>(),
                        origin: origin.clone(),
                        path: hop(relation),
                    });
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

                // A native's declaration answers the same way a
                // configured container's does, through the same split:
                // a trait it never implements conflicts, one it
                // implements unconditionally is satisfied and goes no
                // further, and one it implements only when every type
                // parameter does passes onward to each parameter,
                // exactly as a container's own IfParameters traits
                // reach its parameters. A trait its declaration cannot
                // answer for (Unknown) is neither bad nor pass here, so
                // it silently passes without reaching any parameter:
                // refusing to generate for a valid schema is worse than
                // a compile error naming the real missing impl, and a
                // source like typify's `x-rust-type` has no way to
                // declare more.
                Type::Native(native) => {
                    let (bad, pass) = container_split(&native.container, &traits);
                    conflict(
                        bad,
                        OffenderReason::NativeMissingImpl {
                            type_name: path_text(native.path()),
                        },
                    );
                    if !pass.is_empty() {
                        for (index, param_id) in native.parameters().iter().enumerate() {
                            work.push_back(WorkItem {
                                target: param_id.clone(),
                                traits: pass.clone(),
                                origin: origin.clone(),
                                path: hop(Relation::Parameter(index)),
                            });
                        }
                    }
                }

                // Option<T> implements everything we care about--except
                // for Display and FromStr--as long as T implements them.
                // Option<T> additionally implements Default unconditionally.
                Type::Option(schema_ref) => {
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "Option".to_string(),
                        },
                    );
                    let pass = strip_default(rest);
                    if !pass.is_empty() {
                        work.push_back(WorkItem {
                            target: schema_ref.clone(),
                            traits: pass,
                            origin,
                            path: hop(Relation::Element),
                        });
                    }
                }

                // The utility of Box is primarily to break containment cycles.
                // We treat it like a container with regard to trait
                // forwarding, except for Copy: a box heap-allocates and is
                // never Copy no matter what it holds.
                Type::Box(schema_ref) => {
                    let (bad, rest) = split(
                        &traits,
                        &[
                            TypespaceTrait::Display,
                            TypespaceTrait::FromStr,
                            TypespaceTrait::Copy,
                        ],
                    );
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "Box".to_string(),
                        },
                    );
                    if !rest.is_empty() {
                        work.push_back(WorkItem {
                            target: schema_ref.clone(),
                            traits: rest,
                            origin,
                            path: hop(Relation::Boxed),
                        });
                    }
                }

                // The configured vec type states which traits it never
                // provides, which it provides whatever the element does,
                // and which follow the element.
                Type::Vec(schema_ref) => {
                    let (bad, pass) = container_split(&settings.vec_type, &traits);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "Vec".to_string(),
                        },
                    );
                    if !pass.is_empty() {
                        work.push_back(WorkItem {
                            target: schema_ref.clone(),
                            traits: pass,
                            origin,
                            path: hop(Relation::Element),
                        });
                    }
                }
                Type::Array(schema_ref, _) => {
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "array".to_string(),
                        },
                    );
                    if !rest.is_empty() {
                        work.push_back(WorkItem {
                            target: schema_ref.clone(),
                            traits: rest,
                            origin,
                            path: hop(Relation::Element),
                        });
                    }
                }
                // Tuples implement everything except for Display and FromStr
                // as long as all their component types do as well.
                Type::Tuple(schema_refs) => {
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "tuple".to_string(),
                        },
                    );
                    if !rest.is_empty() {
                        for schema_ref in schema_refs {
                            work.push_back(WorkItem {
                                target: schema_ref.clone(),
                                traits: rest.clone(),
                                origin: origin.clone(),
                                path: hop(Relation::Element),
                            });
                        }
                    }
                }

                // The configured map and set types answer the same way,
                // over the key and value parameters and over the element
                // parameter respectively.
                Type::Map(key_ref, value_ref) => {
                    let (bad, pass) = container_split(&settings.map_type, &traits);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "map".to_string(),
                        },
                    );
                    if !pass.is_empty() {
                        work.push_back(WorkItem {
                            target: key_ref.clone(),
                            traits: pass.clone(),
                            origin: origin.clone(),
                            path: hop(Relation::Key),
                        });
                        work.push_back(WorkItem {
                            target: value_ref.clone(),
                            traits: pass,
                            origin,
                            path: hop(Relation::Value),
                        });
                    }
                }
                Type::Set(element_ref) => {
                    let (bad, pass) = container_split(&settings.set_type, &traits);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "set".to_string(),
                        },
                    );
                    if !pass.is_empty() {
                        work.push_back(WorkItem {
                            target: element_ref.clone(),
                            traits: pass,
                            origin,
                            path: hop(Relation::Element),
                        });
                    }
                }

                // Floating-point types have no total ordering, no equality
                // relation, and no hash.
                Type::Float(name) => {
                    let (bad, _) = split(
                        &traits,
                        &[
                            TypespaceTrait::Ord,
                            TypespaceTrait::Eq,
                            TypespaceTrait::Hash,
                        ],
                    );
                    let reason = OffenderReason::Primitive {
                        type_name: name.clone(),
                    };
                    conflict(bad, reason);
                }

                // Integers and booleans implement every trait we track.
                Type::Integer(_) | Type::Boolean => (),

                // The unit type implements everything except Display and
                // FromStr.
                Type::Unit => {
                    let (bad, _) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "()".to_string(),
                        },
                    );
                }

                // String implements every trait we track except Copy: an
                // owned heap buffer can never be Copy.
                Type::String => {
                    let (bad, _) = split(&traits, &[TypespaceTrait::Copy]);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "String".to_string(),
                        },
                    );
                }

                // JsonValue implements everything except for Ord,
                // PartialOrd, and Copy: serde_json::Value derives Clone,
                // Eq, PartialEq, and Hash, has no ordering impls, and owns
                // a String and a Vec, which rule out Copy.
                Type::JsonValue => {
                    let unsupported = if settings.typify_compat {
                        vec![
                            TypespaceTrait::Ord,
                            TypespaceTrait::PartialOrd,
                            TypespaceTrait::Copy,
                            TypespaceTrait::Display,
                            TypespaceTrait::FromStr,
                        ]
                    } else {
                        vec![
                            TypespaceTrait::Ord,
                            TypespaceTrait::PartialOrd,
                            TypespaceTrait::Copy,
                        ]
                    };
                    let (bad, _) = split(&traits, &unsupported);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "serde_json::Value".to_string(),
                        },
                    );
                }

                // ::json_serde::Absent derives Clone, Debug, Default, Eq,
                // Hash, Ord, PartialEq, and PartialOrd, and hand-writes
                // Serialize, Deserialize, and (under the schemars08 and
                // schemars1 features) JsonSchema; it has no Display or
                // FromStr impl (see json-serde/src/lib.rs).
                Type::Never => {
                    let (bad, _) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "json_serde::Absent".to_string(),
                        },
                    );
                }
            }
        }
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(Error::TraitConflicts { conflicts })
    }
}

/// `trait_name` and every trait that cannot survive without it.
///
/// The supertrait closure run backwards: `Ord` needs `PartialOrd`,
/// `Eq`, and `PartialEq`, and `Eq` and `PartialOrd` each need
/// `PartialEq`; `Copy` needs `Clone`. A type that loses one of those
/// loses everything resting on it.
fn strip_dependents(trait_name: TypespaceTrait) -> impl Iterator<Item = TypespaceTrait> {
    let dependents: &'static [TypespaceTrait] = match trait_name {
        TypespaceTrait::PartialEq => &[
            TypespaceTrait::Eq,
            TypespaceTrait::PartialOrd,
            TypespaceTrait::Ord,
        ],
        TypespaceTrait::Eq | TypespaceTrait::PartialOrd => &[TypespaceTrait::Ord],
        TypespaceTrait::Clone => &[TypespaceTrait::Copy],
        _ => &[],
    };
    std::iter::once(trait_name).chain(dependents.iter().copied())
}

/// Whether a container-shaped declaration ([`ContainerType`], or the
/// [`ContainerType`] a [`Native`](crate::build::Native) composes)
/// provides `trait_name`.
///
/// `parameters` answers whether every one of the declaration's type
/// parameters has the trait; it is consulted only when the declaration
/// makes the impl conditional on them. `Unknown` answers `false`, the
/// same as `Never`: reached only through a native (finalization
/// rejects a configured container that declares it), and a desired
/// trait is never granted on a guess.
fn container_provides(
    declaration: &ContainerType,
    trait_name: TypespaceTrait,
    parameters: impl FnOnce() -> bool,
) -> bool {
    match declaration.provision(trait_name) {
        TraitProvision::Never | TraitProvision::Unknown => false,
        TraitProvision::Always => true,
        TraitProvision::IfParameters => parameters(),
    }
}

/// What an unnamed (built-in or container) type provides for
/// `trait_name`, given `child_has`.
///
/// This is the non-named branch of [`provides`], shared with
/// [`view::Type::has_impl`](crate::view::Type::has_impl) so the two
/// answer from one rule set. `provides` asks a child through its `has`
/// map (what desired resolution has not yet stripped); the view asks a
/// child by recursing into the finalized typespace. Either way: a
/// configured container answers from its declaration, and the
/// containers a consumer cannot configure forward a trait to their
/// parameters, except that none has `Display` or `FromStr` and
/// `Option` provides `Default` whatever it holds.
pub(crate) fn unnamed_provides<Id>(
    ty: &Type<Id>,
    trait_name: TypespaceTrait,
    settings: &Settings,
    child_has: &mut dyn FnMut(&Id) -> bool,
) -> bool {
    let supported = !CONTAINER_UNSUPPORTED.contains(&trait_name);
    let is_default = matches!(trait_name, TypespaceTrait::Default);

    match ty {
        Type::Enum(_)
        | Type::Struct(_)
        | Type::UnitStruct(_)
        | Type::TupleStruct(_)
        | Type::NewtypeStruct(_)
        | Type::TypeAlias(_) => unreachable!(),

        // A native answers exactly as a configured container does,
        // through the same table and the same helper: unconditionally,
        // never, or when every type parameter provides the trait.
        // Unknown answers false here too--granting a desired trait a
        // declaration cannot answer for would emit a derive nobody
        // asked for.
        Type::Native(native) => container_provides(&native.container, trait_name, || {
            native.parameters.iter().all(child_has)
        }),

        // Pass the buck, minus Display and FromStr, which an Option<T> never
        // implements, and Default, which Option<T> always implements.
        Type::Option(schema_ref) => supported && (is_default || child_has(schema_ref)),
        // Box is never Copy, whatever it holds; every other trait
        // follows the boxed type.
        Type::Box(schema_ref) => {
            supported && trait_name != TypespaceTrait::Copy && child_has(schema_ref)
        }

        // The configurable containers answer from what their
        // declaration says they provide, the same table required
        // resolution consults.
        Type::Vec(schema_ref) => {
            container_provides(&settings.vec_type, trait_name, || child_has(schema_ref))
        }
        Type::Set(schema_ref) => {
            container_provides(&settings.set_type, trait_name, || child_has(schema_ref))
        }
        Type::Map(key_ref, value_ref) => container_provides(&settings.map_type, trait_name, || {
            child_has(key_ref) && child_has(value_ref)
        }),

        // Arrays and tuples forward everything they can provide,
        // Default included.
        Type::Array(schema_ref, _) => supported && child_has(schema_ref),
        Type::Tuple(schema_refs) => supported && schema_refs.iter().all(child_has),

        // Child-free built-ins answer from the shared leaf table.
        Type::Integer(_)
        | Type::Boolean
        | Type::String
        | Type::Unit
        | Type::Never
        | Type::Float(_)
        | Type::JsonValue => leaf_provides(ty, trait_name, settings)
            .expect("every arm above is a leaf the table answers"),
    }
}

/// Whether `ty` provides `trait_name`, given `has`.
///
/// `has` records what the types `ty` is built from still have. A
/// named type answers from the feasibility table: an impossible
/// trait is never provided, a derived or forwarded one needs every
/// contained child, and a manually realized one needs only the targets
/// that realization obligates--often none at all, as with an attached
/// default value. Every other type answers through
/// [`unnamed_provides`].
fn provides<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    ty: &Type<Id>,
    trait_name: TypespaceTrait,
    has: &BTreeMap<Id, TypespaceTraitSet>,
    settings: &Settings,
) -> bool
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let mut child_has = |child: &Id| {
        has.get(child)
            .is_some_and(|traits| traits.contains(&trait_name))
    };

    if ty.is_named() {
        match feasibility(types, ty, trait_name, settings) {
            Feasibility::Impossible(_) => false,
            // A wrapped property edge answers from the declared custom
            // optional-nullable wrapper, exactly as required resolution
            // routes it; every other edge asks the child directly.
            Feasibility::Derivable | Feasibility::Forward => classify_edges(ty, types, settings)
                .into_iter()
                .all(|edge| match edge {
                    Edge::Wrapped {
                        container,
                        value_id,
                        ..
                    } => container_provides(container, trait_name, || child_has(&value_id)),
                    Edge::Plain(_, child_id) => child_has(&child_id),
                }),
            Feasibility::ManuallyRealizable(obligations) => obligations
                .iter()
                .all(|(_, obligated)| child_has(obligated)),
        }
    } else {
        unnamed_provides(ty, trait_name, settings, &mut child_has)
    }
}

/// The traits required resolution granted `type_id`.
///
/// `None` for a type with no trait set of its own.
fn granted_traits<'a, Id>(
    types: &'a BTreeMap<Id, Type<Id>>,
    type_id: &Id,
) -> Option<&'a TypespaceTraitSet>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    types
        .get(type_id)
        .and_then(|ty| ty.common())
        .and_then(|common| common.built.as_ref())
        .map(|built| &built.traits)
}

/// One type's loss of one desired trait, yet to propagate.
struct Loss<Id> {
    loser: Id,
    trait_name: TypespaceTrait,
}

/// The desired phase's working state.
///
/// What each type still has, and the losses waiting to reach the
/// types that refer to the loser.
struct Poison<Id> {
    has: BTreeMap<Id, TypespaceTraitSet>,
    queue: VecDeque<Loss<Id>>,
}

impl<Id> Poison<Id>
where
    Id: Clone + Ord + std::fmt::Display,
{
    /// Take `trait_name` and its dependents from `loser`.
    ///
    /// Each trait that was still there is queued for propagation.
    /// `blocker` is the type whose own loss caused this one, or
    /// `loser` itself when the type simply cannot provide the trait;
    /// it names the cause in the log line. `granted` is what required
    /// resolution gave the type: those traits stay, since phase 1
    /// proved them realizable through every hop below.
    fn lose(
        &mut self,
        loser: &Id,
        trait_name: TypespaceTrait,
        blocker: &Id,
        granted: Option<&TypespaceTraitSet>,
    ) {
        let traits = self.has.get_mut(loser).unwrap();
        for lost in strip_dependents(trait_name) {
            if granted.is_some_and(|granted| granted.contains(&lost)) {
                continue;
            }
            if traits.remove(lost) {
                debug!("desired trait {lost} dropped from {loser}, blocked by {blocker}");
                self.queue.push_back(Loss {
                    loser: loser.clone(),
                    trait_name: lost,
                });
            }
        }
    }
}

/// Give each named type the desired traits that it's capable of supporting.
///
/// Each type starts out assumed to have every desired trait. The traits a type
/// cannot implement seed a work queue that poisons that trait in the
/// referencing types. This poisons their own referrers in turn, until the
/// queue drains. Some types don't require a referenced type to implement a
/// trait in order to provide it. For example a `Vec<T>` can implement
/// `Default` irrespective of whether `T` does.
///
/// This is effectively the reverse of what we do when forward-propagating
/// required traits. Types retain the desired trait because no transitive
/// child poisons it.
///
/// Unlike with required traits, a failure to implement a desired trait is
/// logged but doesn't produce an error.
fn desired_resolution<Id>(types: &mut BTreeMap<Id, Type<Id>>, settings: &Settings)
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    // Poison tracking covers each desired trait's supertraits as well,
    // not just the traits settings named, since a trait is granted
    // below only when every supertrait it needs survived too: granting
    // Ord without PartialEq renders a derive that does not compile.
    let desired = close_supertraits(settings.desired_traits.clone());
    if desired.is_empty() {
        return;
    }

    // Each desired trait paired with the supertraits it needs. A
    // supertrait is not desired on its own account--it is there so
    // that a granted derive compiles--so a group is granted or dropped
    // whole: a type that cannot have Eq has no use for the PartialEq
    // that Eq pulled in.
    let grant_groups = settings
        .desired_traits
        .iter()
        .map(|trait_name| close_supertraits([*trait_name].into_iter().collect()))
        .collect::<Vec<_>>();

    // The inverse of Type::children: the types referring to each type,
    // which is the direction a loss travels. This reaches a native's
    // type parameters too--Type::children reports them, exactly as it
    // reports a container's--so a loss at a parameter poisons the
    // native back through the same route a loss at a map's key
    // poisons the map.
    let referrers = types.iter().fold(
        BTreeMap::<Id, Vec<Id>>::new(),
        |mut referrers, (type_id, ty)| {
            for child in ty.children() {
                referrers.entry(child).or_default().push(type_id.clone());
            }
            referrers
        },
    );

    let mut state = Poison {
        has: types
            .keys()
            .map(|type_id| (type_id.clone(), desired.clone()))
            .collect(),
        queue: VecDeque::new(),
    };

    // Seed the queue with what a type cannot provide even with every
    // constituent assumed capable: a native or built-in leaf without
    // the impl, a container that has no such impl at all, and a named
    // type whose kind rules the trait out.
    let seeds = types
        .iter()
        .flat_map(|(type_id, ty)| {
            desired
                .iter()
                .filter(|trait_name| !provides(types, ty, **trait_name, &state.has, settings))
                .map(|trait_name| (type_id.clone(), *trait_name))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (type_id, trait_name) in seeds {
        let granted = granted_traits(types, &type_id);
        state.lose(&type_id, trait_name, &type_id, granted);
    }

    while let Some(Loss { loser, trait_name }) = state.queue.pop_front() {
        for referrer in referrers.get(&loser).into_iter().flatten() {
            let ty = types.get(referrer).unwrap();
            if state.has[referrer].contains(&trait_name)
                && !provides(types, ty, trait_name, &state.has, settings)
            {
                let granted = granted_traits(types, referrer);
                state.lose(referrer, trait_name, &loser, granted);
            }
        }
    }

    // A named type's built set is what required resolution absorbed
    // plus the desired groups that survived whole.
    for (type_id, ty) in types.iter_mut() {
        if let Some(common) = ty.common_mut() {
            let survivors = state.has.remove(type_id).unwrap();
            let built = common.built.as_mut().unwrap();
            for group in &grant_groups {
                if group
                    .iter()
                    .all(|trait_name| survivors.contains(trait_name))
                {
                    for trait_name in group.iter() {
                        built.traits.add(*trait_name);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Feasibility, feasibility, from_string_irrefutable};
    use crate::{
        Typespace, TypespaceBuilder, TypespaceTrait, TypespaceTraitSet,
        build::{
            JsonValue, Native, NewtypeConstraints, NewtypeStruct, Struct, StructProperty,
            StructPropertySerde, StructPropertyState, TupleStruct, Type,
        },
        error::{Error, OffenderReason, Relation, RequirementOrigin},
        no_cycles,
        settings::{ContainerType, Settings},
    };
    use typespace_test_macro::typespace_builder;

    /// The built trait set of the type with `id`.
    fn built_traits(typespace: &Typespace<String>, id: &str) -> TypespaceTraitSet {
        typespace
            .types
            .get(id)
            .unwrap()
            .common()
            .unwrap()
            .built
            .as_ref()
            .unwrap()
            .traits
            .clone()
    }

    /// Required traits reach every named type and flow through
    /// containment: settings-required traits land on both the outer
    /// struct and the struct it contains, and nothing else lands.
    #[test]
    fn required_propagates_through_struct_graph() {
        let builder = typespace_builder!(Settings::typical(), {
            struct Inner {
                count: u32,
            }

            struct Outer {
                name: String,
                inner: Inner,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Clone,
            TypespaceTrait::Debug,
            TypespaceTrait::Serialize,
            TypespaceTrait::Deserialize,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "Outer"), expected);
        assert_eq!(built_traits(&typespace, "Inner"), expected);
    }

    /// An unsatisfiable requirement reports a coherent chain: the
    /// origin (map key), each containment hop, the offending type, and
    /// the reason--and exactly one conflict per failing trait, not one
    /// per ancestor.
    #[test]
    fn map_key_conflict_reports_path() {
        // The Map<KeyStruct, String> node has to be reachable from some
        // item for typespace_builder! to insert it (unlike the builder,
        // it has no way to insert a type with no name and no
        // reference); a throwaway alias is the closest fit.
        let builder = typespace_builder!(Settings::minimal(), {
            struct KeyStruct {
                weight: f64,
            }

            type KeyStructMap = Map<KeyStruct, String>;
        });

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        // The default map key requirements are Eq, PartialEq, Ord, and
        // PartialOrd; floats provide the partial pair, so exactly Eq
        // and Ord fail.
        assert_eq!(conflicts.len(), 2, "conflicts: {conflicts:#?}");
        for required in [TypespaceTrait::Eq, TypespaceTrait::Ord] {
            let conflict = conflicts
                .iter()
                .find(|conflict| conflict.required == required)
                .unwrap_or_else(|| panic!("no conflict for {required}"));
            assert!(matches!(
                &conflict.origin,
                RequirementOrigin::ContainerParameter { container, relation }
                    if container == "Map<KeyStruct, String>"
                        && matches!(relation, Relation::Key)
            ));
            assert_eq!(conflict.offender, "f64");
            assert!(matches!(
                &conflict.reason,
                OffenderReason::Primitive { type_name } if type_name == "f64"
            ));
            // One hop: the key struct passes the requirement to its
            // field.
            assert_eq!(conflict.path.len(), 1, "path: {:#?}", conflict.path);
            assert_eq!(conflict.path[0].type_id, "KeyStruct");
            assert!(matches!(
                &conflict.path[0].relation,
                Relation::Field(name) if name == "weight"
            ));
        }
    }

    /// A property in the `Default` state renders as
    /// `#[serde(default)]`, and serde's derive expands that into a call
    /// to `T::default()`, so the property's type is required to
    /// implement `Default`. A type that cannot is a finalization error
    /// naming the field it came from.
    #[test]
    fn serde_default_property_conflicts_when_type_cannot_default() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Inner {
                count: u32,
            }

            struct Outer {
                #[default]
                inner: Inner,
            }
        });

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::Default);
        assert!(matches!(
            &conflict.origin,
            RequirementOrigin::PropertyDefault(id) if id == "Outer"
        ));
        assert_eq!(conflict.offender, "Inner");
        assert!(matches!(
            &conflict.reason,
            OffenderReason::TypeCannotImplement { kind } if *kind == "struct"
        ));
        // One hop: the outer struct passes the requirement to its
        // field.
        assert_eq!(conflict.path.len(), 1, "path: {:#?}", conflict.path);
        assert_eq!(conflict.path[0].type_id, "Outer");
        assert!(matches!(
            &conflict.path[0].relation,
            Relation::Field(name) if name == "inner"
        ));
    }

    /// A struct-shaped enum variant renders its fields through the same
    /// path a struct's go through, `#[serde(default)]` included, so a
    /// property in the `Default` state there makes the same
    /// requirement.
    #[test]
    fn serde_default_property_in_variant_conflicts() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Inner {
                count: u32,
            }

            enum Shape {
                Rect {
                    #[default]
                    inner: Inner,
                },
            }
        });

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::Default);
        assert!(matches!(
            &conflict.origin,
            RequirementOrigin::PropertyDefault(id) if id == "Shape"
        ));
        assert_eq!(conflict.offender, "Inner");
        assert_eq!(conflict.path.len(), 1, "path: {:#?}", conflict.path);
        assert_eq!(conflict.path[0].type_id, "Shape");
        assert!(matches!(
            &conflict.path[0].relation,
            Relation::Field(name) if name == "inner"
        ));
    }

    /// The type of a property in the `Default` state takes `Default`
    /// when it can. The requirement lands on that type alone: the
    /// struct holding the property is not required to implement
    /// anything.
    #[test]
    fn serde_default_property_grants_default_to_its_type() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Inner {
                #[default]
                count: u32,
            }

            struct Outer {
                #[default]
                inner: Inner,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Inner"),
            trait_set([TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&typespace, "Outer"),
            TypespaceTraitSet::empty()
        );
    }

    /// The requirement stops at a container that implements `Default`
    /// whatever it holds: a `Vec` property in the `Default` state asks
    /// nothing of the element type.
    #[test]
    fn serde_default_property_requirement_stops_at_vec() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Tag {
                id: u32,
            }

            struct S {
                #[default]
                tags: Vec<Tag>,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(built_traits(&typespace, "Tag"), TypespaceTraitSet::empty());
        assert_eq!(built_traits(&typespace, "S"), TypespaceTraitSet::empty());
    }

    /// A required trait that a type can neither derive nor realize
    /// manually is a finalization error, not a rendering panic.
    #[test]
    fn required_display_on_struct_conflicts() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Display),
            {
                struct S {
                    s: String,
                }
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::Display);
        assert!(matches!(conflict.origin, RequirementOrigin::GlobalSettings));
        assert_eq!(conflict.offender, "S");
        assert!(conflict.path.is_empty(), "path: {:#?}", conflict.path);
        assert!(matches!(
            conflict.reason,
            OffenderReason::TypeCannotImplement { kind: "struct" }
        ));
    }

    /// A required Display reaching an Option conflicts at the Option,
    /// naming it as the offender, instead of forwarding to the element.
    #[test]
    fn required_display_on_option_conflicts() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Display),
            {
                struct Wrapper(Nullable<String>);
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::Display);
        assert_eq!(conflict.offender, "Nullable<String>");
        assert!(matches!(
            &conflict.reason,
            OffenderReason::Primitive { type_name } if type_name == "Option"
        ));
    }

    /// The same as `required_display_on_option_conflicts`, for FromStr.
    #[test]
    fn required_fromstr_on_option_conflicts() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::FromStr),
            {
                struct Wrapper(Nullable<String>);
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::FromStr);
        assert_eq!(conflict.offender, "Nullable<String>");
        assert!(matches!(
            &conflict.reason,
            OffenderReason::Primitive { type_name } if type_name == "Option"
        ));
    }

    /// The same as `required_display_on_option_conflicts`, for a Box.
    #[test]
    fn required_display_on_box_conflicts() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Display),
            {
                struct Wrapper(Box<String>);
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        let conflict = &conflicts[0];
        assert_eq!(conflict.required, TypespaceTrait::Display);
        assert!(matches!(
            &conflict.reason,
            OffenderReason::Primitive { type_name } if type_name == "Box"
        ));
    }

    /// A trait an Option genuinely provides still passes through to the
    /// element and is satisfied there: the default set-element traits reach
    /// Color through the Option and land on it.
    #[test]
    fn option_forwards_supported_trait_to_element() {
        let builder = typespace_builder!(Settings::minimal(), {
            enum Color {
                Red,
                Green,
            }

            type ColorSet = Set<Nullable<Color>>;
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
            TypespaceTrait::Ord,
            TypespaceTrait::PartialOrd,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "Color"), expected);
    }

    /// A required Default is satisfied at the Option itself and never
    /// reaches the element, which here could not provide it: a set
    /// element required to be Default reaches Color only through the
    /// Option, and Color's own built trait set stays empty.
    #[test]
    fn option_default_satisfied_without_reaching_element() {
        let settings = Settings::minimal().with_set_type(
            ContainerType::hash_set()
                .with_obligations([[TypespaceTrait::Default].into_iter().collect()]),
        );
        let builder = typespace_builder!(settings, {
            enum Color {
                Red,
                Green,
            }

            type ColorSet = Set<Nullable<Color>>;
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert!(built_traits(&typespace, "Color").is_empty());
    }

    /// A required trait that a type realizes with a manual impl is
    /// satisfied: an all-unit enum provides Display and FromStr, so
    /// requiring them succeeds and they land in the built trait set.
    #[test]
    fn required_display_on_simple_enum_accepted() {
        let builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::Display)
                .with_required_trait(TypespaceTrait::FromStr),
            {
                enum Color {
                    Red,
                    Green,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [TypespaceTrait::Display, TypespaceTrait::FromStr]
            .into_iter()
            .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "Color"), expected);
    }

    /// A `Never` field satisfies every trait `Settings::typical`
    /// requires: `::json_serde::Absent` derives `Clone` and `Debug` and
    /// hand-writes `Serialize` and `Deserialize` (see
    /// json-serde/src/lib.rs), so a struct containing one finalizes
    /// without conflicts and the field's own trait set matches the
    /// struct's.
    #[test]
    fn never_field_satisfies_typical_settings() {
        let builder = typespace_builder!(Settings::typical(), {
            struct S {
                gone: Optional<!>,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Clone,
            TypespaceTrait::Debug,
            TypespaceTrait::Serialize,
            TypespaceTrait::Deserialize,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "S"), expected);
    }

    /// A newtype struct wrapping `Never` is rejected by validation
    /// before trait resolution runs, so a required `Display` on
    /// `Wrapper` never reaches the `Never` leaf to conflict over.
    #[test]
    fn required_display_conflicts_on_never_field() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Display),
            {
                struct Wrapper(!);
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::NeverInTransparentWrapper { wrapper, type_id } = err else {
            panic!("expected NeverInTransparentWrapper, got: {err}");
        };
        assert_eq!(wrapper, "newtype struct");
        assert_eq!(type_id, "Wrapper");
    }

    /// Requiring a trait requires its supertraits: Ord alone expands
    /// to every comparison trait, or the emitted derive would not
    /// compile.
    #[test]
    fn supertrait_closure_expands_ord() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Ord),
            {
                struct S {
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Ord,
            TypespaceTrait::PartialOrd,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "S"), expected);
    }

    /// A `native` item declares only the traits it lists: unlike a
    /// required trait set, which closes over supertraits, `Ord` here
    /// does not also mean `PartialOrd` or `Eq`.
    #[test]
    fn native_traits_add_no_supertraits() {
        let builder = typespace_builder!(Settings::minimal(), {
            native ::chrono::NaiveDate: Eq + PartialEq + Ord + PartialOrd;

            type DateMap = Map<::chrono::NaiveDate, String>;
        });
        builder.finalize(no_cycles).expect("finalization succeeds");

        // `Ord` alone implies nothing: the other three map key
        // requirements are still missing.
        let builder = typespace_builder!(Settings::minimal(), {
            native ::chrono::NaiveDate: Ord;

            type DateMap = Map<::chrono::NaiveDate, String>;
        });
        let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
            panic!("expected finalization to report trait conflicts");
        };
        assert_eq!(conflicts.len(), 3, "conflicts: {conflicts:#?}");
        for conflict in &conflicts {
            assert_eq!(conflict.offender, "::chrono::NaiveDate");
            assert!(matches!(
                &conflict.reason,
                OffenderReason::NativeMissingImpl { type_name }
                    if type_name == "::chrono::NaiveDate"
            ));
        }
    }

    #[test]
    #[ignore]
    fn required_display_panics_at_render_not_finalize() {
        let settings = Settings::minimal().with_required_trait(TypespaceTrait::Display);
        let builder = typespace_builder!(settings, {
            enum Color {
                Red,
                Blue,
            }
        });

        let ts = builder
            .finalize(no_cycles)
            .expect("finalize should succeed per with_required_trait docs");

        // This should panic per the new render_derives guard, even though
        // finalize() succeeded without error.
        let _ = ts.to_codespace();
    }

    // Desired-trait resolution (phase 2). Every test below states what
    // the greatest-fixed-point phase must do with `desired_traits`; a
    // desired trait a type cannot realize is dropped with no error and
    // no record, which is the whole contrast with a required trait.

    /// The trait set holding exactly `traits`.
    fn trait_set(traits: impl IntoIterator<Item = TypespaceTrait>) -> TypespaceTraitSet {
        traits.into_iter().collect()
    }

    /// [`Settings::minimal`] with each of `traits` desired. Minimal
    /// settings require nothing, so a built trait set holds exactly
    /// what the desired phase granted.
    fn minimal_with_desired(traits: impl IntoIterator<Item = TypespaceTrait>) -> Settings {
        traits
            .into_iter()
            .fold(Settings::minimal(), Settings::with_desired_trait)
    }

    /// A struct whose every field realizes a desired trait takes it:
    /// `u32` and `String` are both `Eq` and `Hash`.
    #[test]
    fn desired_granted_on_capable_struct() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Eq, TypespaceTrait::Hash]),
            {
                struct S {
                    count: u32,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        // Desiring Eq desires PartialEq: supertrait expansion applies
        // to desired demands exactly as it does to required ones.
        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Hash,
            ])
        );
    }

    /// Desiring `Ord` alone desires every comparison trait: the
    /// supertrait closure runs over the desired demand set as well, or
    /// a granted `Ord` would render a derive that does not compile.
    #[test]
    fn desired_request_closure_expands_ord() {
        let builder = typespace_builder!(minimal_with_desired([TypespaceTrait::Ord]), {
            struct S {
                name: String,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
    }

    // REVIEW: "recomputing it into a strip" doesn't really make any sense
    /// A trait that is both required and desired is granted once and
    /// never stripped: phase 1 absorbs it, and phase 2 counts a phase-1
    /// grant as true rather than recomputing it into a strip. This one
    /// passes with the desired phase absent, because phase 1 alone
    /// produces the expected set; phase 2 must leave that set alone.
    #[test]
    fn desired_trait_already_required_survives() {
        let builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::Eq)
                .with_desired_trait(TypespaceTrait::Eq),
            {
                struct Inner {
                    count: u32,
                }

                struct Outer {
                    inner: Inner,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = trait_set([TypespaceTrait::Eq, TypespaceTrait::PartialEq]);
        assert_eq!(built_traits(&typespace, "Outer"), expected);
        assert_eq!(built_traits(&typespace, "Inner"), expected);
    }

    /// An alias has no impl site of its own, so its desired outcome is
    /// its target's: the alias of a capable struct takes the trait and
    /// the alias of a blocked struct does not.
    #[test]
    fn desired_forwards_through_alias() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Capable {
                    count: u32,
                }

                struct Blocked {
                    weight: f64,
                }

                type CapableAlias = Capable;

                type BlockedAlias = Blocked;
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let with_eq = trait_set([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]);
        assert_eq!(built_traits(&typespace, "Capable"), with_eq);
        assert_eq!(built_traits(&typespace, "CapableAlias"), with_eq);

        let without_eq = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "Blocked"), without_eq);
        assert_eq!(built_traits(&typespace, "BlockedAlias"), without_eq);
    }

    /// Containers answer structurally from their parameters: `Vec<T>`,
    /// `Map<K, V>`, `Set<T>`, and `Option<T>` are all `Eq` when their
    /// parameters are.
    #[test]
    fn desired_evaluates_container_structure() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct S {
                    list: Vec<u32>,
                    lookup: Map<String, u32>,
                    unique: Set<u32>,
                    maybe: Nullable<u32>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
    }

    /// A container whose parameter blocks a desired trait blocks it for
    /// the type holding the container: an `f64` element, an `f64` map
    /// value, and an `f64` inside an `Option` each deny `Eq` while
    /// leaving `Clone` alone. A set is absent here because a set
    /// element that cannot be `Eq` is a phase-1 conflict (the default
    /// set element requirements are the ordering traits), so the blocked
    /// set case is pinned by `desired_blocked_by_set_element` instead.
    #[test]
    fn desired_blocked_by_container_parameter() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct InVec {
                    list: Vec<f64>,
                }

                struct InMapValue {
                    lookup: Map<String, f64>,
                }

                struct InOption {
                    maybe: Nullable<f64>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "InVec"), only_clone);
        assert_eq!(built_traits(&typespace, "InMapValue"), only_clone);
        assert_eq!(built_traits(&typespace, "InOption"), only_clone);
    }

    /// A float has no total ordering, no equality relation, and no
    /// hash, so a struct with an `f64` field loses `Eq`, `Ord`, and
    /// `Hash`--and keeps `Clone`, `Debug`, `PartialEq`, and
    /// `PartialOrd`, which floats do provide. The kept half is the
    /// point: stripping is per trait, not a blanket rejection of the
    /// type.
    #[test]
    fn float_field_strips_ordering_traits_keeps_rest() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::PartialEq,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::Ord,
                TypespaceTrait::Hash,
            ]),
            {
                struct S {
                    weight: f64,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::PartialEq,
                TypespaceTrait::PartialOrd,
            ])
        );
    }

    /// A `JsonValue` field takes the ordering traits from the type
    /// holding it and leaves the rest.
    ///
    /// `serde_json::Value` derives `Clone`, `Eq`, `PartialEq`, and
    /// `Hash`, and implements neither `Ord` nor `PartialOrd`. Losing
    /// `PartialOrd` strips `Ord` a second way; `Eq` and `Hash` survive
    /// because the value really does implement them.
    #[test]
    fn json_value_field_strips_ordering_traits() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::PartialEq,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::Ord,
                TypespaceTrait::Hash,
            ]),
            {
                struct S {
                    blob: JsonValue,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Eq,
                TypespaceTrait::Hash,
            ])
        );
    }

    /// Stripping runs the supertrait closure in reverse: losing
    /// `PartialEq` also loses `Eq`, `PartialOrd`, and `Ord`, whatever
    /// the constituents claim about those traits on their own.
    ///
    /// The native here declares an incoherent set on purpose--`Eq`,
    /// `Ord`, and `Hash` with neither `PartialEq` nor `PartialOrd`--so
    /// that a per-trait answer and a closed answer differ: taken one
    /// trait at a time the struct would keep `Eq` and `Ord`, and only
    /// the reverse closure removes them.
    #[test]
    fn strip_closure_removes_supertrait_dependents() {
        let mut builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::PartialEq,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::Ord,
                TypespaceTrait::Hash,
            ]),
            {
                struct S {
                    odd: Weird,
                }
            }
        );

        // The macro has no syntax for native types, so this one is
        // inserted by hand under the id the field references.
        // REVIEW: remember to fix this once we do have native syntax
        builder
            .insert(
                "Weird".to_string(),
                Type::Native(Native::new(
                    "weird::Weird",
                    trait_set([
                        TypespaceTrait::Clone,
                        TypespaceTrait::Debug,
                        TypespaceTrait::Eq,
                        TypespaceTrait::Ord,
                        TypespaceTrait::Hash,
                    ]),
                    Vec::new(),
                )),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Hash,
            ])
        );
    }

    /// A blocked leaf strips its trait from every named type that
    /// reaches it, not just the one that holds it: the float is two
    /// containment hops below `Top`, and `Top` loses `Eq` all the same.
    #[test]
    fn blocked_leaf_strips_through_two_hops() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Bottom {
                    weight: f64,
                }

                struct Middle {
                    bottom: Bottom,
                }

                struct Top {
                    middle: Middle,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "Bottom"), only_clone);
        assert_eq!(built_traits(&typespace, "Middle"), only_clone);
        assert_eq!(built_traits(&typespace, "Top"), only_clone);
    }

    /// A cycle is assumed to satisfy a desired trait until something in
    /// it says otherwise: `A` and `B` refer to each other and every
    /// other constituent is capable, so both keep `Eq`. Assuming false
    /// on the cycle instead would strip `Eq` from both.
    ///
    /// The `Box` is written out rather than left to finalize's cycle
    /// breaking so that the graph under test is exactly the one
    /// described here.
    #[test]
    fn desired_survives_cycle_through_box() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct A {
                    b: Box<B>,
                    count: u32,
                }

                struct B {
                    a: Box<A>,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let with_eq = trait_set([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]);
        assert_eq!(built_traits(&typespace, "A"), with_eq);
        assert_eq!(built_traits(&typespace, "B"), with_eq);
    }

    /// Optimism on a cycle is not credulity: one `f64` anywhere in the
    /// cycle denies `Eq` to every member of it, including the member
    /// that holds no float itself. `A` reaches the float only by going
    /// around the cycle, so a strip that stopped at the type nearest
    /// the blocker would leave `A` wrongly holding `Eq`.
    #[test]
    fn cycle_with_blocker_strips_every_scc_member() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct A {
                    b: Box<B>,
                }

                struct B {
                    a: Box<A>,
                    weight: f64,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "A"), only_clone);
        assert_eq!(built_traits(&typespace, "B"), only_clone);
    }

    /// The same unsatisfiable trait fails loudly when required and
    /// quietly when desired. Requiring `Eq` of a struct with an `f64`
    /// field is `Error::TraitConflicts`, exactly as
    /// `map_key_conflict_reports_path` and
    /// `required_display_on_struct_conflicts` expect of a required
    /// trait; desiring it finalizes successfully with `Eq` simply
    /// absent from the built set, with no conflict, no recorded state,
    /// and nothing to query.
    #[test]
    fn infeasible_desired_is_silent_where_required_conflicts() {
        let required = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Eq),
            {
                struct S {
                    weight: f64,
                }
            }
        );

        let Err(err) = required.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };
        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");

        let desired = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct S {
                    weight: f64,
                }
            }
        );

        let typespace = desired
            .finalize(no_cycles)
            .expect("a desired trait that cannot be realized is dropped, not reported");
        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A struct has no `Display` and no `FromStr` under any realization,
    /// so desiring them drops them without an error, while the desired
    /// traits the struct can realize are unaffected.
    #[test]
    fn desired_display_on_struct_dropped_silently() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Display,
                TypespaceTrait::FromStr,
            ]),
            {
                struct S {
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// An enum with no attached default value cannot implement
    /// `Default`: there is no derive for it and no `#[default]` variant
    /// is invented. Desiring `Default` therefore drops it silently,
    /// while an enum that does carry a default value takes it.
    #[test]
    fn desired_default_on_enum_without_value_dropped() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Plain {
                    Red,
                    Green,
                }

                #[default = "Red"]
                enum WithValue {
                    Red,
                    Green,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Plain"),
            trait_set([TypespaceTrait::Clone])
        );
        assert_eq!(
            built_traits(&typespace, "WithValue"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
    }

    /// An attached default value realizes `Default` with a hand-written
    /// impl that asks nothing of the fields the value names: `S` takes
    /// `Default` even though its `Color` field cannot implement
    /// `Default` at all.
    #[test]
    fn desired_default_from_attached_value_needs_nothing_of_fields() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                #[default = { count: 0, color: "Red" }]
                struct S {
                    count: u32,
                    color: Color,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&typespace, "Color"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// An optional property the default value leaves out denies
    /// nothing: it renders as an `Option`, whose `Default` is `None`
    /// whatever the property's own type is. `S` keeps `Default` even
    /// though the omitted property's `Color` cannot implement it.
    #[test]
    fn desired_default_from_attached_value_exempts_omitted_optional() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                #[default = { count: 0 }]
                struct S {
                    count: u32,
                    color: Optional<Color>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&typespace, "Color"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// The hand-written `Default` impl a whole-type default value
    /// realizes takes each property the value names from the value and
    /// fills the rest with `Default::default()`, so the properties the
    /// value leaves out are what it obligates. The value is searched
    /// for the wire name, not the Rust name (`trap` is obligated: the
    /// value's `trap` key is not the `trap_wire` the property
    /// serializes under). An optional property is exempt, and a
    /// flattened property, having no wire name at all, is obligated.
    #[test]
    fn attached_struct_default_obligates_the_properties_it_omits() {
        let ty = Struct::<String>::new()
            .name("S")
            .default(serde_json::json!({
                "plain": 0,
                "wire": 0,
                "trap": 0,
            }))
            .properties([
                StructProperty::new("plain", "u32".to_string()),
                StructProperty::new("renamed", "u32".to_string())
                    .with_json_name(StructPropertySerde::Rename("wire".to_string())),
                StructProperty::new("trap", "u32".to_string())
                    .with_json_name(StructPropertySerde::Rename("trap_wire".to_string())),
                StructProperty::new("missing", "Missing".to_string()),
                StructProperty::new("optional", "Omitted".to_string())
                    .with_state(StructPropertyState::Optional),
                StructProperty::new("flattened", "Flattened".to_string())
                    .with_json_name(StructPropertySerde::Flatten),
            ])
            .build()
            .unwrap();

        let Feasibility::ManuallyRealizable(obligations) = feasibility(
            &BTreeMap::new(),
            &ty,
            TypespaceTrait::Default,
            &Settings::minimal(),
        ) else {
            panic!("expected a hand-written impl");
        };

        let obligated = obligations
            .iter()
            .map(|(relation, type_id)| match relation {
                Relation::Field(name) => (name.as_str(), type_id.as_str()),
                other => panic!("unexpected relation: {other}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            obligated,
            vec![
                ("trap", "u32"),
                ("missing", "Missing"),
                ("flattened", "Flattened"),
            ]
        );
    }

    /// A desired trait imposes no requirement on anything. Desiring
    /// `Ord` everywhere reaches a native that declares only `Clone` and
    /// `Debug`; the native is not in conflict, finalization succeeds,
    /// and the struct holding it simply goes without the comparison
    /// traits. A required `Ord` in the same graph would be
    /// `OffenderReason::NativeMissingImpl`.
    #[test]
    fn desired_imposes_no_requirement_on_native() {
        let mut builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Ord,
            ]),
            {
                struct S {
                    plain: Plain,
                }
            }
        );

        // The macro has no syntax for native types, so this one is
        // inserted by hand under the id the field references.
        // REVIEW: remmeber to replace
        builder
            .insert(
                "Plain".to_string(),
                Type::Native(Native::new(
                    "plain::Plain",
                    trait_set([TypespaceTrait::Clone, TypespaceTrait::Debug]),
                    Vec::new(),
                )),
            )
            .unwrap();

        let typespace = builder
            .finalize(no_cycles)
            .expect("a desired trait never becomes a requirement on a native");

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Debug])
        );
    }

    /// A required trait a native's declaration cannot answer for
    /// passes. The native is asked for the ordering traits and leaves
    /// all four unknown, so nothing conflicts and the struct holding it
    /// derives them.
    #[test]
    fn required_trait_passes_native_unknown() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Ord),
            {
                native ::opaque::Opaque: Clone + Debug + ?Ord + ?PartialOrd + ?Eq + ?PartialEq;

                struct S {
                    key: ::opaque::Opaque,
                }
            }
        );

        let typespace = builder
            .finalize(no_cycles)
            .expect("a native's unknown trait satisfies a requirement for it");

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
    }

    /// The `x-rust-type` case: a source that knows only the serde
    /// traits declares those and leaves the rest unknown with `..`.
    /// The type is then usable as a map key, whose `Eq`, `PartialEq`,
    /// `Ord`, and `PartialOrd` requirements it cannot answer for.
    #[test]
    fn required_map_key_passes_native_rest_unknown() {
        let builder = typespace_builder!(Settings::typical(), {
            native ::chrono::naive::NaiveDate:
                Clone + Debug + Serialize + Deserialize + ..;

            type DateMap = Map<::chrono::naive::NaiveDate, String>;
        });

        builder
            .finalize(no_cycles)
            .expect("an unknown map key trait is not a conflict");
    }

    /// A desired trait is never granted from what a native cannot
    /// answer for. The same `..` declaration keeps the two traits it
    /// names, so the struct takes `Clone` and `Debug` and goes without
    /// the desired `Ord`.
    #[test]
    fn desired_trait_not_granted_by_native_unknown() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Ord,
            ]),
            {
                native ::opaque::Opaque: Clone + Debug + ..;

                struct S {
                    odd: ::opaque::Opaque,
                }
            }
        );

        let typespace = builder
            .finalize(no_cycles)
            .expect("a desired trait never becomes a requirement on a native");

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Debug])
        );
    }

    /// A native that declares no unknowns answers for every trait, so
    /// a required trait it does not declare still conflicts.
    #[test]
    fn required_trait_conflicts_on_native_known_missing() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Hash),
            {
                native ::opaque::Opaque: Clone + Debug;

                struct S {
                    key: ::opaque::Opaque,
                }
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Hash);
        assert_eq!(conflicts[0].offender, "::opaque::Opaque");
        assert!(matches!(
            &conflicts[0].reason,
            OffenderReason::NativeMissingImpl { type_name }
                if type_name == "::opaque::Opaque"
        ));
    }

    /// `!Tr` carves an exception out of `..`: the trait it names is
    /// known missing and conflicts when required, while the rest of the
    /// requirement's supertrait closure stays unknown and passes.
    #[test]
    fn required_trait_conflicts_on_native_excepted_from_rest_unknown() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Ord),
            {
                native ::opaque::Opaque: Clone + Debug + .. + !Ord;

                struct S {
                    key: ::opaque::Opaque,
                }
            }
        );

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("finalization unexpectedly succeeded");
        };
        let Error::TraitConflicts { conflicts } = err else {
            panic!("expected TraitConflicts, got: {err}");
        };

        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Ord);
        assert_eq!(conflicts[0].offender, "::opaque::Opaque");
        assert!(matches!(
            &conflicts[0].reason,
            OffenderReason::NativeMissingImpl { type_name }
                if type_name == "::opaque::Opaque"
        ));
    }

    /// A generic native's `IfParameters` disposition (`*Tr` in the
    /// macro) forwards a required trait to its type parameter, exactly
    /// as a configured container forwards to the parameters it holds.
    /// The requirement is satisfied when the parameter has the trait
    /// and conflicts, at the parameter, when it does not.
    #[test]
    fn required_trait_forwards_through_native_if_parameters() {
        let settings = || Settings::minimal().with_required_trait(TypespaceTrait::Clone);

        // Inner derives Clone from its own field, so the requirement
        // Wrapper<Inner> forwards to it is satisfiable.
        let builder = typespace_builder!(settings(), {
            struct Inner {
                count: u32,
            }

            native ::foo::Wrapper<Inner>: Debug + *Clone;

            struct Holder {
                wrapped: ::foo::Wrapper<Inner>,
            }
        });
        let typespace = builder
            .finalize(no_cycles)
            .expect("Wrapper<Inner> is Clone because Inner is");
        assert!(
            built_traits(&typespace, "Holder").contains(&TypespaceTrait::Clone),
            "Holder should derive Clone through Wrapper<Inner>"
        );

        // NoClone is a native that never declares Clone, so the same
        // requirement, forwarded to it, conflicts there rather than at
        // Wrapper itself.
        let builder = typespace_builder!(settings(), {
            native ::bar::NoClone: Debug;
            native ::foo::Wrapper<::bar::NoClone>: Debug + *Clone;

            struct Holder {
                wrapped: ::foo::Wrapper<::bar::NoClone>,
            }
        });
        let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
            panic!("expected finalization to report a trait conflict at NoClone");
        };
        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Clone);
        assert_eq!(conflicts[0].offender, "::bar::NoClone");
        assert!(matches!(
            &conflicts[0].reason,
            OffenderReason::NativeMissingImpl { type_name }
                if type_name == "::bar::NoClone"
        ));
        // The hop out of Wrapper names the parameter position the
        // requirement rode through.
        assert!(
            conflicts[0]
                .path
                .iter()
                .any(|step| matches!(step.relation, Relation::Parameter(0))),
            "expected a hop through the native's parameter 0: {:#?}",
            conflicts[0].path
        );
    }

    /// The desired phase forwards through `IfParameters` too: a struct
    /// holding a generic native gets a desired trait exactly when the
    /// native's parameter has it, poisoned back through the same
    /// referrer route a container parameter uses.
    #[test]
    fn desired_trait_forwards_through_native_if_parameters() {
        let settings = || minimal_with_desired([TypespaceTrait::Debug, TypespaceTrait::Clone]);

        let builder = typespace_builder!(settings(), {
            struct Inner {
                count: u32,
            }

            native ::foo::Wrapper<Inner>: Debug + *Clone;

            struct Holder {
                wrapped: ::foo::Wrapper<Inner>,
            }
        });
        let typespace = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&typespace, "Holder"),
            trait_set([TypespaceTrait::Debug, TypespaceTrait::Clone])
        );

        let builder = typespace_builder!(settings(), {
            native ::bar::NoClone: Debug;
            native ::foo::Wrapper<::bar::NoClone>: Debug + *Clone;

            struct Holder {
                wrapped: ::foo::Wrapper<::bar::NoClone>,
            }
        });
        let typespace = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&typespace, "Holder"),
            trait_set([TypespaceTrait::Debug]),
            "Clone should be dropped: NoClone never declares it"
        );
    }

    /// A native's own obligations--what it demands of a type parameter
    /// to exist at all, independent of anything a caller requires of
    /// the native itself--are seeded unconditionally, exactly as a
    /// configured container's obligations are for its key, value, or
    /// element parameter. The parameter here is never otherwise
    /// constrained, so the only source of the conflict is the native's
    /// own declaration.
    #[test]
    fn native_obligation_is_seeded_unconditionally() {
        let mut builder = TypespaceBuilder::<String>::new(Settings::minimal());
        builder
            .insert(
                "Plain".to_string(),
                Type::Native(Native::new(
                    "plain::Plain",
                    TypespaceTraitSet::empty(),
                    Vec::new(),
                )),
            )
            .unwrap();
        builder
            .insert(
                "Set".to_string(),
                Type::Native(
                    Native::new(
                        "set::Set",
                        trait_set([TypespaceTrait::Debug]),
                        vec!["Plain".to_string()],
                    )
                    .with_obligations([trait_set([TypespaceTrait::Hash])]),
                ),
            )
            .unwrap();

        let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
            panic!("expected the unmet obligation to conflict");
        };
        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Hash);
        assert_eq!(conflicts[0].offender, "Plain");
        assert!(matches!(
            &conflicts[0].origin,
            RequirementOrigin::ContainerParameter { container, relation: Relation::Parameter(0) }
                if container == "Set"
        ));
    }

    /// A native whose obligation count does not match its parameter
    /// count is rejected at insertion, exactly as a configured
    /// container's mismatch is rejected at finalization
    /// (`check_containers`).
    #[test]
    fn native_obligation_count_mismatch_is_rejected_at_insertion() {
        let mut builder = TypespaceBuilder::<String>::new(Settings::minimal());
        let err = builder
            .insert(
                "Bad".to_string(),
                Type::Native(
                    Native::<String>::new("bad::Bad", TypespaceTraitSet::empty(), Vec::new())
                        .with_obligations([trait_set([TypespaceTrait::Hash])]),
                ),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            Error::NativeParameterCount {
                declared: 1,
                parameters: 0,
                ..
            }
        ));
    }

    /// A native's type parameters are children of the type, exactly
    /// like a container's: a dangling parameter id is caught by
    /// `check_references`, and is no longer invisible the way it was
    /// before `Type::children` reported them.
    #[test]
    fn native_parameter_is_a_child_reference() {
        let mut builder = TypespaceBuilder::<String>::new(Settings::minimal());
        builder
            .insert(
                "Wrapper".to_string(),
                Type::Native(Native::new(
                    "foo::Wrapper",
                    TypespaceTraitSet::empty(),
                    vec!["Missing".to_string()],
                )),
            )
            .unwrap();

        let Err(err) = builder.finalize(no_cycles) else {
            panic!("expected finalization to reject the dangling parameter id");
        };
        assert!(matches!(
            err,
            Error::UnknownTypeId { type_id, child_id }
                if type_id == "Wrapper" && child_id == "Missing"
        ));
    }

    /// A set element that cannot realize a desired trait denies the
    /// holder that trait and nothing else. The native declares the
    /// ordering traits the default set element requirements demand, so
    /// phase 1 is satisfied, and it declares no `Hash`, so the desired
    /// `Hash` is dropped.
    #[test]
    fn desired_blocked_by_set_element() {
        let mut builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Hash]),
            {
                struct S {
                    unique: Set<Ordered>,
                }
            }
        );

        // The macro has no syntax for native types, so this one is
        // inserted by hand under the id the element references.
        builder
            .insert(
                "Ordered".to_string(),
                Type::Native(Native::new(
                    "ordered::Ordered",
                    trait_set([
                        TypespaceTrait::Clone,
                        TypespaceTrait::Debug,
                        TypespaceTrait::Eq,
                        TypespaceTrait::PartialEq,
                        TypespaceTrait::Ord,
                        TypespaceTrait::PartialOrd,
                    ]),
                    Vec::new(),
                )),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A newtype's `Display` is its inner value's `Display`, which is
    /// an obligation when `Display` is required and only a question
    /// when it is desired: the inner struct is asked whether it can
    /// implement `Display`, answers no, and is neither modified nor
    /// reported. The newtype goes without.
    #[test]
    fn desired_display_does_not_obligate_newtype_inner() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                struct Inner {
                    count: u32,
                }

                struct Wrapper(Inner);
            }
        );

        let typespace = builder
            .finalize(no_cycles)
            .expect("a desired trait never becomes a requirement on a field");

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "Wrapper"), only_clone);
        assert_eq!(built_traits(&typespace, "Inner"), only_clone);
    }

    /// The `Settings::all_traits` preset over a struct with a float: the
    /// required set lands whole, and of the desired set `PartialEq` and
    /// `PartialOrd` survive while `Display` and `FromStr` (impossible
    /// for any struct), `Default` (impossible for this struct, whose
    /// properties are required), and `Eq`, `Ord`, and `Hash` (blocked
    /// by the float) are dropped.
    #[test]
    fn all_traits_preset_over_float_struct() {
        let builder = typespace_builder!(Settings::maximal(), {
            struct S {
                weight: f64,
                name: String,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
                TypespaceTrait::JsonSchema,
                TypespaceTrait::PartialEq,
                TypespaceTrait::PartialOrd,
            ])
        );
    }

    // Desired-phase cases beyond the batch above: the seam between a
    // trait desired in its own right and one a supertrait pulled in,
    // a phase-1 grant meeting a phase-2 removal, the container and leaf
    // answers the desired phase reads, untagged enums, cycles entered
    // from any member, the causal chain the skip log carries, and the
    // depth resolution has to survive. Each states what typespace
    // should do, which is not always what it does.

    /// A supertrait is granted on its own account only when desired.
    #[test]
    fn desired_supertrait_survives_only_when_desired_directly() {
        // PartialEq reaches the demand set only as Eq's supertrait, so
        // a struct that cannot have Eq has no use for it.
        let supertrait_only = typespace_builder!(minimal_with_desired([TypespaceTrait::Eq]), {
            struct S {
                weight: f64,
            }
        });

        let typespace = supertrait_only.finalize(no_cycles).unwrap();
        assert_eq!(built_traits(&typespace, "S"), TypespaceTraitSet::empty());

        // Desired in its own right, PartialEq stands whether or not Eq
        // survives.
        let desired_directly = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Eq, TypespaceTrait::PartialEq]),
            {
                struct S {
                    weight: f64,
                }
            }
        );

        let typespace = desired_directly.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::PartialEq])
        );
    }

    /// A supertrait outlives one of the two desired traits implying it.
    #[test]
    fn desired_partial_eq_survives_via_other_desired_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Eq, TypespaceTrait::PartialOrd]),
            {
                struct S {
                    weight: f64,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::PartialOrd, TypespaceTrait::PartialEq])
        );
    }

    /// A trait desired directly outlives a dead request that needed it.
    #[test]
    fn request_survives_the_death_of_a_dependent_request() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::PartialEq, TypespaceTrait::Ord]),
            {
                struct S {
                    weight: f64,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::PartialEq])
        );
    }

    /// A supertrait only a dead request asked for leaves with it.
    #[test]
    fn supertrait_borrowed_by_one_request_leaves_with_it() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Ord,
            ]),
            {
                native ::weird::Weird: Clone + PartialEq + Eq + PartialOrd;

                struct Inner {
                    odd: ::weird::Weird,
                }

                struct Outer {
                    inner: Inner,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = trait_set([TypespaceTrait::Clone, TypespaceTrait::PartialEq]);
        assert_eq!(built_traits(&typespace, "Inner"), expected);
        assert_eq!(built_traits(&typespace, "Outer"), expected);
    }

    /// A required trait outlives a desired strip in the same group.
    #[test]
    fn required_trait_survives_desired_strip_in_same_group() {
        let mut builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::Eq)
                .with_desired_trait(TypespaceTrait::Ord),
            {
                struct S {
                    stamp: PartialOnly,
                }
            }
        );

        // The macro has no syntax for native types, so this one is
        // inserted by hand under the id the field references.
        builder
            .insert(
                "PartialOnly".to_string(),
                Type::Native(Native::new(
                    "partial::PartialOnly",
                    trait_set([TypespaceTrait::Eq, TypespaceTrait::PartialEq]),
                    Vec::new(),
                )),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Eq, TypespaceTrait::PartialEq])
        );
    }

    /// A required `PartialEq` satisfies what a desired `Eq` needs.
    #[test]
    fn required_partial_eq_supports_desired_eq() {
        let builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::PartialEq)
                .with_desired_trait(TypespaceTrait::Eq),
            {
                struct S {
                    count: u32,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Eq, TypespaceTrait::PartialEq])
        );
    }

    /// A phase-1 grant stays when a desired request that needs it dies.
    #[test]
    fn phase_one_grant_outlives_a_dead_desired_request() {
        let builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::PartialEq)
                .with_desired_trait(TypespaceTrait::Ord),
            {
                struct S {
                    weight: f64,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::PartialEq])
        );
    }

    /// A map key keeps its phase-1 grant while its holder drops the trait.
    #[test]
    fn map_key_grant_is_untouched_by_a_desired_drop() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Key {
                    name: String,
                }

                struct S {
                    lookup: Map<Key, f64>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Key"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
            ])
        );
        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A required conflict is reported even with desired traits pending.
    #[test]
    fn required_conflict_reported_alongside_desired_traits() {
        let builder = typespace_builder!(
            Settings::minimal()
                .with_required_trait(TypespaceTrait::Display)
                .with_desired_trait(TypespaceTrait::Eq),
            {
                struct S {
                    name: String,
                }
            }
        );

        let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
            panic!("expected finalization to report trait conflicts");
        };
        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Display);
    }

    /// A required `Display` reaching an `Option` conflicts there.
    #[test]
    fn required_display_through_option_conflicts() {
        let builder = typespace_builder!(
            Settings::minimal().with_required_trait(TypespaceTrait::Display),
            {
                struct Wrapper(Nullable<String>);
            }
        );

        let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
            panic!("expected finalization to report trait conflicts");
        };
        assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
        assert_eq!(conflicts[0].required, TypespaceTrait::Display);
        assert_eq!(conflicts[0].offender, "Nullable<String>");
    }

    /// `Option<T>` has no `Display` and no `FromStr`, whatever `T` has.
    #[test]
    fn option_has_no_display_or_from_str() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Display,
                TypespaceTrait::FromStr,
            ]),
            {
                type MaybeName = Nullable<String>;
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "MaybeName"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// `Vec<T>` has no `Display`, whatever `T` has.
    #[test]
    fn desired_display_dropped_through_vec_inner() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                struct Wrapper(Vec<String>);
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Wrapper"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// `Box<T>` has no `Display` and no `FromStr`, whatever `T` has.
    #[test]
    fn box_has_no_display_or_from_str() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::Display,
                TypespaceTrait::FromStr,
            ]),
            {
                struct Wrapper(Box<String>);
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Wrapper"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A tuple has no `Display` for a desired `Display` to cross.
    #[test]
    fn desired_display_not_available_through_tuple() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                struct Wrapper((String, String));
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Wrapper"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A newtype takes a desired `Display` its inner type can realize.
    #[test]
    fn desired_display_forwards_to_simple_enum_inner() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                enum Color {
                    Red,
                    Green,
                }

                struct Wrapper(Color);
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let with_display = trait_set([TypespaceTrait::Clone, TypespaceTrait::Display]);
        assert_eq!(built_traits(&typespace, "Wrapper"), with_display);
        assert_eq!(built_traits(&typespace, "Color"), with_display);
    }

    /// A unit struct blocks nothing desired, and still has no `Display`.
    #[test]
    fn desired_granted_on_unit_struct() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Eq,
                TypespaceTrait::Hash,
                TypespaceTrait::Display,
            ]),
            {
                #[json = "marker"]
                struct Marker;
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Marker"),
            trait_set([
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Hash,
            ])
        );
    }

    /// A container forwards one trait of a group without the rest.
    #[test]
    fn desired_partial_group_survives_float_in_vec() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::PartialEq, TypespaceTrait::Eq]),
            {
                struct S {
                    list: Vec<f64>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::PartialEq])
        );
    }

    /// A variant payload blocks a desired trait the way a field does.
    #[test]
    fn variant_payload_blocks_desired_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Clone,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Eq,
            ]),
            {
                enum Reading {
                    Missing,
                    Weight(f64),
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Reading"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::PartialEq])
        );
    }

    /// A derived `Default` needs every field to have one.
    #[test]
    fn derived_default_needs_every_field() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                struct S {
                    color: Color,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "S"), only_clone);
        assert_eq!(built_traits(&typespace, "Color"), only_clone);
    }

    /// An array is `Default` only when its element is.
    #[test]
    fn desired_default_blocked_by_array_element() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                struct S {
                    swatch: [Color; 3],
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A vector is `Default` whatever its element is.
    #[test]
    fn desired_default_survives_vec_of_default_less_element() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                struct S {
                    #[default]
                    swatch: Vec<Color>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
    }

    /// `Option<T>` provides `Default` whatever it wraps.
    #[test]
    fn option_provides_default_whatever_it_wraps() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum Color {
                    Red,
                    Green,
                }

                struct S {
                    #[default]
                    color: Nullable<Color>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&typespace, "Color"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A hop that provides `Default` itself absorbs an inner loss.
    #[test]
    fn desired_default_absorbed_by_container_hop() {
        let builder = typespace_builder!(minimal_with_desired([TypespaceTrait::Default]), {
            enum NoDefault {
                Red,
                Green,
            }

            struct S {
                #[default]
                list: Vec<NoDefault>,
                #[default]
                maybe: Nullable<NoDefault>,
            }
        });

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&typespace, "NoDefault"),
            TypespaceTraitSet::empty()
        );
    }

    /// A loss climbs through every container between it and a holder.
    #[test]
    fn desired_loss_climbs_nested_containers() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct S {
                    deep: Map<String, Vec<Nullable<f64>>>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A desired trait reaches a leaf nested deep in containers.
    #[test]
    fn desired_evaluates_nested_container_structure() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Capable {
                    deep: Vec<Map<String, Nullable<Vec<u32>>>>,
                }

                struct Blocked {
                    deep: Vec<Map<String, Nullable<Vec<f64>>>>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Capable"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
        assert_eq!(
            built_traits(&typespace, "Blocked"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// `::json_serde::Absent` supplies the std traits it derives.
    #[test]
    fn desired_never_field_keeps_derived_std_traits() {
        let builder = typespace_builder!(
            minimal_with_desired([
                TypespaceTrait::Eq,
                TypespaceTrait::Hash,
                TypespaceTrait::Default,
            ]),
            {
                struct S {
                    gone: Optional<!>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Hash,
                TypespaceTrait::Default,
            ])
        );
    }

    /// An untagged enum of single-payload variants can have `Display`.
    #[test]
    fn desired_display_on_untagged_item_variants() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                #[untagged]
                enum U {
                    Text(String),
                    Count(u32),
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "U"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Display])
        );
    }

    /// An untagged enum realizes `Display` only with single-payload variants.
    #[test]
    fn untagged_enum_display_needs_single_payload_variants() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                #[untagged]
                enum E {
                    One(String),
                    Pair(String, String),
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "E"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// An untagged enum with a struct variant has no `Display`.
    #[test]
    fn desired_display_rejected_on_untagged_struct_variant() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                #[untagged]
                enum U {
                    Text(String),
                    Pair { left: String, right: String },
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "U"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A type that refers to itself keeps a desired trait.
    #[test]
    fn self_referential_type_keeps_desired_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Node {
                    next: Box<Node>,
                    count: u32,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Node"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
    }

    /// A type recursive through a vector keeps an unblocked desired trait.
    #[test]
    fn desired_survives_cycle_through_vec() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Node {
                    kids: Vec<Node>,
                    name: String,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Node"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
    }

    /// A cycle through a vector does not hide a blocker inside it.
    #[test]
    fn desired_stripped_in_cycle_through_vec() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Node {
                    kids: Vec<Node>,
                    weight: f64,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "Node"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// A cycle entered at the member that holds the blocker.
    #[test]
    fn cycle_entered_from_the_blocking_member_strips_both() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct A {
                    b: Box<B>,
                    weight: f64,
                }

                struct B {
                    a: Box<A>,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "A"), only_clone);
        assert_eq!(built_traits(&typespace, "B"), only_clone);
    }

    /// One blocker in a three-member cycle denies all three.
    #[test]
    fn three_member_cycle_strips_every_member() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct A {
                    b: Box<B>,
                }

                struct B {
                    c: Box<C>,
                }

                struct C {
                    a: Box<A>,
                    weight: f64,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        for id in ["A", "B", "C"] {
            assert_eq!(built_traits(&typespace, id), only_clone, "{id}");
        }
    }

    /// A type reaching a blocked cycle from above loses the trait too.
    #[test]
    fn type_above_a_blocked_cycle_loses_the_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Above {
                    member: Cycle1,
                }

                struct Cycle1 {
                    other: Box<Cycle2>,
                }

                struct Cycle2 {
                    other: Box<Cycle1>,
                    weight: f64,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        for id in ["Above", "Cycle1", "Cycle2"] {
            assert_eq!(built_traits(&typespace, id), only_clone, "{id}");
        }
    }

    /// A type reaching a cycle that blocks nothing keeps the trait.
    #[test]
    fn type_above_a_clean_cycle_keeps_the_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Clean1 {
                    other: Box<Clean2>,
                    count: u32,
                }

                struct Clean2 {
                    other: Box<Clean1>,
                }

                struct Holder {
                    member: Clean1,
                }
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let with_eq = trait_set([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]);
        for id in ["Clean1", "Clean2", "Holder"] {
            assert_eq!(built_traits(&typespace, id), with_eq, "{id}");
        }
    }

    /// A chain of aliases forwards a blocked desired trait upward.
    #[test]
    fn alias_chain_forwards_desired_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Blocked {
                    weight: f64,
                }

                type Near = Blocked;

                type Far = Near;
            }
        );

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        for id in ["Blocked", "Near", "Far"] {
            assert_eq!(built_traits(&typespace, id), only_clone, "{id}");
        }
    }

    /// Every captured log message, in the order logged.
    static SKIP_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    struct SkipLogCapture;

    impl log::Log for SkipLogCapture {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            SKIP_LOG.lock().unwrap().push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    static SKIP_LOG_CAPTURE: SkipLogCapture = SkipLogCapture;

    /// Start capturing log messages. Tests run in one process and
    /// share the capture, so each one matches only its own type ids.
    fn capture_skip_log() {
        log::set_logger(&SKIP_LOG_CAPTURE).ok();
        log::set_max_level(log::LevelFilter::Debug);
    }

    /// The message logged when `loser` loses `trait_name` to
    /// `blocker`.
    fn skip_line(trait_name: TypespaceTrait, loser: &str, blocker: &str) -> String {
        format!("desired trait {trait_name} dropped from {loser}, blocked by {blocker}")
    }

    /// Whether any captured message is exactly `line`.
    fn skip_logged(line: &str) -> bool {
        SKIP_LOG.lock().unwrap().iter().any(|entry| entry == line)
    }

    /// The skip log names the constituent to blame at each hop.
    #[test]
    fn desired_skip_log_names_the_blocker_at_each_hop() {
        capture_skip_log();

        let builder = typespace_builder!(minimal_with_desired([TypespaceTrait::Eq]), {
            struct ChainBottom {
                weight: f64,
            }

            struct ChainMiddle {
                bottom: ChainBottom,
            }

            struct ChainTop {
                middle: ChainMiddle,
            }
        });

        builder.finalize(no_cycles).unwrap();

        for (loser, blocker) in [
            ("ChainBottom", "f64"),
            ("ChainMiddle", "ChainBottom"),
            ("ChainTop", "ChainMiddle"),
        ] {
            let line = skip_line(TypespaceTrait::Eq, loser, blocker);
            assert!(skip_logged(&line), "missing skip line: {line}");
        }
    }

    /// A trait dropped by the strip closure is logged like any other.
    #[test]
    fn desired_skip_log_covers_strip_closure_removals() {
        capture_skip_log();

        let mut builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Eq, TypespaceTrait::Ord]),
            {
                struct StripHolder {
                    odd: StripWeird,
                }
            }
        );

        // The macro has no syntax for native types, so this one is
        // inserted by hand under the id the field references. It
        // declares Eq and Ord without the partial pair on purpose: the
        // holder's Eq and Ord die only by the strip closure.
        builder
            .insert(
                "StripWeird".to_string(),
                Type::Native(Native::new(
                    "weird::StripWeird",
                    trait_set([TypespaceTrait::Eq, TypespaceTrait::Ord]),
                    Vec::new(),
                )),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        assert_eq!(
            built_traits(&typespace, "StripHolder"),
            TypespaceTraitSet::empty()
        );
        for trait_name in [
            TypespaceTrait::PartialEq,
            TypespaceTrait::PartialOrd,
            TypespaceTrait::Eq,
            TypespaceTrait::Ord,
        ] {
            let line = skip_line(trait_name, "StripHolder", "StripWeird");
            assert!(skip_logged(&line), "missing skip line: {line}");
        }
    }

    /// A removal reaches the top of a 32-level containment chain.
    #[test]
    fn desired_removal_crosses_a_deep_chain() {
        use crate::TypespaceBuilder;
        use crate::build::NewtypeStruct;

        // Deeper than a macro literal wants to be: the blocked leaf is
        // 32 hops down, so the removal is queued and requeued all the
        // way up the chain before the work list drains.
        const DEPTH: usize = 32;

        let mut builder = TypespaceBuilder::<String>::new(minimal_with_desired([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
        ]));
        builder
            .insert("Leaf".to_string(), Type::Float("f64".to_string()))
            .unwrap();
        for level in 0..DEPTH {
            let inner = match level + 1 == DEPTH {
                true => "Leaf".to_string(),
                false => format!("L{}", level + 1),
            };
            let wrapper = NewtypeStruct::new(inner)
                .name(format!("L{level}"))
                .build()
                .unwrap();
            builder.insert(format!("L{level}"), wrapper).unwrap();
        }

        let typespace = builder.finalize(no_cycles).unwrap();

        let only_clone = trait_set([TypespaceTrait::Clone]);
        for level in 0..DEPTH {
            assert_eq!(
                built_traits(&typespace, &format!("L{level}")),
                only_clone,
                "level {level}"
            );
        }
    }

    /// A containment chain thousands of levels deep resolves.
    #[test]
    fn deep_containment_chain_resolves() {
        use crate::build::{Struct, StructProperty};

        const DEPTH: usize = 2000;

        let mut builder = crate::TypespaceBuilder::new(minimal_with_desired([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
        ]));

        // Each level holds the next, and the last holds a float, so
        // the loss has to travel every hop of the chain.
        for level in 0..DEPTH {
            let next = match level + 1 == DEPTH {
                true => "f64".to_string(),
                false => format!("Level{}", level + 1),
            };
            builder
                .insert(
                    format!("Level{level}"),
                    Struct::<String>::new()
                        .name(format!("Level{level}"))
                        .properties([StructProperty::new("next", next)])
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
        builder
            .insert("f64".to_string(), Type::Float("f64".to_string()))
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        // The float takes Eq from every level, top and bottom alike.
        let only_clone = trait_set([TypespaceTrait::Clone]);
        assert_eq!(built_traits(&typespace, "Level0"), only_clone);
        assert_eq!(
            built_traits(&typespace, &format!("Level{}", DEPTH - 1)),
            only_clone
        );
    }

    /// all_traits desires Display; an all-unit enum is granted it; the
    /// enum renderer strips it from the derive set and emits the
    /// manual impl.
    #[test]
    fn probe_all_traits_simple_enum_renders() {
        let builder = typespace_builder!(Settings::maximal(), {
            enum Color {
                Red,
                Green,
            }
        });
        let ts = builder.finalize(no_cycles).unwrap();
        assert!(built_traits(&ts, "Color").contains(&TypespaceTrait::Display));
        let out = ts.to_codespace();
        println!("{}", out.into_stream());
    }

    /// diamond graph plus duplicate fields.
    #[test]
    fn probe_diamond_and_duplicate_edges() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Bottom {
                    w: f64,
                }

                struct Left {
                    b: Bottom,
                    b2: Bottom,
                }

                struct Right {
                    b: Bottom,
                }

                struct Top {
                    l: Left,
                    r: Right,
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        let only_clone = trait_set([TypespaceTrait::Clone]);
        for id in ["Bottom", "Left", "Right", "Top"] {
            assert_eq!(built_traits(&ts, id), only_clone, "{id}");
        }
    }

    /// attached-value Default at a named type is a firebreak.
    #[test]
    fn probe_named_firebreak_stops_loss() {
        let builder = typespace_builder!(minimal_with_desired([TypespaceTrait::Default]), {
            enum NoDef {
                A,
                B,
            }

            #[default = { x: "A" }]
            struct Middle {
                x: NoDef,
            }

            struct Top {
                #[default]
                m: Middle,
            }
        });
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(built_traits(&ts, "NoDef"), TypespaceTraitSet::empty());
        assert_eq!(
            built_traits(&ts, "Middle"),
            trait_set([TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&ts, "Top"),
            trait_set([TypespaceTrait::Default]),
            "Top should keep Default: Middle absorbs the loss"
        );
    }

    /// chained manual obligations, blocked at the bottom.
    #[test]
    fn probe_chained_manual_obligations() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                struct Inner {
                    s: String,
                }

                struct Wrapper(Inner);

                #[untagged]
                enum U {
                    W(Wrapper),
                    T(String),
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        let only_clone = trait_set([TypespaceTrait::Clone]);
        for id in ["Inner", "Wrapper", "U"] {
            assert_eq!(built_traits(&ts, id), only_clone, "{id}");
        }
    }

    /// same chain but capable.
    #[test]
    fn probe_chained_manual_obligations_capable() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                struct Wrapper(String);

                #[untagged]
                enum U {
                    W(Wrapper),
                    T(u32),
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        let with_display = trait_set([TypespaceTrait::Clone, TypespaceTrait::Display]);
        for id in ["Wrapper", "U"] {
            assert_eq!(built_traits(&ts, id), with_display, "{id}");
        }
    }

    /// newtype cycle through a box keeps Eq.
    #[test]
    fn probe_cycle_through_newtype() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct A(Box<B>);

                struct B(A);
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        let with_eq = trait_set([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]);
        for id in ["A", "B"] {
            assert_eq!(built_traits(&ts, id), with_eq, "{id}");
        }
    }

    /// self-referential untagged enum through Box loses Display,
    /// terminates.
    #[test]
    fn probe_untagged_self_cycle_display() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Display]),
            {
                #[untagged]
                enum U {
                    Nested(Box<U>),
                    Leaf(String),
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(built_traits(&ts, "U"), trait_set([TypespaceTrait::Clone]));
    }

    /// a phase-1 grant on a map key supports desired Eq at another
    /// holder of the key, while the map holder drops it.
    #[test]
    fn probe_grant_supports_desired_elsewhere() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Eq]),
            {
                struct Key {
                    name: String,
                }

                struct HoldsMap {
                    lookup: Map<Key, f64>,
                }

                struct HoldsKey {
                    k: Key,
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&ts, "HoldsKey"),
            trait_set([
                TypespaceTrait::Clone,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ])
        );
        assert_eq!(
            built_traits(&ts, "HoldsMap"),
            trait_set([TypespaceTrait::Clone])
        );
    }

    /// tuple struct rest field participates in poisoning.
    #[test]
    fn probe_tuple_struct_rest_blocks() {
        let mut builder = crate::TypespaceBuilder::new(minimal_with_desired([
            TypespaceTrait::Clone,
            TypespaceTrait::Eq,
        ]));
        builder
            .insert(
                "floats".to_string(),
                crate::build::Type::Vec("f64".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "f64".to_string(),
                crate::build::Type::Float("f64".to_string()),
            )
            .unwrap();
        builder
            .insert("String".to_string(), crate::build::Type::String)
            .unwrap();
        builder
            .insert(
                "T".to_string(),
                TupleStruct::new()
                    .name("T")
                    .fields(["String".to_string()])
                    .rest("floats".to_string())
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(built_traits(&ts, "T"), trait_set([TypespaceTrait::Clone]));
    }

    /// an alias to a container keeps unconditional Default.
    #[test]
    fn probe_alias_to_container_default() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                enum NoDef {
                    A,
                    B,
                }

                type L = Vec<NoDef>;
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&ts, "L"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
    }

    #[test]
    fn test_optional_and_default_result_in_default_trait() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                struct Foo {
                    a: Optional<String>,
                    #[default = 12]
                    b: u32,
                    #[default]
                    c: String,
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&ts, "Foo"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
    }

    #[test]
    fn test_no_obligation_with_default_value() {
        let builder = typespace_builder!(
            minimal_with_desired([TypespaceTrait::Clone, TypespaceTrait::Default]),
            {
                #[default = {}]
                struct Outer {
                    #[default = { a: "x"}]
                    inner: Inner,
                }

                struct Inner {
                    a: String,
                }
            }
        );
        let ts = builder.finalize(no_cycles).unwrap();
        assert_eq!(
            built_traits(&ts, "Outer"),
            trait_set([TypespaceTrait::Clone, TypespaceTrait::Default])
        );
        assert_eq!(
            built_traits(&ts, "Inner"),
            trait_set([TypespaceTrait::Clone])
        );
    }
    // FromStrIrrefutable: whether a type's `FromStr` returns `Ok` for
    // every `&str`, because the value is stored verbatim with no
    // validation step between the `&str` and the constructed value.
    // The property is syntactic, not semantic: a constraint counts as
    // validation even where it happens to accept every string.

    /// Whether the type with `id` has an irrefutable `FromStr`.
    fn is_irrefutable(typespace: &Typespace<String>, id: &str) -> bool {
        from_string_irrefutable(&typespace.types, &id.to_string())
    }

    /// `Type::String` is the base case: it takes the input verbatim and
    /// has nothing to reject. An integer parses, and parsing fails.
    /// A struct has no `FromStr` at all.
    #[test]
    fn irrefutable_holds_for_string_alone_among_primitives() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Holder {
                text: String,
                count: u32,
            }
        });
        let ts = builder.finalize(no_cycles).unwrap();

        assert!(is_irrefutable(&ts, "String"));
        assert!(!is_irrefutable(&ts, "u32"));
        assert!(!is_irrefutable(&ts, "Holder"));
    }

    /// An unconstrained newtype's `FromStr` is its inner type's, so the
    /// property passes through it, and through a chain of them. A
    /// newtype over an integer parses, and parsing fails.
    #[test]
    fn irrefutable_follows_unconstrained_newtypes() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Wrapper(String);
            struct Rewrapper(Wrapper);
            struct Port(u32);
        });
        let ts = builder.finalize(no_cycles).unwrap();

        assert!(is_irrefutable(&ts, "Wrapper"));
        assert!(is_irrefutable(&ts, "Rewrapper"));
        assert!(!is_irrefutable(&ts, "Port"));
    }

    /// A type alias has no impl site of its own, so its `FromStr` is
    /// its target's and so is the property.
    #[test]
    fn irrefutable_follows_type_aliases() {
        let builder = typespace_builder!(Settings::minimal(), {
            type Text = String;
            type Count = u32;
            type Wrapped = Wrapper;

            struct Wrapper(String);
        });
        let ts = builder.finalize(no_cycles).unwrap();

        assert!(is_irrefutable(&ts, "Text"));
        assert!(is_irrefutable(&ts, "Wrapped"));
        assert!(!is_irrefutable(&ts, "Count"));
    }

    /// Each of the three constraint kinds is a validation step between
    /// the `&str` and the constructed value, so a constrained newtype
    /// is refutable even where the constraint accepts every string:
    /// `Patterned` admits everything `Plain` does and is still
    /// refutable. This is typify 1's `IdOrYoloYolo`.
    #[test]
    fn irrefutable_stops_at_every_constraint_kind() {
        let mut builder = TypespaceBuilder::new(Settings::minimal());
        builder.insert("string".to_string(), Type::String).unwrap();

        let newtype = |name: &str, constraints: NewtypeConstraints| {
            Type::NewtypeStruct(
                NewtypeStruct::new("string".to_string())
                    .name(name)
                    .constraints(constraints),
            )
        };

        builder
            .insert(
                "Plain".to_string(),
                newtype("Plain", NewtypeConstraints::None),
            )
            .unwrap();
        builder
            .insert(
                "Patterned".to_string(),
                newtype(
                    "Patterned",
                    NewtypeConstraints::String {
                        min: None,
                        max: None,
                        patterns: vec![".*".to_string()],
                    },
                ),
            )
            .unwrap();
        builder
            .insert(
                "Allowed".to_string(),
                newtype(
                    "Allowed",
                    NewtypeConstraints::AllowList(vec![JsonValue::new(serde_json::json!("yes"))]),
                ),
            )
            .unwrap();
        builder
            .insert(
                "Denied".to_string(),
                newtype(
                    "Denied",
                    NewtypeConstraints::DenyList(vec![JsonValue::new(serde_json::json!("no"))]),
                ),
            )
            .unwrap();

        let ts = builder.finalize(no_cycles).unwrap();

        assert!(is_irrefutable(&ts, "Plain"));
        assert!(!is_irrefutable(&ts, "Patterned"));
        assert!(!is_irrefutable(&ts, "Allowed"));
        assert!(!is_irrefutable(&ts, "Denied"));
    }

    /// Nothing outside `Type::String`, an unconstrained newtype, and a
    /// type alias carries the property. A native cannot declare it
    /// however string-like the declaration is, and a `Box`, an
    /// `Option`, and a `Vec` have no `FromStr` at all.
    #[test]
    fn irrefutable_excludes_natives_and_containers() {
        let mut builder = TypespaceBuilder::new(Settings::minimal());
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "::string::Str".to_string(),
                Type::Native(Native::new_string_like("::string::Str")),
            )
            .unwrap();
        builder
            .insert("Box<String>".to_string(), Type::Box("string".to_string()))
            .unwrap();
        builder
            .insert(
                "Nullable<String>".to_string(),
                Type::Option("string".to_string()),
            )
            .unwrap();
        builder
            .insert("Vec<String>".to_string(), Type::Vec("string".to_string()))
            .unwrap();

        let ts = builder.finalize(no_cycles).unwrap();

        assert!(is_irrefutable(&ts, "string"));
        assert!(!is_irrefutable(&ts, "::string::Str"));
        assert!(!is_irrefutable(&ts, "Box<String>"));
        assert!(!is_irrefutable(&ts, "Nullable<String>"));
        assert!(!is_irrefutable(&ts, "Vec<String>"));
    }

    /// The walk terminates on a cycle and answers false for it:
    /// `break_cycles` puts a `Box` in every cycle before this pass
    /// runs, and a `Box` is never irrefutable, so the recursion has
    /// somewhere to stop. `Plain` sits outside the cycle and still
    /// answers true, so the `false` for `Loop` and `Knot` is the walk's
    /// answer rather than the walk giving up.
    #[test]
    fn irrefutable_terminates_on_a_cycle() {
        let builder = typespace_builder!(Settings::minimal(), {
            struct Loop(Knot);
            struct Knot(Loop);
            struct Plain(String);
        });
        let ts = builder
            .finalize(|id: &String| format!("Box<{id}>"))
            .unwrap();

        assert!(!is_irrefutable(&ts, "Loop"));
        assert!(!is_irrefutable(&ts, "Knot"));
        assert!(is_irrefutable(&ts, "Plain"));
    }
}

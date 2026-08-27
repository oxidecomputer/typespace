// Copyright 2026 Oxide Computer Company

//! Trait resolution: determine the trait for each type.
//!
//! Runs during finalization in
//! two phases. Phase 1 resolves required traits: requirements (from
//! settings and from use sites such as map keys and set elements)
//! descend the type graph, each named type absorbs what it can
//! satisfy, and every unsatisfiable requirement is collected into a
//! [`TraitConflict`](crate::error::TraitConflict) that records the
//! requirement's origin and the containment path to the offending
//! type. Phase 2 resolves desired traits: each named type takes a
//! desired trait exactly when it can realize it and every type it
//! depends on has it; unsatisfiable desired traits are dropped
//! silently. The resulting per-type trait set is authoritative: query
//! answers and rendered derives both read it.

use std::collections::{BTreeMap, VecDeque};

use log::debug;

use crate::build::{
    EnumTagType, Native, NewtypeConstraints, NewtypeStruct, Struct, TupleStruct, Type,
};
use crate::error::{Error, OffenderReason, PathStep, Relation, RequirementOrigin, TraitConflict};
use crate::settings::Settings;
use crate::{TypespaceTrait, TypespaceTraitSet};

/// Resolve the trait set for every named type in the graph.
///
/// On success, each named type's built trait set holds exactly the traits its
/// generated code should implement (either derived or with a generated impl).
/// On failure, [`Error::TraitConflicts`] lists every required trait that some
/// type cannot satisfy.
pub(crate) fn resolve_traits<Id>(
    types: &mut BTreeMap<Id, Type<Id>>,
    settings: &Settings,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    required_resolution(types, settings)?;

    // The desired phase runs only once every required trait is
    // settled: it counts what required resolution granted as present,
    // and never takes any of it away.
    desired_resolution(types, settings);

    Ok(())
}

/// Expand a requirement set to include the supertraits its members
/// imply.
///
/// `Ord` requires `PartialOrd`, `Eq`, and `PartialEq`; `Eq` requires
/// `PartialEq`; `PartialOrd` requires `PartialEq`. Applied to every
/// requirement set as it is formed (the settings-required set, map key
/// traits, and set element traits) so that a work item's `traits` is
/// always already closed--otherwise a lone `Ord` requirement would
/// derive `Ord` without the `Eq`/`PartialEq`/`PartialOrd` impls it
/// needs, and the emitted derive would not compile.
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
    traits
}

// Traits no container can provide.
const CONTAINER_UNSUPPORTED: &[TypespaceTrait] =
    &[TypespaceTrait::Display, TypespaceTrait::FromStr];

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
    /// trait.
    Impossible,
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
/// `Default` needs every constituent to implement it, unless the type
/// carries an attached default value, in which case the manual impl
/// needs nothing further (an enum with no attached default value has
/// no derive and no invented `#[default]` variant, so it is
/// impossible). Every other trait derives normally.
fn feasibility<Id>(ty: &Type<Id>, trait_name: TypespaceTrait) -> Feasibility<Id>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    match ty {
        // An alias has no impl site of its own to realize anything
        // with; every trait's fate belongs entirely to its target.
        Type::TypeAlias(_) => Feasibility::Forward,

        Type::Struct(Struct { common, .. }) | Type::TupleStruct(TupleStruct { common, .. }) => {
            match trait_name {
                // Neither has a derive, and neither has a sensible
                // manual rendering: a struct's fields have no implied
                // textual order or separator.
                TypespaceTrait::Display | TypespaceTrait::FromStr => Feasibility::Impossible,
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
            TypespaceTrait::Display | TypespaceTrait::FromStr => Feasibility::Impossible,
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
            TypespaceTrait::Default => {
                if common.default().is_some() {
                    Feasibility::ManuallyRealizable(Vec::new())
                } else {
                    Feasibility::Derivable
                }
            }
            _ => Feasibility::Derivable,
        },

        Type::Enum(e) => match trait_name {
            TypespaceTrait::Display | TypespaceTrait::FromStr => {
                if e.all_simple_variants() {
                    // A hand-written impl maps variants to and from
                    // their serialized names; no variant has payload
                    // types to forward to.
                    Feasibility::ManuallyRealizable(Vec::new())
                } else if matches!(e.tag_type, Some(EnumTagType::Untagged)) {
                    // An untagged enum's serialized form is exactly
                    // one variant's payload's serialized form, so
                    // Display/FromStr forward to whichever payload
                    // types the variants carry.
                    Feasibility::ManuallyRealizable(ty.contained_children_related())
                } else {
                    Feasibility::Impossible
                }
            }
            TypespaceTrait::Default => {
                if e.common.default().is_some() {
                    Feasibility::ManuallyRealizable(Vec::new())
                } else {
                    // No derive exists, and there is no invented
                    // #[default] variant.
                    Feasibility::Impossible
                }
            }
            _ => Feasibility::Derivable,
        },

        _ => unreachable!("feasibility is only called for named types"),
    }
}

fn required_resolution<Id>(
    types: &mut BTreeMap<Id, Type<Id>>,
    settings: &Settings,
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

    // Maps and sets require traits of their type parameters: the
    // configured map and set types state what they require of their key
    // and element types (the built-in defaults require the Ord family).
    // Look through all types for maps and sets and seed the work queue
    // with those requirements.
    let mut work = types
        .iter()
        .filter_map(|(type_id, ty)| match ty {
            Type::Map(key_schema_ref, _) => Some(WorkItem {
                target: key_schema_ref.clone(),
                traits: close_supertraits(settings.map_key_traits.clone()),
                origin: RequirementOrigin::MapKey(type_id.clone()),
                path: Vec::new(),
            }),
            Type::Set(element_schema_ref) => Some(WorkItem {
                target: element_schema_ref.clone(),
                traits: close_supertraits(settings.set_element_traits.clone()),
                origin: RequirementOrigin::SetElement(type_id.clone()),
                path: Vec::new(),
            }),
            _ => None,
        })
        .collect::<VecDeque<_>>();

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

    // In each iteration, we need to assert the set of required traits to the
    // current type. If the current type is generated, that means consulting
    // its feasibility for each newly-required trait, absorbing what
    // it can derive or manually realize, and pushing obligations onward. If
    // the type is **not** generated (native or otherwise external to our
    // control), we need to check that is implements (or is capable of
    // implementing) the required traits; if it doesn't (or can't), we'll
    // produce an error. We don't stop on the first failure, but want to
    // identify as many, distinct failures as is reasonable and as would be
    // useful for a consumer.
    while let Some(WorkItem {
        target,
        traits,
        origin,
        path,
    }) = work.pop_front()
    {
        let ty = types.get_mut(&target).unwrap();

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
            // Named types no longer absorb unconditionally: consult
            // feasibility per required trait. Derivable and forwarded
            // (alias) traits share one obligation set--every contained
            // child, via contained_children_related--so we batch them
            // and push once per child, exactly as unconditional
            // absorption used to. Manually realized traits carry their
            // own, often-empty, obligation list, so each gets its own
            // push. Impossible traits become conflicts and are never
            // absorbed.
            let mut derivable_new = TypespaceTraitSet::empty();
            let mut manual_pushes = Vec::<(TypespaceTrait, Vec<(Relation, Id)>)>::new();

            // Work against a copy of the built trait set: feasibility
            // borrows the type, so we cannot hold a live reference into
            // it across the loop. Written back below in one shot.
            let mut built = ty.common().unwrap().built.as_ref().unwrap().traits.clone();

            for trait_name in traits {
                if built.contains(&trait_name) {
                    continue;
                }

                match feasibility(ty, trait_name) {
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
                    Feasibility::Impossible => {
                        let kind = type_kind(ty);
                        conflict(
                            vec![trait_name],
                            OffenderReason::TypeCannotImplement { kind },
                        );
                    }
                }
            }

            ty.common_mut().unwrap().built.as_mut().unwrap().traits = built;

            if !derivable_new.is_empty() {
                for (relation, child_id) in ty.contained_children_related() {
                    work.push_back(WorkItem {
                        target: child_id,
                        traits: derivable_new.clone(),
                        origin: origin.clone(),
                        path: hop(relation),
                    });
                }
            }
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

                Type::Native(Native { name, impls, .. }) => {
                    let missing_traits = traits.difference(impls).copied().collect::<Vec<_>>();
                    let reason = OffenderReason::NativeMissingImpl {
                        type_name: name.clone(),
                    };
                    conflict(missing_traits, reason);
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
                Type::Box(schema_ref) => {
                    work.push_back(WorkItem {
                        target: schema_ref.clone(),
                        traits,
                        origin,
                        path: hop(Relation::Boxed),
                    });
                }

                // Vec<T> and arrays impl everything we care about--except for
                // Display and FromStr--as long as T implemented them. Vec<T>
                // additionally implements Default unconditionally.
                Type::Vec(schema_ref) => {
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "Vec".to_string(),
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

                // Like Vec, the map and set containers implement the traits
                // we care about--except for Display and FromStr--as long as
                // their key/value/element types do; both implement Default
                // unconditionally.
                Type::Map(key_ref, value_ref) => {
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "map".to_string(),
                        },
                    );
                    let pass = strip_default(rest);
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
                    let (bad, rest) = split(&traits, CONTAINER_UNSUPPORTED);
                    conflict(
                        bad,
                        OffenderReason::Primitive {
                            type_name: "set".to_string(),
                        },
                    );
                    let pass = strip_default(rest);
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

                // String implements every trait we track.
                Type::String => (),

                // JsonValue implements everything except for Eq, Ord,
                // PartialOrd, and Hash.
                Type::JsonValue => {
                    let (bad, _) = split(
                        &traits,
                        &[
                            TypespaceTrait::Eq,
                            TypespaceTrait::Ord,
                            TypespaceTrait::PartialOrd,
                            TypespaceTrait::Hash,
                        ],
                    );
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
/// `PartialEq`, so a type that loses one of those loses everything
/// resting on it.
fn strip_dependents(trait_name: TypespaceTrait) -> impl Iterator<Item = TypespaceTrait> {
    let dependents: &'static [TypespaceTrait] = match trait_name {
        TypespaceTrait::PartialEq => &[
            TypespaceTrait::Eq,
            TypespaceTrait::PartialOrd,
            TypespaceTrait::Ord,
        ],
        TypespaceTrait::Eq | TypespaceTrait::PartialOrd => &[TypespaceTrait::Ord],
        _ => &[],
    };
    std::iter::once(trait_name).chain(dependents.iter().copied())
}

/// Whether `ty` provides `trait_name`, given `has`.
///
/// `has` records what the types `ty` is built from still have. A
/// named type answers from the feasibility table: an impossible
/// trait is never provided, a derived or forwarded one needs every
/// contained child, and a manually realized one needs only the targets
/// that realization obligates--often none at all, as with an attached
/// default value. Containers and built-in types answer with the rules
/// required resolution applies to them, hop for hop: a container
/// forwards a trait to its parameters, except that no container has
/// `Display` or `FromStr` and `Option`, `Vec`, maps, and sets provide
/// `Default` whatever they hold.
fn provides<Id>(
    ty: &Type<Id>,
    trait_name: TypespaceTrait,
    has: &BTreeMap<Id, TypespaceTraitSet>,
) -> bool
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let child_has = |child: &Id| {
        has.get(child)
            .is_some_and(|traits| traits.contains(&trait_name))
    };

    if ty.is_named() {
        match feasibility(ty, trait_name) {
            Feasibility::Impossible => false,
            Feasibility::Derivable | Feasibility::Forward => ty
                .contained_children_related()
                .iter()
                .all(|(_, child)| child_has(child)),
            Feasibility::ManuallyRealizable(obligations) => obligations
                .iter()
                .all(|(_, obligated)| child_has(obligated)),
        }
    } else {
        let supported = !CONTAINER_UNSUPPORTED.contains(&trait_name);
        let is_default = matches!(trait_name, TypespaceTrait::Default);

        match ty {
            Type::Enum(_)
            | Type::Struct(_)
            | Type::UnitStruct(_)
            | Type::TupleStruct(_)
            | Type::NewtypeStruct(_)
            | Type::TypeAlias(_) => unreachable!(),

            Type::Native(Native { impls, .. }) => impls.contains(&trait_name),

            // Pass the buck... except for Default, which Option<T>
            // implements no matter what T is.
            Type::Option(schema_ref) => is_default || child_has(schema_ref),
            Type::Box(schema_ref) => child_has(schema_ref),

            // Vec<T> and the map and set containers implement the
            // traits we care about--except for Display and
            // FromStr--as long as their parameters do; all three
            // implement Default unconditionally.
            Type::Vec(schema_ref) | Type::Set(schema_ref) => {
                supported && (is_default || child_has(schema_ref))
            }
            Type::Map(key_ref, value_ref) => {
                supported && (is_default || (child_has(key_ref) && child_has(value_ref)))
            }

            // Arrays and tuples forward everything they can provide,
            // Default included.
            Type::Array(schema_ref, _) => supported && child_has(schema_ref),
            Type::Tuple(schema_refs) => supported && schema_refs.iter().all(child_has),

            // Integers, booleans, and String implement every trait we
            // track.
            Type::Integer(_) | Type::Boolean | Type::String => true,

            // The unit type and ::json_serde::Absent implement
            // everything except Display and FromStr.
            Type::Unit | Type::Never => supported,

            // Floating-point types have no total ordering, no equality
            // relation, and no hash.
            Type::Float(_) => !matches!(
                trait_name,
                TypespaceTrait::Ord | TypespaceTrait::Eq | TypespaceTrait::Hash
            ),

            // JsonValue implements everything except for Eq, Ord,
            // PartialOrd, and Hash.
            Type::JsonValue => !matches!(
                trait_name,
                TypespaceTrait::Eq
                    | TypespaceTrait::Ord
                    | TypespaceTrait::PartialOrd
                    | TypespaceTrait::Hash
            ),
        }
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

/// Give every named type the desired traits nothing blocks.
///
/// Each type starts out assumed to have every desired trait. The
/// traits a type cannot provide seed a work queue, and each loss
/// poisons that trait in the types that refer to the loser, which
/// poison their own referrers in turn, until the queue drains. A
/// referrer only loses the trait if it can no longer provide it, so a
/// hop that absorbs the loss--`Vec<T>` keeps `Default` however `T`
/// fares--stops the poison there.
///
/// Running the queue along referrer edges is required resolution's
/// descent in reverse, and it needs no rule for cycles: a recursive
/// type keeps a trait precisely because nothing ever poisoned it.
///
/// What survives is granted family by family: a trait the request
/// closure added rides on the trait that asked for it and goes when it
/// goes.
///
/// Losses are silent. A desired trait that does not survive is absent
/// from the built set with nothing recorded, which is the whole
/// contrast with a required trait.
fn desired_resolution<Id>(types: &mut BTreeMap<Id, Type<Id>>, settings: &Settings)
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    // Every member of the request closure has to be evaluated, whether
    // or not it was desired directly, since a family survives only if
    // all of it does.
    let desired = close_supertraits(settings.desired_traits.clone());
    if desired.is_empty() {
        return;
    }

    // Each desired trait paired with the supertraits its request
    // closure adds. A closure member is not desired on its own
    // account--it is there so that a granted derive compiles--so a
    // family is granted or dropped whole: a type that cannot have Eq
    // has no use for the PartialEq that Eq's closure asked for.
    let families = settings
        .desired_traits
        .iter()
        .map(|trait_name| close_supertraits([*trait_name].into_iter().collect()))
        .collect::<Vec<_>>();

    // The inverse of Type::children: the types referring to each type,
    // which is the direction a loss travels. Type::children reports
    // nothing for a native type, so a native's type parameters are not
    // reached from here.
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
                .filter(|trait_name| !provides(ty, **trait_name, &state.has))
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
            if state.has[referrer].contains(&trait_name) && !provides(ty, trait_name, &state.has) {
                let granted = granted_traits(types, referrer);
                state.lose(referrer, trait_name, &loser, granted);
            }
        }
    }

    // A named type's built set is what required resolution absorbed
    // plus the desired families that survived whole.
    for (type_id, ty) in types.iter_mut() {
        if let Some(common) = ty.common_mut() {
            let survivors = state.has.remove(type_id).unwrap();
            let built = common.built.as_mut().unwrap();
            for family in &families {
                if family
                    .iter()
                    .all(|trait_name| survivors.contains(trait_name))
                {
                    for trait_name in family.iter() {
                        built.traits.add(*trait_name);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        build::{Native, Type},
        error::{Error, OffenderReason, Relation, RequirementOrigin},
        no_cycles,
        settings::Settings,
        Typespace, TypespaceTrait, TypespaceTraitSet,
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
                RequirementOrigin::MapKey(id) if id == "Map<KeyStruct, String>"
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

    /// A trait an Option genuinely provides still passes through to the
    /// element and is satisfied there: the default set-element traits
    /// (the Ord family) reach Color through the Option and land on it.
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
            "::std::collections::HashSet",
            [TypespaceTrait::Default].into_iter().collect(),
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
    /// satisfied: an all-simple enum provides Display and FromStr, so
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
                gone: !,
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
    /// to the full comparison family, or the emitted derive would not
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

        // Desiring Eq desires PartialEq: the request closure applies to
        // desired demands exactly as it does to required ones.
        assert_eq!(
            built_traits(&typespace, "S"),
            trait_set([
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Hash,
            ])
        );
    }

    /// Desiring `Ord` alone desires the whole comparison family: the
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
    /// value, and an `f64` inside an `Option` each cost `Eq` while
    /// leaving `Clone` alone. A set is absent here because a set
    /// element that cannot be `Eq` is a phase-1 conflict (the default
    /// set element requirements are the `Ord` family), so the blocked
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
    fn float_field_strips_ordering_family_keeps_rest() {
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

    // REVIEW: "costs"? use some different term
    /// A `JsonValue` field costs the same traits an `f64` field does,
    /// plus `PartialOrd`: the ground truth for `Type::JsonValue` in
    /// `required_resolution` is that `serde_json::Value` implements
    /// everything except `Eq`, `Ord`, `PartialOrd`, and `Hash`. Losing
    /// `PartialOrd` strips `Ord` a second way, so `Clone`, `Debug`, and
    /// `PartialEq` are all that survive.
    ///
    /// ATTN REVIEWER: that ground truth is stale. `serde_json::Value`
    /// derives `Eq` (since well before the 1.0.148 this workspace
    /// depends on) and derives `Hash` as of the 1.0.151 in Cargo.lock;
    /// it implements neither `PartialOrd` nor `Ord`. Correcting the
    /// `Type::JsonValue` arm changes this test's expectation to keep
    /// `Eq` and `Hash`.
    ///
    ///
    #[test]
    fn json_value_field_strips_ordering_family() {
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
    /// cycle costs `Eq` for every member of it, including the member
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
    /// impl that asks nothing of the type's fields: `S` takes `Default`
    /// even though its `Color` field cannot implement `Default` at all.
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

    /// A desired trait imposes no requirement on anything. Desiring
    /// `Ord` everywhere reaches a native that declares only `Clone` and
    /// `Debug`; the native is not in conflict, finalization succeeds,
    /// and the struct holding it simply goes without the comparison
    /// family. A required `Ord` in the same graph would be
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

    /// A set element that cannot realize a desired trait costs the
    /// holder that trait and nothing else. The native declares the
    /// `Ord` family the default set element requirements demand, so
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

    /// The `Settings::all_traits` preset over a struct with a float:
    /// the required set lands whole, and of the desired set
    /// `PartialEq`, `PartialOrd`, and `Default` survive while
    /// `Display` and `FromStr` (impossible for a struct) and `Eq`,
    /// `Ord`, and `Hash` (blocked by the float) are dropped.
    #[test]
    fn all_traits_preset_over_float_struct() {
        let builder = typespace_builder!(Settings::all_traits(), {
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
                TypespaceTrait::Default,
            ])
        );
    }
}

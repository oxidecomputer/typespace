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

    // Desired-trait resolution (the greatest-fixed-point phase over
    // settings.desired_traits) lands with its own tests; until then it
    // is a deliberate no-op and desired_traits is silently ignored.

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

    // Traits no container can provide.
    const CONTAINER_UNSUPPORTED: &[TypespaceTrait] =
        &[TypespaceTrait::Display, TypespaceTrait::FromStr];

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
            // Consult feasibility for each trait newly required of this
            // named type. Derivable and forwarded (alias) traits share
            // one obligation set--every contained child, via
            // contained_children_related--so we batch them into a
            // single push per child. Manually realized traits carry
            // their own, often-empty, obligation lists, so each gets
            // its own push. Impossible traits become conflicts and
            // never enter the built set.
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

                // Pass the buck... except for Default, which Option<T>
                // implements no matter what T is.
                Type::Option(schema_ref) => {
                    let pass = strip_default(traits);
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
            }
        }
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(Error::TraitConflicts { conflicts })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        build::{Enum, EnumTagType, EnumVariant, Struct, StructProperty, Type, VariantDetails},
        error::{Error, OffenderReason, Relation, RequirementOrigin},
        no_cycles,
        settings::Settings,
        Typespace, TypespaceBuilder, TypespaceTrait, TypespaceTraitSet,
    };

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
        // struct Inner {
        //     count: u32,
        // }
        //
        // struct Outer {
        //     name: String,
        //     inner: Inner,
        // }
        let mut builder = TypespaceBuilder::new(Settings::typical());
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert("u32".to_string(), Type::Integer("u32".to_string()))
            .unwrap();
        builder
            .insert(
                "inner".to_string(),
                Struct::new()
                    .name("Inner")
                    .properties(vec![StructProperty::new("count", "u32".to_string())])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                "outer".to_string(),
                Struct::new()
                    .name("Outer")
                    .properties(vec![
                        StructProperty::new("name", "string".to_string()),
                        StructProperty::new("inner", "inner".to_string()),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Clone,
            TypespaceTrait::Debug,
            TypespaceTrait::Serialize,
            TypespaceTrait::Deserialize,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "outer"), expected);
        assert_eq!(built_traits(&typespace, "inner"), expected);
    }

    /// An unsatisfiable requirement reports a coherent chain: the
    /// origin (map key), each containment hop, the offending type, and
    /// the reason--and exactly one conflict per failing trait, not one
    /// per ancestor.
    #[test]
    fn map_key_conflict_reports_path() {
        // struct KeyStruct {
        //     weight: f64,
        // }
        //
        // BTreeMap<KeyStruct, String>
        let mut builder = TypespaceBuilder::new(Settings::minimal());
        builder
            .insert("f64".to_string(), Type::Float("f64".to_string()))
            .unwrap();
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "key".to_string(),
                Struct::new()
                    .name("KeyStruct")
                    .properties(vec![StructProperty::new("weight", "f64".to_string())])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                "map".to_string(),
                Type::Map("key".to_string(), "string".to_string()),
            )
            .unwrap();

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
                RequirementOrigin::MapKey(id) if id == "map"
            ));
            assert_eq!(conflict.offender, "f64");
            assert!(matches!(
                &conflict.reason,
                OffenderReason::Primitive { type_name } if type_name == "f64"
            ));
            // One hop: the key struct passes the requirement to its
            // field.
            assert_eq!(conflict.path.len(), 1, "path: {:#?}", conflict.path);
            assert_eq!(conflict.path[0].type_id, "key");
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
        // struct S {
        //     s: String,
        // }
        let settings = Settings::minimal().with_required_trait(TypespaceTrait::Display);
        let mut builder = TypespaceBuilder::new(settings);
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "s".to_string(),
                Struct::new()
                    .name("S")
                    .properties(vec![StructProperty::new("s", "string".to_string())])
                    .build()
                    .unwrap(),
            )
            .unwrap();

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
        assert_eq!(conflict.offender, "s");
        assert!(conflict.path.is_empty(), "path: {:#?}", conflict.path);
        assert!(matches!(
            conflict.reason,
            OffenderReason::TypeCannotImplement { kind: "struct" }
        ));
    }

    /// A required trait that a type realizes with a manual impl is
    /// satisfied: an all-simple enum provides Display and FromStr, so
    /// requiring them succeeds and they land in the built trait set.
    #[test]
    fn required_display_on_simple_enum_accepted() {
        // enum Color {
        //     Red,
        //     Green,
        // }
        let settings = Settings::minimal()
            .with_required_trait(TypespaceTrait::Display)
            .with_required_trait(TypespaceTrait::FromStr);
        let mut builder = TypespaceBuilder::new(settings);
        builder
            .insert(
                "color".to_string(),
                Enum::new()
                    .name("Color")
                    .tag_type(EnumTagType::External)
                    .variants(vec![
                        EnumVariant::new("Red", VariantDetails::<String>::Unit),
                        EnumVariant::new("Green", VariantDetails::Unit),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [TypespaceTrait::Display, TypespaceTrait::FromStr]
            .into_iter()
            .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "color"), expected);
    }

    /// Requiring a trait requires its supertraits: Ord alone expands
    /// to the full comparison family, or the emitted derive would not
    /// compile.
    #[test]
    fn supertrait_closure_expands_ord() {
        // struct S {
        //     name: String,
        // }
        let settings = Settings::minimal().with_required_trait(TypespaceTrait::Ord);
        let mut builder = TypespaceBuilder::new(settings);
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "s".to_string(),
                Struct::new()
                    .name("S")
                    .properties(vec![StructProperty::new("name", "string".to_string())])
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let typespace = builder.finalize(no_cycles).unwrap();

        let expected = [
            TypespaceTrait::Ord,
            TypespaceTrait::PartialOrd,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]
        .into_iter()
        .collect::<TypespaceTraitSet>();
        assert_eq!(built_traits(&typespace, "s"), expected);
    }
}

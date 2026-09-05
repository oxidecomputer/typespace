// Copyright 2026 Oxide Computer Company

//! What a container provides, checked from both ends.
//!
//! A container declaration answers, for every trait typespace tracks,
//! one of three things: the container never implements it, implements
//! it whatever its parameters do, or implements it when every parameter
//! does. Two things have to hold for generated code to compile, and
//! this file checks each separately.
//!
//! 1. The declaration matches the Rust container it describes. The
//!    compiler answers that one, through the probes in `common`: the
//!    `matches_std` tests instantiate the real container with
//!    parameters that have every trait and with a parameter that has
//!    none, and hold the impls that exist up against what the
//!    declaration claims. This is the direction that catches a claim
//!    typespace could not back up: output deriving `Ord` over a
//!    `HashMap` does not compile, and a probe answering `false` is the
//!    compiler saying so.
//!
//! 2. Both resolution phases obey the declaration. The `required_phase`
//!    and `desired_phase` tests drive every (declaration, trait,
//!    parameter position) cell through finalization and check where the
//!    trait came to rest.
//!
//! The two together say that what typespace accepts compiles and what
//! it refuses would not have.

use typespace::{
    build::{Native, NewtypeStruct, Type},
    error::{Error, OffenderReason, Relation, RequirementOrigin, TraitConflict},
    no_cycles,
    settings::{ContainerType, Settings, TraitProvision},
    Typespace, TypespaceBuilder, TypespaceTrait, TypespaceTraitSet,
};
use typespace_test_macro::{check_and_include, typespace_builder};

mod common;

/// Every trait typespace tracks, from the one public enumeration of
/// them: a declaration answers for each.
fn all_traits() -> Vec<TypespaceTrait> {
    let declaration = ContainerType::vec();
    declaration
        .provisions()
        .map(|(trait_, _)| trait_)
        .collect::<Vec<_>>()
}

/// `traits` less `removed`.
fn without(traits: &TypespaceTraitSet, removed: &[TypespaceTrait]) -> TypespaceTraitSet {
    traits
        .iter()
        .filter(|trait_| !removed.contains(trait_))
        .copied()
        .collect::<TypespaceTraitSet>()
}

/// A type parameter with no impls at all: no derive, nothing written by
/// hand. Only ever named as a type argument.
#[allow(dead_code)]
struct Nothing;

// ---------------------------------------------------------------------
// 1. The declarations match the containers they describe
// ---------------------------------------------------------------------

/// What `declaration` claims the container implements when every
/// parameter implements everything, and when one parameter implements
/// nothing.
fn claimed(declaration: &ContainerType) -> (TypespaceTraitSet, TypespaceTraitSet) {
    let with_parameters = declaration
        .provisions()
        .filter(|(_, provision)| !matches!(provision, TraitProvision::Never))
        .map(|(trait_, _)| trait_)
        .collect::<TypespaceTraitSet>();
    let without_parameters = declaration
        .provisions()
        .filter(|(_, provision)| matches!(provision, TraitProvision::Always))
        .map(|(trait_, _)| trait_)
        .collect::<TypespaceTraitSet>();
    (with_parameters, without_parameters)
}

// `u32` has every trait typespace tracks, `Copy` included, which is
// what makes it the parameter the propagation matrix reaches for when
// it wants one that blocks nothing; `String` no longer qualifies once
// `Copy` is tracked, since an owned heap buffer can never be `Copy`.
// Asking the probes about it also states that a probe exists per
// trait: a trait added to the vocabulary without one would leave a
// hole here rather than quietly narrowing every check below.
#[test]
fn probes_cover_every_tracked_trait() {
    assert_eq!(
        crate::implemented_traits!(u32),
        all_traits().into_iter().collect::<TypespaceTraitSet>()
    );
}

#[test]
fn btree_map_matches_std() {
    let (with_parameters, without_parameters) = claimed(&ContainerType::btree_map());
    assert_eq!(
        crate::implemented_traits!(::std::collections::BTreeMap<String, String>),
        with_parameters
    );
    assert_eq!(
        crate::implemented_traits!(::std::collections::BTreeMap<String, Nothing>),
        without_parameters
    );
    // A map key is treated as bound by JsonSchema, which schemars 1.x
    // requires and the 0.8 these tests build against does not: 0.8
    // bounds only the value, so a key with nothing keeps the impl. The
    // stronger claim is deliberate; see TypespaceTrait::JsonSchema.
    let mut key_unbounded = without_parameters.clone();
    key_unbounded.add(TypespaceTrait::JsonSchema);
    assert_eq!(
        crate::implemented_traits!(::std::collections::BTreeMap<Nothing, String>),
        key_unbounded
    );
}

#[test]
fn hash_map_matches_std() {
    let (with_parameters, without_parameters) = claimed(&ContainerType::hash_map());
    assert_eq!(
        crate::implemented_traits!(::std::collections::HashMap<String, String>),
        with_parameters
    );
    assert_eq!(
        crate::implemented_traits!(::std::collections::HashMap<String, Nothing>),
        without_parameters
    );
    // As for BTreeMap: schemars 0.8 leaves the key unbounded.
    let mut key_unbounded = without_parameters.clone();
    key_unbounded.add(TypespaceTrait::JsonSchema);
    assert_eq!(
        crate::implemented_traits!(::std::collections::HashMap<Nothing, String>),
        key_unbounded
    );
}

#[test]
fn btree_set_matches_std() {
    let (with_parameters, without_parameters) = claimed(&ContainerType::btree_set());
    assert_eq!(
        crate::implemented_traits!(::std::collections::BTreeSet<String>),
        with_parameters
    );
    assert_eq!(
        crate::implemented_traits!(::std::collections::BTreeSet<Nothing>),
        without_parameters
    );
}

#[test]
fn hash_set_matches_std() {
    let (with_parameters, without_parameters) = claimed(&ContainerType::hash_set());
    assert_eq!(
        crate::implemented_traits!(::std::collections::HashSet<String>),
        with_parameters
    );
    assert_eq!(
        crate::implemented_traits!(::std::collections::HashSet<Nothing>),
        without_parameters
    );
}

#[test]
fn vec_matches_std() {
    let (with_parameters, without_parameters) = claimed(&ContainerType::vec());
    assert_eq!(crate::implemented_traits!(Vec<String>), with_parameters);
    assert_eq!(crate::implemented_traits!(Vec<Nothing>), without_parameters);
}

// Option and Box are not consumer-configurable: their answers live in
// trait_resolution's own arms, and nothing installs a declaration for
// them. Stating those answers in the declaration vocabulary anyway lets
// the same checks drive them -- neither has Display or FromStr, Option
// has Default whatever it holds, Option forwards Copy but Box never has
// it (a box heap-allocates), and everything else follows the parameter.
fn option_rules() -> ContainerType {
    ContainerType::vec()
        .with_path("::std::option::Option")
        .with_provision(TypespaceTrait::Copy, TraitProvision::IfParameters)
}

fn box_rules() -> ContainerType {
    ContainerType::vec()
        .with_path("::std::boxed::Box")
        .with_provision(TypespaceTrait::Default, TraitProvision::IfParameters)
}

#[test]
fn option_rules_match_std() {
    let (with_parameter, without_parameter) = claimed(&option_rules());
    // u32, not String: Option<T> forwards Copy when T has it, and String
    // never does.
    assert_eq!(
        crate::implemented_traits!(::std::option::Option<u32>),
        with_parameter
    );
    assert_eq!(
        crate::implemented_traits!(::std::option::Option<Nothing>),
        without_parameter
    );
}

#[test]
fn box_rules_match_std() {
    let (with_parameter, without_parameter) = claimed(&box_rules());
    // Box does forward Display, which typespace does not claim; that
    // divergence is pinned as known_gaps::box_provides_display, and
    // dropping Display from both sides here is what leaves it the only
    // one.
    assert_eq!(
        without(
            &crate::implemented_traits!(::std::boxed::Box<String>),
            &[TypespaceTrait::Display]
        ),
        without(&with_parameter, &[TypespaceTrait::Display])
    );
    assert_eq!(
        crate::implemented_traits!(::std::boxed::Box<Nothing>),
        without_parameter
    );
}

// ---------------------------------------------------------------------
// 2. Both phases obey the declaration
// ---------------------------------------------------------------------

/// The id of the container node every probe graph is built around.
const CONTAINER: &str = "container";

/// The id of the newtype struct wrapping it.
const WRAPPER: &str = "Wrapper";

/// The container position under test, which decides both the node the
/// graph holds and the setting the declaration is installed as.
#[derive(Debug, Clone, Copy)]
enum Position {
    Map,
    Set,
    Vec,
    Option,
    Box,
}

impl Position {
    /// The relation naming each of the container's parameters, in
    /// parameter order.
    fn relations(self) -> Vec<Relation> {
        match self {
            Position::Map => vec![Relation::Key, Relation::Value],
            Position::Set | Position::Vec | Position::Option => vec![Relation::Element],
            Position::Box => vec![Relation::Boxed],
        }
    }

    /// Settings that render this position as `declaration`.
    fn settings(self, declaration: ContainerType) -> Settings {
        let settings = Settings::minimal();
        match self {
            Position::Map => settings.with_map_type(declaration),
            Position::Set => settings.with_set_type(declaration),
            Position::Vec => settings.with_vec_type(declaration),
            // Nothing installs a declaration for these two; the rules
            // the matrix drives them against are trait_resolution's
            // own.
            Position::Option | Position::Box => settings,
        }
    }

    /// The container node, over the given parameter ids.
    fn node(self, parameters: &[String]) -> Type<String> {
        match self {
            Position::Map => Type::Map(parameters[0].clone(), parameters[1].clone()),
            Position::Set => Type::Set(parameters[0].clone()),
            Position::Vec => Type::Vec(parameters[0].clone()),
            Position::Option => Type::Option(parameters[0].clone()),
            Position::Box => Type::Box(parameters[0].clone()),
        }
    }
}

/// What stands in one of the container's parameter positions.
#[derive(Debug, Clone, Copy)]
enum Parameter {
    /// An integer, which has every trait typespace tracks -- `Copy`
    /// included -- and so blocks nothing. `String` no longer serves
    /// here: it cannot be `Copy`.
    Everything,
    /// A native declaring every trait but one.
    AllBut(TypespaceTrait),
}

impl Parameter {
    fn node(self) -> Type<String> {
        match self {
            Parameter::Everything => Type::Integer("u32".to_string()),
            Parameter::AllBut(missing) => Type::Native(Native::new(
                "::ext::Poison",
                all_traits()
                    .into_iter()
                    .filter(|trait_| *trait_ != missing)
                    .collect::<TypespaceTraitSet>(),
                Vec::new(),
            )),
        }
    }
}

/// A newtype struct wrapping one container over `parameters`.
///
/// Approximately, for the map position:
///
///     struct Wrapper(Map<Poison, u32>);
///
/// A newtype carries every trait typespace tracks to its inner type,
/// Display and FromStr included, so this one graph probes all
/// fourteen. It is built by hand because `typespace_builder!` cannot
/// state a native's traits from a set computed at run time.
fn wrapped_container(
    settings: Settings,
    position: Position,
    parameters: &[Parameter],
) -> TypespaceBuilder<String> {
    let mut builder = TypespaceBuilder::new(settings);
    let ids = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let id = format!("p{index}");
            builder.insert(id.clone(), parameter.node()).unwrap();
            id
        })
        .collect::<Vec<_>>();
    builder
        .insert(CONTAINER.to_string(), position.node(&ids))
        .unwrap();
    builder
        .insert(
            WRAPPER.to_string(),
            NewtypeStruct::new(CONTAINER.to_string())
                .name(WRAPPER)
                .build()
                .unwrap(),
        )
        .unwrap();
    builder
}

/// Each parameter list one trait's cell is run against: every parameter
/// clean, then each position in turn held by a parameter without the
/// trait.
fn parameter_runs(count: usize, missing: TypespaceTrait) -> Vec<(Option<usize>, Vec<Parameter>)> {
    let clean = vec![Parameter::Everything; count];
    std::iter::once((None, clean))
        .chain((0..count).map(|slot| {
            let parameters = (0..count)
                .map(|index| {
                    if index == slot {
                        Parameter::AllBut(missing)
                    } else {
                        Parameter::Everything
                    }
                })
                .collect::<Vec<_>>();
            (Some(slot), parameters)
        }))
        .collect::<Vec<_>>()
}

/// Where a requirement for one trait came to rest.
#[derive(Debug, PartialEq)]
enum Landing {
    /// Nothing refused it: the container either provides it outright or
    /// passed it to parameters that have it.
    Satisfied,
    /// The container itself has no such impl.
    Container,
    /// It reached the parameter in this position, which refused it.
    Parameter(usize),
}

/// The conflicts finalization reported, or a description of any other
/// failure.
fn conflicts_of(
    result: Result<Typespace<String>, Error<String>>,
) -> Result<Vec<TraitConflict<String>>, String> {
    match result {
        Ok(_) => Ok(Vec::new()),
        Err(Error::TraitConflicts { conflicts }) => Ok(conflicts),
        Err(err) => Err(format!("finalization failed with {err}")),
    }
}

/// Where the settings-required `trait_` came to rest.
///
/// Only conflicts tracing back to that requirement count: a container's
/// blanket obligation raises conflicts of its own, and those are a
/// separate question.
fn required_landing(
    conflicts: &[TraitConflict<String>],
    trait_: TypespaceTrait,
    position: Position,
) -> Landing {
    let from_settings = conflicts
        .iter()
        .filter(|conflict| {
            conflict.required == trait_
                && matches!(conflict.origin, RequirementOrigin::GlobalSettings)
        })
        .collect::<Vec<_>>();
    let at_container = from_settings
        .iter()
        .any(|conflict| conflict.offender == CONTAINER);
    // A conflict at a parameter counts only if the requirement got
    // there through the container by the relation that parameter sits
    // at, which is what makes this a check of the hop and not just of
    // the offender.
    let at_parameter = position
        .relations()
        .into_iter()
        .enumerate()
        .find_map(|(slot, relation)| {
            from_settings
                .iter()
                .any(|conflict| {
                    conflict.offender == format!("p{slot}")
                        && conflict.path.last().is_some_and(|hop| {
                            hop.type_id == CONTAINER
                                && hop.relation.to_string() == relation.to_string()
                        })
                })
                .then_some(slot)
        });
    match (at_container, at_parameter) {
        (true, _) => Landing::Container,
        (false, Some(slot)) => Landing::Parameter(slot),
        (false, None) => Landing::Satisfied,
    }
}

/// Run the required phase over every trait and parameter position, and
/// describe every cell that came out other than the declaration says.
fn required_mismatches(position: Position, name: &str, declaration: &ContainerType) -> Vec<String> {
    all_traits()
        .into_iter()
        .flat_map(|trait_| {
            let provision = declaration.provision(trait_);
            parameter_runs(declaration.obligations().len(), trait_)
                .into_iter()
                .filter_map(|(poisoned, parameters)| {
                    let settings = position
                        .settings(declaration.clone())
                        .with_required_trait(trait_);
                    let result =
                        wrapped_container(settings, position, &parameters).finalize(no_cycles);
                    let expected = match (provision, poisoned) {
                        (TraitProvision::Never, _) => Landing::Container,
                        (TraitProvision::Always, _) | (_, None) => Landing::Satisfied,
                        (TraitProvision::IfParameters, Some(slot)) => Landing::Parameter(slot),
                    };
                    match conflicts_of(result) {
                        Err(message) => Some(format!("{name} {position:?} {trait_}: {message}")),
                        Ok(conflicts) => {
                            let actual = required_landing(&conflicts, trait_, position);
                            (actual != expected).then(|| {
                                format!(
                                    "{name} {position:?} {trait_} with parameter \
                                     {poisoned:?} poisoned: expected {expected:?}, \
                                     got {actual:?}"
                                )
                            })
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

/// Run the desired phase over every trait and parameter position, and
/// describe every cell that came out other than the declaration says.
fn desired_mismatches(position: Position, name: &str, declaration: &ContainerType) -> Vec<String> {
    all_traits()
        .into_iter()
        .flat_map(|trait_| {
            let provision = declaration.provision(trait_);
            parameter_runs(declaration.obligations().len(), trait_)
                .into_iter()
                // A parameter without a trait its position is blanket
                // obligated to have is a required-phase conflict, so the
                // desired phase never gets to answer for it. The preset
                // obligation sets are closed over supertraits already,
                // so membership is the whole test.
                .filter(|(poisoned, _)| {
                    poisoned.is_none_or(|slot| !declaration.obligation(slot).contains(&trait_))
                })
                .filter_map(|(poisoned, parameters)| {
                    let settings = position
                        .settings(declaration.clone())
                        .with_desired_trait(trait_);
                    let expected = match (provision, poisoned) {
                        (TraitProvision::Never, _) => false,
                        (TraitProvision::Always, _) | (_, None) => true,
                        (TraitProvision::IfParameters, Some(_)) => false,
                    };
                    match wrapped_container(settings, position, &parameters).finalize(no_cycles) {
                        Err(err) => Some(format!(
                            "{name} {position:?} {trait_} with parameter \
                             {poisoned:?} poisoned: finalization failed with \
                             {err}"
                        )),
                        Ok(typespace) => {
                            let file =
                                syn::parse2::<syn::File>(typespace.to_codespace().into_stream())
                                    .unwrap();
                            let actual = common::implements(&file, WRAPPER, &trait_.to_string());
                            (actual != expected).then(|| {
                                format!(
                                    "{name} {position:?} {trait_} with parameter \
                                     {poisoned:?} poisoned: expected {expected}, \
                                     got {actual}"
                                )
                            })
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

/// A declaration for a consumer path with nothing said about it, at the
/// parameter count `position` renders.
fn opaque(position: Position) -> ContainerType {
    let obligations = position
        .relations()
        .into_iter()
        .map(|_| TypespaceTraitSet::empty())
        .collect::<Vec<_>>();
    ContainerType::new("::custom::Container", obligations)
}

/// The declarations the map position is driven with.
fn map_declarations() -> Vec<(&'static str, ContainerType)> {
    vec![
        ("btree_map", ContainerType::btree_map()),
        ("hash_map", ContainerType::hash_map()),
        ("opaque", opaque(Position::Map)),
    ]
}

/// The declarations the set position is driven with, starting with the
/// one absent configuration: a `Vec` carrying the ordered-lookup
/// obligation that set deduplication policy imposes.
fn set_declarations() -> Vec<(&'static str, ContainerType)> {
    let ordered = [
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
    ]
    .into_iter()
    .collect::<TypespaceTraitSet>();
    vec![
        (
            "default set",
            ContainerType::vec().with_obligations([ordered]),
        ),
        ("btree_set", ContainerType::btree_set()),
        ("hash_set", ContainerType::hash_set()),
        ("opaque", opaque(Position::Set)),
    ]
}

/// The declarations the vec position is driven with.
fn vec_declarations() -> Vec<(&'static str, ContainerType)> {
    vec![
        ("vec", ContainerType::vec()),
        ("opaque", opaque(Position::Vec)),
    ]
}

/// The rules the wrapper positions are driven with.
fn option_declarations() -> Vec<(&'static str, ContainerType)> {
    vec![("Option", option_rules())]
}

fn box_declarations() -> Vec<(&'static str, ContainerType)> {
    vec![("Box", box_rules())]
}

/// Every mismatch the declarations produced, run through `drive`.
fn mismatches(
    position: Position,
    declarations: Vec<(&'static str, ContainerType)>,
    drive: fn(Position, &str, &ContainerType) -> Vec<String>,
) -> Vec<String> {
    declarations
        .iter()
        .flat_map(|(name, declaration)| drive(position, name, declaration))
        .collect::<Vec<_>>()
}

#[test]
fn required_phase_follows_the_map_declaration() {
    let found = mismatches(Position::Map, map_declarations(), required_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn required_phase_follows_the_set_declaration() {
    let found = mismatches(Position::Set, set_declarations(), required_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn required_phase_follows_the_vec_declaration() {
    let found = mismatches(Position::Vec, vec_declarations(), required_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn desired_phase_follows_the_map_declaration() {
    let found = mismatches(Position::Map, map_declarations(), desired_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn desired_phase_follows_the_set_declaration() {
    let found = mismatches(Position::Set, set_declarations(), desired_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn desired_phase_follows_the_vec_declaration() {
    let found = mismatches(Position::Vec, vec_declarations(), desired_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn required_phase_follows_the_option_rules() {
    let found = mismatches(Position::Option, option_declarations(), required_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn required_phase_follows_the_box_rules() {
    let found = mismatches(Position::Box, box_declarations(), required_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn desired_phase_follows_the_option_rules() {
    let found = mismatches(Position::Option, option_declarations(), desired_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn desired_phase_follows_the_box_rules() {
    let found = mismatches(Position::Box, box_declarations(), desired_mismatches);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

// ---------------------------------------------------------------------
// The blanket obligations, the other half of a declaration
// ---------------------------------------------------------------------

// The motivating bug, as a case of its own: with map_type set to
// HashMap, requiring Ord of every type is a conflict at the map rather
// than a derive over a container that has no Ord impl.
#[test]
fn hash_map_refuses_a_required_ord() {
    let settings = Settings::minimal()
        .with_map_type(ContainerType::hash_map())
        .with_required_trait(TypespaceTrait::Ord);
    let builder = typespace_builder!(settings, {
        struct Holder {
            m: Map<String, u32>,
        }
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };
    assert!(conflicts.iter().any(|conflict| {
        conflict.offender == "Map<String, u32>"
            && matches!(conflict.required, TypespaceTrait::Ord)
            && matches!(
                &conflict.reason,
                OffenderReason::Primitive { type_name } if type_name == "map"
            )
    }));
}

// A map's value obligation reaches the value type. The presets demand
// nothing there, so this states one.
#[test]
fn map_value_obligation_is_imposed() {
    let settings =
        Settings::minimal().with_map_type(ContainerType::btree_map().with_obligations([
            TypespaceTraitSet::empty(),
            [TypespaceTrait::Ord].into_iter().collect(),
        ]));
    let builder = typespace_builder!(settings, {
        type Entries = Map<String, f64>;
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };
    let conflict = conflicts
        .iter()
        .find(|conflict| conflict.required == TypespaceTrait::Ord)
        .expect("a conflict for Ord");
    assert_eq!(conflict.offender, "f64");
    assert!(matches!(
        &conflict.origin,
        RequirementOrigin::ContainerParameter { container, relation }
            if container == "Map<String, f64>" && matches!(relation, Relation::Value)
    ));
    assert_eq!(
        conflict.to_string(),
        "type `f64` (id `f64`) cannot implement the required trait `Ord`\n    \
         required because the container `Map<String, f64>` requires `Ord` of \
         its value type"
    );
}

// A vec's element obligation reaches the element type; the vec preset
// demands nothing, so this states one.
#[test]
fn vec_element_obligation_is_imposed() {
    let settings = Settings::minimal().with_vec_type(
        ContainerType::vec().with_obligations([[TypespaceTrait::Eq].into_iter().collect()]),
    );
    let builder = typespace_builder!(settings, {
        type Samples = Vec<f64>;
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };
    let conflict = conflicts
        .iter()
        .find(|conflict| conflict.required == TypespaceTrait::Eq)
        .expect("a conflict for Eq");
    assert_eq!(conflict.offender, "f64");
    assert!(matches!(
        &conflict.origin,
        RequirementOrigin::ContainerParameter { container, relation }
            if container == "Vec<f64>" && matches!(relation, Relation::Element)
    ));
    assert_eq!(
        conflict.to_string(),
        "type `f64` (id `f64`) cannot implement the required trait `Eq`\n    \
         required because the container `Vec<f64>` requires `Eq` of its \
         element type"
    );
}

// Filtering the forwarded set can leave a subtrait without the
// supertraits it rests on, so a container re-closes what it forwards.
#[test]
fn a_forwarded_set_stays_closed() {
    // The set demands the Ord family of its element, the map. The map
    // claims PartialEq unconditionally, so PartialEq is not forwarded,
    // and without the re-closure the map's value would absorb Eq, Ord,
    // and PartialOrd with no PartialEq and derive code that does not
    // compile.
    let settings = Settings::minimal().with_map_type(
        ContainerType::btree_map()
            .with_provision(TypespaceTrait::PartialEq, TraitProvision::Always),
    );
    let builder = typespace_builder!(settings, {
        struct Value {
            n: u32,
        }

        type Grouped = Set<Map<String, Value>>;
    });

    let typespace = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(typespace.to_codespace().into_stream()).unwrap();
    let mut derives = common::derives_of(&file, "Value");
    derives.sort();
    assert_eq!(derives, ["Eq", "Ord", "PartialEq", "PartialOrd"]);
}

// A string-keyed map of JSON values renders as ::serde_json::Map
// whatever map_type is set to, so the node propagation reasons over and
// the type rendering emits are two different containers. Nothing lines
// them up; what keeps the output compiling is that everything the map
// node grants, serde_json::Map happens to implement. This says so, and
// says it against the real impls rather than against a table.
#[test]
fn a_json_value_map_grants_only_what_serde_json_map_has() {
    let settings = all_traits()
        .into_iter()
        .fold(Settings::minimal(), |settings, trait_| {
            settings.with_desired_trait(trait_)
        });
    let builder = typespace_builder!(settings, {
        struct Wrapper(Map<String, JsonValue>);
    });

    let typespace = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(typespace.to_codespace().into_stream()).unwrap();
    let granted = all_traits()
        .into_iter()
        .filter(|trait_| common::implements(&file, WRAPPER, &trait_.to_string()))
        .collect::<TypespaceTraitSet>();

    let rendered = crate::implemented_traits!(::serde_json::Map<String, ::serde_json::Value>);
    let overclaimed = without(&granted, &rendered.iter().copied().collect::<Vec<_>>());
    assert!(
        overclaimed.is_empty(),
        "granted over a container that does not implement them: {overclaimed:?}"
    );
}

// ---------------------------------------------------------------------
// The output compiles
// ---------------------------------------------------------------------

/// Every comparison trait desired, with serde and the two traits every
/// generated type carries required, over a type holding all three
/// configurable containers.
fn comparison_settings() -> Settings {
    [
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
        TypespaceTrait::Hash,
    ]
    .into_iter()
    .fold(Settings::minimal(), |settings, trait_| {
        settings.with_desired_trait(trait_)
    })
    .with_required_trait(TypespaceTrait::Clone)
    .with_required_trait(TypespaceTrait::Debug)
    .with_required_trait(TypespaceTrait::Serialize)
    .with_required_trait(TypespaceTrait::Deserialize)
}

// The ordered containers provide every comparison trait, so the derives
// land, and the snapshot -- which is compiled and run -- says they were
// derives the containers could carry.
#[test]
fn ordered_containers_render_what_they_provide() {
    let settings = comparison_settings()
        .with_map_type(ContainerType::btree_map())
        .with_set_type(ContainerType::btree_set());
    let builder = typespace_builder!(settings, {
        struct Holder {
            m: Map<String, u32>,
            s: Set<String>,
            v: Vec<u32>,
        }
    });
    let typespace = builder.finalize(no_cycles).unwrap();

    let file = syn::parse2::<syn::File>(typespace.to_codespace().into_stream()).unwrap();
    let mut derives = common::derives_of(&file, "Holder");
    derives.sort();
    assert_eq!(
        derives,
        [
            "Clone",
            "Debug",
            "Deserialize",
            "Eq",
            "Hash",
            "Ord",
            "PartialEq",
            "PartialOrd",
            "Serialize"
        ]
    );

    #[check_and_include(
        "tests/output/ordered_containers_render_what_they_provide.rs",
        typespace.to_codespace().into_stream()
    )]
    fn inner() {
        // Every container the holder names implements Default, whatever
        // the holder itself can do about it.
        let holder = import::Holder {
            m: Default::default(),
            s: Default::default(),
            v: Default::default(),
        };
        let same = import::Holder {
            m: Default::default(),
            s: Default::default(),
            v: Default::default(),
        };
        assert_eq!(holder, same);
        assert!(holder <= same);
    }
}

// The hash containers have neither an ordering nor a Hash impl, so
// those derives are dropped and only the ones the containers carry are
// left.
#[test]
fn hash_containers_render_what_they_provide() {
    let settings = comparison_settings()
        .with_map_type(ContainerType::hash_map())
        .with_set_type(ContainerType::hash_set());
    let builder = typespace_builder!(settings, {
        struct Holder {
            m: Map<String, u32>,
            s: Set<String>,
            v: Vec<u32>,
        }
    });
    let typespace = builder.finalize(no_cycles).unwrap();

    let file = syn::parse2::<syn::File>(typespace.to_codespace().into_stream()).unwrap();
    let mut derives = common::derives_of(&file, "Holder");
    derives.sort();
    assert_eq!(
        derives,
        [
            "Clone",
            "Debug",
            "Deserialize",
            "Eq",
            "PartialEq",
            "Serialize"
        ]
    );

    #[check_and_include(
        "tests/output/hash_containers_render_what_they_provide.rs",
        typespace.to_codespace().into_stream()
    )]
    fn inner() {
        let holder = import::Holder {
            m: Default::default(),
            s: Default::default(),
            v: Default::default(),
        };
        let same = import::Holder {
            m: Default::default(),
            s: Default::default(),
            v: Default::default(),
        };
        assert_eq!(holder, same);
    }
}

// Copyright 2026 Oxide Computer Company

//! What a caller states about a container type, and what reads back.

use typespace::{
    TraitProvision, TypespaceBuilder, TypespaceTrait, TypespaceTraitSet,
    build::Type,
    error::{Error, PathStep, Relation},
    no_cycles,
    settings::{ContainerType, OptionalNullable, Settings},
};
use typespace_test_macro::typespace_builder;

fn set(traits: impl IntoIterator<Item = TypespaceTrait>) -> TypespaceTraitSet {
    traits.into_iter().collect()
}

fn ordered_lookup() -> TypespaceTraitSet {
    set([
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
    ])
}

fn hash_lookup() -> TypespaceTraitSet {
    set([
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Hash,
    ])
}

/// What a container implements, in a fixed order: the tables are
/// iterated in whatever order the trait enum sorts in, which is no part
/// of what these tests are checking.
fn provisions(container: &ContainerType) -> Vec<(TypespaceTrait, TraitProvision)> {
    let mut provisions = container.provisions().collect::<Vec<_>>();
    provisions.sort_by_key(|(trait_, _)| *trait_);
    provisions
}

// An unconfigured Settings declares the containers typespace renders on
// its own: a BTreeMap for maps, a Vec for both the set and the vec
// position, and the ordered-lookup demand on set elements that
// deduplication policy--not the Vec--imposes.
#[test]
fn defaults_declare_the_std_containers() {
    let default_set = ContainerType::vec().with_obligations([ordered_lookup()]);

    for settings in [
        Settings::minimal(),
        Settings::typical(),
        Settings::maximal(),
        serde_json::from_str::<Settings>("{}").unwrap(),
    ] {
        assert_eq!(settings.map_type(), &ContainerType::btree_map());
        assert_eq!(
            settings.map_type().obligations(),
            [ordered_lookup(), set([])]
        );
        assert_eq!(settings.set_type(), &default_set);
        assert_eq!(settings.vec_type(), &ContainerType::vec());
        assert_eq!(settings.vec_type().obligations(), [set([])]);
    }
}

// A declaration answers every trait typespace tracks; there is no
// partial declaration to read back.
#[test]
fn a_declaration_answers_every_trait() {
    use TraitProvision::{Always, IfParameters, Never};
    use TypespaceTrait::*;

    let mut expected = vec![
        (Clone, IfParameters),
        (Copy, Never),
        (Debug, IfParameters),
        (Serialize, IfParameters),
        (Deserialize, IfParameters),
        (JsonSchema, IfParameters),
        (Display, Never),
        (FromStr, Never),
        (Eq, IfParameters),
        (PartialEq, IfParameters),
        (Ord, Never),
        (PartialOrd, Never),
        (Hash, Never),
        (Default, Always),
    ];
    expected.sort_by_key(|(trait_, _)| *trait_);

    assert_eq!(provisions(&ContainerType::hash_map()), expected);
    assert_eq!(provisions(&ContainerType::hash_set()), expected);
}

// The hash and the ordered presets differ in exactly the traits std's
// hash containers have no impl for, and in what they demand of the key.
#[test]
fn the_presets_differ_where_std_differs() {
    let btree = ContainerType::btree_map();
    let hash = ContainerType::hash_map();

    let mut differing = hash
        .provisions()
        .filter(|(trait_, provision)| btree.provision(*trait_) != *provision)
        .map(|(trait_, _)| trait_)
        .collect::<Vec<_>>();
    differing.sort();

    let mut expected = vec![
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
        TypespaceTrait::Hash,
    ];
    expected.sort();

    assert_eq!(differing, expected);
    assert_eq!(btree.obligation(0), &ordered_lookup());
    assert_eq!(hash.obligation(0), &hash_lookup());
}

// A container outside the std presets states the whole truth: an
// obligation the preset does not carry, one on the value position, and
// a per-trait answer that departs from the preset.
#[test]
fn an_exotic_map_declaration_reads_back_as_written() {
    let settings = Settings::minimal().with_map_type(
        ContainerType::btree_map()
            .with_path("::im::OrdMap")
            .with_obligations([
                set([TypespaceTrait::Ord, TypespaceTrait::Clone]),
                set([TypespaceTrait::Clone]),
            ])
            .with_provision(TypespaceTrait::Hash, TraitProvision::Never)
            .with_provision(TypespaceTrait::Clone, TraitProvision::Always),
    );

    let declared = settings.map_type();
    assert_eq!(
        declared.obligations(),
        [
            set([TypespaceTrait::Ord, TypespaceTrait::Clone]),
            set([TypespaceTrait::Clone]),
        ]
    );
    assert_eq!(
        declared.provision(TypespaceTrait::Hash),
        TraitProvision::Never
    );
    assert_eq!(
        declared.provision(TypespaceTrait::Clone),
        TraitProvision::Always
    );

    // What the declaration did not touch keeps the preset's answers.
    assert_eq!(
        declared.provision(TypespaceTrait::Ord),
        TraitProvision::IfParameters
    );
    assert_eq!(
        declared.provision(TypespaceTrait::Display),
        TraitProvision::Never
    );
    assert_eq!(
        declared.provision(TypespaceTrait::Default),
        TraitProvision::Always
    );
}

// The set and vec positions carry their own declarations.
#[test]
fn set_and_vec_containers_are_declared_separately() {
    let settings = Settings::minimal()
        .with_set_type(ContainerType::hash_set())
        .with_vec_type(
            ContainerType::vec()
                .with_path("::im::Vector")
                .with_obligations([set([TypespaceTrait::Clone])]),
        );

    assert_eq!(settings.set_type(), &ContainerType::hash_set());
    assert_eq!(settings.set_type().obligation(0), &hash_lookup());
    assert_eq!(
        settings.set_type().provision(TypespaceTrait::Ord),
        TraitProvision::Never
    );

    assert_eq!(
        settings.vec_type().obligation(0),
        &set([TypespaceTrait::Clone])
    );
    assert_eq!(
        settings.vec_type().provision(TypespaceTrait::Ord),
        TraitProvision::IfParameters
    );
}

// A vec demands nothing of its element by default, and states an
// obligation the same way a map or a set does.
#[test]
fn a_vec_states_an_element_obligation() {
    assert!(ContainerType::vec().obligation(0).is_empty());

    let settings = Settings::minimal().with_vec_type(
        ContainerType::vec()
            .with_path("::smallvec::SmallVec")
            .with_obligations([set([TypespaceTrait::Clone, TypespaceTrait::Default])]),
    );

    assert_eq!(
        settings.vec_type().obligation(0),
        &set([TypespaceTrait::Clone, TypespaceTrait::Default])
    );
    assert_eq!(settings.vec_type().obligations().len(), 1);
    // The obligation is the only departure from `Vec`: what the
    // container implements is the forwarding table still.
    assert_eq!(
        provisions(settings.vec_type()),
        provisions(&ContainerType::vec())
    );
}

// A preset at another path declares the same behavior as the preset it
// came from: the path is what changed.
#[test]
fn a_preset_at_another_path_keeps_its_behavior() {
    let settings =
        Settings::minimal().with_map_type(ContainerType::hash_map().with_path("CustomMap"));

    assert_eq!(settings.map_type().obligation(0), &hash_lookup());
    assert_eq!(
        provisions(settings.map_type()),
        provisions(&ContainerType::hash_map())
    );
}

// A container that matches no preset claims only what rendering
// assumes of any container, so a requirement it could in fact satisfy
// is a conflict rather than generated code that does not compile.
#[test]
fn an_opaque_container_claims_only_the_rendering_baseline() {
    let declared = ContainerType::new("::im::Vector", [TypespaceTraitSet::empty()]);

    assert!(declared.obligation(0).is_empty());
    for (trait_, provision) in declared.provisions() {
        let expected = match trait_ {
            TypespaceTrait::Clone
            | TypespaceTrait::Debug
            | TypespaceTrait::Serialize
            | TypespaceTrait::Deserialize => TraitProvision::IfParameters,
            TypespaceTrait::Default => TraitProvision::Always,
            _ => TraitProvision::Never,
        };
        assert_eq!(provision, expected, "{trait_}");
    }
}

// A declaration in settings data names the preset it behaves as and
// lists what differs from it.
#[test]
fn declarations_deserialize_from_settings_data() {
    let settings = serde_json::from_str::<Settings>(
        r#"{
            "map_type": {
                "like": "hash-map",
                "path": "::im::HashMap",
                "obligations": [["eq", "partial-eq", "hash"], ["clone"]],
                "provides": { "hash": "always" }
            },
            "set_type": { "like": "btree-set" },
            "vec_type": { "path": "::im::Vector", "obligations": [["clone"]] }
        }"#,
    )
    .unwrap();

    assert_eq!(
        settings.map_type(),
        &ContainerType::hash_map()
            .with_path("::im::HashMap")
            .with_obligations([hash_lookup(), set([TypespaceTrait::Clone])])
            .with_provision(TypespaceTrait::Hash, TraitProvision::Always)
    );
    assert_eq!(
        settings.map_type().provision(TypespaceTrait::Ord),
        TraitProvision::Never
    );

    assert_eq!(settings.set_type(), &ContainerType::btree_set());

    // No preset named: the container states its own path and
    // obligations, and claims only the rendering baseline.
    assert_eq!(
        settings.vec_type(),
        &ContainerType::new("::im::Vector", [set([TypespaceTrait::Clone])])
    );
    assert_eq!(
        settings.vec_type().provision(TypespaceTrait::JsonSchema),
        TraitProvision::Never
    );
}

// A declaration in settings data states a preset or states its own path
// and obligations, and cannot carry a key the declaration has no place
// for.
#[test]
fn a_deserialized_declaration_is_whole() {
    let partial =
        serde_json::from_str::<Settings>(r#"{ "map_type": { "path": "CustomMap" } }"#).unwrap_err();
    assert!(
        partial.to_string().contains("`path` and `obligations`"),
        "{partial}"
    );

    let no_declaration =
        serde_json::from_str::<Settings>(r#"{ "vec_type": { "provides": {} } }"#).unwrap_err();
    assert!(
        no_declaration.to_string().contains("`like`"),
        "{no_declaration}"
    );

    let unknown_key =
        serde_json::from_str::<Settings>(r#"{ "vec_type": { "like": "vec", "key": [] } }"#)
            .unwrap_err();
    assert!(
        unknown_key.to_string().contains("unknown field `key`"),
        "{unknown_key}"
    );
}

// A declaration states what the container demands of each parameter the
// position renders, so a one-parameter declaration configured as the
// map type is rejected rather than silently misapplied.
#[test]
fn cardinality_mismatch_is_rejected_at_finalization() {
    let settings = Settings::minimal()
        .with_map_type(ContainerType::hash_map().with_obligations([hash_lookup()]));

    let mut builder = TypespaceBuilder::new(settings);
    builder.insert("string".to_string(), Type::String).unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the map declaration");
    };
    assert!(
        matches!(
            err,
            Error::ContainerParameterCount {
                position: "map",
                declared: 1,
                parameters: 2,
                ..
            }
        ),
        "{err}"
    );
}

// `TraitProvision::Unknown` is only meaningful for a machine-authored
// native; a configured container is hand-authored, so declaring it
// there is a configuration error rather than a silent pass.
#[test]
fn unknown_provision_on_a_container_is_rejected_at_finalization() {
    let settings = Settings::minimal().with_vec_type(
        ContainerType::vec().with_provision(TypespaceTrait::Hash, TraitProvision::Unknown),
    );

    let mut builder = TypespaceBuilder::new(settings);
    builder.insert("string".to_string(), Type::String).unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the Unknown provision");
    };
    assert!(
        matches!(
            err,
            Error::ContainerProvisionUnknown {
                position: "vec",
                trait_: TypespaceTrait::Hash,
                ..
            }
        ),
        "{err}"
    );
}

// A map key is demanded JsonSchema even where schemars 0.8 would not
// need it. The graph, roughly:
//
//     struct Holder { m: BTreeMap<::ext::RawKey, u32> }
//
// where RawKey declares neither JsonSchema nor anything else beyond the
// serde four. Under schemars 0.8 the derive compiles anyway, since its
// map impls bound only the value; under 1.x the key is bound and the
// refusal is right. typespace has one JsonSchema trait for both and
// states the stronger form, so this refusal is deliberate.
#[test]
fn json_schema_demanded_of_map_key() {
    let settings = Settings::minimal().with_required_trait(TypespaceTrait::JsonSchema);
    let builder = typespace_builder!(settings, {
        native ::ext::RawKey: Clone + Debug + Serialize + Deserialize;

        struct Holder {
            m: Map<::ext::RawKey, u32>,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("the key is demanded JsonSchema");
    };
    let rendered = err.to_string();
    assert!(rendered.contains("JsonSchema"), "{rendered}");
}

// The custom optional-nullable wrapper is declared like any other
// container, in code and in settings data, with the option preset as
// its base.
#[test]
fn a_custom_optional_wrapper_deserializes_like_a_container() {
    let settings = serde_json::from_str::<Settings>(
        r#"{
            "optional_nullable": {
                "custom-type": { "like": "option", "path": "::my::Opt" }
            }
        }"#,
    )
    .unwrap();

    let OptionalNullable::CustomType(declaration) = settings.optional_nullable() else {
        panic!("the settings data names a custom wrapper");
    };
    assert_eq!(declaration, &ContainerType::option().with_path("::my::Opt"));
    assert_eq!(declaration.obligations(), [set([])]);
    assert_eq!(
        declaration.provision(TypespaceTrait::Copy),
        TraitProvision::IfParameters
    );
    assert_eq!(
        declaration.provision(TypespaceTrait::Display),
        TraitProvision::Never
    );
}

// The wrapper takes exactly one parameter; a declaration with any other
// obligation count is rejected at finalization like a mis-declared map.
#[test]
fn custom_optional_cardinality_mismatch_is_rejected_at_finalization() {
    let settings = Settings::minimal().with_optional_nullable(OptionalNullable::CustomType(
        ContainerType::option()
            .with_path("::my::Opt")
            .with_obligations(Vec::<TypespaceTraitSet>::new()),
    ));

    let mut builder = TypespaceBuilder::new(settings);
    builder.insert("string".to_string(), Type::String).unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the wrapper declaration");
    };
    assert!(
        matches!(
            err,
            Error::ContainerParameterCount {
                position: "optional-nullable",
                declared: 0,
                parameters: 1,
                ..
            }
        ),
        "{err}"
    );
}

// What the wrapper demands of its value type is imposed like a map
// key's demand, from each optional property the declaration wraps: a
// value type without the trait conflicts, whatever else is required.
#[test]
fn a_custom_optional_wrapper_states_a_value_obligation() {
    let settings = Settings::minimal().with_optional_nullable(OptionalNullable::CustomType(
        ContainerType::option()
            .with_path("::my::Opt")
            .with_obligations([set([TypespaceTrait::Hash])]),
    ));

    let builder = typespace_builder!(settings, {
        native ::ext::NoHash: Clone + Debug + Serialize + Deserialize;

        struct Holder {
            o: OptionalNullable<::ext::NoHash>,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("the value type is demanded Hash");
    };
    let rendered = err.to_string();
    assert!(rendered.contains("Hash"), "{rendered}");
}

// The declaration models exactly the positions rendering wraps. One
// Option node shared between an optional property (wrapped) and a vec
// element (std Option) conflicts only through the property.
#[test]
fn a_weakened_wrapper_conflicts_only_at_wrapped_properties() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Hash)
        .with_optional_nullable(OptionalNullable::CustomType(
            ContainerType::option()
                .with_path("::custom::Opt")
                .with_provision(TypespaceTrait::Hash, TraitProvision::Never),
        ));

    let builder = typespace_builder!(settings, {
        struct Holder {
            wrapped: OptionalNullable<u32>,
            listed: Vec<Nullable<u32>>,
        }
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("the wrapped property is refused Hash");
    };
    assert_eq!(conflicts.len(), 1, "{conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Hash);
    assert_eq!(conflict.offender, "Nullable<u32>");
    assert!(
        matches!(
            conflict.path.as_slice(),
            [PathStep { type_id, relation: Relation::Field(name), .. }]
                if type_id == "Holder" && name == "wrapped"
        ),
        "{conflict:#?}"
    );
}

// A struct-shaped enum variant's optional property wraps exactly as a
// struct's does: the same declaration answers, and a weakened wrapper
// conflicts at the variant edge.
#[test]
fn a_weakened_wrapper_conflicts_at_struct_variant_properties() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Hash)
        .with_optional_nullable(OptionalNullable::CustomType(
            ContainerType::option()
                .with_path("::custom::Opt")
                .with_provision(TypespaceTrait::Hash, TraitProvision::Never),
        ));

    let builder = typespace_builder!(settings, {
        enum Holder {
            Named { wrapped: OptionalNullable<u32> },
        }
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("the wrapped variant property is refused Hash");
    };
    assert_eq!(conflicts.len(), 1, "{conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Hash);
    assert_eq!(conflict.offender, "Nullable<u32>");
    assert!(
        matches!(
            conflict.path.as_slice(),
            [PathStep { type_id, relation: Relation::Variant(name), .. }]
                if type_id == "Holder" && name == "Named"
        ),
        "{conflict:#?}"
    );
}

// A required property whose type happens to be an Option renders as
// std Option, so the declaration is not consulted there and the std
// model answers.
#[test]
fn a_required_option_property_keeps_the_std_model() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Hash)
        .with_optional_nullable(OptionalNullable::CustomType(
            ContainerType::option()
                .with_path("::custom::Opt")
                .with_provision(TypespaceTrait::Hash, TraitProvision::Never),
        ));

    let builder = typespace_builder!(settings, {
        struct Holder {
            r: Nullable<u32>,
        }
    });

    builder.finalize(no_cycles).unwrap();
}

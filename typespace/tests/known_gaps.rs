// Copyright 2026 Oxide Computer Company

//! Tests for defects typespace has not yet fixed.
//!
//! Every test here is `#[ignore]`d and asserts what SHOULD happen, so
//! it starts passing when the defect does. `cargo test -- --ignored`
//! runs the set. When one passes, move it to whichever file covers the
//! behavior and drop the attribute.

use typespace::{
    TypespaceBuilder, TypespaceTrait,
    build::{
        JsonValue, NewtypeConstraints, NewtypeStruct, Struct, StructProperty, StructPropertyState,
        Type,
    },
    error::Error,
    no_cycles,
    settings::{ContainerType, Settings},
};
use typespace_test_macro::typespace_builder;

mod common;

// The desired phase must not grant a trait through a container whose
// blanket demand landed on a native that never promised it.
//
// The graph, roughly:
//
//     struct Holder { m: HashMap<::ext::NativeK, u32> }
//
// where NativeK declares Eq but leaves Hash UNKNOWN, the x-rust-type
// contour. HashMap demands Eq + Hash of its key. The requirement-side
// policy lets an unknown pass, deliberately, so finalization succeeds.
// The desired phase then reads that pass as possession and grants Eq to
// the map and to Holder. The derive it emits needs
// HashMap<NativeK, u32>: Eq, which needs NativeK: Hash, which nobody
// promised: if the real NativeK has no Hash, the output does not
// compile.
//
// TraitProvision's own doc says an unknown must never grant a
// desired trait. The fix is for the desired phase to read a
// container's blanket obligations, not just the trait it is asked
// about; it waits on the typify merge.
#[test]
#[ignore = "deferred until after the typify merge"]
fn desired_eq_not_granted_over_unknown_hash_key() {
    let settings = Settings::minimal()
        .with_map_type(ContainerType::hash_map())
        .with_desired_trait(TypespaceTrait::Eq);
    let builder = typespace_builder!(settings, {
        native ::ext::NativeK: Clone + Debug + Serialize + Deserialize + Eq + PartialEq + ..;

        struct Holder {
            m: Map<::ext::NativeK, u32>,
        }
    });

    let ts = builder.finalize(no_cycles).expect("an unknown passes");
    let file = syn::parse2::<syn::File>(common::codespace(&ts).into_stream()).unwrap();
    let derives = common::derives_of(&file, "Holder");

    assert!(
        !derives.iter().any(|name| name == "Eq"),
        "Eq was granted through a hash map whose key never promised \
         Hash: {derives:?}"
    );
}

// A requirement that reaches a generic native stops there when the
// native's declaration cannot answer for the trait, so a generated
// type sitting in its parameter is never asked for an impl it would
// have supplied.
//
// Required resolution reads an `Unknown` provision optimistically:
// `container_split` puts it in neither the conflict bucket nor the
// pass-through bucket, so the requirement is satisfied at the native
// and goes no further. For a parameterless native that is the whole
// story. For `::ext::Wrapper<Inner>`, where the truth is almost always
// `Wrapper<T>: Hash where T: Hash`, it means `Inner` never derives
// `Hash` and the emitted `HashSet<Wrapper<Inner>>` does not compile.
//
// Reading an unanswered trait as `IfParameters` rather than `Always`
// would push the requirement to `Inner`, refuse nothing, and cost
// nothing where the native has no parameters. Note ahl's estimate,
// 2026-09-12: a generic native with an unanswered trait is expected
// "very infrequently".
#[test]
#[ignore]
fn an_unknown_trait_on_a_generic_native_reaches_its_parameters() {
    let builder = typespace_builder!(
        Settings::minimal().with_set_type(ContainerType::hash_set()),
        {
            native ::ext::Wrapper<Inner>: Clone + ..;

            struct Inner {
                a: u32,
            }

            struct Holder {
                keys: Set<::ext::Wrapper<Inner>>,
            }
        }
    );

    let typespace = builder
        .finalize(no_cycles)
        .expect("an unknown provision satisfies a requirement");

    assert!(
        typespace
            .get_type(&"Inner".to_string())
            .has_impl(TypespaceTrait::Hash),
        "the set demands Hash of its element, so Inner has to derive it \
         for HashSet<Wrapper<Inner>> to compile"
    );
}

fn flatten_settings() -> Settings {
    Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_desired_trait(TypespaceTrait::Default)
}

// A flattened property has no key of its own, so there is nothing for
// `null` to attach to: its type may not be an `Option`. Whether the inner
// keys may be missing is what `Optional` says. Today `flattens_as_object`
// looks through `Type::Option`, so this shape is accepted and the second
// `Option` is handed to `optional_nullable`.
#[test]
#[ignore = "finalize does not yet refuse an Option-typed flattened property"]
fn flattened_property_may_not_be_an_option() {
    let builder = typespace_builder!(flatten_settings(), {
        struct Inner {
            a: u32,
        }
        struct Outer {
            #[flatten]
            inner: Optional<Nullable<Inner>>,
        }
    });
    builder
        .finalize(no_cycles)
        .map(|_| ())
        .expect_err("a flattened property's type may not be an Option");

// A property default on a constrained newtype is constructed directly
// in the generated `defaults::` function, as `Holder`'s `code` default
// becomes `super::Code("xyz".to_string())`, without going through the
// newtype's `TryFrom`. So a value the constraints refuse is accepted at
// finalize and baked into the output, where it can never be produced by
// deserializing. This is typify issue 902, typespace issue 61.
//
// The graph, roughly:
//
//     struct Code(String)  // pattern ^[0-9]+$
//     struct Short(String) // at most 2 characters
//     struct Holder { #[default = "xyz"] code: Code,
//                     #[default = "toolong"] short: Short }
//
// The check belongs in the `NewtypeStruct` arm of default.rs's
// `default_impl`, which carries a TODO for it. A default that does not
// fit the base type already fails with `Error::InvalidDefault`; one that
// fits the base type and fails the constraints should too. The controls
// show that a default that satisfies the constraints still finalizes.
#[test]
#[ignore = "finalize does not check a property default against its newtype's constraints"]
fn default_violating_newtype_constraints_is_rejected() {
    let build = |constraints: NewtypeConstraints, default: &str| {
        let mut builder = TypespaceBuilder::new(Settings::minimal());
        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "Constrained".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("string".to_string())
                        .name("Constrained")
                        .constraints(constraints),
                ),
            )
            .unwrap();
        builder
            .insert(
                "Holder".to_string(),
                Struct::new()
                    .name("Holder")
                    .properties(vec![
                        StructProperty::new("value", "Constrained".to_string()).with_state(
                            StructPropertyState::DefaultValue(JsonValue(serde_json::json!(
                                default
                            ))),
                        ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };

    let pattern = || NewtypeConstraints::String {
        min: None,
        max: None,
        patterns: vec!["^[0-9]+$".to_string()],
    };
    let max_two = || NewtypeConstraints::String {
        min: None,
        max: Some(2),
        patterns: vec![],
    };

    // Controls: defaults that satisfy the constraints finalize.
    build(pattern(), "123")
        .finalize(no_cycles)
        .expect("\"123\" matches the pattern");
    build(max_two(), "ok")
        .finalize(no_cycles)
        .expect("\"ok\" fits in two characters");

    let failures = [(pattern(), "xyz"), (max_two(), "toolong")]
        .into_iter()
        .filter(|(constraints, default)| {
            !matches!(
                build(constraints.clone(), default).finalize(no_cycles),
                Err(Error::InvalidDefault { value, .. }) if value == serde_json::json!(default)
            )
        })
        .map(|(_, default)| default)
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "finalize accepted defaults that violate the constraints: {failures:?}"
    );
}

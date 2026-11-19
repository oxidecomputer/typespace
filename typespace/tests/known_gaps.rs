// Copyright 2026 Oxide Computer Company

//! Tests for defects typespace has not yet fixed.
//!
//! Every test here is `#[ignore]`d and asserts what SHOULD happen, so
//! it starts passing when the defect does. `cargo test -- --ignored`
//! runs the set. When one passes, move it to whichever file covers the
//! behavior and drop the attribute.

use typespace::{
    TypespaceTrait, no_cycles,
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

// Trait resolution accepts the graph: the `#[serde(default)]` seeding
// requires Default of the property's type, and the `Type::JsonValue`
// arm in `required_resolution` conflicts only over Ord and PartialOrd
// (`serde_json::Value` does implement Default; its default is `Null`).
// Rendering then calls `render_struct_property_add_skip` in `lib.rs`,
// whose JsonValue arm is `panic!("Default value for JsonValue is not
// supported")`.
//
// The panic message describes a different situation (a default value
// there is no way to compare against), and the correct output needs
// nothing exotic: emit `#[serde(default)]` with no
// `skip_serializing_if`, exactly as the integer and float arms do. So
// finalize accepts a graph that render refuses, and the refusal guards
// nothing.
#[test]
#[ignore = "independent and cheap; nobody is blocking on it"]
fn json_value_property_in_default_state_renders() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Debug)
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
        {
            struct Blob {
                #[default]
                data: JsonValue,
            }
        }
    );
    let ts = builder
        .finalize(no_cycles)
        .expect("Value implements Default; resolution accepts this graph");
    let file = syn::parse2::<syn::File>(common::codespace(&ts).into_stream())
        .expect("rendered output parses as a Rust file");

    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Struct(s) if s.ident == "Blob")),
        "Blob missing from output"
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
}

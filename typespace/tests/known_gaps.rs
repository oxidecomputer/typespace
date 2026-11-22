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

// An allow or deny list is a set of JSON documents, so membership is
// JSON equality: a wire value is on the list when it is one of the
// documents as written. The rendered check compares constructed values
// instead. `TryFrom<Inner>` builds each listed document into an `Inner`
// and tests whether the candidate equals one of them, and `Deserialize`
// reads an `Inner` first and then calls that `TryFrom`. Two documents
// that construct the same `Inner` are therefore the same to the
// rendered check. With
//
//     struct Inner { a: u32, b: u32 = 0 }
//     struct Listed(Inner) allowing [{"a": 1, "b": 0}]
//
// the generated code accepts the wire value `{"a": 1}`, which the list
// does not contain.
//
// Finalize already reads the list literally: `check_value` compares a
// default against the documents as written, so a default of `{"a": 1}`
// on `Listed` is refused even though the generated code would accept
// the same value at runtime. The rendered side has to become literal
// too: `TryFrom<Inner>` serializes the candidate with
// `serde_json::to_value` and compares documents, and `Deserialize`
// reads a `serde_json::Value`, checks it against the list, and only
// then converts it into an `Inner`. ahl, 2026-11-19: "we should treat
// the allow and deny list literally".
#[test]
#[ignore = "the rendered list check compares constructed values"]
fn list_membership_is_json_equality_in_rendered_code() {
    use serde_json::json;
    use typespace::TypespaceBuilder;
    use typespace::build::{
        JsonValue, NewtypeConstraints, NewtypeStruct, Struct, StructProperty, StructPropertyState,
        Type,
    };
    use typespace::error::Error;

    // `Holder.value` is a `Listed`, with the given default if any.
    fn build(
        default: Option<serde_json::Value>,
    ) -> Result<typespace::Typespace<String>, Error<String>> {
        let settings = Settings::minimal().with_required_trait(TypespaceTrait::Deserialize);
        let mut builder = TypespaceBuilder::new(settings);
        builder
            .insert("u32".to_string(), Type::Integer("u32".to_string()))
            .unwrap();
        builder
            .insert(
                "Inner".to_string(),
                Struct::new()
                    .name("Inner")
                    .properties(vec![
                        StructProperty::new("a", "u32".to_string())
                            .with_state(StructPropertyState::Required),
                        StructProperty::new("b", "u32".to_string())
                            .with_state(StructPropertyState::DefaultValue(JsonValue(json!(0)))),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                "Listed".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("Inner".to_string())
                        .name("Listed")
                        .constraints(NewtypeConstraints::AllowList(vec![JsonValue(
                            json!({"a": 1, "b": 0}),
                        )])),
                ),
            )
            .unwrap();
        let state = match default {
            Some(value) => StructPropertyState::DefaultValue(JsonValue(value)),
            None => StructPropertyState::Required,
        };
        builder
            .insert(
                "Holder".to_string(),
                Struct::new()
                    .name("Holder")
                    .properties(vec![
                        StructProperty::new("value", "Listed".to_string()).with_state(state),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder.finalize(no_cycles)
    }

    // The impl block that starts with `header`, through its closing brace.
    fn impl_block<'a>(rendered: &'a str, header: &str) -> &'a str {
        let start = rendered
            .find(header)
            .unwrap_or_else(|| panic!("no `{header}` in:\n{rendered}"));
        let end = rendered[start..]
            .find("\n}\n")
            .map_or(rendered.len(), |n| start + n + 2);
        &rendered[start..end]
    }

    // Finalize reads the list literally.
    let err = build(Some(json!({"a": 1})))
        .err()
        .expect("a default the list does not contain");
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");

    // The rendered impls do too.
    let ts = build(None).unwrap();
    let file = syn::parse2::<syn::File>(common::codespace(&ts).into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);

    let try_from = impl_block(&rendered, "impl ::std::convert::TryFrom<Inner> for Listed");
    assert!(
        try_from.contains("to_value"),
        "TryFrom compares constructed values, not documents:\n{try_from}"
    );
    let deserialize = impl_block(&rendered, "Deserialize<'de> for Listed");
    assert!(
        deserialize.contains("serde_json::Value"),
        "Deserialize reads an Inner before checking the list:\n{deserialize}"
    );
}

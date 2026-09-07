// Copyright 2026 Oxide Computer Company

//! Tests for defects typespace has and has not yet fixed.
//!
//! Every test here is `#[ignore]`d and asserts what SHOULD happen, so
//! it starts passing when the defect does. `cargo test -- --ignored`
//! runs the set. When one passes, move it to whichever file covers the
//! behavior and drop the attribute.

use typespace::{
    TypespaceBuilder, TypespaceTrait,
    build::{JsonValue, Native, NewtypeConstraints, NewtypeStruct, Struct, StructProperty, Type},
    no_cycles,
    settings::{ContainerType, Settings},
};
use typespace_test_macro::{check_and_include, typespace_builder};

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
// TraitDisposition's own doc says an unknown must never grant a
// desired trait. The fix is for the desired phase to read a
// container's blanket obligations, not just the trait it is asked
// about; it waits on the typify merge.
#[test]
#[ignore = "deferred until after the typify merge"]
fn desired_eq_not_granted_over_unknown_hash_key() {
    let native = Native::new(
        "::ext::NativeK",
        [
            TypespaceTrait::Clone,
            TypespaceTrait::Debug,
            TypespaceTrait::Serialize,
            TypespaceTrait::Deserialize,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]
        .into_iter()
        .collect(),
        Vec::new(),
    )
    .with_rest_unknown();

    let settings = Settings::minimal()
        .with_map_type(ContainerType::hash_map())
        .with_desired_trait(TypespaceTrait::Eq);
    let mut builder = TypespaceBuilder::new(settings);
    builder
        .insert("key".to_string(), Type::Native(native))
        .unwrap();
    builder
        .insert("value".to_string(), Type::Integer("u32".to_string()))
        .unwrap();
    builder
        .insert(
            "map".to_string(),
            Type::Map("key".to_string(), "value".to_string()),
        )
        .unwrap();
    builder
        .insert(
            "Holder".to_string(),
            Struct::new()
                .name("Holder")
                .properties(vec![StructProperty::new("m", "map".to_string())])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).expect("an unknown passes");
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let derives = common::derives_of(&file, "Holder");

    assert!(
        !derives.iter().any(|name| name == "Eq"),
        "Eq was granted through a hash map whose key never promised \
         Hash: {derives:?}"
    );
}

// Box forwards Display, and typespace does not claim it.
//
// The graph, roughly:
//
//     struct Wrapper(Box<String>);
//
// `impl<T: Display + ?Sized> Display for Box<T>` exists, so a Display
// desired of Wrapper can be granted: the newtype's generated impl
// prints its inner value, and the inner value is a Box<String>, which
// prints. Trait resolution treats Display and FromStr alike at every
// container, and Box is where the two part company: Box has no FromStr
// at any parameter, but it does have a Display.
//
// The answer is conservative rather than wrong -- a trait that could
// have been granted is dropped, and a required one is refused rather
// than emitted -- so this is a refinement, on the list with the rest of
// the fixed containers' per-trait behavior.
#[test]
#[ignore = "Box's per-trait forwarding is a pending refinement"]
fn box_provides_display() {
    let settings = Settings::minimal().with_desired_trait(TypespaceTrait::Display);
    let builder = typespace_test_macro::typespace_builder!(settings, {
        struct Wrapper(Box<String>);
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "Wrapper"),
        "Display was not granted through a Box, which forwards it"
    );
}

// `feasibility` documents the contract for a struct with an attached
// default value: "The hand-written impl takes each property the
// default value names from that value and fills the rest with
// Default::default()". `Struct::render` in `build/structs.rs` consults
// `common.default` only to decide whether the derive shortcut applies;
// the impl body it writes comes entirely from each property's
// `DefaultConstructor`, so the attached value's contents are
// discarded. The code compiles and misbehaves.
//
// The ignored test `test_default_whole_type_value_with_required_
// property` in `render.rs` pins the loud half of this gap (a required
// property's constructor is `DefaultConstructor::None`, which the impl
// body maps to `unreachable!()`). This test pins the quiet half: when
// every property has a constructor of its own the render succeeds, and
// the emitted `Default::default()` contradicts the attached value.
#[test]
#[ignore = "deferred until after the typify merge"]
fn whole_type_default_value_populates_default_impl() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Debug)
            .with_required_trait(TypespaceTrait::PartialEq)
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_desired_trait(TypespaceTrait::Default),
        {
            #[default = { b: 7, name: "bob" }]
            struct Config {
                #[default]
                b: u32,
                #[default]
                name: String,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/whole_type_default_value_populates_default_impl.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::Config::default(),
            import::Config {
                b: 7,
                name: "bob".to_string(),
            }
        );
    }
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
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream())
        .expect("rendered output parses as a Rust file");

    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Struct(s) if s.ident == "Blob")),
        "Blob missing from output"
    );
}

// An allow-list or deny-list newtype is granted FromStr and Display but
// gets neither impl.
//
// `feasibility` in `trait_resolution.rs` answers
// `ManuallyRealizable(vec![])` for both traits on any constrained
// newtype: a constrained newtype's FromStr parses the inner value and
// then validates it, so the inner type owes nothing. The allow/deny arm
// of `render_constraint_impl` in `build/structs.rs` then writes
// `traits.remove(FromStr).then(|| quote! {})` and the same for Display,
// consuming each trait and rendering an empty token stream for it.
//
// The result is silent: the trait never reaches `render_derives`, so
// nothing panics and the output still compiles. It turns into a compile
// error as soon as such a newtype is an untagged enum's payload, since
// the enum's forwarding impls call the payload's FromStr and Display.
//
// The fix is to write the two impls, the way the String arm of the same
// function already does.
#[test]
#[ignore = "the allow/deny arm of render_constraint_impl writes neither impl"]
fn allow_list_newtype_renders_from_str_and_display() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Display)
        .with_required_trait(TypespaceTrait::FromStr);
    let mut builder = TypespaceBuilder::new(settings);

    builder.insert("string".to_string(), Type::String).unwrap();
    builder
        .insert(
            "constrained string".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("string".to_string())
                    .name("ConstrainedString")
                    .constraints(NewtypeConstraints::AllowList(vec![
                        JsonValue(serde_json::json! { "tomax" }),
                        JsonValue(serde_json::json! { "xamot" }),
                    ])),
            ),
        )
        .unwrap();

    let ts = builder
        .finalize(no_cycles)
        .expect("a constrained newtype realizes both traits itself");
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "FromStr", "ConstrainedString"),
        "no FromStr impl for an allow-list newtype granted the trait"
    );
    assert!(
        common::has_impl(&file, "Display", "ConstrainedString"),
        "no Display impl for an allow-list newtype granted the trait"
    );
}

// Copyright 2026 Oxide Computer Company

//! Tests for defects typespace has and has not yet fixed.
//!
//! Every test here is `#[ignore]`d and asserts what SHOULD happen, so
//! it starts passing when the defect does. `cargo test -- --ignored`
//! runs the set. When one passes, move it to whichever file covers the
//! behavior and drop the attribute.

use typespace::{
    TypespaceBuilder, TypespaceTrait,
    build::{Native, Struct, StructProperty, Type},
    no_cycles,
    settings::{ContainerType, Settings},
};

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

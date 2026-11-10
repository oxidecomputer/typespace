// Copyright 2026 Oxide Computer Company

//! Compile generated tuple remainders and compare their schemas with Serde.
//! The whole-array controls show why suffix constraints cannot simply be
//! copied onto the enclosing array.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use typespace::build::{
    NewtypeConstraints, NewtypeStruct, Struct, StructProperty, TupleStruct, Type,
};
use typespace::settings::Settings;
use typespace::{Typespace, TypespaceBuilder, TypespaceTrait, no_cycles};
use typespace_test_macro::check_and_include;

mod common;

fn case_typespace() -> Typespace<&'static str> {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_required_trait(TypespaceTrait::JsonSchema);
    let mut builder = TypespaceBuilder::new(settings);
    for (id, typ) in [
        ("string", Type::String),
        ("strings", Type::Vec("string")),
        ("arrays", Type::Vec("strings")),
        ("bounded inner arrays", Type::Vec("bounded strings")),
        ("patterned strings", Type::Vec("patterned string")),
        ("nodes", Type::Vec("node")),
    ] {
        builder.insert(id, typ).unwrap();
    }

    for (id, name, inner) in [
        ("bounded strings", "BoundedStrings", "strings"),
        ("bounded arrays", "BoundedArrays", "arrays"),
    ] {
        builder
            .insert(
                id,
                NewtypeStruct::new(inner)
                    .name(name)
                    .constraints(NewtypeConstraints::Array {
                        min: Some(2),
                        max: Some(4),
                    })
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }

    for (id, name, inner, constraint) in [
        (
            "patterned string",
            "PatternedString",
            "string",
            json!({"pattern": "^a"}),
        ),
        (
            "contains strings",
            "ContainsStrings",
            "strings",
            json!({"contains": {"const": "needle"}}),
        ),
    ] {
        builder
            .insert(
                id,
                NewtypeStruct::new(inner)
                    .name(name)
                    .constraints(NewtypeConstraints::JsonSchema(constraint.into()))
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }

    builder
        .insert(
            "node",
            Struct::new()
                .name("Node")
                .properties([StructProperty::new("children", "nodes")])
                .deny_unknown_fields()
                .build()
                .unwrap(),
        )
        .unwrap();

    for (name, rest) in [
        ("PlainTail", "strings"),
        ("NestedTail", "arrays"),
        ("BoundedInnerTail", "bounded inner arrays"),
        ("BoundedOuterTail", "bounded arrays"),
        ("BoundedStringTail", "bounded strings"),
        ("PatternedTail", "patterned strings"),
        ("ContainsTail", "contains strings"),
        ("RecursiveTail", "nodes"),
    ] {
        builder
            .insert(
                name,
                TupleStruct::new()
                    .name(name)
                    .fields(["string"])
                    .rest(rest)
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }
    builder.finalize(no_cycles).unwrap()
}

enum CaseKind {
    Nested,
    InnerBounds,
    OuterBounds,
    WholeArrayUniqueness,
    WholeArrayContains,
    SuffixContains,
    StringBounds,
    ElementAllOf,
    Recursive,
}

fn check_schema<T: Serialize + DeserializeOwned>(
    id: &str,
    schema: schemars::schema::RootSchema,
    document: Value,
    schema_valid: bool,
    serde_valid: bool,
) {
    let decoded = serde_json::from_value::<T>(document.clone());
    assert_eq!(decoded.is_ok(), serde_valid, "{id}: Serde acceptance");
    if let Ok(value) = decoded {
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            document,
            "{id}: serialization must preserve the document"
        );
    }

    let schema = serde_json::to_value(schema).unwrap();
    jsonschema::draft7::meta::validate(&schema).expect("generated schema must be valid draft-07");
    let validator = jsonschema::draft7::new(&schema).expect("generated schema must compile");
    let result = validator.validate(&document);
    assert_eq!(
        result.is_ok(),
        schema_valid,
        "{id}: schema acceptance\nschema: {schema:#}\ndocument: {document}\nvalidation: {result:?}"
    );
}

fn check_case(id: &str, kind: CaseKind, document: Value, valid: bool) {
    let ts = case_typespace();

    #[check_and_include(
        "tests/output/tuple_remainder_schema_cases.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        match kind {
            CaseKind::Nested => check_schema::<import::NestedTail>(
                id,
                schemars::schema_for!(import::NestedTail),
                document,
                valid,
                valid,
            ),
            CaseKind::InnerBounds => check_schema::<import::BoundedInnerTail>(
                id,
                schemars::schema_for!(import::BoundedInnerTail),
                document,
                valid,
                valid,
            ),
            CaseKind::OuterBounds => check_schema::<import::BoundedOuterTail>(
                id,
                schemars::schema_for!(import::BoundedOuterTail),
                document,
                valid,
                valid,
            ),
            CaseKind::StringBounds => check_schema::<import::BoundedStringTail>(
                id,
                schemars::schema_for!(import::BoundedStringTail),
                document,
                valid,
                valid,
            ),
            CaseKind::ElementAllOf => check_schema::<import::PatternedTail>(
                id,
                schemars::schema_for!(import::PatternedTail),
                document,
                valid,
                valid,
            ),
            CaseKind::SuffixContains => check_schema::<import::ContainsTail>(
                id,
                schemars::schema_for!(import::ContainsTail),
                document,
                valid,
                valid,
            ),
            CaseKind::Recursive => check_schema::<import::RecursiveTail>(
                id,
                schemars::schema_for!(import::RecursiveTail),
                document,
                valid,
                valid,
            ),
            CaseKind::WholeArrayUniqueness => {
                // A suffix's uniqueness requirement must not compare its
                // elements with the prefix. This deliberately wrong placement
                // rejects a document that PlainTail can round-trip.
                let mut schema = schemars::schema_for!(import::PlainTail);
                schema.schema.array().unique_items = Some(true);
                check_schema::<import::PlainTail>(id, schema, document, valid, true);
            }
            CaseKind::WholeArrayContains => {
                // With contains on the outer array, the prefix can satisfy it
                // even though the generated suffix deserializer rejects it.
                let mut schema = schemars::schema_for!(import::PlainTail);
                schema.schema.array().contains = Some(Box::new(
                    serde_json::from_value(json!({"const": "needle"})).unwrap(),
                ));
                check_schema::<import::ContainsTail>(id, schema, document, valid, false);
            }
        }
    }
}

macro_rules! schema_cases {
    ($($kind:ident { $($id:ident: $document:tt => $valid:literal,)+ })+) => {
        $( $(
            #[test]
            fn $id() {
                check_case(stringify!($id), CaseKind::$kind, json!($document), $valid);
            }
        )+ )+
    };
}

schema_cases! {
    Nested {
        nested_tail_may_be_empty: ["head"] => true,
        nested_tail_accepts_empty_inner_array: ["head", []] => true,
        nested_tail_accepts_string_arrays: ["head", ["a"], ["b"]] => true,
        nested_tail_rejects_flat_strings: ["head", "a"] => false,
        nested_tail_rejects_an_extra_array_level: ["head", [["a"]]] => false,
    }
    InnerBounds {
        inner_bounds_allow_empty_outer_tail: ["head"] => true,
        inner_bounds_accept_two_strings: ["head", ["a", "b"]] => true,
        inner_bounds_reject_single_string_arrays: ["head", ["a"], ["b"]] => false,
        inner_bounds_accept_four_strings: ["head", ["a", "b", "c", "d"]] => true,
        inner_bounds_reject_five_strings: ["head", ["a", "b", "c", "d", "e"]] => false,
    }
    OuterBounds {
        outer_bounds_reject_empty_tail: ["head"] => false,
        outer_bounds_reject_one_array_even_with_two_strings: ["head", ["a", "b"]] => false,
        outer_bounds_accept_two_single_string_arrays: ["head", ["a"], ["b"]] => true,
        outer_bounds_accept_four_empty_arrays: ["head", [], [], [], []] => true,
        outer_bounds_reject_five_empty_arrays: ["head", [], [], [], [], []] => false,
    }
    WholeArrayUniqueness {
        whole_array_uniqueness_rejects_prefix_collision: ["a", "a"] => false,
        whole_array_uniqueness_accepts_distinct_prefix: ["head", "a"] => true,
    }
    WholeArrayContains {
        whole_array_contains_can_be_satisfied_by_prefix: ["needle", "other"] => true,
    }
    SuffixContains {
        suffix_contains_ignores_a_match_in_prefix: ["needle", "other"] => false,
        suffix_contains_rejects_empty_tail_even_if_prefix_matches: ["needle"] => false,
        suffix_contains_rejects_tail_without_match: ["head", "other"] => false,
        suffix_contains_accepts_first_tail_match: ["head", "needle"] => true,
        suffix_contains_accepts_later_tail_match: ["head", "other", "needle"] => true,
    }
    StringBounds {
        string_tail_bounds_reject_one_element: ["head", "a"] => false,
        string_tail_bounds_accept_two_elements: ["head", "a", "b"] => true,
        string_tail_bounds_accept_four_elements: ["head", "a", "b", "c", "d"] => true,
        string_tail_bounds_reject_five_elements: ["head", "a", "b", "c", "d", "e"] => false,
    }
    ElementAllOf {
        element_all_of_accepts_matching_strings: ["head", "a", "aa"] => true,
        element_all_of_rejects_nonmatching_string: ["head", "b"] => false,
    }
    Recursive {
        recursive_items_accept_leaf: ["head", {"children": []}] => true,
        recursive_items_accept_nested_node: ["head", {"children": [{"children": []}]}] => true,
        recursive_items_reject_non_node_child: ["head", {"children": [0]}] => false,
    }
}

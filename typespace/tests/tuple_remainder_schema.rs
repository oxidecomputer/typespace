// Copyright 2026 Oxide Computer Company

use typespace::settings::{ContainerType, GeneratedCrate, Settings};
use typespace::{TraitProvision, TypespaceTrait, TypespaceTraitSet, no_cycles};
use typespace_test_macro::{check_and_include, typespace_builder};

mod common;

// Exercise generated paths through a facade as well as the default crate name.
pub mod schema_facade {
    pub use schemars::*;
}

#[test]
fn standard_vec_remainder_schemas() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_crate_path(GeneratedCrate::Schemars, "super::schema_facade"),
        {
            native serde_json::Value: Serialize + Deserialize + JsonSchema;
            struct Nested(u32, #[flatten] Vec<Vec<String>>);
            struct Referenced(u32, #[flatten] Vec<Element>);
            struct Element {
                text: String,
            }
            struct Unconstrained(u32, #[flatten] Vec<serde_json::Value>);
            struct Ordinary(Vec<String>);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/standard_vec_remainder_schemas.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        let schema = serde_json::to_value(schemars::schema_for!(import::Nested)).unwrap();
        let validator = jsonschema::draft7::new(&schema).unwrap();
        let value = import::Nested(42, vec![vec!["a".to_string()], vec![]]);
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json, serde_json::json!([42, ["a"], []]));
        assert!(validator.is_valid(&json));
        let round_trip: import::Nested = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.1, value.1);
        assert!(validator.is_valid(&serde_json::json!([42])));
        for invalid in [
            serde_json::json!([42, "a"]),
            serde_json::json!([42, [["a"]]]),
        ] {
            assert!(!validator.is_valid(&invalid), "{invalid}");
            assert!(serde_json::from_value::<import::Nested>(invalid).is_err());
        }

        let schema = serde_json::to_value(schemars::schema_for!(import::Referenced)).unwrap();
        assert_eq!(schema["additionalItems"]["$ref"], "#/definitions/Element");
        let validator = jsonschema::draft7::new(&schema).unwrap();
        let value = import::Referenced(
            42,
            vec![import::Element {
                text: "a".to_string(),
            }],
        );
        let json = serde_json::to_value(value).unwrap();
        assert!(validator.is_valid(&json));
        let round_trip: import::Referenced = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.1[0].text, "a");
        assert!(!validator.is_valid(&serde_json::json!([42, { "text": 1 }])));
        assert!(!validator.is_valid(&serde_json::json!([42, [{ "text": "a" }]])));

        let schema = serde_json::to_value(schemars::schema_for!(import::Unconstrained)).unwrap();
        let validator = jsonschema::draft7::new(&schema).unwrap();
        let tail = vec![
            serde_json::json!(null),
            serde_json::json!(1),
            serde_json::json!({}),
        ];
        let value = import::Unconstrained(42, tail.clone());
        let json = serde_json::to_value(value).unwrap();
        assert!(validator.is_valid(&json));
        let round_trip: import::Unconstrained = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.1, tail);

        // A vector outside a remainder keeps its container schema.
        let schema = serde_json::to_value(schemars::schema_for!(import::Ordinary)).unwrap();
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["items"], serde_json::json!({ "type": "string" }));
    }
}

// This element deliberately has no JsonSchema implementation. The container's
// unconditional implementation must remain sufficient for the generated type.
pub struct NoSchema;
pub struct OpaqueVec<T>(std::marker::PhantomData<T>);

impl<T> schemars::JsonSchema for OpaqueVec<T> {
    fn schema_name() -> String {
        "OpaqueVec".to_string()
    }

    fn is_referenceable() -> bool {
        false
    }

    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        serde_json::from_value(serde_json::json!({"type": "array", "minItems": 3})).unwrap()
    }
}

#[test]
fn custom_vec_remainder_keeps_container_schema() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_vec_type(
                ContainerType::new("super::OpaqueVec", [TypespaceTraitSet::empty()])
                    .with_provision(TypespaceTrait::JsonSchema, TraitProvision::Always)
            ),
        {
            native super::NoSchema: !JsonSchema;
            struct Custom(u32, #[flatten] Vec<super::NoSchema>);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/custom_vec_remainder_keeps_container_schema.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        let schema = serde_json::to_value(schemars::schema_for!(import::Custom)).unwrap();
        assert_eq!(
            schema["additionalItems"],
            serde_json::json!({"type": "array", "minItems": 3})
        );
    }
}

#[test]
fn indirect_vec_remainders_keep_existing_schema() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::JsonSchema),
        {
            type Tail = Vec<String>;
            struct Wrapped(Vec<String>);
            struct Aliased(u32, #[flatten] Tail);
            struct Boxed(u32, #[flatten] Box<Vec<String>>);
            struct Newtype(u32, #[flatten] Wrapped);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/indirect_vec_remainders_keep_existing_schema.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        for schema in [
            schemars::schema_for!(import::Aliased),
            schemars::schema_for!(import::Boxed),
            schemars::schema_for!(import::Newtype),
        ] {
            let schema = serde_json::to_value(schema).unwrap();
            // Indirect remainder shapes are deliberately outside this fix.
            let validator = jsonschema::draft7::new(&schema).unwrap();
            assert!(validator.is_valid(&serde_json::json!([42, ["a"]])));
            assert!(!validator.is_valid(&serde_json::json!([42, "a"])));
        }
    }
}

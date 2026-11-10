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
pub struct OpaqueVec<T>(Vec<String>, std::marker::PhantomData<T>);

impl<T> serde::Serialize for OpaqueVec<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for OpaqueVec<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let items = Vec::<String>::deserialize(deserializer)?;
        if items.len() < 3 {
            return Err(serde::de::Error::custom("expected at least three strings"));
        }
        Ok(Self(items, std::marker::PhantomData))
    }
}

impl<T> schemars::JsonSchema for OpaqueVec<T> {
    fn schema_name() -> String {
        "OpaqueVec".to_string()
    }

    fn is_referenceable() -> bool {
        false
    }

    fn json_schema(g: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = <Vec<String> as schemars::JsonSchema>::json_schema(g).into_object();
        schema.array().min_items = Some(3);
        schema.into()
    }
}

#[test]
fn custom_vec_remainder_schema() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_vec_type(
                ContainerType::new("super::OpaqueVec", [TypespaceTraitSet::empty()])
                    .with_provision(TypespaceTrait::JsonSchema, TraitProvision::Always)
                    .with_provision(TypespaceTrait::Serialize, TraitProvision::Always)
                    .with_provision(TypespaceTrait::Deserialize, TraitProvision::Always)
            ),
        {
            native super::NoSchema: !JsonSchema + !Serialize + !Deserialize;
            struct Custom(u32, #[flatten] Vec<super::NoSchema>);
            struct NestedCustom(u32, #[flatten] Vec<Vec<super::NoSchema>>);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/custom_vec_remainder_schema.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        check(
            import::Custom(
                42,
                OpaqueVec(
                    vec!["a".into(), "b".into(), "c".into()],
                    std::marker::PhantomData,
                ),
            ),
            serde_json::json!([42, "a", "b", "c"]),
            &[
                serde_json::json!([42]),
                serde_json::json!([42, "a", "b"]),
                serde_json::json!([42, ["a"], ["b"], ["c"]]),
                serde_json::json!([42, "a", "b", 1]),
            ],
        );
        // The outer container's representation is independent of its Rust
        // parameter, even when that parameter is another configured container.
        check(
            import::NestedCustom(
                42,
                OpaqueVec(
                    vec!["a".into(), "b".into(), "c".into()],
                    std::marker::PhantomData,
                ),
            ),
            serde_json::json!([42, "a", "b", "c"]),
            &[serde_json::json!([42, ["a"], ["b"], ["c"]])],
        );
    }
}

#[test]
fn indirect_vec_remainder_schemas() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
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
        "tests/output/indirect_vec_remainder_schemas.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        for schema in [
            schemars::schema_for!(import::Aliased),
            schemars::schema_for!(import::Boxed),
            schemars::schema_for!(import::Newtype),
        ] {
            let schema = serde_json::to_value(schema).unwrap();
            let validator = jsonschema::draft7::new(&schema).unwrap();
            assert!(validator.is_valid(&serde_json::json!([42, "a"])));
            assert!(!validator.is_valid(&serde_json::json!([42, ["a"]])));
        }
        let invalid = [serde_json::json!([42, ["a"]]), serde_json::json!([42, 1])];
        check(
            import::Aliased(42, vec!["a".into()]),
            serde_json::json!([42, "a"]),
            &invalid,
        );
        check(
            import::Boxed(42, Box::new(vec!["a".into()])),
            serde_json::json!([42, "a"]),
            &invalid,
        );
        check(
            import::Newtype(42, import::Wrapped(vec!["a".into()])),
            serde_json::json!([42, "a"]),
            &invalid,
        );
    }
}

fn check<T>(value: T, expected: serde_json::Value, invalid: &[serde_json::Value])
where
    T: serde::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema,
{
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    jsonschema::draft7::meta::validate(&schema).unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let json = serde_json::to_value(value).unwrap();
    assert_eq!(json, expected);
    assert!(validator.is_valid(&json), "{schema:#}\nrejects {json}");
    let round_trip: T = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(round_trip).unwrap(), json);
    for json in invalid {
        assert!(!validator.is_valid(json), "{schema:#}\naccepts {json}");
        assert!(serde_json::from_value::<T>(json.clone()).is_err(), "{json}");
    }
}

#[test]
fn structural_remainder_schemas() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::JsonSchema),
        {
            native super::NoSchema: ..;
            struct Fixed(u32, #[flatten] [String; 2]);
            struct Empty(u32, #[flatten] [super::NoSchema; 0]);
            struct Tuple(u32, #[flatten] (String, bool));
            struct Inner(String, #[flatten] Vec<bool>);
            struct Outer(u32, #[flatten] Inner);
            struct Closed(String, bool);
            struct OuterClosed(u32, #[flatten] Closed);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/structural_remainder_schemas.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        // json-serde 1.0's adapters reject the tuple operations used by Rust
        // arrays and tuples. Verify their schema layouts without asserting a
        // Serde round-trip that the existing adapter cannot perform.
        check_schema::<import::Fixed>(
            &[serde_json::json!([42, "a", "b"])],
            &[
                serde_json::json!([42, "a"]),
                serde_json::json!([42, "a", "b", "c"]),
                serde_json::json!([42, ["a", "b"]]),
                serde_json::json!([42, "a", false]),
            ],
        );
        check_schema::<import::Empty>(&[serde_json::json!([42])], &[serde_json::json!([42, "a"])]);
        check_schema::<import::Tuple>(
            &[serde_json::json!([42, "a", true])],
            &[
                serde_json::json!([42, true, "a"]),
                serde_json::json!([42, "a"]),
                serde_json::json!([42, "a", true, false]),
            ],
        );
        check(
            import::Outer(42, import::Inner("a".into(), vec![true, false])),
            serde_json::json!([42, "a", true, false]),
            &[
                serde_json::json!([42]),
                serde_json::json!([42, "a", "b"]),
                serde_json::json!([42, "a", [true, false]]),
            ],
        );
        check(
            import::Outer(42, import::Inner("a".into(), vec![])),
            serde_json::json!([42, "a"]),
            &[],
        );
        check(
            import::OuterClosed(42, import::Closed("a".into(), true)),
            serde_json::json!([42, "a", true]),
            &[
                serde_json::json!([42, "a"]),
                serde_json::json!([42, "a", true, false]),
                serde_json::json!([42, true, "a"]),
            ],
        );
    }
}

fn check_schema<T: schemars::JsonSchema>(
    valid: &[serde_json::Value],
    invalid: &[serde_json::Value],
) {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    jsonschema::draft7::meta::validate(&schema).unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for value in valid {
        assert!(validator.is_valid(value), "{schema:#}\nrejects {value}");
    }
    for value in invalid {
        assert!(!validator.is_valid(value), "{schema:#}\naccepts {value}");
    }
}

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct TransparentVec<T>(Vec<T>);

// Exercise a native schema that delegates through a definition reference.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct NativeTail(Vec<String>);

impl schemars::JsonSchema for NativeTail {
    fn schema_name() -> String {
        "NativeTail".into()
    }

    fn json_schema(g: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        g.subschema_for::<TransparentVec<String>>()
    }
}

#[test]
fn custom_and_recursive_remainder_schemas() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_crate_path(GeneratedCrate::Schemars, "super::schema_facade")
            .with_vec_type(ContainerType::vec().with_path("super::TransparentVec"))
            .with_set_type(ContainerType::btree_set()),
        {
            native super::NativeTail: Serialize + Deserialize + JsonSchema;
            struct Custom(u32, #[flatten] Vec<Vec<String>>);
            struct Native(u32, #[flatten] super::NativeTail);
            struct SetTail(String, #[flatten] Set<String>);
            #[deny_unknown_fields]
            struct Node {
                children: Vec<Node>,
            }
            struct Recursive(u32, #[flatten] Vec<Node>);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/custom_and_recursive_remainder_schemas.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        check(
            import::Custom(42, TransparentVec(vec![TransparentVec(vec!["a".into()])])),
            serde_json::json!([42, ["a"]]),
            &[
                serde_json::json!([42, "a"]),
                serde_json::json!([42, [["a"]]]),
            ],
        );
        check(
            import::Native(42, NativeTail(vec!["a".into()])),
            serde_json::json!([42, "a"]),
            &[serde_json::json!([42, ["a"]]), serde_json::json!([42, 1])],
        );
        // The prefix is not part of the set's uniqueness requirement.
        check(
            import::SetTail("a".into(), ["a".to_string()].into_iter().collect()),
            serde_json::json!(["a", "a"]),
            &[serde_json::json!(["a", ["a"]]), serde_json::json!(["a", 1])],
        );
        check(
            import::Recursive(
                42,
                TransparentVec(vec![import::Node {
                    children: TransparentVec(vec![import::Node {
                        children: TransparentVec(vec![]),
                    }]),
                }]),
            ),
            serde_json::json!([42, {"children": [{"children": []}]}]),
            &[
                serde_json::json!([42, {"children": [0]}]),
                serde_json::json!([42, [{"children": []}]]),
            ],
        );
    }
}

#[test]
fn composed_remainder_constraints() {
    use typespace::build::{NewtypeConstraints, NewtypeStruct, TupleStruct, Type};
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_required_trait(TypespaceTrait::JsonSchema);
    let mut builder = typespace::TypespaceBuilder::new(settings);
    builder.insert("string", Type::String).unwrap();
    builder.insert("strings", Type::Vec("string")).unwrap();
    for (id, name, constraint) in [
        (
            "composed",
            "Composed",
            serde_json::json!({
                "allOf": [
                    {"anyOf": [{"const": 0}, {"const": []}, {"enum": [["a"], ["a", "b"]]}]},
                    {"not": {"contains": {"const": "needle"}}},
                    {"if": {"minItems": 1}, "then": {"maxItems": 2}, "else": {"maxItems": 0}},
                    {"oneOf": [{"maxItems": 0}, {"minItems": 1}]}
                ]
            }),
        ),
        (
            "duplicates",
            "Duplicates",
            serde_json::json!({
                "allOf": [
                    {"not": {"uniqueItems": true}},
                    {"if": {"uniqueItems": true}, "then": false, "else": {"minItems": 2}},
                    {"oneOf": [{"uniqueItems": true}, {"minItems": 2}]}
                ]
            }),
        ),
    ] {
        builder
            .insert(
                id,
                NewtypeStruct::new("strings")
                    .name(name)
                    .constraints(NewtypeConstraints::JsonSchema(constraint.into()))
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }
    for (name, rest) in [
        ("ComposedTail", "composed"),
        ("DuplicateTail", "duplicates"),
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
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/composed_remainder_constraints.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        for document in [
            serde_json::json!(["head"]),
            serde_json::json!(["needle", "a"]),
            serde_json::json!(["head", "a", "b"]),
        ] {
            let value = serde_json::from_value::<import::ComposedTail>(document.clone()).unwrap();
            check(
                value,
                document,
                &[
                    serde_json::json!(["head", "b"]),
                    serde_json::json!(["head", "a", "c"]),
                    serde_json::json!(["head", "needle"]),
                    serde_json::json!(["head", "a", "b", "c"]),
                ],
            );
        }
        // uniqueItems cannot be copied onto the enclosing array. Its omission
        // must also remain conservative under not, oneOf and if/then/else.
        let document = serde_json::json!(["head", "a", "a"]);
        let value = serde_json::from_value::<import::DuplicateTail>(document.clone()).unwrap();
        check(value, document, &[serde_json::json!(["head", 1, 1])]);
    }
}

#[test]
fn cyclic_remainder_layout_terminates() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::JsonSchema),
        {
            struct Cycle(Box<OtherCycle>);
            struct OtherCycle(Box<Cycle>);
            struct Cyclic(u32, #[flatten] Cycle);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/cyclic_remainder_layout_terminates.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        let schema = serde_json::to_value(schemars::schema_for!(import::Cyclic)).unwrap();
        jsonschema::draft7::meta::validate(&schema).unwrap();
        assert_eq!(schema["minItems"], 1);
    }
}

#[test]
fn other_array_representations() {
    use typespace::build::{TupleStruct, Type};
    let mut builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::JsonSchema),
        {
            #[json = ["a", "b"]]
            struct Literal;
            struct LiteralTail(u32, #[flatten] Literal);
            #[untagged]
            enum Choice {
                Strings(Vec<String>),
                Bools(Vec<bool>),
                Text(String),
            }
            struct ChoiceTail(u32, #[flatten] Choice);
            struct NestedChoice(String, #[flatten] Choice);
            struct NestedChoiceTail(u32, #[flatten] NestedChoice);
            struct OptionalTail(u32, #[flatten] Nullable<Vec<String>>);
        }
    );
    builder.insert("any".to_string(), Type::JsonValue).unwrap();
    builder
        .insert("prefix".to_string(), Type::Integer("u32".into()))
        .unwrap();
    builder
        .insert(
            "any tail".to_string(),
            TupleStruct::new()
                .name("AnyTail")
                .fields(["prefix".to_string()])
                .rest("any".to_string())
                .build()
                .unwrap(),
        )
        .unwrap();
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/other_array_representations.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        // These values serialize as sequences, but json-serde 1.0 cannot
        // deserialize the corresponding deserialize_any/option operations.
        let literal = serde_json::to_value(import::LiteralTail(42, import::Literal)).unwrap();
        assert_eq!(literal, serde_json::json!([42, "a", "b"]));
        check_schema::<import::LiteralTail>(
            &[literal],
            &[
                serde_json::json!([42, "a"]),
                serde_json::json!([42, "b", "a"]),
            ],
        );
        let choices = [
            serde_json::to_value(import::ChoiceTail(
                42,
                import::Choice::Strings(vec!["a".into()]),
            ))
            .unwrap(),
            serde_json::to_value(import::ChoiceTail(
                42,
                import::Choice::Bools(vec![true, false]),
            ))
            .unwrap(),
        ];
        assert_eq!(
            choices,
            [
                serde_json::json!([42, "a"]),
                serde_json::json!([42, true, false])
            ]
        );
        check_schema::<import::ChoiceTail>(&choices, &[serde_json::json!([42, "a", true])]);
        // The enum's length is unknown to array_bounds. The fixed fields
        // of its enclosing tuple structs must still be required.
        let nested = serde_json::to_value(import::NestedChoiceTail(
            42,
            import::NestedChoice("inner".into(), import::Choice::Strings(Vec::new())),
        ))
        .unwrap();
        assert_eq!(nested, serde_json::json!([42, "inner"]));
        check_schema::<import::NestedChoiceTail>(&[nested], &[serde_json::json!([42])]);
        // The adapter also rejects serialize_some. Check the schema separately
        // and retain evidence of this existing runtime limitation.
        assert!(serde_json::to_value(import::OptionalTail(42, Some(vec!["a".into()]))).is_err());
        check_schema::<import::OptionalTail>(
            &[serde_json::json!([42, "a"])],
            &[serde_json::json!([42, ["a"]])],
        );
        let any = serde_json::to_value(import::AnyTail(
            42,
            serde_json::json!(["a", {"nested": [1]}]),
        ))
        .unwrap();
        assert_eq!(any, serde_json::json!([42, "a", {"nested": [1]}]));
        check_schema::<import::AnyTail>(&[any], &[serde_json::json!(["wrong prefix"])]);
    }
}

// The schemas in these fixtures exercise the generated reference resolver.
// Each available target is registered before json_schema returns; item-level
// recursion deliberately stays in the schema instead of being expanded.
macro_rules! schema_source {
    ($name:ident, $g:ident, $schema:expr) => {
        pub struct $name;

        impl schemars::JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).into()
            }

            fn json_schema($g: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
                $schema
            }
        }
    };
}

schema_source!(InlineArraySchema, g, {
    let mut schema = <Vec<String> as schemars::JsonSchema>::json_schema(g).into_object();
    schema.array().min_items = Some(2);
    schema.array().max_items = Some(4);
    schema.into()
});
schema_source!(
    ChainedArraySchema,
    g,
    g.subschema_for::<InlineArraySchema>()
);
schema_source!(
    DoubleChainedArraySchema,
    g,
    g.subschema_for::<ChainedArraySchema>()
);
schema_source!(SharedArraySchema, g, {
    let item = g.subschema_for::<InlineArraySchema>();
    let mut schema = schemars::schema::SchemaObject::default();
    schema.subschemas().all_of = Some(vec![item.clone(), item]);
    schema.into()
});
schema_source!(ForwardArraySchema, g, {
    // Creating the reference before its target is fine: the target is ready
    // when this method returns and Typespace begins translating the schema.
    let reference = schemars::schema::Schema::new_ref(format!(
        "{}ForwardTarget",
        g.settings().definitions_path,
    ));
    let target = <InlineArraySchema as schemars::JsonSchema>::json_schema(g);
    g.definitions_mut().insert("ForwardTarget".into(), target);
    reference
});
schema_source!(
    MissingArraySchema,
    g,
    schemars::schema::Schema::new_ref(format!("{}MissingTarget", g.settings().definitions_path,))
);
schema_source!(
    ExternalArraySchema,
    _g,
    schemars::schema::Schema::new_ref("https://example.invalid/array.json".into(),)
);
schema_source!(CyclicArraySchema, g, {
    let path = &g.settings().definitions_path;
    let a = schemars::schema::Schema::new_ref(format!("{path}CycleA"));
    let b = schemars::schema::Schema::new_ref(format!("{path}CycleB"));
    g.definitions_mut().insert("CycleA".into(), b);
    g.definitions_mut().insert("CycleB".into(), a.clone());
    a
});
schema_source!(FalseArraySchema, _g, schemars::schema::Schema::Bool(false));
schema_source!(
    ReferencedFalseArraySchema,
    g,
    g.subschema_for::<FalseArraySchema>()
);
schema_source!(TrueArraySchema, _g, schemars::schema::Schema::Bool(true));
schema_source!(
    ReferencedTrueArraySchema,
    g,
    g.subschema_for::<TrueArraySchema>()
);
schema_source!(NegatedMissingArraySchema, g, {
    let missing = <MissingArraySchema as schemars::JsonSchema>::json_schema(g);
    let mut schema = schemars::schema::SchemaObject::default();
    schema.subschemas().not = Some(Box::new(missing));
    schema.into()
});
schema_source!(NestedPointerArraySchema, g, {
    let target = <InlineArraySchema as schemars::JsonSchema>::json_schema(g);
    let mut holder = schemars::schema::SchemaObject::default();
    holder.subschemas().all_of = Some(vec![target]);
    g.definitions_mut()
        .insert("PointerHolder".into(), holder.into());
    schemars::schema::Schema::new_ref(format!(
        "{}PointerHolder/allOf/0",
        g.settings().definitions_path,
    ))
});
schema_source!(RecursiveArraySchema, g, {
    <Vec<Self> as schemars::JsonSchema>::json_schema(g)
});

pub struct PendingSchemaHost<T>(std::marker::PhantomData<T>);
pub struct PendingSchemaReference<T>(std::marker::PhantomData<T>);

impl<T: schemars::JsonSchema> schemars::JsonSchema for PendingSchemaHost<T> {
    fn schema_name() -> String {
        format!("PendingSchemaHost_{}", T::schema_name())
    }

    fn json_schema(g: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(g)
    }
}

impl<T: schemars::JsonSchema> schemars::JsonSchema for PendingSchemaReference<T> {
    fn schema_name() -> String {
        format!("PendingSchemaReference_{}", T::schema_name())
    }

    fn json_schema(g: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        g.subschema_for::<PendingSchemaHost<T>>()
    }
}

fn assert_schema_panics<T: schemars::JsonSchema>(name: &str, reason: &str) {
    let error = std::panic::catch_unwind(|| schemars::schema_for!(T))
        .expect_err("an unsupported sequence schema must fail explicitly");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .expect("schema generation panic must explain the unsupported reference");
    assert!(
        message.contains(&format!("cannot generate schema for `{name}`")),
        "{message}"
    );
    assert!(message.contains(reason), "{message}");
}

#[test]
fn opaque_remainder_reference_resolution() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::JsonSchema)
            .with_crate_path(GeneratedCrate::Schemars, "super::schema_facade"),
        {
            native super::InlineArraySchema: JsonSchema;
            native super::DoubleChainedArraySchema: JsonSchema;
            native super::SharedArraySchema: JsonSchema;
            native super::ForwardArraySchema: JsonSchema;
            native super::MissingArraySchema: JsonSchema;
            native super::ExternalArraySchema: JsonSchema;
            native super::CyclicArraySchema: JsonSchema;
            native super::FalseArraySchema: JsonSchema;
            native super::ReferencedFalseArraySchema: JsonSchema;
            native super::ReferencedTrueArraySchema: JsonSchema;
            native super::NegatedMissingArraySchema: JsonSchema;
            native super::NestedPointerArraySchema: JsonSchema;
            native super::RecursiveArraySchema: JsonSchema;
            native super::PendingSchemaReference<Pending>: JsonSchema;
            struct Inline(u32, #[flatten] super::InlineArraySchema);
            struct Chained(u32, #[flatten] super::DoubleChainedArraySchema);
            struct Shared(u32, #[flatten] super::SharedArraySchema);
            struct Forward(u32, #[flatten] super::ForwardArraySchema);
            struct Missing(u32, #[flatten] super::MissingArraySchema);
            struct External(u32, #[flatten] super::ExternalArraySchema);
            struct CyclicReference(u32, #[flatten] super::CyclicArraySchema);
            struct InlineFalse(u32, #[flatten] super::FalseArraySchema);
            struct ReferencedFalse(u32, #[flatten] super::ReferencedFalseArraySchema);
            struct ReferencedTrue(u32, #[flatten] super::ReferencedTrueArraySchema);
            struct NegatedMissing(u32, #[flatten] super::NegatedMissingArraySchema);
            struct NestedPointer(u32, #[flatten] super::NestedPointerArraySchema);
            struct RecursiveItems(u32, #[flatten] super::RecursiveArraySchema);
            struct Pending(u32, #[flatten] super::PendingSchemaReference<Pending>);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/opaque_remainder_reference_resolution.rs",
        common::codespace(&ts).into_stream()
    )]
    fn inner() {
        let valid = [
            serde_json::json!([42, "a", "b"]),
            serde_json::json!([42, "a", "b", "c", "d"]),
        ];
        let invalid = [
            serde_json::json!([42]),
            serde_json::json!([42, "a"]),
            serde_json::json!([42, "a", "b", "c", "d", "e"]),
            serde_json::json!([42, ["a", "b"]]),
        ];
        check_schema::<import::Inline>(&valid, &invalid);
        check_schema::<import::Chained>(&valid, &invalid);
        check_schema::<import::Shared>(&valid, &invalid);
        check_schema::<import::Forward>(&valid, &invalid);
        check_schema::<import::InlineFalse>(
            &[],
            &[serde_json::json!([42]), serde_json::json!([42, "a"])],
        );
        check_schema::<import::ReferencedTrue>(
            &[
                serde_json::json!([42]),
                serde_json::json!([42, null, [], {}]),
            ],
            &[serde_json::json!(["wrong prefix"])],
        );
        check_schema::<import::RecursiveItems>(
            &[
                serde_json::json!([42]),
                serde_json::json!([42, []]),
                serde_json::json!([42, [[]]]),
            ],
            &[serde_json::json!([42, "a"]), serde_json::json!([42, ["a"]])],
        );

        // The resolution contract follows the generator's configured namespace.
        let mut settings = schemars::r#gen::SchemaSettings::draft07();
        settings.definitions_path = "#/components/schemas/".into();
        let root = settings
            .into_generator()
            .root_schema_for::<import::Chained>();
        let mut document = serde_json::to_value(root).unwrap();
        let definitions = document
            .as_object_mut()
            .unwrap()
            .remove("definitions")
            .unwrap();
        document["components"] = serde_json::json!({"schemas": definitions});
        let validator = jsonschema::draft7::new(&document).unwrap();
        assert!(validator.is_valid(&valid[0]));
        assert!(!validator.is_valid(&invalid[1]));

        assert_schema_panics::<import::Missing>("Missing", "MissingTarget");
        assert_schema_panics::<import::NegatedMissing>("NegatedMissing", "MissingTarget");
        assert_schema_panics::<import::External>("External", "https://example.invalid/array.json");
        assert_schema_panics::<import::CyclicReference>("CyclicReference", "cycle");
        assert_schema_panics::<import::ReferencedFalse>("ReferencedFalse", "false");
        assert_schema_panics::<import::NestedPointer>("NestedPointer", "PointerHolder/allOf/0");
        // The target below is still being generated. It must not be mistaken
        // for a finished false schema and silently reject every document.
        assert_schema_panics::<import::Pending>("Pending", "in progress");
    }
}

#[test]
fn shared_schema_helper_composes() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::JsonSchema)
        .with_crate_path(GeneratedCrate::Schemars, "super::schema_facade");
    let mut first = typespace_builder!(settings.clone(), {
        native super::InlineArraySchema: JsonSchema;
        struct First(u32, #[flatten] super::InlineArraySchema);
        struct FirstAgain(String, u32, #[flatten] super::InlineArraySchema);
    });
    // A free-standing utility with this name would collide with this
    // caller-named tuple constructor in the value namespace.
    first
        .insert(
            "helper name".into(),
            typespace::build::TupleStruct::new()
                .name("__typespace_prefix_array_schema")
                .fields(["u32".into()])
                .extra_attrs(["#[allow(non_camel_case_types)]"])
                .build()
                .unwrap(),
        )
        .unwrap();
    let second = typespace_builder!(settings, {
        native super::InlineArraySchema: JsonSchema;
        struct Second(u32, #[flatten] super::InlineArraySchema);
        struct SecondAgain(bool, bool, bool, #[flatten] super::InlineArraySchema);
    });
    let first = first.finalize(no_cycles).unwrap();
    let second = second.finalize(no_cycles).unwrap();
    let mut code = common::codespace(&first);
    // Codespace appends same-key items; it does not deduplicate helper
    // definitions. Disjoint outputs must therefore use disjoint owners.
    code.get_root_mod()
        .merge(common::codespace(&second).into_root_mod());
    let code = code.into_stream();
    #[check_and_include(
        "tests/output/shared_schema_helper_composes.rs",
        code.clone()
    )]
    fn inner() {
        check_schema::<import::First>(
            &[serde_json::json!([42, "a", "b"])],
            &[serde_json::json!([42, "a"])],
        );
        check_schema::<import::FirstAgain>(
            &[serde_json::json!(["prefix", 42, "a", "b"])],
            &[serde_json::json!(["prefix", 42, "a"])],
        );
        check_schema::<import::Second>(
            &[serde_json::json!([42, "a", "b"])],
            &[serde_json::json!([42, ["a", "b"]])],
        );
        check_schema::<import::SecondAgain>(
            &[serde_json::json!([true, false, true, "a", "b"])],
            &[serde_json::json!([true, false, true])],
        );
        let _ = import::__typespace_prefix_array_schema(0);

        let file = syn::parse2::<syn::File>(code).unwrap();
        let owners = file
            .items
            .iter()
            .filter_map(|item| {
                let syn::Item::Impl(item) = item else {
                    return None;
                };
                item.items
                    .iter()
                    .any(|item| {
                        matches!(item,
                            syn::ImplItem::Fn(method)
                                if method.sig.ident == "__typespace_prefix_array_schema"
                        )
                    })
                    .then(|| {
                        let syn::Type::Path(path) = item.self_ty.as_ref() else {
                            panic!("the shared utility must belong to a generated type");
                        };
                        path.path.segments.last().unwrap().ident.to_string()
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            owners,
            ["First", "Second"],
            "one utility per typespace output"
        );
    }
}

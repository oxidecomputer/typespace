// Copyright 2026 Oxide Computer Company

//! Rendering tests for a variety of constructions.

use codespace::Codespace;
use quote::{format_ident, quote};
use typespace::build::{
    Enum, EnumTagType, EnumVariant, JsonValue, Native, NewtypeConstraints, NewtypeStruct, Struct,
    StructProperty, StructPropertySerde, StructPropertyState, TupleStruct, Type, VariantDetails,
};
use typespace::error::{Error, NameAxis, OffenderReason, Relation, RequirementOrigin};
use typespace::settings::{ContainerType, GeneratedCrate, OptionalNullable, Settings, Std};
use typespace::{TypespaceBuilder, TypespaceTrait, TypespaceTraitSet, no_cycles};
use typespace_test_macro::{check_and_include, typespace_builder};

mod common;

// Stub for the user-provided type referenced by OptionalNullable::CustomType.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum OptionField<T> {
    #[default]
    #[serde(skip)]
    Absent,
    Null,
    Present(T),
}

impl<T> json_serde::OptionalNullable for OptionField<T> {
    type Target = T;

    fn is_absent(&self) -> bool {
        matches!(self, OptionField::Absent)
    }

    fn null() -> Self {
        Self::Null
    }

    fn value(value: Self::Target) -> Self {
        Self::Present(value)
    }
}

impl<T: schemars::JsonSchema> schemars::JsonSchema for OptionField<T> {
    fn schema_name() -> String {
        format!("OptionField_for_{}", T::schema_name())
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        Option::<T>::json_schema(generator)
    }

    fn is_referenceable() -> bool {
        false
    }
}

#[test]
fn test_struct_field_serde() {
    // For each configuration we create a type with the following fields:
    // - optional_string: A string that may be absent
    // - required_option: Either a string or null, but must be present
    // - optional_option: A string, null, or absent
    // - default_string: A string with the intrinsic default (i.e. "")
    // - default_option: A string or null with the intrinsic default (i.e. null)
    // - peanut_string: A string with a custom default of "peanuts"
    // - peanut_option: A string or null with a custom default of "peanuts"
    let conflated_as_absent = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_optional_nullable(OptionalNullable::ConflateAsAbsent);
    let conflated_as_absent = typespace_builder!(conflated_as_absent, {
        struct ConflatedAsAbsent {
            optional_string: Optional<String>,
            required_option: Nullable<String>,
            optional_option: OptionalNullable<String>,
            #[default]
            default_string: String,
            #[default]
            default_option: Nullable<String>,
            #[default = "peanuts"]
            peanut_string: String,
            #[default = "peanuts"]
            peanut_option: Nullable<String>,
        }
    });

    let conflated_as_null = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_optional_nullable(OptionalNullable::ConflateAsNull);
    let conflated_as_null = typespace_builder!(conflated_as_null, {
        struct ConflatedAsNull {
            optional_string: Optional<String>,
            required_option: Nullable<String>,
            optional_option: OptionalNullable<String>,
            #[default]
            default_string: String,
            #[default]
            default_option: Nullable<String>,
            #[default = "peanuts"]
            peanut_string: String,
            #[default = "peanuts"]
            peanut_option: Nullable<String>,
        }
    });

    let double_option = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_optional_nullable(OptionalNullable::DoubleOption);
    let double_option = typespace_builder!(double_option, {
        struct DoubleOption {
            optional_string: Optional<String>,
            required_option: Nullable<String>,
            optional_option: OptionalNullable<String>,
            #[default]
            default_string: String,
            #[default]
            default_option: Nullable<String>,
            #[default = "peanuts"]
            peanut_string: String,
            #[default = "peanuts"]
            peanut_option: Nullable<String>,
        }
    });

    let custom_type = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_optional_nullable(OptionalNullable::CustomType(
            ContainerType::option().with_path("super::OptionField"),
        ));
    let custom_type = typespace_builder!(custom_type, {
        struct CustomType {
            optional_string: Optional<String>,
            required_option: Nullable<String>,
            optional_option: OptionalNullable<String>,
            #[default]
            default_string: String,
            #[default]
            default_option: Nullable<String>,
            #[default = "peanuts"]
            peanut_string: String,
            #[default = "peanuts"]
            peanut_option: Nullable<String>,
        }
    });

    let builders = [
        ("ConflatedAsAbsent", conflated_as_absent),
        ("ConflatedAsNull", conflated_as_null),
        ("DoubleOption", double_option),
        ("CustomType", custom_type),
    ];

    let outputs = builders.into_iter().map(|(name, builder)| {
        let ts = builder.finalize(no_cycles).unwrap();
        let out = ts.to_codespace();

        (name, out)
    });

    let mut codespace = Codespace::default();

    for (name, sub_codespace) in outputs {
        let modname = heck::ToSnakeCase::to_snake_case(name);
        let modname_ident = format_ident!("{modname}");
        let m = sub_codespace.into_root_mod();

        let root_mod = codespace.get_root_mod();

        root_mod.replace_mod(modname, m);
        root_mod.add_item(
            name,
            quote! {
                pub use #modname_ident::*;
            },
        );
    }

    let out = codespace.into_stream();

    #[check_and_include("tests/output/test_struct_field_serde.rs", out)]
    fn inner() {
        use serde::Deserialize;

        // optional_string absent -> None
        let v: import::ConflatedAsAbsent =
            serde_json::from_str(r#"{"required_option": null}"#).unwrap();
        assert!(v.optional_string.is_none());
        assert!(v.required_option.is_none());
        assert_eq!(v.peanut_string, "peanuts");
        assert_eq!(v.peanut_option, Some("peanuts".to_string()));

        // optional_string present -> Some
        let v: import::ConflatedAsAbsent =
            serde_json::from_str(r#"{"required_option": null, "optional_string": "hi"}"#).unwrap();
        assert_eq!(v.optional_string, Some("hi".to_string()));

        // DoubleOption: optional_option absent -> None, present-null -> Some(None)
        let v: import::DoubleOption = serde_json::from_str(r#"{"required_option": null}"#).unwrap();
        assert!(v.optional_option.is_none());

        // CustomType: use OptionField stub from outer scope
        let value = serde_json::json!({
            "required_option": null,
        });
        let value = import::CustomType::deserialize(value).unwrap();
        assert!(matches!(value.optional_option, OptionField::Absent));

        let value = serde_json::json!({
            "required_option": null,
            "optional_option": null,
        });
        let value = import::CustomType::deserialize(value).unwrap();
        assert!(
            matches!(value.optional_option, OptionField::Null),
            "expected Null, got {:?}",
            value.optional_option,
        );

        let value = serde_json::json!({
            "required_option": null,
            "optional_option": "howdy",
        });
        let value = import::CustomType::deserialize(value).unwrap();
        assert!(matches!(
            value.optional_option,
            OptionField::Present(s) if s == "howdy"
        ));

        let value = serde_json::json!({
            "required_option": null,
            "optional_option": null,
        });
        let value = import::CustomType::deserialize(value).unwrap();
        assert!(matches!(value.optional_option, OptionField::Null));
    }
}

#[test]
fn test_unit_struct() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
        {
            #[json = "<<+>>"]
            struct MyUnitStruct;
        }
    );

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include("tests/output/test_unit_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = import::MyUnitStruct;
        assert_eq!(serde_json::to_string(&value).unwrap(), "\"<<+>>\"");

        assert!(serde_json::from_str::<import::MyUnitStruct>("\"<<+>>\"").is_ok());
        assert!(serde_json::from_str::<import::MyUnitStruct>("null").is_err());
    }
}

// A non-integer number in a unit struct's representation goes through
// Number::from_f64 in the emitted Serialize and Deserialize impls; the
// emitted call carries the unwrap it needs to yield a Number rather
// than an Option, and the unwrap cannot fire since a serde_json Number
// is always finite.
#[test]
fn test_unit_struct_float_repr() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
        {
            #[json = 1.5]
            struct FloatUnitStruct;
        }
    );

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include(
        "tests/output/test_unit_struct_float_repr.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        let value = import::FloatUnitStruct;
        assert_eq!(serde_json::to_string(&value).unwrap(), "1.5");

        assert!(serde_json::from_str::<import::FloatUnitStruct>("1.5").is_ok());
        assert!(serde_json::from_str::<import::FloatUnitStruct>("1.25").is_err());
        assert!(serde_json::from_str::<import::FloatUnitStruct>("null").is_err());
    }
}

#[test]
fn test_tuple_struct() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_std(Std::Unqualified)
            .with_required_trait(TypespaceTrait::Default)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::JsonSchema),
        {
            #[default = ["one", 2, "three", "four"]]
            struct MyTupleStruct(String, u32, #[flatten] Vec<String>);
        }
    );

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include("tests/output/test_tuple_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // Serialization: rest Vec<String> is flattened into the outer sequence.
        let value = import::MyTupleStruct("hello".to_string(), 42, vec![]);
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"["hello",42]"#);

        let value = import::MyTupleStruct(
            "hello".to_string(),
            42,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"["hello",42,"a","b","c"]"#
        );

        // Deserialization.
        let value = serde_json::from_str::<import::MyTupleStruct>(r#"["hello",42]"#).unwrap();
        assert_eq!(value.0, "hello");
        assert_eq!(value.1, 42);
        assert!(value.2.is_empty());

        let value =
            serde_json::from_str::<import::MyTupleStruct>(r#"["hello",42,"a","b"]"#).unwrap();
        assert_eq!(value.0, "hello");
        assert_eq!(value.1, 42);
        assert_eq!(value.2, vec!["a", "b"]);

        assert!(serde_json::from_str::<import::MyTupleStruct>(r#"[]"#).is_err());
        assert!(serde_json::from_str::<import::MyTupleStruct>(r#"["hello"]"#).is_err());

        let schema = schemars::schema_for!(import::MyTupleStruct);
        let schema = serde_json::to_value(&schema).unwrap();
        let expected = serde_json::json!(
            {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "MyTupleStruct",
                "default": ["one", 2, "three", "four"],
                "type": "array",
                "items": [
                    {
                        "type": "string"
                    },
                    {
                        "type": "integer",
                        "format": "uint32",
                        "minimum": 0.0
                    }
                ],
                "additionalItems": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "minItems": 2
            }
        );

        assert_eq!(
            schema,
            expected,
            "{}",
            serde_json::to_string_pretty(&schema).unwrap(),
        );
    }
}

// Enums: one test covering all four serde tag types.
#[test]
fn test_enums() {
    fn settings() -> Settings {
        Settings::minimal()
            .with_std(Std::Unqualified)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::Serialize)
    }

    // Internal tagging doesn't support newtype variants wrapping non-struct
    // types, so we use only unit and struct variants for Internal.
    let external = typespace_builder!(settings(), {
        enum External {
            Unit,
            Item(String),
            Named { x: u32 },
        }
    });

    let internal = typespace_builder!(settings(), {
        #[tag = "type"]
        enum Internal {
            Unit,
            Named { x: u32 },
        }
    });

    let adjacent = typespace_builder!(settings(), {
        #[tag = "t", content = "c"]
        enum Adjacent {
            Unit,
            Item(String),
            Named { x: u32 },
        }
    });

    let untagged = typespace_builder!(settings(), {
        #[untagged]
        enum Untagged {
            Unit,
            Item(String),
            Named { x: u32 },
        }
    });

    let outputs = [external, internal, adjacent, untagged]
        .into_iter()
        .map(|builder| {
            builder
                .finalize(no_cycles)
                .unwrap()
                .to_codespace()
                .into_stream()
        });

    let output = quote! { #( #outputs )* };

    #[check_and_include("tests/output/test_enums.rs", output)]
    fn inner() {
        // External tagging
        let v: import::External = serde_json::from_str(r#""Unit""#).unwrap();
        assert!(matches!(v, import::External::Unit));

        let v: import::External = serde_json::from_str(r#"{"Item": "hello"}"#).unwrap();
        assert!(matches!(v, import::External::Item(s) if s == "hello"));

        let v: import::External = serde_json::from_str(r#"{"Named": {"x": 42}}"#).unwrap();
        assert!(matches!(v, import::External::Named { x: 42 }));

        assert_eq!(
            serde_json::to_string(&import::External::Unit).unwrap(),
            r#""Unit""#
        );
        assert_eq!(
            serde_json::to_string(&import::External::Item("hi".to_string())).unwrap(),
            r#"{"Item":"hi"}"#
        );

        // Internal tagging
        let v: import::Internal = serde_json::from_str(r#"{"type": "Unit"}"#).unwrap();
        assert!(matches!(v, import::Internal::Unit));

        let v: import::Internal = serde_json::from_str(r#"{"type": "Named", "x": 7}"#).unwrap();
        assert!(matches!(v, import::Internal::Named { x: 7 }));

        // Adjacent tagging
        let v: import::Adjacent = serde_json::from_str(r#"{"t": "Unit"}"#).unwrap();
        assert!(matches!(v, import::Adjacent::Unit));

        let v: import::Adjacent = serde_json::from_str(r#"{"t": "Item", "c": "hello"}"#).unwrap();
        assert!(matches!(v, import::Adjacent::Item(s) if s == "hello"));

        // Untagged
        let v: import::Untagged = serde_json::from_str(r#"null"#).unwrap();
        assert!(matches!(v, import::Untagged::Unit));

        let v: import::Untagged = serde_json::from_str(r#""hello""#).unwrap();
        assert!(matches!(v, import::Untagged::Item(s) if s == "hello"));

        let v: import::Untagged = serde_json::from_str(r#"{"x": 3}"#).unwrap();
        assert!(matches!(v, import::Untagged::Named { x: 3 }));
    }
}

// An all-unit-variant enum carries Display and FromStr as bespoke
// impls over the variants' serialized names, and FromStr brings
// TryFrom<&str> and TryFrom<String> with it.
#[test]
fn test_simple_enum_str_impls() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_required_trait(TypespaceTrait::Display)
        .with_required_trait(TypespaceTrait::FromStr);

    let builder = typespace_builder!(settings, {
        enum Color {
            #[json = "red"]
            Red,
            #[json = "sea green"]
            SeaGreen,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_simple_enum_str_impls.rs", ts.to_codespace().into_stream())]
    fn inner() {
        assert_eq!(import::Color::Red.to_string(), "red");
        assert_eq!(import::Color::SeaGreen.to_string(), "sea green");

        assert_eq!("red".parse::<import::Color>().unwrap(), import::Color::Red);
        assert_eq!(
            import::Color::try_from("sea green").unwrap(),
            import::Color::SeaGreen
        );
        assert_eq!(
            import::Color::try_from("red".to_string()).unwrap(),
            import::Color::Red
        );
        assert!("chartreuse".parse::<import::Color>().is_err());
        assert_eq!(
            "chartreuse"
                .parse::<import::Color>()
                .unwrap_err()
                .to_string(),
            "invalid value"
        );

        // Every variant survives the round trip through its
        // serialized name.
        for color in [import::Color::Red, import::Color::SeaGreen] {
            assert_eq!(color.to_string().parse::<import::Color>().unwrap(), color);
        }
    }
}

// Item and Tuple variants get a From impl converting a payload value
// into the variant. A payload signature carried by more than one
// variant gets none, nor does a bare String payload, and a Tuple
// variant converts from a tuple type.
#[test]
fn test_enum_variant_from() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        // Two u32 variants suppress each other; the bool variant is
        // unaffected.
        enum Collide {
            First(u32),
            Second(u32),
            Only(bool),
        }

        // A bare String payload gets no impl; its sibling does.
        enum Label {
            Text(String),
            Count(u32),
        }

        // A two-element tuple, and a one-element tuple written with
        // #[tuple] so that it stays a Tuple rather than an Item.
        enum Point {
            Pair(u32, bool),
            #[tuple]
            One(u32),
        }

        // An Item variant and a one-element Tuple variant over the
        // same type key alike, so neither gets an impl even though
        // `From<u32>` and `From<(u32,)>` would both compile.
        enum Overlap {
            Single(u32),
            #[tuple]
            Wrapped(u32),
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_enum_variant_from.rs", ts.to_codespace().into_stream())]
    fn inner() {
        assert_eq!(import::Collide::from(true), import::Collide::Only(true));
        assert_eq!(import::Label::from(7u32), import::Label::Count(7));
        assert_eq!(
            import::Point::from((1u32, true)),
            import::Point::Pair(1, true)
        );
        assert_eq!(import::Point::from((2u32,)), import::Point::One(2));
    }
}

// `Default` precedes an enum's per-variant payload conversions. This
// enum carries both, so the order guard over tests/output reads a group
// that ranks the two against each other.
#[test]
fn test_enum_default_precedes_variant_from() {
    let settings = Settings::minimal()
        .with_desired_trait(TypespaceTrait::Default)
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq);

    let builder = typespace_builder!(settings, {
        #[default = { Count: 0 }]
        enum Choice {
            Count(u32),
            Flag(bool),
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_enum_default_precedes_variant_from.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(import::Choice::default(), import::Choice::Count(0));
        assert_eq!(import::Choice::from(true), import::Choice::Flag(true));
    }
}

// A single-type tuple body is a newtype struct unless #[tuple] asks
// otherwise, in which case it is a one-field tuple struct. The two
// differ on the wire: a newtype is transparent, a tuple struct is a
// sequence.
#[test]
fn test_tuple_marker_struct() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        struct Wrapped(u32);

        #[tuple]
        struct Listed(u32);
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_tuple_marker_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        assert_eq!(serde_json::to_string(&import::Wrapped(7)).unwrap(), "7");
        assert_eq!(serde_json::to_string(&import::Listed(7)).unwrap(), "[7]");

        let v: import::Wrapped = serde_json::from_str("7").unwrap();
        assert_eq!(v.0, 7);
        let v: import::Listed = serde_json::from_str("[7]").unwrap();
        assert_eq!(v.0, 7);
    }
}

// Variants whose payloads are different nodes but the same Rust type
// collide, in the two ways that can happen.
//
// Twin: typespace gives each anonymous type its own node, so its two
// Vec<String> nodes are distinct ids; keying the From impls on ids
// would emit two `impl From<Vec<String>> for Twin`, which does not
// compile. Mixed: with no set_type override a set renders as the vec
// type, so its Vec<String> and Set<String> nodes are one Rust type.
//
// Built through the builder API rather than typespace_builder!, and it
// has to stay that way: the macro creates one anonymous node per
// distinct type it reads and reuses it, so it cannot produce two nodes
// that render alike.
#[test]
fn test_enum_variant_from_distinct_ids() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);
    let mut builder = TypespaceBuilder::new(settings);

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let first_vec_id = "first_vec".to_string();
    builder
        .insert(first_vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    let second_vec_id = "second_vec".to_string();
    builder
        .insert(second_vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    let third_vec_id = "third_vec".to_string();
    builder
        .insert(third_vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    let set_id = "set".to_string();
    builder
        .insert(set_id.clone(), Type::Set(string_id.clone()))
        .unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    builder
        .insert(
            "Twin".to_string(),
            Enum::new()
                .name("Twin")
                .tag_type(EnumTagType::External)
                .variants(vec![
                    EnumVariant::new("Left", VariantDetails::Item(first_vec_id)),
                    EnumVariant::new("Right", VariantDetails::Item(second_vec_id)),
                    EnumVariant::new("Count", VariantDetails::Item(int_id.clone())),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    // The set/vec collapse needs its own enum: were it a fourth
    // variant of Twin, the two Vec variants would suppress each other
    // and leave the set variant as the only From impl, which compiles
    // whether or not the two are treated as one type.
    builder
        .insert(
            "Mixed".to_string(),
            Enum::new()
                .name("Mixed")
                .tag_type(EnumTagType::External)
                .variants(vec![
                    EnumVariant::new("Listed", VariantDetails::Item(third_vec_id)),
                    EnumVariant::new("Bagged", VariantDetails::Item(set_id)),
                    EnumVariant::new("Count", VariantDetails::Item(int_id)),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_enum_variant_from_distinct_ids.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // In each enum only the u32 variant gets a From impl; the
        // payloads that render as Vec<String> suppress one another.
        assert_eq!(import::Mixed::from(4u32), import::Mixed::Count(4));
        assert_eq!(import::Twin::from(3u32), import::Twin::Count(3));
    }
}

#[test]
fn test_newtype_struct() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
        {
            /// A newtype wrapping String.
            struct MyString(String);

            struct MyInt(u32);
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_newtype_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v = import::MyString("hello".to_string());
        assert_eq!(serde_json::to_string(&v).unwrap(), r#""hello""#);
        assert_eq!(*v, "hello");

        let v: import::MyString = serde_json::from_str(r#""world""#).unwrap();
        assert_eq!(v.0, "world");

        let v = import::MyInt(42);
        assert_eq!(serde_json::to_string(&v).unwrap(), "42");
        assert_eq!(*v, 42u32);

        let v: import::MyInt = serde_json::from_str("7").unwrap();
        assert_eq!(v.0, 7);

        // The wrap direction, mirroring From<MyString> for String.
        let v: import::MyString = "wrapped".to_string().into();
        assert_eq!(v.0, "wrapped");
        assert_eq!(String::from(v), "wrapped");

        let v = import::MyInt::from(9u32);
        assert_eq!(v.0, 9);
    }
}

#[test]
fn test_type_alias() {
    let builder = typespace_builder!(Settings::minimal().with_std(Std::Unqualified), {
        type MyAlias = String;

        /// A list of strings.
        type StringList = Vec<String>;
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_type_alias.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v: import::MyAlias = "hello".to_string();
        assert_eq!(v, "hello");

        let v: import::StringList = vec!["a".to_string(), "b".to_string()];
        assert_eq!(v.len(), 2);
    }
}

#[test]
fn test_struct_serde_rename_flatten() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
        {
            // Inner struct that will be flattened.
            struct Inner {
                value: u32,
            }

            // Outer struct with a renamed field and a flattened inner struct.
            struct Outer {
                #[rename = "my-field"]
                my_field: String,
                #[flatten]
                inner: Inner,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_struct_serde_rename_flatten.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v: import::Outer =
            serde_json::from_str(r#"{"my-field": "hello", "value": 42}"#).unwrap();
        assert_eq!(v.my_field, "hello");
        assert_eq!(v.inner.value, 42);

        let json = serde_json::to_string(&v).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["my-field"], "hello");
        assert_eq!(parsed["value"], 42);
    }
}

#[test]
fn test_native_type() {
    let builder = typespace_builder!(Settings::minimal().with_std(Std::Unqualified), {
        native std::path::PathBuf: Clone
            + Debug
            + Serialize
            + Deserialize
            + JsonSchema
            + Ord
            + PartialOrd
            + Eq
            + PartialEq
            + Hash
            + Display
            + FromStr;

        struct Resource {
            location: std::path::PathBuf,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_native_type.rs", ts.to_codespace().into_stream())]
    fn inner() {}
}

/// A required struct property is a value position; finalize rejects one
/// whose type is `Type::Never`.
#[test]
fn test_never_field() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::Debug),
        {
            struct Gone {
                value: !,
            }
        }
    );

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "property");
    assert_eq!(name, "value");
    assert_eq!(type_id, "Gone");
}

#[test]
fn test_compound_field_types() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
        {
            struct All {
                a_bool: bool,
                an_int: u32,
                a_float: f64,
                a_string: String,
                a_json: JsonValue,
                a_vec: Vec<String>,
                a_map: Map<String, u32>,
                a_set: Set<String>,
                an_array: [u32; 3],
                a_tuple: (String, u32),
                a_box_string: Box<String>,
                a_box_vec: Box<Vec<String>>,
            }

            // Exercise StructPropertyState::Default for each applicable field
            // type. JsonValue is excluded: Default is not supported for it.
            struct Defaults {
                #[default]
                a_bool: bool,
                #[default]
                an_int: u32,
                #[default]
                a_float: f64,
                #[default]
                a_string: String,
                #[default]
                a_vec: Vec<String>,
                #[default]
                a_map: Map<String, u32>,
                #[default]
                a_set: Set<String>,
                #[default]
                an_array: [u32; 3],
                #[default]
                a_tuple: (String, u32),
                #[default]
                an_option: Nullable<String>,
                #[default]
                a_box_string: Box<String>,
                #[default]
                a_box_vec: Box<Vec<String>>,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_compound_field_types.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v: import::All = serde_json::from_value(serde_json::json!({
            "a_bool": true,
            "an_int": 7,
            "a_float": 3.5,
            "a_string": "hello",
            "a_json": {"any": "thing"},
            "a_vec": ["x", "y"],
            "a_map": {"k": 1},
            "a_set": ["a", "b"],
            "an_array": [1, 2, 3],
            "a_tuple": ["hi", 99],
            "a_box_string": "boxed",
            "a_box_vec": ["p", "q"],
        }))
        .unwrap();

        assert_eq!(v.a_bool, true);
        assert_eq!(v.an_int, 7);
        assert_eq!(v.a_string, "hello");
        assert_eq!(v.a_vec, vec!["x", "y"]);
        assert_eq!(v.an_array, [1u32, 2, 3]);
        assert_eq!(*v.a_box_string, "boxed");
        assert_eq!(*v.a_box_vec, vec!["p", "q"]);

        // All fields omitted--each should take its intrinsic default.
        let d: import::Defaults = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(d.a_bool, false);
        assert_eq!(d.an_int, 0u32);
        assert_eq!(d.a_string, "");
        assert!(d.a_vec.is_empty());
        assert!(d.a_map.is_empty());
        assert!(d.a_set.is_empty());
        assert_eq!(d.an_option, None);
        assert_eq!(*d.a_box_string, "");
        assert!(d.a_box_vec.is_empty());

        // Fields with skip_serializing_if are absent from the serialized form
        // when at their default; fields without it are always present.
        let serialized = serde_json::to_value(&d).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!({"a_bool": false, "an_int": 0, "a_float": 0.0, "an_array": [0, 0, 0], "a_tuple": ["", 0]})
        );
    }
}

// A map whose key is a struct containing a float cannot satisfy the Ord and Eq
// constraints required of BTreeMap keys; finalize should return Err.
#[test]
fn test_map_key_struct_with_float() {
    let mut builder = TypespaceBuilder::default();

    let float_id = "float".to_string();
    builder
        .insert(float_id.clone(), Type::Float("f64".to_string()))
        .unwrap();

    let key_id = "key".to_string();
    builder
        .insert(
            key_id.clone(),
            Struct::new()
                .name("Key")
                .properties(vec![StructProperty::new("value", float_id)])
                .build()
                .unwrap(),
        )
        .unwrap();

    let value_id = "value".to_string();
    builder.insert(value_id.clone(), Type::String).unwrap();

    builder
        .insert("map".to_string(), Type::Map(key_id, value_id))
        .unwrap();

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // Of the map-key requirements (Eq, PartialEq, Ord, PartialOrd), floats
    // can satisfy the partial comparisons but not Eq and Ord.
    assert_eq!(conflicts.len(), 2);
    for conflict in &conflicts {
        assert_eq!(conflict.offender, "float");
        assert!(matches!(
            &conflict.origin,
            RequirementOrigin::ContainerParameter { container, relation }
                if container == "map" && matches!(relation, Relation::Key)
        ));
        assert!(matches!(
            &conflict.reason,
            OffenderReason::Primitive { type_name } if type_name == "f64"
        ));
    }
    assert!(matches!(conflicts[0].required, TypespaceTrait::Eq));
    assert!(matches!(conflicts[1].required, TypespaceTrait::Ord));

    assert_eq!(
        conflicts[0].to_string(),
        "type `f64` (id `float`) cannot implement the required trait `Eq`\n    \
         required because `key` passes the requirement to its field `value`\n    \
         required because the container `map` requires `Eq` of its key type"
    );
}

// Inserting two types with the same ID is a caller error.
#[test]
fn test_duplicate_type_id() {
    let mut builder = TypespaceBuilder::default();

    builder.insert("s".to_string(), Type::String).unwrap();
    let err = builder.insert("s".to_string(), Type::Boolean).unwrap_err();
    assert!(matches!(
        err,
        Error::DuplicateTypeId { ref type_id } if type_id == "s"
    ));
}

// Building a named type without a name is a caller error; names come from
// the caller and rendering cannot invent one. A name that was set goes
// through identifier validation, where the empty string fails like any
// other garbage.
#[test]
fn test_missing_type_name() {
    let err = Struct::<String>::new().build().unwrap_err();
    assert!(matches!(
        err,
        Error::MissingTypeName { kind } if kind == "struct"
    ));

    let err = Struct::<String>::new().name("").build().unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidName { kind, ref name, .. } if kind == "struct" && name.is_empty()
    ));
}

// Type, property, and variant names must be plain Rust identifiers:
// keywords (including gen, a keyword only as of edition 2024), raw
// identifiers, and lexical garbage are all rejected at build().
#[test]
fn test_invalid_names() {
    let err = Struct::<String>::new().name("type").build().unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidName { kind, ref name, message }
            if kind == "struct" && name == "type" && message.contains("keyword")
    ));

    let err = Struct::<String>::new().name("gen").build().unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidName { ref name, message, .. }
            if name == "gen" && message.contains("keyword")
    ));

    let err = Struct::<String>::new().name("r#type").build().unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidName { ref name, message, .. }
            if name == "r#type" && message.contains("raw identifiers")
    ));

    let err = Struct::<String>::new()
        .name("not a name")
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::InvalidName { .. }));

    let err = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![EnumVariant::new("true", VariantDetails::Unit)])
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidName { kind, ref name, .. } if kind == "variant" && name == "true"
    ));
}

// An enum's tagging scheme has no presumed default; building without one
// is an error.
#[test]
fn test_missing_tag_type() {
    let err = Enum::<String>::new().name("E").build().unwrap_err();
    assert!(matches!(
        err,
        Error::MissingTagType { ref name } if name == "E"
    ));
}

// Property and variant names must be unique on both the Rust axis and the
// wire axis (the serialized name after any rename).
#[test]
fn test_duplicate_item_names() {
    // Two properties with the same Rust name.
    let err = Struct::new()
        .name("S")
        .properties(vec![
            StructProperty::new("x", "t".to_string()),
            StructProperty::new("x", "t".to_string()),
        ])
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::DuplicateItemName { kind, ref type_name, ref name, axis }
            if kind == "property" && type_name == "S" && name == "x" && axis == NameAxis::Rust
    ));

    // Distinct Rust names colliding on the wire via a rename.
    let err = Struct::new()
        .name("S")
        .properties(vec![
            StructProperty::new("x", "t".to_string()),
            StructProperty::new("y", "t".to_string())
                .with_json_name(StructPropertySerde::Rename("x".to_string())),
        ])
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::DuplicateItemName { ref name, axis, .. }
            if name == "x" && axis == NameAxis::Wire
    ));

    // Two variants with the same Rust name.
    let err = Enum::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![
            EnumVariant::new("A", VariantDetails::<String>::Unit),
            EnumVariant::new("A", VariantDetails::Unit),
        ])
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::DuplicateItemName { kind, ref name, axis, .. }
            if kind == "variant" && name == "A" && axis == NameAxis::Rust
    ));

    // Distinct variant names colliding on the wire via a rename.
    let err = Enum::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![
            EnumVariant::new("A", VariantDetails::<String>::Unit),
            EnumVariant::new("B", VariantDetails::Unit).with_rename("A"),
        ])
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::DuplicateItemName { kind, ref name, axis, .. }
            if kind == "variant" && name == "A" && axis == NameAxis::Wire
    ));
}

// Two types sharing a name is a converter bug caught at validate() and
// finalize(); typespace reports rather than renames.
#[test]
fn test_duplicate_type_names() {
    let mut builder = TypespaceBuilder::default();
    builder
        .insert(
            "first".to_string(),
            Struct::new().name("Twin").build().unwrap(),
        )
        .unwrap();
    builder
        .insert(
            "second".to_string(),
            Struct::new().name("Twin").build().unwrap(),
        )
        .unwrap();

    let Err(Error::DuplicateTypeName {
        name,
        first,
        second,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with a duplicate type name");
    };
    assert_eq!(name, "Twin");
    assert_eq!(first, "first");
    assert_eq!(second, "second");
}

// Type's variants are not sealed, so an unbuilt shape can be smuggled into
// a Type value directly; insertion re-runs the build() checks and rejects
// it.
#[test]
fn test_insert_unbuilt_shape() {
    let mut builder = TypespaceBuilder::default();
    let err = builder
        .insert("sneaky".to_string(), Type::Struct(Struct::new()))
        .unwrap_err();
    assert!(matches!(
        err,
        Error::MissingTypeName { kind } if kind == "struct"
    ));

    let err = builder
        .insert(
            "sneakier".to_string(),
            Type::Enum(Enum::new().name("NoTag")),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        Error::MissingTagType { ref name } if name == "NoTag"
    ));
}

// A tuple struct with no fixed fields is a second way to write a type
// that already exists: `UnitStruct` if there is no rest, `NewtypeStruct`
// over the sequence type if there is. `build()` rejects both, naming the
// alternative.
#[test]
fn test_fieldless_tuple_struct() {
    let err = TupleStruct::<String>::new().name("T").build().unwrap_err();
    assert!(matches!(
        err,
        Error::FieldlessTupleStruct { ref name, alternative }
            if name == "T" && alternative == "`UnitStruct`"
    ));

    let err = TupleStruct::new()
        .name("T")
        .rest("seq".to_string())
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::FieldlessTupleStruct { ref name, alternative }
            if name == "T" && alternative == "`NewtypeStruct` over the sequence type"
    ));
}

// Before `TupleStruct::validate` rejected zero fields, a fieldless tuple
// struct whose rest referred back to itself (directly, or mutually
// through a pair) could be inserted into a `TypespaceBuilder` and would
// overflow the stack at `finalize()`: `default_impl_tuple_struct` walks
// the rest as the whole input array when there are no fixed fields to
// split off, so the recursion never narrows, and unlike the other
// self-referential kinds (newtype struct, type alias, Option, Box) it
// is not covered by the expansion guard.
//
// `build()` rejects a fieldless tuple struct before it can be
// inserted at all, regardless of what its rest resolves to, so this
// path cannot be constructed through the builder API well enough to
// reach `finalize()`, let alone overflow.
#[test]
fn test_fieldless_tuple_struct_self_reference_would_have_overflowed() {
    // A tuple struct whose rest is itself.
    let err = TupleStruct::new()
        .name("T1")
        .rest("t1".to_string())
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::FieldlessTupleStruct { ref name, alternative }
            if name == "T1" && alternative == "`NewtypeStruct` over the sequence type"
    ));

    // A mutual pair: T1's rest is T2, T2's rest is T1.
    let err = TupleStruct::new()
        .name("T1")
        .rest("t2".to_string())
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::FieldlessTupleStruct { ref name, .. } if name == "T1"));
    let err = TupleStruct::new()
        .name("T2")
        .rest("t1".to_string())
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::FieldlessTupleStruct { ref name, .. } if name == "T2"));
}

// A configured map type carries its own key-trait demands: a hash map
// requires Hash and Eq of its keys--not Ord--and conflicts name exactly
// the configured traits.
#[test]
fn test_map_key_traits_override() {
    let settings = Settings::minimal().with_map_type(ContainerType::hash_map());
    let builder = typespace_builder!(settings, {
        struct Holder {
            m: Map<f64, String>,
        }
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // Floats satisfy PartialEq but neither Eq nor Hash; Ord is not
    // demanded at all.
    assert_eq!(conflicts.len(), 2);
    assert!(matches!(conflicts[0].required, TypespaceTrait::Eq));
    assert!(matches!(conflicts[1].required, TypespaceTrait::Hash));
}

// Referencing a type ID for which no type was inserted is a caller error.
#[test]
fn test_unknown_type_id() {
    let mut builder = TypespaceBuilder::default();

    builder
        .insert("v".to_string(), Type::Vec("missing".to_string()))
        .unwrap();
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail");
    };
    assert!(matches!(
        err,
        Error::UnknownTypeId { ref type_id, ref child_id }
            if type_id == "v" && child_id == "missing"
    ));
}

// Test a some simple cyclic types.
#[test]
fn test_cycles() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
    );

    let mut id = 0;

    let mut next = || {
        id += 1;
        id
    };

    let struct_a_id = next();
    builder
        .insert(
            struct_a_id,
            Struct::new()
                .name("A")
                .properties(vec![
                    StructProperty::new("a", struct_a_id).with_state(StructPropertyState::Optional),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    let b_id = next();
    let c_id = next();
    builder
        .insert(
            b_id,
            Struct::new()
                .name("B")
                .properties(vec![
                    StructProperty::new("c", c_id).with_state(StructPropertyState::Optional),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();
    builder
        .insert(
            c_id,
            Struct::new()
                .name("C")
                .properties(vec![
                    StructProperty::new("b", b_id).with_state(StructPropertyState::Optional),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder
        .finalize(|_: &i32| next())
        .expect("finalize typespace");

    #[check_and_include("tests/output/test_cycles.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = serde_json::json!({
            "a": {
                "a": {}
            }
        });
        let value = serde_json::from_value::<import::A>(value).unwrap();
        assert!(value.a.is_some());
        assert!(value.a.as_ref().unwrap().a.is_some());
        assert!(value.a.unwrap().a.unwrap().a.is_none());

        // A with a nested A that has no 'a' field (absent = None).
        let a: import::A = serde_json::from_str(r#"{"a": {}}"#).unwrap();
        assert!(a.a.is_some());
        assert!(a.a.unwrap().a.is_none());

        // B and C cycle: B with absent 'c', C with nested B with absent 'c'.
        let _b: import::B = serde_json::from_str(r#"{}"#).unwrap();
        let _c: import::C = serde_json::from_str(r#"{"b": {}}"#).unwrap();
    }
}

// A cycle made up entirely of anonymous nodes has no name anywhere to
// terminate its code generation: `Outer = Vec<Inner>` and `Inner = Vec<Outer>`
// would each expand into the other forever. Neither node is infinitely sized
// (`Vec` is heap indirection), so `break_cycles` leaves them alone; this is a
// distinct problem from containment.
#[test]
fn test_anonymous_cycle_rejected() {
    let mut builder = typespace_builder!(Settings::minimal(), {
        struct S {
            outer: Outer,
        }
    });

    // The two container nodes refer to each other, which the macro
    // has no way to write: `Outer = Vec<Inner>` and
    // `Inner = Vec<Outer>` are anonymous nodes, not items.
    builder
        .insert("Outer".to_string(), Type::Vec("Inner".to_string()))
        .unwrap();
    builder
        .insert("Inner".to_string(), Type::Vec("Outer".to_string()))
        .unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail");
    };
    let Error::AnonymousCycle { type_id, child_id } = err else {
        panic!("expected AnonymousCycle, got: {err}");
    };
    let edge = [type_id, child_id]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        edge,
        ["Inner".to_string(), "Outer".to_string()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
}

// A cycle through a named type is neither a containment cycle (Vec is
// heap-allocated) nor a code-generation cycle (A is a named type).
#[test]
fn test_named_cycle_finalizes() {
    let builder = typespace_builder!(Settings::minimal(), {
        struct A {
            b: Vec<A>,
        }
    });

    builder.finalize(no_cycles).expect("finalize succeeds");
}

/// `make_box_id` is called at most once for a type, however many edges
/// into it the walk cuts: `A` closes a cycle through two other types,
/// and `D` through two of its own fields.
#[test]
fn make_box_id_is_called_once_per_boxed_type() {
    let builder = typespace_builder!(Settings::minimal(), {
        struct A {
            b: B,
            c: C,
        }
        struct B {
            a: A,
        }
        struct C {
            a: A,
        }
        struct D {
            x: D,
            y: D,
        }
    });

    let mut calls = std::collections::BTreeMap::<String, usize>::new();
    builder
        .finalize(|id: &String| {
            *calls.entry(id.clone()).or_default() += 1;
            format!("Box<{id}>")
        })
        .expect("finalize succeeds");

    assert_eq!(calls.get("A"), Some(&1), "{calls:?}");
    assert_eq!(calls.get("D"), Some(&1), "{calls:?}");
    assert!(calls.values().all(|&calls| calls == 1), "{calls:?}");
}

/// We check for containment cycles before checking for representation cycles,
/// but insertion of a Box only makes an existing cycle longer.
#[test]
fn test_anonymous_cycle_after_boxing_rejected() {
    let mut builder = TypespaceBuilder::default();

    builder
        .insert("Outer".to_string(), Type::Tuple(vec!["Inner".to_string()]))
        .unwrap();
    builder
        .insert("Inner".to_string(), Type::Tuple(vec!["Outer".to_string()]))
        .unwrap();

    let mut box_count = 0;
    let mut make_box_id = |_: &String| {
        box_count += 1;
        format!("Box{box_count}")
    };

    let Err(err) = builder.finalize(&mut make_box_id) else {
        panic!("expected finalize to fail");
    };
    assert!(matches!(err, Error::AnonymousCycle { .. }));
}

// Container overrides replace the rendered map/set/vec types; a
// string-to-JSON-value map is always ::serde_json::Map regardless of the
// map override.
#[test]
fn test_container_overrides() {
    // HashMap wants hashing of its keys, not ordering; BTreeSet keeps
    // the ordered-comparison demands.
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_map_type(ContainerType::hash_map())
        .with_set_type(ContainerType::btree_set())
        .with_vec_type(ContainerType::new(
            "::std::collections::VecDeque",
            [TypespaceTraitSet::empty()],
        ));
    let builder = typespace_builder!(settings, {
        struct Containers {
            a_map: Map<String, u32>,
            a_set: Set<String>,
            a_vec: Vec<String>,
            an_obj: Map<String, JsonValue>,
        }

        // Default-state fields exercise the is_empty path against the
        // overridden container types (and the ::serde_json::Map special
        // case).
        struct ContainerDefaults {
            #[default]
            a_map: Map<String, u32>,
            #[default]
            a_set: Set<String>,
            #[default]
            a_vec: Vec<String>,
            #[default]
            an_obj: Map<String, JsonValue>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_container_overrides.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v: import::Containers = serde_json::from_value(serde_json::json!({
            "a_map": {"k": 1},
            "a_set": ["b", "a", "b"],
            "a_vec": ["x", "y"],
            "an_obj": {"any": ["thing"]},
        }))
        .unwrap();

        let map: &std::collections::HashMap<String, u32> = &v.a_map;
        assert_eq!(map["k"], 1);
        let set: &std::collections::BTreeSet<String> = &v.a_set;
        assert_eq!(set.len(), 2);
        let vec: &std::collections::VecDeque<String> = &v.a_vec;
        assert_eq!(vec.len(), 2);
        let obj: &serde_json::Map<String, serde_json::Value> = &v.an_obj;
        assert_eq!(obj["any"], serde_json::json!(["thing"]));

        // All fields at their defaults serialize to an empty object.
        let d: import::ContainerDefaults = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(serde_json::to_value(&d).unwrap(), serde_json::json!({}));
    }
}

// Traits requested via with_trait_impl seed every named type and are
// realized as derives; with_derive paths are appended opaquely.
#[test]
fn test_trait_impls() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(typespace::TypespaceTrait::Serialize)
        .with_required_trait(typespace::TypespaceTrait::Deserialize)
        .with_required_trait(typespace::TypespaceTrait::Clone)
        .with_required_trait(typespace::TypespaceTrait::Debug)
        .with_required_trait(typespace::TypespaceTrait::PartialEq)
        .with_required_trait(typespace::TypespaceTrait::Eq)
        .with_derive("::std::hash::Hash");
    let builder = typespace_builder!(settings, {
        struct Widget {
            name: String,
            tags: Vec<String>,
        }

        enum Gadget {
            Off,
            On(u32),
        }

        struct Wrapper(String);

        #[json = "marker"]
        struct Marker;

        struct Pair(String, u32);

        type Named = String;
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_trait_impls.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // Clone + PartialEq/Eq from with_trait_impl; Hash from with_derive.
        let v: import::Widget = serde_json::from_str(r#"{"name": "w", "tags": ["a"]}"#).unwrap();
        let w = v.clone();
        assert_eq!(v, w);
        let mut hashset = std::collections::HashSet::new();
        hashset.insert(v);

        let g: import::Gadget = serde_json::from_str(r#"{"On": 3}"#).unwrap();
        assert_eq!(g.clone(), g);
        assert!(format!("{g:?}").contains("On"));

        let n: import::Wrapper = serde_json::from_str(r#""hi""#).unwrap();
        assert_eq!(n.clone(), n);

        let m = import::Marker;
        assert_eq!(m.clone(), m);

        let p: import::Pair = serde_json::from_str(r#"["x", 1]"#).unwrap();
        assert_eq!(p.clone(), p);
    }
}

// A trait requested via with_trait_impl propagates into contained types and
// surfaces an error when a leaf type cannot satisfy it.
#[test]
fn test_trait_impls_conflict() {
    let settings = Settings::minimal().with_required_trait(typespace::TypespaceTrait::Eq);
    let builder = typespace_builder!(settings, {
        struct Holder {
            value: f64,
        }
    });

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // The requested Eq propagates into Holder's field and fails there.
    assert_eq!(conflicts.len(), 1);
    let conflict = &conflicts[0];
    assert!(matches!(conflict.required, TypespaceTrait::Eq));
    assert!(matches!(conflict.origin, RequirementOrigin::GlobalSettings));
    assert_eq!(conflict.offender, "f64");
    assert_eq!(
        conflict.to_string(),
        "type `f64` (id `f64`) cannot implement the required trait `Eq`\n    \
         required because `Holder` passes the requirement to its field `value`\n    \
         required because global settings require `Eq` of all types"
    );
}

// An extra derive that isn't a valid Rust path is rejected at finalize.
#[test]
fn test_invalid_derive() {
    let settings = Settings::minimal().with_derive("not a path!");
    let builder = TypespaceBuilder::<String>::new(settings);

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail");
    };
    assert!(matches!(
        err,
        Error::InvalidDerive { ref derive, .. } if derive == "not a path!"
    ));
}

// The acceptance test for conflict path rendering: a set whose elements are
// structs containing Vec<f64>. The comparison requirements imposed on set
// elements propagate through the struct and the vec to the float, and the
// reported chain names every hop.
#[test]
fn test_set_element_float_path() {
    let mut builder = TypespaceBuilder::default();

    let float_id = "float".to_string();
    builder
        .insert(float_id.clone(), Type::Float("f64".to_string()))
        .unwrap();

    let vec_id = "vec".to_string();
    builder
        .insert(vec_id.clone(), Type::Vec(float_id.clone()))
        .unwrap();

    let sample_id = "Sample".to_string();
    builder
        .insert(
            sample_id.clone(),
            Struct::new()
                .name("Sample")
                .properties(vec![StructProperty::new("values", vec_id.clone())])
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert("set".to_string(), Type::Set(sample_id.clone()))
        .unwrap();

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    assert_eq!(conflicts.len(), 2);
    assert!(matches!(conflicts[0].required, TypespaceTrait::Eq));
    assert!(matches!(conflicts[1].required, TypespaceTrait::Ord));
    assert_eq!(
        conflicts[0].to_string(),
        "type `f64` (id `float`) cannot implement the required trait `Eq`\n    \
         required because `vec` passes the requirement to its element type\n    \
         required because `Sample` passes the requirement to its field `values`\n    \
         required because the container `set` requires `Eq` of its element \
         type"
    );
}

// A native type used as a map key must declare the comparison traits; a
// missing declaration is a reportable, fixable conflict rather than a
// panic. validate() reports the same conflicts without consuming the
// builder.
#[test]
fn test_native_map_key() {
    // chrono::NaiveDate does implement Ord and friends, but this consumer
    // neglected to declare them.
    let undeclared = [
        TypespaceTrait::Clone,
        TypespaceTrait::Debug,
        TypespaceTrait::Serialize,
        TypespaceTrait::Deserialize,
        TypespaceTrait::Display,
        TypespaceTrait::FromStr,
    ]
    .into_iter()
    .collect::<TypespaceTraitSet>();

    let mut builder = TypespaceBuilder::default();
    builder
        .insert(
            "date".to_string(),
            Type::Native(Native::new("chrono::NaiveDate", undeclared, Vec::new())),
        )
        .unwrap();
    builder.insert("value".to_string(), Type::String).unwrap();
    builder
        .insert(
            "map".to_string(),
            Type::Map("date".to_string(), "value".to_string()),
        )
        .unwrap();

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };
    assert_eq!(conflicts.len(), 4);
    for conflict in &conflicts {
        assert_eq!(conflict.offender, "date");
        assert!(matches!(
            &conflict.origin,
            RequirementOrigin::ContainerParameter { container, relation }
                if container == "map" && matches!(relation, Relation::Key)
        ));
        assert!(matches!(
            &conflict.reason,
            OffenderReason::NativeMissingImpl { type_name } if type_name == "chrono::NaiveDate"
        ));
        assert!(conflict.path.is_empty());
    }
    assert_eq!(
        conflicts[0].to_string(),
        "native type `chrono::NaiveDate` (id `date`) does not declare the \
         required trait `Eq`\n    \
         required because the container `map` requires `Eq` of its key \
         type"
    );

    // Declaring the comparison impls fixes the conflict.
    let declared = [
        TypespaceTrait::Clone,
        TypespaceTrait::Debug,
        TypespaceTrait::Serialize,
        TypespaceTrait::Deserialize,
        TypespaceTrait::Display,
        TypespaceTrait::FromStr,
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
    ]
    .into_iter()
    .collect::<TypespaceTraitSet>();

    let mut builder = TypespaceBuilder::default();
    builder
        .insert(
            "date".to_string(),
            Type::Native(Native::new("chrono::NaiveDate", declared, Vec::new())),
        )
        .unwrap();
    builder.insert("value".to_string(), Type::String).unwrap();
    builder
        .insert(
            "map".to_string(),
            Type::Map("date".to_string(), "value".to_string()),
        )
        .unwrap();

    builder.finalize(no_cycles).expect("finalize passes");
}

// Tests for `Type::Never` in composite positions: containers, the
// optional and nullable property states, tuples, arrays, generated item
// bodies, aliases, and nesting. The bare `!` struct property is covered
// by test_never_field above.

// The settings these tests share: the serde traits the round-trip
// assertions need, plus Debug so failures print something useful.
fn never_settings() -> Settings {
    Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_required_trait(TypespaceTrait::Debug)
}

// `Vec<!>` is the "array that must be empty" case: the element type can
// never be produced, so an empty vec round-trips and any element fails
// in both directions.
#[test]
fn test_never_in_vec() {
    let builder = typespace_builder!(never_settings(), {
        struct VecHolder {
            values: Vec<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_vec.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::VecHolder { values: Vec::new() };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"values":[]}"#);
        assert!(serde_json::from_str::<import::VecHolder>(r#"{"values":[]}"#).is_ok());

        // Any element on the wire fails to deserialize.
        assert!(serde_json::from_str::<import::VecHolder>(r#"{"values":[null]}"#).is_err());
        assert!(serde_json::from_str::<import::VecHolder>(r#"{"values":[1]}"#).is_err());
    }
}

// `Set<!>` makes the same "must be empty" claim as `Vec<!>`, through
// the set container.
#[test]
fn test_never_in_set() {
    let builder = typespace_builder!(never_settings(), {
        struct SetHolder {
            values: Set<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_set.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::SetHolder { values: Vec::new() };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"values":[]}"#);
        assert!(serde_json::from_str::<import::SetHolder>(r#"{"values":[]}"#).is_ok());
        assert!(serde_json::from_str::<import::SetHolder>(r#"{"values":[null]}"#).is_err());
        assert!(serde_json::from_str::<import::SetHolder>(r#"{"values":[1]}"#).is_err());
    }
}

// `Map<String, !>` is a map that must be empty: no value can be
// produced, so no entry can exist.
#[test]
fn test_never_in_map_value() {
    let builder = typespace_builder!(never_settings(), {
        struct MapValueHolder {
            entries: Map<String, !>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_map_value.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::MapValueHolder {
            entries: std::collections::BTreeMap::new(),
        };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"entries":{}}"#);
        assert!(serde_json::from_str::<import::MapValueHolder>(r#"{"entries":{}}"#).is_ok());

        assert!(
            serde_json::from_str::<import::MapValueHolder>(r#"{"entries":{"k":null}}"#).is_err()
        );
    }
}

// `Map<!, String>` is the mirror case: no key can be produced, so the
// map must be empty.
#[test]
fn test_never_in_map_key() {
    let builder = typespace_builder!(never_settings(), {
        struct MapKeyHolder {
            entries: Map<!, String>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_map_key.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::MapKeyHolder {
            entries: std::collections::BTreeMap::new(),
        };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"entries":{}}"#);
        assert!(serde_json::from_str::<import::MapKeyHolder>(r#"{"entries":{}}"#).is_ok());
        assert!(serde_json::from_str::<import::MapKeyHolder>(r#"{"entries":{"k":"v"}}"#).is_err());
    }
}

// An `Option<!>` property that must be present may be null and nothing
// else: null round-trips, a present value does not.
#[test]
fn test_never_nullable() {
    let builder = typespace_builder!(never_settings(), {
        struct NullableHolder {
            value: Nullable<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_nullable.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let null = import::NullableHolder { value: None };
        assert_eq!(serde_json::to_string(&null).unwrap(), r#"{"value":null}"#);
        assert!(serde_json::from_str::<import::NullableHolder>(r#"{"value":null}"#).is_ok());

        assert!(serde_json::from_str::<import::NullableHolder>(r#"{"value":1}"#).is_err());

        // The property is required, so omitting it is an error.
        assert!(serde_json::from_str::<import::NullableHolder>("{}").is_err());
    }
}

// A `!` property that may be absent behaves exactly like the bare `!`
// property: the state does not change what a value that cannot exist
// does on the wire.
#[test]
fn test_never_optional() {
    let builder = typespace_builder!(never_settings(), {
        struct OptionalHolder {
            value: Optional<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_optional.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = import::OptionalHolder {
            value: ::json_serde::Absent,
        };
        assert_eq!(serde_json::to_string(&value).unwrap(), "{}");
        assert!(serde_json::from_str::<import::OptionalHolder>("{}").is_ok());
        assert!(serde_json::from_str::<import::OptionalHolder>(r#"{"value":null}"#).is_err());
    }
}

/// A flattened property cannot be omitted, so `!` there is rejected.
#[test]
#[ignore]
fn test_never_flattened_property() {
    let builder = typespace_builder!(never_settings(), {
        struct FlatHolder {
            keep: u32,
            #[flatten]
            gone: Optional<!>,
        }
    });

    let Err(Error::NeverInValuePosition { position, name, .. }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "property");
    assert_eq!(name, "gone");
}

// A `!` property in the Default state has no default to fall back to.
#[test]
fn test_never_default_property() {
    let builder = typespace_builder!(never_settings(), {
        struct DefaultHolder {
            #[default]
            value: !,
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "property");
    assert_eq!(name, "value");
    assert_eq!(type_id, "DefaultHolder");
}

// A `!` property in the DefaultValue state cannot hold the given value.
#[test]
fn test_never_default_value_property() {
    let builder = typespace_builder!(never_settings(), {
        struct DefaultValueHolder {
            #[default = null]
            value: !,
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "property");
    assert_eq!(name, "value");
    assert_eq!(type_id, "DefaultValueHolder");
}

// An `Option<!>` property that may be absent accepts both wire forms it
// names, omitted and null, and nothing else.
#[test]
fn test_never_optional_nullable() {
    let builder = typespace_builder!(never_settings(), {
        struct OptionalNullableHolder {
            value: OptionalNullable<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_optional_nullable.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let absent = import::OptionalNullableHolder { value: None };
        assert_eq!(serde_json::to_string(&absent).unwrap(), "{}");

        assert!(serde_json::from_str::<import::OptionalNullableHolder>("{}").is_ok());
        assert!(
            serde_json::from_str::<import::OptionalNullableHolder>(r#"{"value":null}"#).is_ok()
        );
        assert!(serde_json::from_str::<import::OptionalNullableHolder>(r#"{"value":1}"#).is_err());
    }
}

// `Box<T>` is transparent on the wire, so a `Box<!>` property would be
// wire-identical to a bare `!` property but escape the skip logic that
// makes a bare `!` property work (it only recognizes a property whose
// immediate type is `Type::Never`). typespace rejects the wrapping
// outright rather than render a struct that can never serialize or
// deserialize.
#[test]
fn test_never_in_box() {
    let builder = typespace_builder!(never_settings(), {
        struct BoxHolder {
            value: Box<!>,
        }
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "Box");
    assert_eq!(type_id, "Box<Never>");
}

// A tuple component is a value position: rejected.
#[test]
fn test_never_in_tuple() {
    let builder = typespace_builder!(never_settings(), {
        struct TupleHolder {
            value: (u32, !),
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "tuple component");
    assert_eq!(name, "1");
    assert_eq!(type_id, "(u32, Never)");
}

// A zero-length array of `!` is the only inhabited fixed-size array of
// it: it round-trips as an empty JSON array.
#[test]
fn test_never_in_array_zero() {
    let builder = typespace_builder!(never_settings(), {
        struct ArrayZeroHolder {
            values: [!; 0],
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_array_zero.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = import::ArrayZeroHolder { values: [] };
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"values":[]}"#);
        assert!(serde_json::from_str::<import::ArrayZeroHolder>(r#"{"values":[]}"#).is_ok());
        assert!(serde_json::from_str::<import::ArrayZeroHolder>(r#"{"values":[null]}"#).is_err());
    }
}

// A non-empty fixed-size array element is a value position: rejected.
#[test]
fn test_never_in_array_three() {
    let builder = typespace_builder!(never_settings(), {
        struct ArrayThreeHolder {
            values: [!; 3],
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "array element");
    assert_eq!(name, "item");
    assert_eq!(type_id, "[Never; 3]");
}

// A newtype struct wrapping `!` would be transparent on the wire, so it
// would be wire-identical to a bare `!` property but escape the skip
// logic that makes a bare `!` property work (it only recognizes a
// property whose immediate type is `Type::Never`). typespace rejects
// the wrapping outright rather than render a type that can never
// serialize or deserialize.
#[test]
fn test_never_newtype_struct() {
    let builder = typespace_builder!(never_settings(), {
        struct NeverNewtype(!);
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "newtype struct");
    assert_eq!(type_id, "NeverNewtype");
}

// A tuple struct field is a value position: rejected.
#[test]
fn test_never_tuple_struct() {
    let builder = typespace_builder!(never_settings(), {
        struct NeverTupleStruct(u32, !);
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "tuple struct field");
    assert_eq!(name, "1");
    assert_eq!(type_id, "NeverTupleStruct");
}

// A `!` in a tuple struct's rest slot is rejected as a field is.
#[test]
fn test_never_tuple_struct_rest() {
    let builder = typespace_builder!(never_settings(), {
        struct RestTupleStruct(u32, #[flatten] !);
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "tuple struct field");
    assert_eq!(name, "1");
    assert_eq!(type_id, "RestTupleStruct");
}

// An enum variant's item payload is a value position: rejected.
#[test]
fn test_never_enum_item_variant() {
    let builder = typespace_builder!(never_settings(), {
        enum ItemEnum {
            Gone(!),
            Kept(u32),
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "variant payload");
    assert_eq!(name, "Gone");
    assert_eq!(type_id, "ItemEnum");
}

// An enum variant's tuple payload component is a value position:
// rejected.
#[test]
fn test_never_enum_tuple_variant() {
    let builder = typespace_builder!(never_settings(), {
        enum TupleEnum {
            Gone(u32, !),
            Kept(u32),
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "variant payload component");
    assert_eq!(name, "Gone.1");
    assert_eq!(type_id, "TupleEnum");
}

// A struct-shaped variant's required field is a value position:
// rejected.
#[test]
fn test_never_enum_struct_variant() {
    let builder = typespace_builder!(never_settings(), {
        enum StructEnum {
            Gone { gone: ! },
            Kept(u32),
        }
    });

    let Err(Error::NeverInValuePosition {
        position,
        name,
        type_id,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInValuePosition");
    };
    assert_eq!(position, "variant property");
    assert_eq!(name, "Gone.gone");
    assert_eq!(type_id, "StructEnum");
}

// A struct-shaped variant's `!` field that may be absent leaves the
// variant selectable, with the field always omitted.
#[test]
fn test_never_optional_enum_struct_variant() {
    let builder = typespace_builder!(never_settings(), {
        enum OptionalStructEnum {
            Gone { gone: Optional<!> },
            Kept(u32),
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_optional_enum_struct_variant.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let gone = import::OptionalStructEnum::Gone {
            gone: ::json_serde::Absent,
        };
        assert_eq!(serde_json::to_string(&gone).unwrap(), r#"{"Gone":{}}"#);
        assert!(serde_json::from_str::<import::OptionalStructEnum>(r#"{"Gone":{}}"#).is_ok());
        assert!(
            serde_json::from_str::<import::OptionalStructEnum>(r#"{"Gone":{"gone":null}}"#)
                .is_err()
        );

        let kept = import::OptionalStructEnum::Kept(1);
        assert_eq!(serde_json::to_string(&kept).unwrap(), r#"{"Kept":1}"#);
        assert!(serde_json::from_str::<import::OptionalStructEnum>(r#"{"Kept":1}"#).is_ok());
    }
}

// A type alias is transparent, so an alias for `!` would be
// wire-identical to a bare `!` property but escape the skip logic that
// makes a bare `!` property work (it only recognizes a property whose
// immediate type is `Type::Never`). typespace rejects the alias outright
// rather than render a struct that can never serialize or deserialize.
#[test]
fn test_never_type_alias() {
    let builder = typespace_builder!(never_settings(), {
        type NeverAlias = !;

        struct AliasHolder {
            value: NeverAlias,
        }
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "type alias");
    assert_eq!(type_id, "NeverAlias");
}

// Nesting does not change the rule: the outer vec may hold empty inner
// vecs, and nothing deeper than that can exist.
#[test]
fn test_never_in_nested_vec() {
    let builder = typespace_builder!(never_settings(), {
        struct NestedHolder {
            values: Vec<Vec<!>>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_nested_vec.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::NestedHolder { values: Vec::new() };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"values":[]}"#);

        let one_empty = import::NestedHolder {
            values: vec![Vec::new()],
        };
        assert_eq!(
            serde_json::to_string(&one_empty).unwrap(),
            r#"{"values":[[]]}"#
        );

        assert!(serde_json::from_str::<import::NestedHolder>(r#"{"values":[]}"#).is_ok());
        assert!(serde_json::from_str::<import::NestedHolder>(r#"{"values":[[]]}"#).is_ok());
        assert!(serde_json::from_str::<import::NestedHolder>(r#"{"values":[[1]]}"#).is_err());
    }
}

// An optional property whose type is `Vec<!>` may be omitted or an
// empty array.
#[test]
fn test_never_optional_vec() {
    let builder = typespace_builder!(never_settings(), {
        struct OptionalVecHolder {
            values: Optional<Vec<!>>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_optional_vec.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let absent = import::OptionalVecHolder { values: None };
        assert_eq!(serde_json::to_string(&absent).unwrap(), "{}");

        let empty = import::OptionalVecHolder {
            values: Some(Vec::new()),
        };
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"values":[]}"#);

        assert!(serde_json::from_str::<import::OptionalVecHolder>("{}").is_ok());
        assert!(serde_json::from_str::<import::OptionalVecHolder>(r#"{"values":[]}"#).is_ok());
        assert!(serde_json::from_str::<import::OptionalVecHolder>(r#"{"values":[1]}"#).is_err());
    }
}

// Trait resolution reaches `Never` through a container: under
// Settings::typical() a struct with a `Vec<!>` property finalizes and
// derives the whole typical trait set.
#[test]
fn test_never_in_vec_typical_traits() {
    let builder = typespace_builder!(Settings::typical(), {
        struct TypicalVecHolder {
            values: Vec<!>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_in_vec_typical_traits.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let empty = import::TypicalVecHolder {
            values: ::std::vec::Vec::new(),
        };
        let cloned = empty.clone();
        assert_eq!(serde_json::to_string(&cloned).unwrap(), r#"{"values":[]}"#);
        assert!(format!("{empty:?}").contains("TypicalVecHolder"));
    }
}

// A required Display reaches through Box to a leaf that cannot
// implement it, and stops at the container for the containers that
// cannot forward it.
//
// The Box half uses a unit leaf rather than Never: wrapping Never in a
// Box is rejected by validation before trait resolution ever runs (see
// test_never_in_box), so it cannot stand in for "a leaf Display cannot
// reach" here. The unit type shares Never's relevant property for this
// test--CONTAINER_UNSUPPORTED makes it impossible for Display--without
// tripping the wrapper check.
#[test]
fn test_never_under_container_display_conflict() {
    let boxed = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Display),
        {
            struct BoxWrapper(Box<()>);
        }
    );

    let Err(Error::TraitConflicts { conflicts }) = boxed.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // Box declines Display along with the other containers, so the
    // conflict is reported at the box and never reaches the unit leaf.
    assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Display);
    assert!(matches!(conflict.origin, RequirementOrigin::GlobalSettings));
    assert!(matches!(
        &conflict.reason,
        OffenderReason::Primitive { type_name } if type_name == "Box"
    ));

    let vectored = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Display),
        {
            struct VecWrapper(Vec<!>);
        }
    );

    let Err(Error::TraitConflicts { conflicts }) = vectored.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // Vec<T> implements Display for no T, so the conflict is reported
    // at the container rather than passed down to the Never, and it
    // names the container by the path it was configured with.
    assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Display);
    assert!(matches!(
        &conflict.reason,
        OffenderReason::ContainerMissingImpl { type_name }
            if type_name == "::std::vec::Vec"
    ));
}

// Tests for the rejection of `Type::Never` behind a transparent
// wrapper: a `Box`, a type alias, or a `#[serde(transparent)]` newtype
// struct. Each is checked directly against every type in the graph, so
// the tests below need not route the wrapper through a property, or
// even through anything that references it, to trigger the check.

// A type alias for `!` is rejected even when nothing else in the graph
// references the alias.
#[test]
fn test_never_alias_rejected_unreferenced() {
    let builder = typespace_builder!(never_settings(), {
        type NeverAlias = !;
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "type alias");
    assert_eq!(type_id, "NeverAlias");
}

// `Box<!>` is rejected no matter how deeply it sits in the graph:
// `Vec<Box<!>>` is the same meaningless indirection as a bare `Box<!>`.
#[test]
fn test_never_boxed_rejected_when_nested_in_vec() {
    let builder = typespace_builder!(never_settings(), {
        struct VecOfBoxedHolder {
            values: Vec<Box<!>>,
        }
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "Box");
    assert_eq!(type_id, "Box<Never>");
}

// A newtype struct wrapping `!` is rejected even when nothing else in
// the graph references it.
#[test]
fn test_never_newtype_struct_rejected_unreferenced() {
    let builder = typespace_builder!(never_settings(), {
        struct NeverNewtype(!);
    });

    let Err(Error::NeverInTransparentWrapper { wrapper, type_id }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to fail with NeverInTransparentWrapper");
    };
    assert_eq!(wrapper, "newtype struct");
    assert_eq!(type_id, "NeverNewtype");
}

// `#[deny_unknown_fields]` on a plain struct renders as a standalone
// `#[serde(deny_unknown_fields)]` and actually rejects an unrecognized
// field at deserialization.
#[test]
fn test_deny_unknown_fields_struct() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        #[deny_unknown_fields]
        struct Strict {
            name: String,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_deny_unknown_fields_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let v: import::Strict = serde_json::from_str(r#"{"name": "ok"}"#).unwrap();
        assert_eq!(v.name, "ok");
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"name":"ok"}"#);

        match serde_json::from_str::<import::Strict>(r#"{"name": "ok", "extra": 1}"#) {
            Ok(_) => panic!("expected rejection of an unknown field"),
            Err(err) => assert!(err.to_string().contains("unknown field")),
        }
    }
}

// `#[deny_unknown_fields]` on an enum composes with each tag type's own
// `#[serde(..)]` options into a single attribute, rather than emitting
// two. Each tag type takes its own path through `Enum::render`, so all
// four are covered: external (the attribute would otherwise be empty),
// internal and adjacent (which already carry `tag`/`content`), and
// untagged.
#[test]
fn test_deny_unknown_fields_enum_tags() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        #[deny_unknown_fields]
        enum DenyExternal {
            Only { x: u32 },
        }

        #[deny_unknown_fields]
        #[tag = "type"]
        enum DenyInternal {
            Only { x: u32 },
        }

        #[deny_unknown_fields]
        #[tag = "t", content = "c"]
        enum DenyAdjacent {
            Only { x: u32 },
        }

        #[deny_unknown_fields]
        #[untagged]
        enum DenyUntagged {
            Only { x: u32 },
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_deny_unknown_fields_enum_tags.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // External: the tag type contributes no options of its own, so
        // the composed attribute is `#[serde(deny_unknown_fields)]`.
        let v: import::DenyExternal = serde_json::from_str(r#"{"Only": {"x": 1}}"#).unwrap();
        assert!(matches!(v, import::DenyExternal::Only { x: 1 }));
        assert!(
            serde_json::from_str::<import::DenyExternal>(r#"{"Only": {"x": 1, "y": 2}}"#).is_err()
        );

        // Internal: composes with `tag = "type"`.
        let v: import::DenyInternal = serde_json::from_str(r#"{"type": "Only", "x": 1}"#).unwrap();
        assert!(matches!(v, import::DenyInternal::Only { x: 1 }));
        assert!(
            serde_json::from_str::<import::DenyInternal>(r#"{"type": "Only", "x": 1, "y": 2}"#)
                .is_err()
        );

        // Adjacent: composes with `tag = "t", content = "c"`.
        let v: import::DenyAdjacent =
            serde_json::from_str(r#"{"t": "Only", "c": {"x": 1}}"#).unwrap();
        assert!(matches!(v, import::DenyAdjacent::Only { x: 1 }));
        assert!(
            serde_json::from_str::<import::DenyAdjacent>(r#"{"t": "Only", "c": {"x": 1, "y": 2}}"#)
                .is_err()
        );

        // Untagged: composes with `untagged`.
        let v: import::DenyUntagged = serde_json::from_str(r#"{"x": 1}"#).unwrap();
        assert!(matches!(v, import::DenyUntagged::Only { x: 1 }));
        assert!(serde_json::from_str::<import::DenyUntagged>(r#"{"x": 1, "y": 2}"#).is_err());
    }
}

// `deny_unknown_fields` is gated on the type actually deriving
// `Deserialize`: a type that only requires `Serialize` renders no
// `#[serde(..)]` attribute at all, even with the flag set.
#[test]
fn test_deny_unknown_fields_gate() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Serialize);

    let builder = typespace_builder!(settings, {
        #[deny_unknown_fields]
        struct Loose {
            name: String,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_deny_unknown_fields_gate.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // No Deserialize in the trait set, so there is nothing to
        // round-trip; this just proves the Serialize-only type still
        // compiles and serializes normally with the flag set.
        let v = import::Loose {
            name: "ok".to_string(),
        };
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"name":"ok"}"#);
    }
}

// serde's derive macros are what read `#[serde(..)]`, so a type that
// derives neither `Serialize` nor `Deserialize` must carry no such
// attribute: the compiler rejects one with nothing to consume it. One
// graph, reaching every site that emits an attribute, is rendered here
// under all four combinations of the two traits. The snapshot is
// compiled as part of this test, so the four modules existing is the
// proof that each combination produces valid Rust.
#[test]
fn test_serde_trait_combinations() {
    let configs = [
        ("neither", Vec::new()),
        ("serialize_only", vec![TypespaceTrait::Serialize]),
        ("deserialize_only", vec![TypespaceTrait::Deserialize]),
        (
            "both",
            vec![TypespaceTrait::Serialize, TypespaceTrait::Deserialize],
        ),
    ];

    let outputs = configs.into_iter().map(|(name, serde_traits)| {
        // Everything except the two serde traits is held constant, so
        // the four modules differ only in what this test is about.
        let settings = serde_traits.into_iter().fold(
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Debug),
            Settings::with_required_trait,
        );

        let builder = typespace_builder!(settings, {
            struct Inner {
                value: u32,
            }

            // Every property state that pushes an option: rename,
            // flatten, the optional/nullable spread, the skips that
            // come with `default`, a default value with a generated
            // function, and a property that must be absent.
            struct Outer {
                #[rename = "my-field"]
                my_field: String,
                #[flatten]
                inner: Inner,
                maybe: Optional<String>,
                nullable: Nullable<String>,
                maybe_nullable: OptionalNullable<String>,
                #[default]
                tags: Vec<String>,
                #[default]
                flag: bool,
                #[default]
                nothing: (),
                #[default = "peanuts"]
                peanut: String,
                never: Optional<!>,
            }

            // Kept clear of `Outer`: serde rejects
            // `deny_unknown_fields` alongside a flattened field.
            #[deny_unknown_fields]
            struct Strict {
                name: String,
            }

            // Newtype struct: `transparent`.
            struct Wrapper(String);

            // Unit struct and tuple struct: hand-written serde impls
            // rather than derives, and so no attributes either way.
            #[json = "<<marker>>"]
            struct Marker;

            struct Pair(String, u32);

            enum External {
                Unit,
                Payload(String),
                Fields {
                    #[rename = "cee"]
                    c: u32,
                },
            }

            #[tag = "type"]
            enum Internal {
                X { x: u32 },
                Y { y: u32 },
            }

            #[tag = "t", content = "c"]
            #[deny_unknown_fields]
            enum Adjacent {
                P(String),
                Q(u32),
            }

            #[untagged]
            enum Untagged {
                S(String),
                N(u32),
            }

            enum Renamed {
                #[json = "one"]
                One,
                #[json = "two"]
                Two,
            }

            type Alias = Vec<String>;
        });

        let ts = builder.finalize(no_cycles).unwrap();

        (name, ts.to_codespace())
    });

    let mut codespace = Codespace::default();

    for (name, sub_codespace) in outputs {
        codespace
            .get_root_mod()
            .replace_mod(name, sub_codespace.into_root_mod());
    }

    let out = codespace.into_stream();

    #[check_and_include("tests/output/test_serde_trait_combinations.rs", out)]
    fn inner() {
        // Both traits: the attributes do their job in both directions.
        let v: import::both::Outer =
            serde_json::from_str(r#"{"my-field": "hi", "value": 7, "nullable": null}"#).unwrap();
        assert_eq!(v.my_field, "hi");
        assert_eq!(v.inner.value, 7);
        assert_eq!(v.peanut, "peanuts");
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["my-field"], "hi");
        assert_eq!(json["value"], 7);
        assert_eq!(json.get("tags"), None);

        // Serialize only: the deserialize-side options ride along
        // inert, and serialization is unaffected by them.
        let v = import::serialize_only::Outer {
            my_field: "hi".to_string(),
            inner: import::serialize_only::Inner { value: 7 },
            maybe: None,
            nullable: None,
            maybe_nullable: None,
            tags: Vec::new(),
            flag: false,
            nothing: (),
            peanut: "peanuts".to_string(),
            never: ::json_serde::Absent,
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["my-field"], "hi");
        assert_eq!(json["value"], 7);

        // Deserialize only: the serialize-side options ride along
        // inert, and deserialization is unaffected by them.
        let v: import::deserialize_only::Outer =
            serde_json::from_str(r#"{"my-field": "hi", "value": 7, "nullable": null}"#).unwrap();
        assert_eq!(v.my_field, "hi");
        assert_eq!(v.peanut, "peanuts");

        // Neither: no derive, so no attribute. The module compiling is
        // the whole point; this just reaches into it.
        let v = import::neither::Wrapper("x".to_string());
        assert_eq!(v.0, "x");
    }
}

// Per-type `#[derive = [..]]` renders alongside the computed traits and
// the crate-wide `with_derive` list: computed traits first, then the
// crate-wide derives, then the per-type ones last.
#[test]
fn test_extra_derives_ordering() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::Clone)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Eq)
        .with_derive("::std::hash::Hash");

    let builder = typespace_builder!(settings, {
        #[derive = ["PartialOrd"]]
        struct Widget {
            x: u32,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_extra_derives_ordering.rs", ts.to_codespace().into_stream())]
    fn inner() {
        // Hash comes from the crate-wide with_derive; PartialOrd is the
        // per-type extra, additional to it.
        let a = import::Widget { x: 1 };
        let b = import::Widget { x: 2 };
        assert!(a < b);
        let mut set = std::collections::HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&a));
    }
}

// Per-type `#[derive = [..]]` on each named-type shape: a struct, an
// enum, a newtype, a multi-field tuple struct, and a unit struct, each
// with its own render path. The crate-wide `with_derive` list still
// precedes the per-type one on every shape.
#[test]
fn test_extra_derives_multi_shape() {
    let settings = Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::Clone)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_derive("::std::hash::Hash");

    let builder = typespace_builder!(settings, {
        #[derive = ["PartialOrd"]]
        struct ShapeStruct {
            x: u32,
        }

        #[derive = ["PartialOrd"]]
        enum ShapeEnum {
            Only(u32),
        }

        #[derive = ["PartialOrd"]]
        struct ShapeNewtype(u32);

        #[derive = ["PartialOrd"]]
        struct ShapeTuple(u32, u32);

        #[json = "unit-shape"]
        #[derive = ["PartialOrd"]]
        struct ShapeUnit;
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_extra_derives_multi_shape.rs", ts.to_codespace().into_stream())]
    fn inner() {
        assert!(import::ShapeStruct { x: 1 } < import::ShapeStruct { x: 2 });
        assert!(import::ShapeEnum::Only(1) < import::ShapeEnum::Only(2));
        assert!(import::ShapeNewtype(1) < import::ShapeNewtype(2));
        assert!(import::ShapeTuple(1, 0) < import::ShapeTuple(1, 1));
        assert!(import::ShapeUnit <= import::ShapeUnit);
    }
}

// A fieldless enum has nothing to check Copy against, so a desired
// Copy is granted whole, and Clone--its supertrait--rides along even
// though nothing asked for it directly.
#[test]
fn test_copy_desired_fieldless_enum_gets_copy_and_clone() {
    let builder = typespace_builder!(
        Settings::minimal().with_desired_trait(TypespaceTrait::Copy),
        {
            enum Color {
                Red,
                Green,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let mut derives = common::derives_of(&file, "Color");
    derives.sort();
    assert_eq!(derives, ["Clone", "Copy"]);
}

// A struct holding a String can never be Copy: the derive requires
// every field to be Copy, and a String owns a heap buffer. Clone is
// required directly here (not merely desired as Copy's supertrait) so
// it survives on its own account, isolating the case: Copy alone drops
// out of the desired phase.
#[test]
fn test_copy_desired_struct_with_string_field_drops_copy() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Clone)
            .with_desired_trait(TypespaceTrait::Copy),
        {
            struct Widget {
                name: String,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    assert_eq!(common::derives_of(&file, "Widget"), ["Clone"]);
}

// Requiring Copy directly--not merely desiring it--brings Clone along
// as well, because the supertrait closure runs over required traits
// too: an emitted `derive(Copy)` with no `Clone` would not compile.
#[test]
fn test_copy_required_brings_clone() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Copy),
        {
            enum Flag {
                On,
                Off,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let mut derives = common::derives_of(&file, "Flag");
    derives.sort();
    assert_eq!(derives, ["Clone", "Copy"]);
}

// A newtype over an integer and an all-unit-variant enum both earn
// Copy, and `Settings::maximal` desires it. The snapshot puts the
// emitted `derive(Copy)` through the compiler, which an assertion on
// the derive list alone cannot do.
#[test]
fn test_copy_desired_renders_and_compiles() {
    let builder = typespace_builder!(Settings::maximal(), {
        struct Port(u32);

        enum Color {
            Red,
            Green,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    assert!(
        common::derives_of(&file, "Port").contains(&"Copy".to_string()),
        "Port did not earn Copy"
    );
    assert!(
        common::derives_of(&file, "Color").contains(&"Copy".to_string()),
        "Color did not earn Copy"
    );

    #[check_and_include(
        "tests/output/test_copy_desired_renders_and_compiles.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        fn assert_copy<T: Copy>(_value: T) {}

        assert_copy(import::Port(8080));
        assert_copy(import::Color::Red);
    }
}

// The other half of the rule the test above pins: under typify_compat,
// `Copy` survives only on the form typify itself grants it to. typify
// extends its derive set with Copy in `output_enum` alone, and only
// when every variant is payload-free; `output_struct` and
// `output_newtype` never name it, however Copy-eligible their contents
// are.
#[test]
fn test_copy_withheld_under_typify_compat() {
    let builder = typespace_builder!(Settings::maximal().with_typify_compat(true), {
        struct Port(u32);

        enum Color {
            Red,
            Green,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        !common::derives_of(&file, "Port").contains(&"Copy".to_string()),
        "Port kept Copy under typify_compat, where typify grants it to \
         no newtype"
    );
    assert!(
        common::derives_of(&file, "Color").contains(&"Copy".to_string()),
        "Color lost Copy under typify_compat, where typify grants it to \
         every all-unit-variant enum"
    );
}

// TYPIFY COMPAT: typify never skips serialization of a default-state
// String, so imitation withholds the `is_empty` attribute; a Vec in
// the same state keeps its skip, which typify also emits.
#[test]
fn test_string_default_skip_withheld_under_typify_compat() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_typify_compat(true),
        {
            struct Package {
                #[default]
                label: String,
                #[default]
                tags: Vec<String>,
            }
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();
    let rendered = ts.to_codespace().into_stream().to_string();
    assert!(
        !rendered.contains("String::is_empty"),
        "the String skip leaked under typify_compat"
    );
    assert!(
        rendered.contains("is_empty"),
        "the Vec skip was lost with it"
    );
}

// Neither a Box nor a serde_json::Value is Copy, whatever it holds, so
// a desired Copy drops at a type holding either. Clone is required
// directly, as in the String case above, so a surviving derive
// distinguishes "Copy dropped" from "nothing was granted at all".
#[test]
fn test_copy_desired_box_and_json_value_drop_copy() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_typify_compat(true)
            .with_required_trait(TypespaceTrait::Clone)
            .with_desired_trait(TypespaceTrait::Copy)
            .with_desired_trait(TypespaceTrait::FromStr)
            .with_desired_trait(TypespaceTrait::Display),
        {
            struct Boxed(Box<u32>);

            struct Blob {
                data: JsonValue,
            }

            struct Blob2(JsonValue);
        }
    );

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    assert_eq!(common::derives_of(&file, "Boxed"), ["Clone"]);
    assert_eq!(common::derives_of(&file, "Blob"), ["Clone"]);
    assert_eq!(common::derives_of(&file, "Blob2"), ["Clone"]);

    assert_eq!(common::impls_of(&file, "Boxed"), ["Deref", "From<Box>"]);
    assert!(common::impls_of(&file, "Blob").is_empty());
    assert_eq!(common::impls_of(&file, "Blob2"), ["Deref", "From<Value>"]);
}

#[test]
fn test_struct_builder() {
    let builder = typespace_builder!(Settings::maximal(), {
        struct MyStruct {
            a: String,
            b: Optional<u32>,
            c: Nullable<String>,
            #[default = 42]
            d: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include("tests/output/test_struct_builder.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let builder = import::builder::MyStruct::default();
        assert!(import::MyStruct::try_from(builder).is_err());
        let builder = import::builder::MyStruct::default().a("howdy");
        assert!(import::MyStruct::try_from(builder).is_err());
        let builder = import::builder::MyStruct::default().c(Some("hello".into()));
        assert!(import::MyStruct::try_from(builder).is_err());

        let instance: import::MyStruct = import::builder::MyStruct::default()
            .a("howdy")
            .c(None)
            .try_into()
            .unwrap();
        assert_eq!(
            instance,
            import::MyStruct {
                a: "howdy".into(),
                b: None,
                c: None,
                d: 42
            }
        );

        let instance: import::MyStruct = import::builder::MyStruct::default()
            .a("howdy")
            .b(Some(100))
            .c(Some("there".into()))
            .d(200)
            .try_into()
            .unwrap();
        assert_eq!(
            instance,
            import::MyStruct {
                a: "howdy".into(),
                b: Some(100),
                c: Some("there".into()),
                d: 200
            }
        );
        let builder = import::builder::MyStruct::from(instance.clone());
        assert_eq!(instance, builder.try_into().unwrap());
    }
}

// A `#[tuple]`-forced single-field tuple struct carries its own derives
// and attributes like every other shape; its lowering is the one branch
// that reaches TupleStruct with a single field.
#[test]
fn test_tuple_marker_extras() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Debug)
            .with_required_trait(TypespaceTrait::PartialEq),
        {
            #[tuple]
            #[derive = ["PartialOrd"]]
            struct Listed(u32);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include("tests/output/test_tuple_marker_extras.rs", ts.to_codespace().into_stream())]
    fn inner() {
        assert!(import::Listed(1) < import::Listed(2));
    }
}
// The tests below cover how `Default` reaches rendered code: as a derive,
// as a hand-written impl, or not at all.

// The settings these tests share: `Default` desired so it lands on every
// type that can implement it, plus the traits the assertions need.
// `Display` and `FromStr` stay out because a newtype that has either in
// its trait set panics in render.
// A generated default function is named for its containing type path
// and the property, snake-cased: the enum name, then the variant name,
// then the property for a struct variant, and the struct name then the
// property for a struct. Snake-casing the joined name is what keeps
// `non_snake_case` quiet over a CamelCase type path.
#[test]
fn test_default_fn_names_are_snake_case() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        enum DensityDistribution {
            NormalDist {
                #[default = 1.5]
                stdev: f64,
            },
            UniformDist {
                #[default = 2.5]
                max_value: f64,
            },
        }

        struct ForceTransform {
            #[default = 0.5]
            alpha_min: f64,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    // The name carries the enum once, not twice, and the variant name
    // and property arrive snake-cased.
    let rendered = ts.to_codespace().into_stream().to_string();
    for name in [
        "density_distribution_normal_dist_stdev",
        "density_distribution_uniform_dist_max_value",
        "force_transform_alpha_min",
    ] {
        assert!(rendered.contains(name), "missing {name} in:\n{rendered}");
    }

    #[check_and_include(
        "tests/output/test_default_fn_names_are_snake_case.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        let value =
            serde_json::from_str::<import::DensityDistribution>(r#"{"NormalDist":{}}"#).unwrap();
        assert_eq!(
            value,
            import::DensityDistribution::NormalDist { stdev: 1.5 }
        );

        let value = serde_json::from_str::<import::ForceTransform>("{}").unwrap();
        assert_eq!(value, import::ForceTransform { alpha_min: 0.5 });
    }
}

fn default_settings() -> Settings {
    Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_desired_trait(TypespaceTrait::Default)
}

/// Every property optional or in the `Default` state: `Default` is derived.
///
/// A hand-written impl would be exactly what the derive produces, so the
/// trait stays in the derive list and no impl is emitted.
#[test]
fn test_default_derived_all_defaulted() {
    let builder = typespace_builder!(default_settings(), {
        struct AllDefaulted {
            maybe: Optional<String>,
            #[default]
            count: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_derived_all_defaulted.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::AllDefaulted::default(),
            import::AllDefaulted {
                maybe: None,
                count: 0,
            }
        );
        assert_eq!(
            serde_json::from_str::<import::AllDefaulted>("{}").unwrap(),
            import::AllDefaulted::default()
        );
    }
}

/// A property with its own default value: `Default` is hand written.
///
/// The derive would ignore the attached value, so the trait comes out of
/// the derive list and the impl takes that property from the generated
/// `defaults::` function and every other property from
/// `Default::default()`.
#[test]
fn test_default_impl_from_property_value() {
    let builder = typespace_builder!(default_settings(), {
        struct WithDefaultValue {
            #[default = 42]
            answer: u32,
            #[default]
            name: String,
            maybe: Optional<bool>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_impl_from_property_value.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::WithDefaultValue::default(),
            import::WithDefaultValue {
                answer: 42,
                name: String::new(),
                maybe: None,
            }
        );
    }
}

/// A property default value across every kind the walk in `default.rs`
/// handles: native, JSON value, float, newtype, type alias, `NonZero`
/// integer, and option.
///
/// `test_default_impl_from_property_value` above already covers a
/// plain integer; this rounds out the rest from the property side.
#[test]
fn test_default_value_property_kinds() {
    let builder = typespace_builder!(default_settings(), {
        native ::std::net::IpAddr: Clone + Debug + PartialEq + Serialize + Deserialize;

        type Count = u32;

        struct Wrapped(u32);

        struct PropertyDefaults {
            #[default = "127.0.0.1"]
            address: ::std::net::IpAddr,
            #[default = { "a": [8, 6, 7] }]
            blob: JsonValue,
            #[default = 1.5]
            weight: f64,
            #[default = 7]
            wrapped: Wrapped,
            #[default = 7]
            count: Count,
            #[default = 1]
            nz: NonZeroU64,
            #[default = 5]
            maybe: Nullable<u32>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    // Every property takes its value from a generated function, so the
    // struct's own Default calls each of them, and an empty object
    // deserializes to the same thing.
    #[check_and_include(
        "tests/output/test_default_value_property_kinds.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::PropertyDefaults::default(),
            serde_json::from_str::<import::PropertyDefaults>("{}").unwrap()
        );
        assert_eq!(
            serde_json::from_str::<import::PropertyDefaults>("{}").unwrap(),
            import::PropertyDefaults {
                address: "127.0.0.1".parse().unwrap(),
                blob: serde_json::json!({ "a": [8, 6, 7] }),
                weight: 1.5,
                wrapped: import::Wrapped(7),
                count: 7,
                nz: ::std::num::NonZeroU64::new(1).unwrap(),
                maybe: Some(5),
            }
        );
    }
}

/// One property whose default value a shared function produces: the
/// `defaults` module holds the generic `default_bool` and nothing else.
#[test]
fn test_default_value_shared_fn_alone() {
    let builder = typespace_builder!(default_settings(), {
        struct Switch {
            #[default = true]
            on: bool,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_shared_fn_alone.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(import::Switch::default(), import::Switch { on: true });
        assert_eq!(
            serde_json::from_str::<import::Switch>("{}").unwrap(),
            import::Switch { on: true }
        );
    }
}

/// Boolean and integer default values, which shared functions produce,
/// alongside default values that need a function of their own.
///
/// Every property of `Alpha` and `Beta` defaults, so between them the
/// pair covers each shared function, a type that mixes shared and
/// per-property functions, and two properties in different types that
/// name the same instantiation (`count`, in both). That the snapshot
/// holds one definition of each shared function is what compiling it
/// proves: a second definition of any of them is a duplicate name.
#[test]
fn test_default_value_shared_fns_across_types() {
    let builder = typespace_builder!(default_settings(), {
        struct Alpha {
            #[default = 7]
            count: u32,
            #[default = -3]
            offset: i32,
            #[default = true]
            flag: bool,
            #[default = 2]
            little: NonZeroU8,
            #[default = -3]
            dip: NonZeroI32,
            #[default = "hi"]
            label: String,
        }

        struct Beta {
            #[default = 7]
            count: u32,
            #[default = 9]
            other: u32,
            #[default = false]
            flag: bool,
            #[default = 1.5]
            weight: f64,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_shared_fns_across_types.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::Alpha>("{}").unwrap(),
            import::Alpha {
                count: 7,
                offset: -3,
                flag: true,
                little: ::std::num::NonZeroU8::new(2).unwrap(),
                dip: ::std::num::NonZeroI32::new(-3).unwrap(),
                label: "hi".to_string(),
            }
        );
        assert_eq!(
            import::Alpha::default(),
            serde_json::from_str::<import::Alpha>("{}").unwrap()
        );
        assert_eq!(
            serde_json::from_str::<import::Beta>("{}").unwrap(),
            import::Beta {
                count: 7,
                other: 9,
                flag: false,
                weight: 1.5,
            }
        );
        assert_eq!(
            import::Beta::default(),
            serde_json::from_str::<import::Beta>("{}").unwrap()
        );
    }
}

/// A `Nullable<T>` property (no `Optional` half) whose default value is
/// `null`: the generated default function returns `None`, the other
/// half of the pairing `test_default_value_property_kinds` covers with
/// a present value.
#[test]
fn test_default_null_value_nullable_property() {
    let builder = typespace_builder!(default_settings(), {
        struct NullDefault {
            #[default = null]
            maybe: Nullable<u32>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_null_value_nullable_property.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::NullDefault::default(),
            import::NullDefault { maybe: None }
        );
        assert_eq!(
            serde_json::from_str::<import::NullDefault>("{}").unwrap(),
            import::NullDefault::default()
        );
    }
}

/// A property whose default value is itself a struct-shaped literal.
///
/// The generated `defaults::` function body constructs `Inner` by name;
/// that function lives in the `defaults` submodule, so the name needs
/// the walk's `super::` scope. This is the only place in `default.rs`
/// that renders a plain struct's own name, so a scoping bug there (a
/// bare `format_ident!` instead of `render_ident`) would compile-fail
/// silently until something exercised it.
#[test]
fn test_default_value_property_struct_kind() {
    let builder = typespace_builder!(default_settings(), {
        struct Inner {
            x: u32,
        }

        struct PropertyStructDefault {
            #[default = { "x": 5 }]
            inner: Inner,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_property_struct_kind.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::PropertyStructDefault>("{}").unwrap(),
            import::PropertyStructDefault {
                inner: import::Inner { x: 5 },
            }
        );
    }
}

/// A required property: no `Default` derive and no `Default` impl.
///
/// The schema says the property must be supplied, so there is no honest
/// value for the trait to hand back.
#[test]
fn test_default_impossible_required_property() {
    let builder = typespace_builder!(default_settings(), {
        struct HasRequired {
            required: String,
            #[default]
            count: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_impossible_required_property.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        // The property that blocks `Default` is the same one that has no
        // serde default: an empty object does not deserialize.
        assert!(serde_json::from_str::<import::HasRequired>("{}").is_err());
        assert_eq!(
            serde_json::from_str::<import::HasRequired>(r#"{"required":"x"}"#).unwrap(),
            import::HasRequired {
                required: "x".to_string(),
                count: 0,
            }
        );
    }
}

/// `Default` on the struct shapes that carry no property states.
///
/// All three answer `IfAllChildren` and pick the trait up as a derive,
/// since none of them carries an attached default value. A newtype that
/// does carry one is the separate case below.
#[test]
fn test_default_other_struct_shapes() {
    let builder = typespace_builder!(default_settings(), {
        struct NewtypeShape(String);

        #[json = "unit"]
        struct UnitShape;

        struct TupleShape(String, u32);
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_other_struct_shapes.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            import::TupleShape::default(),
            import::TupleShape(String::new(), 0)
        );
        assert_eq!(import::UnitShape, import::UnitShape);
        assert_eq!(
            import::NewtypeShape::from("x".to_string()),
            import::NewtypeShape("x".to_string())
        );
        assert_eq!(
            import::NewtypeShape::default(),
            import::NewtypeShape(String::new())
        );
    }
}

/// A newtype struct with an attached default value renders a `Default`
/// impl that constructs the value.
///
/// The other shapes derive `Default` from their contents. A newtype
/// with a value of its own cannot: a derive would produce the inner
/// type's default and quietly ignore what was asked for. So the value
/// has to become a hand-written impl, which is what
/// `NewtypeStruct::render` writes.
#[test]
fn newtype_with_an_attached_default_renders_the_impl() {
    let builder = typespace_builder!(default_settings(), {
        #[default = 3]
        struct Count(u32);
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);
    assert!(
        rendered.contains("impl ::std::default::Default for Count"),
        "no Default impl for Count in:\n{rendered}"
    );
    assert!(
        rendered.contains("Count(3_u32)"),
        "the impl does not construct the attached value in:\n{rendered}"
    );
    // The derive would ignore the value, so it must not also appear.
    let derive_line = rendered
        .lines()
        .find(|line| line.contains("derive") && line.contains("Count"))
        .unwrap_or("");
    assert!(
        !derive_line.contains("Default"),
        "Count both derives and hand-writes Default: {derive_line}"
    );
}

/// The impl is withheld when constructing the value would need
/// something the value's own contents cannot provide.
///
/// Rendering the value is what creates the need: a native inside it is
/// built by deserializing, so the value demands `Deserialize` of that
/// native. A native that does not declare it leaves the demand unmet,
/// and `Default` is merely desired here, so it drops rather than
/// failing the build.
#[test]
fn default_is_withheld_when_the_value_needs_an_unmet_trait() {
    // Nothing global demands Deserialize here, so the only thing
    // asking it of the native is the default value's construction.
    let settings = Settings::minimal().with_desired_trait(TypespaceTrait::Default);
    let builder = typespace_builder!(settings, {
        native ::ext::Opaque: Clone;
        native ::ext::Readable: Clone + Deserialize;

        #[default = "whatever"]
        struct Wrapped(::ext::Opaque);

        #[default = { x: "whatever" }]
        struct Wrapped2{ x: ::ext::Opaque }

        #[default = "whatever"]
        struct Fine(::ext::Readable);

        #[default = { x: "whatever" }]
        struct Fine2{ x: ::ext::Readable }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);

    assert!(
        !rendered.contains("Default for Wrapped"),
        "Wrapped got a Default impl the value cannot construct:\n{rendered}"
    );
    assert!(
        rendered.contains("Default for Fine2"),
        "Fine2 met the obligation and still got no Default impl:\n{rendered}"
    );
    assert!(
        !rendered.contains("Default for Wrapped2"),
        "Wrapped2 got a Default impl the value cannot construct:\n{rendered}"
    );
}

/// `Default` on the struct shapes that carry no property states, under
/// `typify_compat`.
///
/// All three shapes answer `Impossible`, matching typify1: no derive and
/// no impl for any of them.
#[test]
fn test_default_other_struct_shapes_typify_compat() {
    let builder = typespace_builder!(default_settings().with_typify_compat(true), {
        struct NewtypeShape(String);

        #[json = "unit"]
        struct UnitShape;

        struct TupleShape(String, u32);
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_other_struct_shapes_typify_compat.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        // TupleShape and UnitShape are neither of typify's two
        // comparison-derive exceptions, so under typify_compat they no
        // longer derive PartialEq; only construction is checked here.
        let tuple_shape = import::TupleShape("a".to_string(), 7);
        assert_eq!(tuple_shape.0, "a");
        assert_eq!(tuple_shape.1, 7);
        assert_eq!(format!("{:?}", import::UnitShape), "UnitShape");
        // NewtypeShape wraps String, so it keeps PartialEq.
        assert_eq!(
            import::NewtypeShape::from("x".to_string()),
            import::NewtypeShape("x".to_string())
        );
    }
}

/// The five comparison traits, desired on every type the way typify
/// asks for them, but withheld at render by `typify_compat` outside its
/// two exceptions.
///
/// `AllUnit` (every variant a unit variant) and `StringWrapper` (a
/// newtype over `String`) keep `Eq`, `PartialEq`, `Ord`, `PartialOrd`,
/// and `Hash` in their derive lists. `Ordinary` (a struct) and
/// `IntWrapper` (a newtype over something other than `String`) resolve
/// the same five traits--every field involved supports all of
/// them--but typify never derives them there, so typify_compat trims
/// them from the rendered list.
#[test]
fn test_comparison_derives_typify_compat_on() {
    let settings = Settings::minimal()
        .with_typify_compat(true)
        .with_desired_trait(TypespaceTrait::Eq)
        .with_desired_trait(TypespaceTrait::PartialEq)
        .with_desired_trait(TypespaceTrait::Ord)
        .with_desired_trait(TypespaceTrait::PartialOrd)
        .with_desired_trait(TypespaceTrait::Hash);

    let builder = typespace_builder!(settings, {
        enum AllUnit {
            First,
            Second,
        }

        struct StringWrapper(String);

        struct IntWrapper(u32);

        struct Ordinary {
            value: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    let comparison_traits = ["Eq", "PartialEq", "Ord", "PartialOrd", "Hash"];
    for name in ["AllUnit", "StringWrapper"] {
        let derives = common::derives_of(&file, name);
        for trait_name in comparison_traits {
            assert!(
                derives.iter().any(|d| d == trait_name),
                "{name} should keep {trait_name}: {derives:?}"
            );
        }
    }
    for name in ["Ordinary", "IntWrapper"] {
        let derives = common::derives_of(&file, name);
        for trait_name in comparison_traits {
            assert!(
                !derives.iter().any(|d| d == trait_name),
                "{name} should not derive {trait_name}: {derives:?}"
            );
        }
    }
}

/// The same graph as [`test_comparison_derives_typify_compat_on`], with
/// `typify_compat` off: nothing narrows the derive list, so every type
/// keeps all five comparison traits.
#[test]
fn test_comparison_derives_typify_compat_off() {
    let settings = Settings::minimal()
        .with_desired_trait(TypespaceTrait::Eq)
        .with_desired_trait(TypespaceTrait::PartialEq)
        .with_desired_trait(TypespaceTrait::Ord)
        .with_desired_trait(TypespaceTrait::PartialOrd)
        .with_desired_trait(TypespaceTrait::Hash);

    let builder = typespace_builder!(settings, {
        enum AllUnit {
            First,
            Second,
        }

        struct StringWrapper(String);

        struct IntWrapper(u32);

        struct Ordinary {
            value: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    let comparison_traits = ["Eq", "PartialEq", "Ord", "PartialOrd", "Hash"];
    for name in ["AllUnit", "StringWrapper", "Ordinary", "IntWrapper"] {
        let derives = common::derives_of(&file, name);
        for trait_name in comparison_traits {
            assert!(
                derives.iter().any(|d| d == trait_name),
                "{name} should keep {trait_name}: {derives:?}"
            );
        }
    }
}

/// An enum carrying a whole-type default value renders no `Default`.
///
/// `feasibility` answers `IfSomeChildren` for such an enum, so
/// `Default` is in its trait set, but nothing writes the impl.
#[test]
fn test_default_enum_with_default_value() {
    let builder = typespace_builder!(default_settings(), {
        #[default = "Red"]
        enum Color {
            Red,
            Green,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_enum_with_default_value.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::Color>(r#""Red""#).unwrap(),
            import::Color::Red
        );
    }
}

/// A whole-type default value alongside a required property.
///
/// The default value supplies the required property, so the type can
/// implement `Default` and `feasibility` grants it. The impl it should
/// get is the one typify writes whenever a type carries its own default
/// value: the body is that value walked as a literal, with no reference
/// to any property's own default.
///
/// ```ignore
/// impl ::std::default::Default for WholeDefault {
///     fn default() -> Self {
///         Self {
///             a: "x".to_string(),
///             b: Some(7),
///         }
///     }
/// }
/// ```
///
/// `Struct::render` writes typify's other impl instead, the one built
/// from each property's `DefaultConstructor`. A required property's
/// constructor is `DefaultConstructor::None`, which that code maps to
/// `unreachable!()`, so rendering this graph panics. The value walk the
/// correct body needs belongs in `default.rs`.
#[test]
fn test_default_whole_type_value_with_required_property() {
    let builder = typespace_builder!(default_settings(), {
        #[default = { "a": "x", "b": 7 }]
        struct WholeDefault {
            a: String,
            b: Optional<u32>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);

    assert!(
        rendered.contains("impl ::std::default::Default for WholeDefault"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("a: Default::default()"),
        "the default value supplies `a`; the impl must not fall back to \
         Default::default(): {rendered}"
    );
}

// The tests below cover the value walk in `default.rs`. Only its check
// half is reachable from here: `finalize` validates a type's own
// default value, and nothing renders one yet.

/// A default value that fits its type survives `finalize`.
///
/// One type per shape the walk handles: a newtype struct over an alias,
/// a native type, a JSON value, and a float. A unit struct's value
/// expression is covered separately by `test_unit_struct` in
/// `default.rs`, since a unit struct has no builder method to carry a
/// default value.
#[test]
fn test_default_value_shapes_accepted() {
    let builder = typespace_builder!(default_settings(), {
        native ::std::net::IpAddr: Clone + Debug + PartialEq + Serialize + Deserialize;

        type Count = u32;

        #[default = 3]
        struct Counted(Count);

        #[default = "127.0.0.1"]
        struct Addressed(::std::net::IpAddr);

        #[default = { "a": [8, 6, 7] }]
        struct Blob(JsonValue);

        #[default = 1.5]
        struct Weight(f64);

        #[json = "marker"]
        struct Marker;
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_shapes_accepted.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(import::Counted::from(3), import::Counted(3));
        assert_eq!(import::Weight::from(1.5), import::Weight(1.5));
        assert_eq!(
            serde_json::from_str::<import::Marker>(r#""marker""#).unwrap(),
            import::Marker
        );

        assert_eq!(import::Counted::default(), import::Counted(3));
        assert_eq!(import::Weight::default(), import::Weight(1.5));
        assert_eq!(
            import::Addressed::default(),
            import::Addressed("127.0.0.1".parse().unwrap())
        );
        // Written the long way because clippy objects to
        // `Marker::default()` on a unit struct, and the point here is
        // that the impl exists at all.
        assert_eq!(<import::Marker as Default>::default(), import::Marker);
    }
}

/// A native type's default value is taken on faith.
///
/// Nothing here knows what the type's own `Deserialize` accepts, so an
/// unusable value reaches the generated code rather than `finalize`.
#[test]
fn test_default_value_native_unchecked() {
    let builder = typespace_builder!(default_settings(), {
        native ::std::net::IpAddr: Clone + Debug + PartialEq + Serialize + Deserialize;

        #[default = { "not": "an address" }]
        struct Addressed(::std::net::IpAddr);
    });
    assert!(builder.finalize(no_cycles).is_ok());
}

/// A default value for a native-typed position is constructed in
/// generated code by deserializing it, so the value demands
/// Deserialize of the native. A native declared without it must
/// conflict at finalize rather than render a Default impl that cannot
/// work.
#[test]
fn native_default_value_requires_deserialize() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Default),
        {
            native chrono::NaiveDate: Clone + Debug + Serialize;

            #[default = { "when": "2024-01-01" }]
            struct Config {
                when: chrono::NaiveDate,
            }
        }
    );

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("a native default value demands Deserialize of the native");
    };
    assert_eq!(conflicts.len(), 1, "{conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Deserialize);
    assert!(matches!(
        &conflict.reason,
        OffenderReason::NativeMissingImpl { type_name } if type_name == "chrono::NaiveDate"
    ));
}

/// A float rejects a value that is not a number.
#[test]
fn test_default_value_float_rejects_non_number() {
    let builder = typespace_builder!(default_settings(), {
        #[default = "1.5"]
        struct Weight(f64);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An integer rejects a value that is not a whole number.
///
/// `as_number` accepts a float, so a value like 1.5 reaches rendering
/// and becomes the literal `1.5_u32`, which the consumer's build
/// refuses.
#[test]
fn test_default_value_integer_rejects_fraction() {
    let builder = typespace_builder!(default_settings(), {
        #[default = 1.5]
        struct Count(u32);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A `NonZero` integer rejects zero.
///
/// Nothing checks the value against the range its type accepts, so
/// zero reaches rendering and becomes
/// `NonZeroU64::new(0).unwrap()`, which panics when the consumer asks
/// for the default.
#[test]
fn test_default_value_nonzero_rejects_zero() {
    let builder = typespace_builder!(default_settings(), {
        #[default = 0]
        struct Count(NonZeroU64);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An unsigned integer rejects a negative value.
///
/// A negative value renders as `defaults::default_i64::<u32, -5>`,
/// whose `try_from` panics when the consumer asks for the default.
#[test]
fn test_default_value_unsigned_rejects_negative() {
    let builder = typespace_builder!(default_settings(), {
        #[default = -5]
        struct Count(u32);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An integer rejects a value its width cannot hold.
///
/// A value past the end of the type's range renders as the literal
/// `300_u8`, which the consumer's build refuses.
#[test]
fn test_default_value_integer_rejects_out_of_range() {
    let builder = typespace_builder!(default_settings(), {
        #[default = 300]
        struct Small(u8);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A float rejects a value its width cannot hold.
///
/// The float arm checks that the value is a number and then writes it
/// with the type as a suffix, so a value past the end of the type's
/// range renders as the literal `1e300_f32`, which the consumer's
/// build refuses.
#[test]
fn test_default_value_float_rejects_out_of_range() {
    let builder = typespace_builder!(default_settings(), {
        #[default = 1e300]
        struct Small(f32);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A newtype struct's default value is checked against its inner type.
///
/// The inner type here is an alias, so the alias has to forward for the
/// mismatch to be found at all.
#[test]
fn test_default_value_newtype_rejects_inner_mismatch() {
    let builder = typespace_builder!(default_settings(), {
        type Count = u32;

        #[default = "3"]
        struct Counted(Count);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

// The tests below cover the container and enum arms the value walk in
// `default.rs` gained alongside this comment: `Vec`, `Map`, `Set`,
// `Array`, `Tuple`, `TupleStruct`, and the internal/adjacent/untagged
// (and external-with-a-payload) enum tag types.

/// A property default value across every container kind: `Vec`, `Map`
/// (string-keyed), `Set`, a fixed-size array, and a tuple.
#[test]
fn test_default_value_container_kinds() {
    let builder = typespace_builder!(default_settings(), {
        struct ContainerDefaults {
            #[default = [8, 6, 7]]
            numbers: Vec<u32>,
            #[default = { "a": 1, "b": 2 }]
            counts: Map<String, u32>,
            #[default = [1, 2, 3]]
            tags: Set<u32>,
            #[default = [1, 2, 3]]
            fixed: [u32; 3],
            #[default = ["x", 1]]
            pair: (String, u32),
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_container_kinds.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::ContainerDefaults>("{}").unwrap(),
            import::ContainerDefaults {
                numbers: vec![8, 6, 7],
                counts: [("a".to_string(), 1), ("b".to_string(), 2)]
                    .into_iter()
                    .collect(),
                tags: vec![1, 2, 3],
                fixed: [1, 2, 3],
                pair: ("x".to_string(), 1),
            }
        );
    }
}

/// A container default renders correctly whatever container the
/// settings configure `vec_type`, `map_type`, or `set_type` as: the
/// `[elem, ..].into_iter().collect()` expression the walk emits only
/// needs `FromIterator`, which every one of these implements.
#[test]
#[ignore]
fn test_default_value_configured_containers() {
    let settings = default_settings()
        .with_vec_type(ContainerType::vec().with_path("::std::collections::VecDeque"))
        .with_map_type(ContainerType::hash_map())
        .with_set_type(ContainerType::hash_set());

    let builder = typespace_builder!(settings, {
        struct ConfiguredContainers {
            #[default = [8, 6, 7]]
            numbers: Vec<u32>,
            #[default = { "a": 1 }]
            counts: Map<String, u32>,
            #[default = [1, 2, 3]]
            tags: Set<u32>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_configured_containers.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        let value = serde_json::from_str::<import::ConfiguredContainers>("{}").unwrap();
        assert_eq!(
            value.numbers,
            [8u32, 6, 7]
                .into_iter()
                .collect::<::std::collections::VecDeque<_>>()
        );
        assert_eq!(
            value.counts,
            [("a".to_string(), 1u32)]
                .into_iter()
                .collect::<::std::collections::HashMap<_, _>>()
        );
        assert_eq!(
            value.tags,
            [1u32, 2, 3]
                .into_iter()
                .collect::<::std::collections::HashSet<_>>()
        );
    }
}

/// A map's key type need not be `String`: a JSON object key is always
/// a string, but that string is wrapped and walked as a value of the
/// key type, whatever it is, exactly as `Map`'s reference
/// implementation does.
#[test]
fn test_default_value_map_key_type() {
    let builder = typespace_builder!(default_settings(), {
        struct Key(String);

        struct KeyedDefaults {
            #[default = { "a": 1, "b": 2 }]
            counts: Map<Key, u32>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_map_key_type.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::KeyedDefaults>("{}").unwrap(),
            import::KeyedDefaults {
                counts: [
                    (import::Key("a".to_string()), 1),
                    (import::Key("b".to_string()), 2),
                ]
                .into_iter()
                .collect(),
            }
        );
    }
}

/// A property default value for a tuple struct, both a plain one and
/// one whose trailing field collects the rest of the sequence.
// A tuple struct with a whole-type default value renders a `Default`
// impl built from that value, so its fields owe no `Default` of their
// own. `feasibility` answers `IfSomeChildren(vec![])` for that case;
// answering `IfAllChildren` instead obligates every field, which is a
// requirement the generated impl never relies on.
#[test]
fn tuple_struct_with_a_default_value_obligates_no_field() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Default)
            .with_required_trait(TypespaceTrait::Clone),
        {
            native ::std::net::IpAddr: Clone + Deserialize;

            #[default = ["127.0.0.1", 8080]]
            struct Addressed(::std::net::IpAddr, u32);
        }
    );
    builder
        .finalize(no_cycles)
        .expect("a default value supplies the field, so IpAddr owes no Default");
}

#[test]
fn test_default_value_tuple_struct_kinds() {
    let builder = typespace_builder!(default_settings(), {
        struct FixedTuple(u32, String);

        struct OpenTuple(u32, #[flatten] Vec<u32>);

        struct TupleStructDefaults {
            #[default = [1, "a"]]
            fixed: FixedTuple,
            #[default = [1, 2, 3, 4]]
            open: OpenTuple,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_tuple_struct_kinds.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::TupleStructDefaults>("{}").unwrap(),
            import::TupleStructDefaults {
                fixed: import::FixedTuple(1, "a".to_string()),
                open: import::OpenTuple(1, vec![2, 3, 4]),
            }
        );
    }
}

/// A property default value across the enum tag/payload combinations
/// the walk handles: external, internal, and adjacent tagging with
/// a newtype-, tuple-, or struct-shaped payload, and untagged picking
/// between a newtype- and a tuple-shaped variant.
#[test]
fn test_default_value_enum_tag_kinds() {
    let builder = typespace_builder!(default_settings(), {
        enum External {
            Solo,
            Newtype(u32),
            Duo(u32, String),
            Trio { x: u32 },
        }

        #[tag = "t"]
        enum Internal {
            Solo,
            Trio { x: u32 },
        }

        #[tag = "t", content = "c"]
        enum Adjacent {
            Solo,
            Newtype(u32),
            Duo(u32, String),
            Trio { x: u32 },
        }

        #[untagged]
        enum Untagged {
            AsNewtype(u32),
            AsDuo(u32, String),
        }

        struct EnumDefaults {
            #[default = { "Newtype": 7 }]
            external_item: External,
            #[default = { "Duo": [3, "hi"] }]
            external_tuple: External,
            #[default = { "Trio": { "x": 5 } }]
            external_struct: External,
            #[default = { "t": "Trio", "x": 5 }]
            internal_struct: Internal,
            #[default = { "t": "Newtype", "c": 7 }]
            adjacent_item: Adjacent,
            #[default = { "t": "Duo", "c": [3, "hi"] }]
            adjacent_tuple: Adjacent,
            #[default = { "t": "Trio", "c": { "x": 5 } }]
            adjacent_struct: Adjacent,
            #[default = 9]
            untagged_item: Untagged,
            #[default = [3, "hi"]]
            untagged_tuple: Untagged,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_enum_tag_kinds.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::EnumDefaults>("{}").unwrap(),
            import::EnumDefaults {
                external_item: import::External::Newtype(7),
                external_tuple: import::External::Duo(3, "hi".to_string()),
                external_struct: import::External::Trio { x: 5 },
                internal_struct: import::Internal::Trio { x: 5 },
                adjacent_item: import::Adjacent::Newtype(7),
                adjacent_tuple: import::Adjacent::Duo(3, "hi".to_string()),
                adjacent_struct: import::Adjacent::Trio { x: 5 },
                untagged_item: import::Untagged::AsNewtype(9),
                untagged_tuple: import::Untagged::AsDuo(3, "hi".to_string()),
            }
        );
    }
}

/// An internally-tagged newtype variant whose payload is itself a
/// struct: serde accepts this shape (the tag sits alongside the
/// payload's own keys in the same map), and the builder does not
/// reject constructing one, so the walk must handle it rather than
/// treat it as unreachable.
#[test]
fn test_default_value_internal_enum_item_variant() {
    let builder = typespace_builder!(default_settings(), {
        struct Payload {
            y: u32,
        }

        #[tag = "t"]
        enum Internal {
            Solo,
            Wrapped(Payload),
        }

        struct InternalItemDefault {
            #[default = { "t": "Wrapped", "y": 9 }]
            wrapped: Internal,
            // A unit variant under internal tagging: serde is already
            // known to support this (it serializes as just the tag), so
            // this confirms the walk agrees.
            #[default = { "t": "Solo" }]
            solo: Internal,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_default_value_internal_enum_item_variant.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        assert_eq!(
            serde_json::from_str::<import::InternalItemDefault>("{}").unwrap(),
            import::InternalItemDefault {
                wrapped: import::Internal::Wrapped(import::Payload { y: 9 }),
                solo: import::Internal::Solo,
            }
        );
    }
}

/// A vec rejects a value that is not a JSON array.
#[test]
fn test_default_value_vec_rejects_non_array() {
    let builder = typespace_builder!(default_settings(), {
        #[default = "nope"]
        struct Numbers(Vec<u32>);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A map rejects a value that is not a JSON object.
#[test]
fn test_default_value_map_rejects_non_object() {
    let builder = typespace_builder!(default_settings(), {
        #[default = [1, 2]]
        struct Counts(Map<String, u32>);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A set rejects a default value containing a duplicate: `Value` has
/// no `Ord` impl to dedup with, and silently dropping one would make
/// the generated value diverge from the default actually declared.
#[test]
fn test_default_value_set_rejects_duplicate() {
    let builder = typespace_builder!(default_settings(), {
        #[default = [1, 2, 1]]
        struct Tags(Set<u32>);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A fixed-size array rejects a value of the wrong length.
#[test]
fn test_default_value_array_rejects_wrong_length() {
    let builder = typespace_builder!(default_settings(), {
        #[default = [1, 2]]
        struct Triple([u32; 3]);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A tuple rejects a value of the wrong length.
#[test]
fn test_default_value_tuple_rejects_wrong_length() {
    let builder = typespace_builder!(default_settings(), {
        #[default = [1]]
        struct Pair((u32, String));
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// A tuple struct rejects a value of the wrong length.
#[test]
fn test_default_value_tuple_struct_rejects_wrong_length() {
    let builder = typespace_builder!(default_settings(), {
        #[default = [1]]
        struct Pair(u32, String);
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An external-tagged enum default naming a variant whose payload
/// doesn't fit is rejected.
#[test]
fn test_default_value_external_enum_rejects_wrong_payload_shape() {
    let builder = typespace_builder!(default_settings(), {
        #[default = { "Newtype": "not a number" }]
        enum External {
            Newtype(u32),
        }
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An internally-tagged enum default naming a tag that matches no
/// variant is rejected.
#[test]
fn test_default_value_internal_enum_rejects_unknown_tag() {
    let builder = typespace_builder!(default_settings(), {
        #[tag = "t"]
        #[default = { "t": "Nope" }]
        enum Internal {
            Solo,
        }
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An internally-tagged enum default naming a tuple-payload variant is
/// rejected: serde has no map-shaped representation for a tuple
/// alongside a tag, so there is no value to build.
#[test]
fn test_default_value_internal_enum_rejects_tuple_variant() {
    let builder = typespace_builder!(default_settings(), {
        #[tag = "t"]
        #[default = { "t": "Duo" }]
        enum Internal {
            Solo,
            Duo(u32, String),
        }
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An adjacently-tagged enum default that names a payload-carrying
/// variant but omits the content field is rejected.
#[test]
fn test_default_value_adjacent_enum_rejects_missing_content() {
    let builder = typespace_builder!(default_settings(), {
        #[tag = "t", content = "c"]
        #[default = { "t": "Newtype" }]
        enum Adjacent {
            Newtype(u32),
        }
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// An untagged enum default that fits none of its variants is
/// rejected.
#[test]
fn test_default_value_untagged_enum_rejects_no_match() {
    let builder = typespace_builder!(default_settings(), {
        #[untagged]
        #[default = "not a number"]
        enum Untagged {
            AsNewtype(u32),
        }
    });
    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject the default value");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

#[test]
fn test_default_simple_struct_cycle() {
    let builder = typespace_builder!(default_settings(), {
        struct A {
            #[default = {}]
            a: Nullable<A>,
            #[default = 1]
            z: u32,
        }
    });

    fn make_box_id(id: &String) -> String {
        format!("boxed {}", id)
    }

    let Err(Error::InvalidDefault { reason, .. }) = builder.finalize(make_box_id) else {
        panic!("expected finalize to reject cyclic default value");
    };

    assert_eq!(reason, "property default value is recursive");

    // Try in the other order--we've messed this up before...
    let builder = typespace_builder!(default_settings(), {
        struct A {
            #[default = 1]
            a: u32,
            #[default = {}]
            z: Nullable<A>,
        }
    });

    let Err(Error::InvalidDefault { reason, .. }) = builder.finalize(make_box_id) else {
        panic!("expected finalize to reject cyclic default value");
    };

    assert_eq!(reason, "property default value is recursive");
}

#[test]
fn test_default_simple_enum_cycle() {
    let builder = typespace_builder!(default_settings(), {
        enum A {
            Whatever,
            B {
                #[default = { B: {} }]
                a: A,
            }
        }
    });

    fn make_box_id(id: &String) -> String {
        format!("boxed {}", id)
    }

    let Err(Error::InvalidDefault { reason, .. }) = builder.finalize(make_box_id) else {
        panic!("expected finalize to reject cyclic default value");
    };

    assert_eq!(reason, "property default value is recursive");
}

// These structures seem like they might have cyclic defaults, but they
// actually don't.
#[test]
fn test_default_not_actually_a_cycle() {
    let builder = typespace_builder!(default_settings(), {
        struct A {
            a: Optional<A>,
        }

        // We pass through A several times, but we don't actually cycle
        // infinitely. We should only ever have a single item in the
        // expansion_set that looks for cycles.
        struct B {
            #[default = { a: { a: {} } }]
            a: A,
        }

        struct C {
            #[default = { x: 1, c: { x: 2, c: null } }]
            c: Nullable<C>,
            x: u32,
        }

        struct D {
            #[default = { x: 100 }]
            c: Nullable<C>,
        }

        // This case highlights the need to track not just nodes visited (we
        // see C twice), but the value that we expand into C.
        struct E {
            #[default = {}]
            d: D,
        }
    });

    fn make_box_id(id: &String) -> String {
        format!("boxed {}", id)
    }

    let ts = builder.finalize(make_box_id).expect("finalize typespace");

    #[check_and_include("tests/output/test_default_not_actually_a_cycle.rs", ts.to_codespace().into_stream())]
    fn inner() {
        use import::*;

        let b = B::default();
        assert_eq!(
            b,
            B {
                a: A {
                    a: Some(Box::new(A {
                        a: Some(Box::new(A { a: None }))
                    })),
                }
            }
        );

        let e = E::default();
        assert_eq!(
            e,
            E {
                d: D {
                    // TODO 9/6/2026
                    // I don't think we need this box here; agents working on
                    // why that's happening and if it's reasonable to avoid
                    // (which it may not be).
                    c: Some(Box::new(C {
                        x: 100,
                        c: Some(Box::new(C {
                            x: 1,
                            c: Some(Box::new(C { x: 2, c: None }))
                        }))
                    }))
                }
            }
        )
    }
}

// A property whose default value omits a property of its own type
// takes that property's declared default, not the language's.
//
// Consider, the almost-a-JSON schema:
//
//     "SeparatorConfig": {
//       "properties": {
//         "lineThickness": { "type": "integer", "default": 1 },
//         "lineColor": { "type": ["string","null"], "default": "#B2000000" }
//       }
//     },
//     "separator": { "$ref": "#/definitions/SeparatorConfig", "default": {} }
//
// Every property of SeparatorConfig declares a default, and the value standing
// in for the whole object names none of them. A poor interpretation would
// yield a default with line_thickness: 0 and line_color: None, rather than 1
// and "#B2000000".
//
// The same JSON text means two different things: deserializing `{}`
// off the wire runs each property's own default and gives 1, while `{}`
// written as the default gives 0. That is what makes it a defect rather
// than a reading. See default_impl_struct's absent-property arm.
#[test]
fn object_default_takes_the_properties_own_defaults() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_desired_trait(TypespaceTrait::Default);

    let builder = typespace_test_macro::typespace_builder!(settings, {
        struct SeparatorConfig {
            #[default = 1]
            line_thickness: u32,
            #[default = "#B2000000"]
            line_color: String,
        }

        struct Holder {
            #[default = {}]
            separator: SeparatorConfig,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/object_default_takes_the_properties_own_defaults.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let holder = import::Holder::default();

        assert_eq!(holder.separator.line_thickness, 1);
        assert_eq!(holder.separator.line_color, "#B2000000");
    }
}

#[test]
fn self_referential_newtype_default_is_rejected() {
    let settings = Settings::minimal();
    let builder = typespace_test_macro::typespace_builder!(settings, {
        struct N(N);

        struct Holder {
            #[default = 0]
            v: N,
        }
    });

    let Err(Error::InvalidDefault { reason, .. }) = builder.finalize(no_cycles) else {
        panic!("a self-referential newtype's recursive default should be rejected, not accepted");
    };

    assert_eq!(reason, "property default value is recursive");
}

#[test]
fn test_cycle_through_item_variant() {
    let settings = Settings::minimal();
    let builder = typespace_test_macro::typespace_builder!(settings, {
        #[untagged]
        enum E {
            E(E),
            X(u32),
        }

        struct Holder {
            #[default = "potato"]
            e: E,
        }
    });

    let Err(Error::InvalidDefault { reason, .. }) = builder.finalize(no_cycles) else {
        panic!("a cycle through an untagged Item variant should be rejected, not accepted");
    };

    // The guard fires on the `E(E)` variant, but `default_impl_enum_untagged`
    // discards a failed variant with `.ok()` and moves on, so what surfaces is
    // the exhausted-variants error rather than the recursion that caused it.
    assert_eq!(reason, "no variant of the untagged enum accepts this value");
}

// An untagged enum whose first variant fails partway through: the walk
// must discard that attempt and succeed on a later variant. This is the
// behavior `default_impl_enum_untagged`'s `.ok()` exists to provide, and
// it is what the guard's pop-on-error discipline has to survive.
#[test]
fn test_default_untagged_backtracks_to_a_later_variant() {
    let builder = typespace_builder!(default_settings(), {
        struct Wrap(u32);

        #[untagged]
        enum U {
            // `a` walks fine, then `b` is required and absent, so the
            // whole variant is rejected after Wrap's guarded frame has
            // already pushed and popped.
            First { a: Wrap, b: u32 },
            Second { a: Wrap },
        }

        struct Holder {
            #[default = { a: 7 }]
            u: U,
        }
    });

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include(
        "tests/output/test_default_untagged_backtracks.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        assert_eq!(Holder::default().u, U::Second { a: Wrap(7) });
    }
}

#[test]
fn test_render_constrained_newtype_string() {
    let mut builder = TypespaceBuilder::new(Settings::maximal());

    builder.insert("string".to_string(), Type::String).unwrap();

    builder
        .insert(
            "constrained string".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("string".to_string())
                    .name("ConstrainedString")
                    .constraints(typespace::build::NewtypeConstraints::String {
                        min: Some(1),
                        max: Some(64),
                        patterns: vec!["^a".to_string(), "k$".to_string()],
                    }),
            ),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();
    let out = ts.to_codespace().into_stream();

    #[check_and_include("tests/output/test_render_constrained_newtype_string.rs", out)]
    fn inner() {
        use import::*;

        let _x = ConstrainedString::try_from("ask").unwrap();
        let _x = ConstrainedString::try_from("").expect_err("nope");
        // Longer than the 64-character max.
        let _x = ConstrainedString::try_from("a".repeat(65) + "k").expect_err("nope");
        // Matches "k$" but not "^a".
        let _x = ConstrainedString::try_from("tick").expect_err("nope");

        let x: ConstrainedString = "alack".parse().unwrap();
        assert_eq!(x.to_string(), "alack");
    }
}

#[test]
fn test_render_constrained_newtype_allow_list() {
    let settings = Settings::typical()
        .with_desired_trait(TypespaceTrait::PartialEq)
        .with_desired_trait(TypespaceTrait::JsonSchema);
    let ts = {
        let mut builder = TypespaceBuilder::new(settings);

        builder.insert("string".to_string(), Type::String).unwrap();

        builder
            .insert(
                "constrained string".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("string".to_string())
                        .name("ConstrainedString")
                        .constraints(typespace::build::NewtypeConstraints::AllowList(vec![
                            JsonValue(serde_json::json! { "tomax" }),
                            JsonValue(serde_json::json! { "xamot" }),
                        ])),
                ),
            )
            .unwrap();

        builder.finalize(no_cycles).unwrap()
    };
    let out = ts.to_codespace().into_stream();

    #[check_and_include("tests/output/test_render_constrained_newtype_allow_list.rs", out)]
    fn inner() {
        use import::*;

        let tomax = ConstrainedString::try_from("tomax".to_string()).unwrap();
        let _xamot = ConstrainedString::try_from("xamot".to_string()).unwrap();
        ConstrainedString::try_from("zartan".to_string()).expect_err("not on the list");

        // The hand-written Deserialize runs the same check.
        assert_eq!(
            serde_json::from_str::<ConstrainedString>("\"tomax\"").unwrap(),
            tomax
        );
        serde_json::from_str::<ConstrainedString>("\"zartan\"").expect_err("not on the list");
    }
}

// The fallback constraint. `Coords` renders the part of the source
// schema typespace can state, and the schema itself carries the rest
// (`multipleOf`), checked at run time against the serialized value. Both
// entry points run the check: the `TryFrom` constructor and the
// hand-written `Deserialize`.
//
// Struct builders are off so that `Coords` owns no inherent impl: the
// newtype's `impl From<EvenCoords> for Coords` is attributed to `Coords`
// by the classifier in tests/item_order.rs, which reads ownership from
// the locally declared side and has no way to tell a newtype's
// out-of-Self conversion from an into-Self one when both sides are
// declared in the same file.
#[test]
fn test_render_constrained_newtype_json_schema() {
    let schema = serde_json::json! {{
        "type": "object",
        "properties": {
            "x": { "type": "integer", "multipleOf": 2 },
            "y": { "type": "integer" },
        },
        "required": ["x", "y"],
    }};

    let ts = {
        let mut builder = typespace_builder!(Settings::maximal().with_struct_builder(false), {
            struct Coords {
                x: i64,
                y: i64,
            }
        });

        builder
            .insert(
                "EvenCoords".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("Coords".to_string())
                        .name("EvenCoords")
                        .constraints(NewtypeConstraints::JsonSchema(JsonValue::new(
                            schema.clone(),
                        ))),
                ),
            )
            .unwrap();

        builder.finalize(no_cycles).unwrap()
    };
    let out = ts.to_codespace().into_stream();

    #[check_and_include("tests/output/test_render_constrained_newtype_json_schema.rs", out)]
    fn inner() {
        use import::*;

        let even = EvenCoords::try_from(Coords { x: 2, y: 5 }).unwrap();
        assert_eq!(even.x, 2);
        EvenCoords::try_from(Coords { x: 3, y: 5 }).expect_err("x is odd");

        // The hand-written Deserialize runs the same check.
        assert_eq!(
            serde_json::from_str::<EvenCoords>(r#"{"x":2,"y":5}"#).unwrap(),
            even
        );
        serde_json::from_str::<EvenCoords>(r#"{"x":3,"y":5}"#).expect_err("x is odd");

        // A value of the newtype is an inner value that also passes
        // the check, so the JsonSchema impl reports the allOf of the
        // stored schema (keyword for keyword) and the inner type's.
        let mut generator = schemars::r#gen::SchemaGenerator::default();
        assert_eq!(
            serde_json::to_value(<EvenCoords as schemars::JsonSchema>::json_schema(
                &mut generator
            ))
            .unwrap(),
            serde_json::json!({
                "allOf": [schema, { "$ref": "#/definitions/Coords" }],
            })
        );
    }
}

// The same constraint over a `String`, which can carry Display and
// FromStr where the struct above cannot. The schema is an `anyOf` of two
// string constraints, which `NewtypeConstraints::String` has no way to
// state: either the value starts with "a" or it is at most three
// characters long.
#[test]
fn test_render_constrained_newtype_json_schema_string() {
    let schema = serde_json::json! {{
        "anyOf": [
            { "type": "string", "pattern": "^a" },
            { "type": "string", "maxLength": 3 },
        ],
    }};

    let ts = {
        let mut builder = TypespaceBuilder::new(Settings::maximal());

        builder.insert("string".to_string(), Type::String).unwrap();
        builder
            .insert(
                "Terse".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("string".to_string())
                        .name("Terse")
                        .constraints(NewtypeConstraints::JsonSchema(JsonValue::new(schema))),
                ),
            )
            .unwrap();

        builder.finalize(no_cycles).unwrap()
    };
    let out = ts.to_codespace().into_stream();

    #[check_and_include(
        "tests/output/test_render_constrained_newtype_json_schema_string.rs",
        out
    )]
    fn inner() {
        use import::*;

        // FromStr parses the inner type and then checks the result.
        let terse = "wx".parse::<Terse>().unwrap();
        assert_eq!(terse.to_string(), "wx");
        "alphabet"
            .parse::<Terse>()
            .expect("an a-word of any length");
        "wxyz"
            .parse::<Terse>()
            .expect_err("neither an a-word nor terse");

        assert_eq!(serde_json::from_str::<Terse>("\"wx\"").unwrap(), terse);
        serde_json::from_str::<Terse>("\"wxyz\"").expect_err("neither an a-word nor terse");
    }
}

// A stored schema names its own draft through `$schema`. This one is
// draft-07 and uses the array form of `items`, which draft 2020-12
// refuses to compile, so the generated code building at all proves the
// declared draft governs validation; the assertions prove the keywords
// are enforced.
#[test]
fn test_render_constrained_newtype_json_schema_draft() {
    let schema = serde_json::json! {{
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "array",
        "items": [{ "type": "integer" }, { "type": "integer" }],
        "additionalItems": false,
    }};

    let ts = {
        let mut builder = TypespaceBuilder::new(Settings::maximal());

        builder
            .insert("i64".to_string(), Type::Integer("i64".to_string()))
            .unwrap();
        builder
            .insert("vec".to_string(), Type::Vec("i64".to_string()))
            .unwrap();
        builder
            .insert(
                "Pair".to_string(),
                Type::NewtypeStruct(
                    NewtypeStruct::new("vec".to_string())
                        .name("Pair")
                        .constraints(NewtypeConstraints::JsonSchema(JsonValue::new(schema))),
                ),
            )
            .unwrap();

        builder.finalize(no_cycles).unwrap()
    };
    let out = ts.to_codespace().into_stream();

    #[check_and_include(
        "tests/output/test_render_constrained_newtype_json_schema_draft.rs",
        out
    )]
    fn inner() {
        use import::*;

        Pair::try_from(vec![2, 4]).unwrap();
        Pair::try_from(vec![2, 4, 6]).expect_err("additionalItems refuses a third element");
    }
}

// `feasibility` in `trait_resolution.rs` answers `ManuallyRealizable`

// `feasibility` in `trait_resolution.rs` answers `IfSomeChildren`
// for Display and FromStr on two kinds of type, and the renderer writes
// the impls that answer stands for:
//
// - a newtype struct ("A newtype's Display is always the inner value's
//   Display"); `NewtypeStruct::render` in `build/structs.rs` forwards
//   both traits to the inner type;
// - an untagged all-item-variant enum ("Display/FromStr forward to
//   whichever payload types the variants carry"); `Enum::render` in
//   `build/enums.rs` writes both from the variants' payloads.
//
// A trait granted this way must never reach `render_derives`
// (`lib.rs`), whose guard panics: "trying to derive Display which
// requires a manual implementation; this is a bug". The tests below
// pin the impls, so a renderer that dropped one would panic rather
// than emit a type missing the trait it was granted.

// `feasibility` answers `IfSomeChildren(inner)` for both traits,
// and required resolution grants them: the `String` inner satisfies the
// forwarded obligations.
#[test]
fn newtype_display_and_from_str_render() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Display)
            .with_required_trait(TypespaceTrait::FromStr),
        {
            struct Wrapper(String);
        }
    );
    let ts = builder
        .finalize(no_cycles)
        .expect("a newtype forwards Display and FromStr to its inner type");
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "Wrapper"),
        "no Display impl for Wrapper"
    );
    assert!(
        common::has_impl(&file, "FromStr", "Wrapper"),
        "no FromStr impl for Wrapper"
    );
}

// An unconstrained newtype's FromStr forwards to the inner type's,
// which is exactly the obligation `feasibility` hands back. The inner
// type here is `u32`, so a body that assumed a `String` inner would not
// compile; the snapshot puts it through the compiler.
#[test]
fn test_newtype_from_str_forwards_to_a_non_string_inner() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Debug)
            .with_required_trait(TypespaceTrait::PartialEq)
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
        {
            struct Port(u32);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "Port"),
        "no Display impl for Port"
    );
    assert!(
        common::has_impl(&file, "FromStr", "Port"),
        "no FromStr impl for Port"
    );

    #[check_and_include(
        "tests/output/test_newtype_from_str_forwards_to_a_non_string_inner.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        assert_eq!("8080".parse::<Port>().unwrap(), Port(8080));
        assert!("not a port".parse::<Port>().is_err());
        assert_eq!(Port(8080).to_string(), "8080");
    }
}

// TYPIFY COMPAT: an unconstrained newtype directly over `String` takes
// the value verbatim, so its `FromStr` cannot fail and typify 1 writes
// no `TryFrom` impls beside it. The pinned form is `StringVersion` in
// typify 1's `various-enums.rs`.
#[test]
fn test_newtype_from_str_wraps_a_string_inner() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Debug)
            .with_required_trait(TypespaceTrait::PartialEq)
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
        {
            struct Wrapper(String);
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    let rendered = ts.to_codespace().into_stream().to_string();

    assert!(
        !rendered.contains(&quote! { ::std::convert::TryFrom }.to_string()),
        "a String newtype carries no TryFrom impls:\n{rendered}"
    );

    #[check_and_include(
        "tests/output/test_newtype_from_str_wraps_a_string_inner.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        // The error type is `Infallible`, so every input parses.
        assert_eq!(
            "not a number".parse::<Wrapper>().unwrap(),
            Wrapper("not a number".to_string())
        );
        assert_eq!(Wrapper("hi".to_string()).to_string(), "hi");
    }
}

// `Settings::maximal` desires Display and FromStr, so the desired phase
// reads the same feasibility table and grants both to any newtype whose
// inner type has them. The preset that asks for the most renders the
// most ordinary wrapper type.
#[test]
fn maximal_settings_newtype_renders() {
    let builder = typespace_builder!(Settings::maximal(), {
        struct Wrapper(String);
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        file.items
            .iter()
            .any(|item| matches!(item, syn::Item::Struct(s) if s.ident == "Wrapper")),
        "Wrapper missing from output"
    );
}

// `feasibility` answers `IfSomeChildren` with the payload types as
// obligations, and both payloads (`String`, `u32`) satisfy them, so
// finalization grants the trait and `Enum::render` writes the impl.
#[test]
fn untagged_enum_display_renders() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Display),
        {
            #[untagged]
            enum U {
                Text(String),
                Count(u32),
            }
        }
    );
    let ts = builder
        .finalize(no_cycles)
        .expect("an untagged item enum forwards Display to its payloads");
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "U"),
        "no Display impl for U"
    );
}

// The FromStr half of the same pair, on a single-variant enum so the
// intended parse is unambiguous.
#[test]
fn untagged_enum_from_str_renders() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::FromStr),
        {
            #[untagged]
            enum Parsed {
                Count(u32),
            }
        }
    );
    let ts = builder
        .finalize(no_cycles)
        .expect("an untagged item enum forwards FromStr to its payloads");
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "FromStr", "Parsed"),
        "no FromStr impl for Parsed"
    );
}
// An untagged enum's generated `FromStr` is a first-match-wins chain, so
// a variant whose payload parses every string always wins and every
// later variant is dead code. `Display` keeps forwarding to the variant
// payloads; `FromStr` does not.
#[test]
fn untagged_enum_with_irrefutable_payload_loses_from_str() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
        {
            #[untagged]
            enum StrOrInt {
                Text(String),
                Count(u32),
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "StrOrInt"),
        "no Display impl for StrOrInt"
    );
    assert!(
        !common::has_impl(&file, "FromStr", "StrOrInt"),
        "StrOrInt implements FromStr, but its Text arm parses every \
         string, so Count is unreachable"
    );
}

// The same enum with the variants swapped. The answer does not depend on
// where the irrefutable payload sits in the variant list: typify 1 uses
// `.any()`, and its `IntOrStr` in `multiple-instance-types.rs` has the
// `String` variant last and still gets `Display` with no `FromStr`.
#[test]
fn untagged_enum_with_irrefutable_payload_last_loses_from_str() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
        {
            #[untagged]
            enum IntOrStr {
                Count(u32),
                Text(String),
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "IntOrStr"),
        "no Display impl for IntOrStr"
    );
    assert!(
        !common::has_impl(&file, "FromStr", "IntOrStr"),
        "IntOrStr implements FromStr, but its Text arm parses every \
         string, so Display and FromStr do not round-trip"
    );
}

// The property is syntactic, not semantic. A native declaring `FromStr`
// says nothing about which strings it accepts, and a newtype whose only
// pattern is `".*"` has a validation step between the `&str` and the
// constructed value even though the pattern accepts every string.
// Neither payload is irrefutable, so the enum keeps both traits. This is
// typify 1's `IdOrName` / `IdOrYolo`.
#[test]
fn untagged_enum_with_pattern_constrained_payload_keeps_from_str() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
    );

    builder.insert("string".to_string(), Type::String).unwrap();
    builder
        .insert(
            "::id::Id".to_string(),
            Type::Native(Native::new(
                "::id::Id",
                [TypespaceTrait::Display, TypespaceTrait::FromStr]
                    .into_iter()
                    .collect::<TypespaceTraitSet>(),
                Vec::new(),
            )),
        )
        .unwrap();
    builder
        .insert(
            "Yolo".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("string".to_string())
                    .name("Yolo")
                    .constraints(NewtypeConstraints::String {
                        min: None,
                        max: None,
                        patterns: vec![".*".to_string()],
                    }),
            ),
        )
        .unwrap();
    builder
        .insert(
            "IdOrYolo".to_string(),
            Enum::new()
                .name("IdOrYolo")
                .tag_type(EnumTagType::Untagged)
                .variants([
                    EnumVariant::new("Id", VariantDetails::Item("::id::Id".to_string())),
                    EnumVariant::new("Yolo", VariantDetails::Item("Yolo".to_string())),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "Display", "IdOrYolo"),
        "no Display impl for IdOrYolo"
    );
    assert!(
        common::has_impl(&file, "FromStr", "IdOrYolo"),
        "no FromStr impl for IdOrYolo: every payload validates, so no \
         arm swallows the rest"
    );
}

// The property is transitive through unconstrained newtypes, so an
// untagged enum over two separate `String` newtypes is in exactly the
// same position as one with a bare `String` payload: the first arm
// always wins. The newtypes keep their own `FromStr`; the enum does not.
// This is typify 1's `ReferencesObjectValue`.
#[test]
fn untagged_enum_over_string_newtypes_loses_from_str() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_desired_trait(TypespaceTrait::Display)
            .with_desired_trait(TypespaceTrait::FromStr),
        {
            struct Reference(String);
            struct Literal(String);

            #[untagged]
            enum ReferenceOrLiteral {
                Reference(Reference),
                Literal(Literal),
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();

    assert!(
        common::has_impl(&file, "FromStr", "Reference"),
        "no FromStr impl for Reference"
    );
    assert!(
        common::has_impl(&file, "Display", "ReferenceOrLiteral"),
        "no Display impl for ReferenceOrLiteral"
    );
    assert!(
        !common::has_impl(&file, "FromStr", "ReferenceOrLiteral"),
        "ReferenceOrLiteral implements FromStr, but its Reference arm \
         parses every string, so Literal is unreachable"
    );

    // The assertions above say why; the snapshot says what, and puts
    // the answer through the compiler: the spliced-in enum has no
    // `FromStr` for a dead `Literal` arm to hide in.
    #[check_and_include(
        "tests/output/untagged_enum_over_string_newtypes_loses_from_str.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let traits = crate::implemented_traits!(ReferenceOrLiteral);
        assert!(traits.contains(&TypespaceTrait::Display));
        assert!(!traits.contains(&TypespaceTrait::FromStr));

        // Each newtype keeps the irrefutable FromStr of its own that
        // denies the enum its FromStr.
        assert_eq!("#/x".parse::<Reference>().unwrap().0, "#/x");
        assert_eq!("plain".parse::<Literal>().unwrap().0, "plain");

        // Display forwards to whichever payload the value holds.
        assert_eq!(
            ReferenceOrLiteral::Reference(Reference("#/x".to_string())).to_string(),
            "#/x"
        );
        assert_eq!(
            ReferenceOrLiteral::Literal(Literal("plain".to_string())).to_string(),
            "plain"
        );
    }
}

// A desired `FromStr` is dropped silently; a required one is a
// finalization error naming the variant whose payload swallows the rest.
#[test]
fn required_from_str_on_irrefutable_untagged_enum_conflicts() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::FromStr),
        {
            #[untagged]
            enum StrOrInt {
                Text(String),
                Count(u32),
            }
        }
    );

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("finalization unexpectedly succeeded");
    };
    let message = err.to_string();
    let Error::TraitConflicts { conflicts } = err else {
        panic!("expected TraitConflicts, got: {message}");
    };

    assert_eq!(conflicts.len(), 1, "conflicts: {conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::FromStr);
    assert!(matches!(conflict.origin, RequirementOrigin::GlobalSettings));
    assert_eq!(conflict.offender, "StrOrInt");
    assert!(
        matches!(
            &conflict.reason,
            OffenderReason::IrrefutableVariantPayload { variant } if variant == "Text"
        ),
        "reason: {:#?}",
        conflict.reason
    );
    assert!(
        message.contains("Text"),
        "the message does not name the offending variant:\n{message}"
    );
}

// A `String` constraint with no minimum, no maximum, and no patterns
// says nothing `NewtypeConstraints::None` does not already say, and it
// is the one case where "syntactically constrained" and "irrefutable"
// would disagree. Validation rejects it.
#[test]
fn vacuous_string_constraints_are_rejected() {
    let result = NewtypeStruct::new("string".to_string())
        .name("Vacuous")
        .constraints(NewtypeConstraints::String {
            min: None,
            max: None,
            patterns: Vec::new(),
        })
        .build();

    let Err(err) = result else {
        panic!("a String constraint with no bounds and no patterns is rejected");
    };
    assert!(
        matches!(
            &err,
            Error::VacuousConstraints { name, kind }
                if name == "Vacuous" && *kind == "string"
        ),
        "expected VacuousConstraints, got: {err}"
    );
}

// An allow list with nothing on it admits no value at all, which is a
// mistake at the source rather than a type worth generating.
#[test]
fn empty_allow_list_constraints_are_rejected() {
    let result = NewtypeStruct::new("string".to_string())
        .name("Vacuous")
        .constraints(NewtypeConstraints::AllowList(Vec::new()))
        .build();

    let Err(err) = result else {
        panic!("an empty allow list is rejected");
    };
    assert!(
        matches!(
            &err,
            Error::VacuousConstraints { name, kind }
                if name == "Vacuous" && *kind == "allow list"
        ),
        "expected VacuousConstraints, got: {err}"
    );
}

// A deny list with nothing on it denies nothing, so the newtype is the
// unconstrained one written the long way.
#[test]
fn empty_deny_list_constraints_are_rejected() {
    let result = NewtypeStruct::new("string".to_string())
        .name("Vacuous")
        .constraints(NewtypeConstraints::DenyList(Vec::new()))
        .build();

    let Err(err) = result else {
        panic!("an empty deny list is rejected");
    };
    assert!(
        matches!(
            &err,
            Error::VacuousConstraints { name, kind }
                if name == "Vacuous" && *kind == "deny list"
        ),
        "expected VacuousConstraints, got: {err}"
    );
}

// The list tests below use the builder directly: the test macro has no
// syntax for allow and deny list constraints.

/// A value on an allow list must be a value of the inner type.
///
/// The list renders through the same walk as a default value, and that
/// walk assumes the value was checked; an unchecked value would panic at
/// render rather than report, and the checked one reports at finalize.
#[test]
fn allow_list_value_must_fit_the_inner_type() {
    let mut builder = TypespaceBuilder::new(Settings::minimal());
    builder
        .insert("u32".to_string(), Type::Integer("u32".to_string()))
        .unwrap();
    builder
        .insert(
            "count".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("u32".to_string())
                    .name("Count")
                    .constraints(NewtypeConstraints::AllowList(vec![
                        JsonValue(serde_json::json!(1)),
                        JsonValue(serde_json::json!("two")),
                    ])),
            ),
        )
        .unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("an allow list value that is not a u32 is rejected");
    };
    assert!(
        matches!(&err, Error::InvalidDefault { value, id, .. }
            if *value == serde_json::json!("two") && id == "u32"),
        "{err:?}"
    );
}

/// A value on a deny list must be a value of the inner type, for the
/// same reason as an allow list value.
#[test]
fn deny_list_value_must_fit_the_inner_type() {
    let mut builder = TypespaceBuilder::new(Settings::minimal());
    builder
        .insert("u32".to_string(), Type::Integer("u32".to_string()))
        .unwrap();
    builder
        .insert(
            "count".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("u32".to_string())
                    .name("Count")
                    .constraints(NewtypeConstraints::DenyList(vec![JsonValue(
                        serde_json::json!(-1),
                    )])),
            ),
        )
        .unwrap();

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("a deny list value that is not a u32 is rejected");
    };
    assert!(
        matches!(&err, Error::InvalidDefault { value, id, .. }
            if *value == serde_json::json!(-1) && id == "u32"),
        "{err:?}"
    );
}

/// A list value on a native is constructed in generated code by
/// deserializing it, inside a `TryFrom` impl the newtype always carries,
/// so the native must implement `Deserialize`. One declared without it
/// conflicts at finalize rather than rendering an impl that cannot work.
#[test]
fn allow_list_value_on_a_native_requires_deserialize() {
    let declared = [
        TypespaceTrait::Clone,
        TypespaceTrait::Debug,
        TypespaceTrait::Serialize,
    ]
    .into_iter()
    .collect::<TypespaceTraitSet>();

    let mut builder = TypespaceBuilder::new(Settings::minimal());
    builder
        .insert(
            "date".to_string(),
            Type::Native(Native::new("chrono::NaiveDate", declared, Vec::new())),
        )
        .unwrap();
    builder
        .insert(
            "holiday".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("date".to_string())
                    .name("Holiday")
                    .constraints(NewtypeConstraints::AllowList(vec![JsonValue(
                        serde_json::json!("2024-01-01"),
                    )])),
            ),
        )
        .unwrap();

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("a list value on a native demands Deserialize of the native");
    };
    assert_eq!(conflicts.len(), 1, "{conflicts:#?}");
    let conflict = &conflicts[0];
    assert_eq!(conflict.required, TypespaceTrait::Deserialize);
    assert!(matches!(
        &conflict.reason,
        OffenderReason::NativeMissingImpl { type_name } if type_name == "chrono::NaiveDate"
    ));
}

// A JSON schema every value satisfies constrains nothing. Only the two
// schemas that say so outright are caught: whether a longer schema
// admits everything is not a question a check at this level can answer.
#[test]
fn vacuous_json_schema_constraints_are_rejected() {
    for schema in [serde_json::json!(true), serde_json::json!({})] {
        let result = NewtypeStruct::new("string".to_string())
            .name("Vacuous")
            .constraints(NewtypeConstraints::JsonSchema(JsonValue::new(
                schema.clone(),
            )))
            .build();

        let Err(err) = result else {
            panic!("the schema `{schema}` admits every value and is rejected");
        };
        assert!(
            matches!(
                &err,
                Error::VacuousConstraints { name, kind }
                    if name == "Vacuous" && *kind == "JSON schema"
            ),
            "expected VacuousConstraints, got: {err}"
        );
    }
}

#[test]
fn test_enum_derive_default() {
    let builder = typespace_builder!(
        Settings::minimal().with_desired_trait(TypespaceTrait::Default),
        {
            #[default = "Foo"]
            enum EnumExternal {
                Foo,
                Bar(String),
                Baz,
            }

            #[tag = "tag"]
            #[default = { tag: "Foo" }]
            enum EnumInternal {
                Foo,
                Bar(String),
                Baz,
            }

            #[tag = "tag", content = "content"]
            #[default = { tag: "Foo" }]
            enum EnumAdjacent {
                Foo,
                Bar(String),
                Baz,
            }

            #[untagged]
            #[default = null]
            enum EnumUntagged {
                Foo,
                Bar(String),
                Baz,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_enum_derive_default.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let instance = EnumExternal::default();
        assert!(matches!(instance, EnumExternal::Foo));

        let instance = EnumInternal::default();
        assert!(matches!(instance, EnumInternal::Foo));

        let instance = EnumAdjacent::default();
        assert!(matches!(instance, EnumAdjacent::Foo));

        let instance = EnumUntagged::default();
        assert!(matches!(instance, EnumUntagged::Foo));
    }
}

#[test]
fn test_enum_generate_default_for_typify() {
    let settings = Settings::minimal()
        .with_desired_trait(TypespaceTrait::Default)
        .with_typify_compat(true);
    let builder = typespace_builder!(settings,
        {
            #[default = "Foo"]
            enum EnumExternal {
                Foo,
                Bar(String),
                Baz,
            }

            #[tag = "tag"]
            #[default = { tag: "Foo" }]
            enum EnumInternal {
                Foo,
                Bar(String),
                Baz,
            }

            #[tag = "tag", content = "content"]
            #[default = { tag: "Foo" }]
            enum EnumAdjacent {
                Foo,
                Bar(String),
                Baz,
            }

            #[untagged]
            #[default = null]
            enum EnumUntagged {
                Foo,
                Bar(String),
                Baz,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_enum_generate_default_for_typify.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let instance = EnumExternal::default();
        assert!(matches!(instance, EnumExternal::Foo));

        let instance = EnumInternal::default();
        assert!(matches!(instance, EnumInternal::Foo));

        let instance = EnumAdjacent::default();
        assert!(matches!(instance, EnumAdjacent::Foo));

        let instance = EnumUntagged::default();
        assert!(matches!(instance, EnumUntagged::Foo));
    }
}

#[test]
fn test_enum_generate_default() {
    let builder = typespace_builder!(
        Settings::minimal().with_desired_trait(TypespaceTrait::Default),
        {
            #[default = { Bar: "None" }]
            enum EnumExternal {
                Foo,
                Bar(String),
                Baz,
            }

            #[tag = "tag"]
            #[default = { tag: "Bar", value: "None" }]
            enum EnumInternal {
                Foo,
                Bar { value: String },
                Baz,
            }

            #[tag = "tag", content = "content"]
            #[default = { tag: "Bar", content: "None" }]
            enum EnumAdjacent {
                Foo,
                Bar(String),
                Baz,
            }

            #[untagged]
            #[default = "None"]
            enum EnumUntagged {
                Foo,
                Bar(String),
                Baz,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_enum_generate_default.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let instance = EnumExternal::default();
        assert!(matches!(instance, EnumExternal::Bar(value) if value == "None"));

        let instance = EnumInternal::default();
        assert!(matches!(instance, EnumInternal::Bar { value } if value == "None"));

        let instance = EnumAdjacent::default();
        assert!(matches!(instance, EnumAdjacent::Bar(value) if value == "None"));

        let instance = EnumUntagged::default();
        assert!(matches!(instance, EnumUntagged::Bar(value) if value == "None"));
    }
}

#[test]
fn test_struct_defaults() {
    let builder = typespace_builder!(
        Settings::minimal().with_desired_trait(TypespaceTrait::Default),
        {

            #[default = { a: "x" }]
            struct StructAllDefault {
                a: String,
                b: Optional<String>,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include(
        "tests/output/test_struct_defaults.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let instance = StructAllDefault::default();
        assert_eq!(instance.a, "x");
        assert_eq!(instance.b, None);
    }
}

// `feasibility` documents the contract for a struct with an attached
// default value: "The hand-written impl takes each property the
// default value names from that value and fills the rest with
// Default::default()". `Struct::render` in `build/structs.rs` consults
// `common.default` only to decide whether the derive shortcut applies;
// the impl body it writes comes entirely from each property's
// `DefaultConstructor`, so the attached value's contents are
// discarded.
#[test]
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

/// The generated default functions group by their containing type, in
/// type-name order, not in function-name order.
///
/// A mod's items sort by the key they were added under, and each
/// default function is keyed by its containing type, so one type's
/// functions stay together and the groups follow the type names. typify
/// keys the same way, which is what makes the two agree.
///
/// The graph below is built so the two orderings disagree: by type name
/// `Density` precedes `DensityDistributionNormal`, while by function
/// name `density_distribution_normal_alpha` precedes `density_zeta`.
/// The context also has to be the type name in both the struct and the
/// enum-variant path, since a snake_case context on one side and a
/// CamelCase one on the other would sort every enum variant's functions
/// ahead of every struct's.
#[test]
fn test_default_fn_items_group_by_containing_type() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize);

    let builder = typespace_builder!(settings, {
        struct Density {
            #[default = 1.5]
            zeta: f64,
        }

        enum DensityDistribution {
            Normal {
                #[default = 2.5]
                alpha: f64,
            },
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);

    let density = rendered
        .find("fn density_zeta")
        .unwrap_or_else(|| panic!("no density_zeta in:\n{rendered}"));
    let distribution = rendered
        .find("fn density_distribution_normal_alpha")
        .unwrap_or_else(|| panic!("no density_distribution_normal_alpha in:\n{rendered}"));

    assert!(
        density < distribution,
        "`Density` sorts before `DensityDistributionNormal`, so its \
         default function comes first; ordering by function name or \
         mixing snake_case and CamelCase contexts would reverse it:\n{rendered}"
    );
}

#[test]
fn test_optional_nullable_custom_default() {
    let settings = Settings::minimal()
        .with_optional_nullable(OptionalNullable::CustomType(
            ContainerType::option().with_path("super::OptionField"),
        ))
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::Default)
        .with_required_trait(TypespaceTrait::JsonSchema);

    let builder = typespace_builder!(settings, {
        #[default = { piggy_a: "roast beef", piggy_c: null }]
        struct Piggies {
            piggy_a: OptionalNullable<String>,
            piggy_b: OptionalNullable<String>,
            piggy_c: OptionalNullable<String>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    #[check_and_include(
        "tests/output/test_optional_nullable_custom_default.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        use import::*;

        let piggies = Piggies::default();

        assert_eq!(
            piggies.piggy_a,
            OptionField::Present("roast beef".to_string()),
        );
        assert_eq!(piggies.piggy_b, OptionField::Absent);
        assert_eq!(piggies.piggy_c, OptionField::Null);

        let schema = schemars::schema_for!(Piggies);
        let schema = serde_json::to_value(&schema).unwrap();
        let expected = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Piggies",
            "type": "object",
            "properties": {
                "piggy_a": {
                    "type": ["string", "null"]
                },
                "piggy_b": {
                    "type": ["string", "null"]
                },
                "piggy_c": {
                    "type": ["string", "null"]
                }
            }
        });
        assert_eq!(
            schema,
            expected,
            "{}",
            serde_json::to_string_pretty(&schema).unwrap(),
        );
    }
}

// The remainder's element count is read by walking its type, and that
// walk follows aliases and boxes, so it can reach an id it already
// passed. `query_api`'s has_impl_survives_an_alias_cycle_through_a_box
// builds the same loop and finalize accepts it. Without a guard the
// walk never returns. No runtime coverage: `type A = Box<A>;` has no
// finite Rust value, so the rendered code cannot compile.
#[test]
fn test_tuple_struct_rest_cycles_through_a_box() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::JsonSchema),
        {
            type A = Box<A>;
            struct T(String, #[flatten] A);
        }
    );
    let ts = builder
        .finalize(no_cycles)
        .expect("finalize accepts the graph");
    let rendered = ts.to_codespace().into_stream().to_string();

    // The fixed field is still guaranteed; an unknown remainder count
    // leaves the upper end open.
    assert!(rendered.contains("min_items : Some (1u32)"));
    assert!(!rendered.contains("max_items"));
}

// A remainder contributes its own element count to the tuple's bounds:
// a fixed-length array contributes exactly, and a tuple struct
// contributes its fields plus its own remainder.
#[test]
fn test_tuple_struct_rest_bounds() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_std(Std::Unqualified)
            .with_required_trait(TypespaceTrait::JsonSchema),
        {
            struct Fixed(u32, #[flatten] [u32; 3]);
            struct Nested(String, #[flatten] Fixed);
        }
    );
    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include(
        "tests/output/test_tuple_struct_rest_bounds.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        let array = schemars::schema_for!(import::Fixed).schema.array.unwrap();
        assert_eq!(array.min_items, Some(4));
        assert_eq!(array.max_items, Some(4));

        let array = schemars::schema_for!(import::Nested).schema.array.unwrap();
        assert_eq!(array.min_items, Some(5));
        assert_eq!(array.max_items, Some(5));
    }
}

// A default value has to supply as many elements as the tuple accepts:
// exactly the fields when there is no remainder, and within the
// remainder's own bounds when there is one.
#[test]
fn test_tuple_struct_default_length_bounds() {
    let too_long = typespace_builder!(Settings::minimal(), {
        #[default = [1, 2, 3]]
        struct Pair(u32, u32);
    });
    assert!(matches!(
        too_long.finalize(no_cycles),
        Err(Error::InvalidDefault { ref reason, .. })
            if reason == "expected an array of length 2"
    ));

    let under = typespace_builder!(Settings::minimal(), {
        #[default = [1, 2]]
        struct Bounded(u32, #[flatten] [u32; 2]);
    });
    assert!(matches!(
        under.finalize(no_cycles),
        Err(Error::InvalidDefault { ref reason, .. })
            if reason == "expected an array of length 3"
    ));

    let over = typespace_builder!(Settings::minimal(), {
        #[default = [1, 2, 3, 4]]
        struct Bounded(u32, #[flatten] [u32; 2]);
    });
    assert!(matches!(
        over.finalize(no_cycles),
        Err(Error::InvalidDefault { .. })
    ));

    let just_right = typespace_builder!(Settings::minimal(), {
        #[default = [1, 2, 3]]
        struct Bounded(u32, #[flatten] [u32; 2]);
    });
    assert!(just_right.finalize(no_cycles).is_ok());
}

#[test]
fn flattened_default_value_supplies_the_flattened_property() {
    let builder = typespace_builder!(
        Settings::minimal().with_desired_trait(TypespaceTrait::Default),
        {
            #[default = { "bar": 1, "baz": 2 }]
            struct Foo {
                #[flatten]
                inner: Bar,
            }
            struct Bar {
                bar: u32,
                baz: u32,
            }
        }
    );
    let ts = builder.finalize(no_cycles).expect("finalize succeeds");
    assert!(
        ts.get_type(&"Foo".to_string())
            .has_impl(TypespaceTrait::Default),
        "Foo lost Default even though its default value supplies every field"
    );
}

// A default value only reaches generated code through an impl that
// trait resolution may never grant. A whole-type default renders
// inside a `Default` impl, so a type that never receives `Default`
// never constructs its value, and the natives inside that value are
// never deserialized.
//
// `check_type_defaults` walks every attached default value regardless
// and seeds `Deserialize` for each native it finds, so the requirement
// lands whether or not the code that would deserialize it is emitted.
// Under `Settings::minimal()`, where nothing requires `Default`, that
// refuses a graph whose output would have been fine. The conflict even
// says "generated code does so by deserializing it", which is a claim
// about a decision that has not been made when the seed is created.
//
// The same conditionality applies to a property-level `DefaultValue`,
// whose `defaults::` function is reached only on a `Deserialize` path.
#[test]
fn a_default_value_that_is_never_rendered_requires_nothing() {
    let builder = typespace_builder!(Settings::minimal(), {
        native ::ext::Thing: Clone + !Deserialize;

        #[default = ["x", 7]]
        struct Holder(::ext::Thing, u32);
    });

    let typespace = builder
        .finalize(no_cycles)
        .expect("Holder never gets Default, so its value is never constructed");

    let rendered = typespace.to_codespace().into_stream().to_string();
    assert!(
        !rendered.contains("Default for Holder"),
        "nothing required Default of Holder, so no impl should be rendered"
    );
}

// A tuple struct's description reaches the schema as the `description`
// keyword, alongside the doc attribute that says the same thing to
// Rust.
#[test]
fn test_tuple_struct_schema_description() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_std(Std::Unqualified)
            .with_required_trait(TypespaceTrait::JsonSchema),
        {
            /// a widget
            struct Widget(String, u32);
        }
    );

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include(
        "tests/output/test_tuple_struct_schema_description.rs",
        ts.to_codespace().into_stream()
    )]
    fn inner() {
        let schema = serde_json::to_value(schemars::schema_for!(import::Widget)).unwrap();
        assert_eq!(
            schema["description"],
            serde_json::json!("a widget"),
            "{schema:#}"
        );
    }
}

// Facade modules for the crate-path override test: generated code
// reaches json-serde and regress through these rather than the
// canonical paths.
pub mod json_helpers {
    pub use json_serde::*;
}
pub mod regex_engine {
    pub use regress::*;
}

// The constrained newtype is inserted raw because the macro has no
// NewtypeConstraints syntax.
#[test]
fn test_crate_path_overrides() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_crate_path(GeneratedCrate::JsonSerde, "super::json_helpers")
        .with_crate_path(GeneratedCrate::Regress, "super::regex_engine");

    let mut builder = typespace_builder!(settings, {
        // `opt` deserializes through json-serde's deserialize_some and
        // `gone` renders as its Absent type; both paths come from the
        // override.
        struct Thing {
            opt: Optional<String>,
            gone: Optional<!>,
        }
    });

    builder
        .insert("code string".to_string(), Type::String)
        .unwrap();
    builder
        .insert(
            "code".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("code string".to_string())
                    .name("Code")
                    .constraints(NewtypeConstraints::String {
                        min: None,
                        max: None,
                        patterns: vec!["^x".to_string()],
                    }),
            ),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();
    let out = ts.to_codespace().into_stream();

    #[check_and_include("tests/output/test_crate_path_overrides.rs", out)]
    fn inner() {
        use import::*;

        // A missing optional property deserializes; an explicit null is
        // rejected by the deserializer reached through the facade path.
        let thing: Thing = serde_json::from_str("{}").unwrap();
        assert_eq!(thing.opt, None);
        assert!(matches!(thing.gone, json_helpers::Absent));
        serde_json::from_str::<Thing>(r#"{"opt": null}"#).unwrap_err();

        // Pattern validation runs through the facade regress path.
        let _ = Code::try_from("xyz").unwrap();
        let _ = Code::try_from("yz").unwrap_err();
    }
}

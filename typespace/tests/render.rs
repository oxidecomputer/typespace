// Copyright 2026 Oxide Computer Company

use codespace::Codespace;
use quote::{format_ident, quote};
use typespace::{
    build::{
        Enum, EnumTagType, EnumVariant, JsonValue, Native, NewtypeStruct, Struct, StructProperty,
        StructPropertySerde, StructPropertyState, TupleStruct, Type, TypeAlias, UnitStruct,
        VariantDetails,
    },
    error::{Error, NameAxis, OffenderReason, RequirementOrigin},
    no_cycles,
    settings::{OptionalNullable, Settings, Std},
    TypespaceBuilder, TypespaceTrait, TypespaceTraitSet,
};
use typespace_test_macro::check_and_include;

// Alias used by test_json_serde_crate_override: generated code refers to
// json-serde helpers through this renamed path.
use json_serde as my_json_serde;

// Stub for the user-provided type referenced by OptionalNullable::CustomType.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum OptionField<T> {
    #[default]
    #[serde(skip)]
    Absent,
    Null,
    Present(T),
}
impl<T> OptionField<T> {
    pub fn is_absent(&self) -> bool {
        matches!(self, OptionField::Absent)
    }
}

#[test]
fn test_struct_field_serde() {
    let configs = [
        (
            "ConflatedAsAbsent",
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Serialize)
                .with_required_trait(TypespaceTrait::Deserialize)
                .with_optional_nullable(OptionalNullable::ConflateAsAbsent),
        ),
        (
            "ConflatedAsNull",
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Serialize)
                .with_required_trait(TypespaceTrait::Deserialize)
                .with_optional_nullable(OptionalNullable::ConflateAsNull),
        ),
        (
            "DoubleOption",
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Serialize)
                .with_required_trait(TypespaceTrait::Deserialize)
                .with_optional_nullable(OptionalNullable::DoubleOption),
        ),
        (
            "CustomType",
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Serialize)
                .with_required_trait(TypespaceTrait::Deserialize)
                .with_optional_nullable(OptionalNullable::CustomType(
                    "super::OptionField".to_string(),
                )),
        ),
    ];

    // For each configuration we create a type with the following fields:
    // - optional_string: A string that may be absent
    // - required_option: Either a string or null, but must be present
    // - optional_option: A string, null, or absent
    // - default_string: A string with the intrinsic default (i.e. "")
    // - default_option: A string or null with the intrinsic default (i.e. null)
    // - peanut_string: A string with a custom default of "peanuts"
    // - peanut_option: A string or null with a custom default of "peanuts"
    let outputs = configs.into_iter().map(|(name, settings)| {
        let mut builder = TypespaceBuilder::new(settings);

        let string_id = "string".to_string();
        builder.insert(string_id.clone(), Type::String).unwrap();

        let option_id = "option_string".to_string();
        builder
            .insert(option_id.clone(), Type::Option(string_id.clone()))
            .unwrap();

        let properties = vec![
            StructProperty::new("optional_string", string_id.clone())
                .with_state(StructPropertyState::Optional),
            StructProperty::new("required_option", option_id.clone()),
            StructProperty::new("optional_option", option_id.clone())
                .with_state(StructPropertyState::Optional),
            StructProperty::new("default_string", string_id.clone())
                .with_state(StructPropertyState::Default),
            StructProperty::new("default_option", option_id.clone())
                .with_state(StructPropertyState::Default),
            StructProperty::new("peanut_string", string_id.clone()).with_state(
                StructPropertyState::DefaultValue(JsonValue::new(serde_json::json!("peanuts"))),
            ),
            StructProperty::new("peanut_option", option_id.clone()).with_state(
                StructPropertyState::DefaultValue(JsonValue::new(serde_json::json!("peanuts"))),
            ),
        ];

        builder
            .insert(
                "X".to_string(),
                Struct::new()
                    .name(name)
                    .properties(properties)
                    .build()
                    .unwrap(),
            )
            .unwrap();

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
    }
}

#[test]
fn test_unit_struct() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
    );

    builder
        .insert(
            "MyUnitStruct".to_string(),
            UnitStruct::new(serde_json::json!("<<+>>"))
                .name("MyUnitStruct")
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).expect("finalize typespace");

    #[check_and_include("tests/output/test_unit_struct.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = import::MyUnitStruct;
        assert_eq!(serde_json::to_string(&value).unwrap(), "\"<<+>>\"");

        assert!(serde_json::from_str::<import::MyUnitStruct>("\"<<+>>\"").is_ok());
        assert!(serde_json::from_str::<import::MyUnitStruct>("null").is_err());
    }
}

#[test]
fn test_tuple_struct() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_std(Std::Unqualified)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::Serialize),
    );

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let string_vec_id = "string_vec".to_string();
    builder
        .insert(string_vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    builder
        .insert(
            "MyTupleStruct".to_string(),
            TupleStruct::new()
                .name("MyTupleStruct")
                .fields(vec![string_id, int_id])
                .rest(string_vec_id)
                .build()
                .unwrap(),
        )
        .unwrap();

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
    }
}

// Enums: one test covering all four serde tag types.
#[test]
fn test_enums() {
    let configs: &[(&str, EnumTagType)] = &[
        ("External", EnumTagType::External),
        (
            "Internal",
            EnumTagType::Internal {
                tag: "type".to_string(),
            },
        ),
        (
            "Adjacent",
            EnumTagType::Adjacent {
                tag: "t".to_string(),
                content: "c".to_string(),
            },
        ),
        ("Untagged", EnumTagType::Untagged),
    ];

    let outputs = configs.iter().map(|(name, tag_type)| {
        let mut builder = TypespaceBuilder::new(
            Settings::minimal()
                .with_std(Std::Unqualified)
                .with_required_trait(TypespaceTrait::Deserialize)
                .with_required_trait(TypespaceTrait::Serialize),
        );

        let string_id = "string".to_string();
        builder.insert(string_id.clone(), Type::String).unwrap();

        let int_id = "integer".to_string();
        builder
            .insert(int_id.clone(), Type::Integer("u32".to_string()))
            .unwrap();

        // Internal tagging doesn't support newtype variants wrapping non-struct
        // types, so we use only unit and struct variants for Internal.
        let variants = match tag_type {
            EnumTagType::Internal { .. } => vec![
                EnumVariant::new("Unit", VariantDetails::Unit),
                EnumVariant::new(
                    "Named",
                    VariantDetails::Struct(vec![StructProperty::new("x", int_id.clone())]),
                ),
            ],
            _ => vec![
                EnumVariant::new("Unit", VariantDetails::Unit),
                EnumVariant::new("Item", VariantDetails::Item(string_id.clone())),
                EnumVariant::new(
                    "Named",
                    VariantDetails::Struct(vec![StructProperty::new("x", int_id.clone())]),
                ),
            ],
        };

        builder
            .insert(
                "E".to_string(),
                Enum::new()
                    .name(*name)
                    .tag_type(tag_type.clone())
                    .variants(variants)
                    .build()
                    .unwrap(),
            )
            .unwrap();

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

#[test]
fn test_newtype_struct() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
    );

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    builder
        .insert(
            "MyString".to_string(),
            NewtypeStruct::new(string_id)
                .name("MyString")
                .description("A newtype wrapping String.".to_string())
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "MyInt".to_string(),
            NewtypeStruct::new(int_id).name("MyInt").build().unwrap(),
        )
        .unwrap();

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
    }
}

#[test]
fn test_type_alias() {
    let mut builder = TypespaceBuilder::new(Settings::minimal().with_std(Std::Unqualified));

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let vec_string_id = "vec_string".to_string();
    builder
        .insert(vec_string_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    builder
        .insert(
            "MyAlias".to_string(),
            TypeAlias::new(string_id).name("MyAlias").build().unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "StringList".to_string(),
            TypeAlias::new(vec_string_id)
                .name("StringList")
                .description("A list of strings.".to_string())
                .build()
                .unwrap(),
        )
        .unwrap();

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
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
    );

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    // Inner struct that will be flattened.
    builder
        .insert(
            "Inner".to_string(),
            Struct::new()
                .name("Inner")
                .properties(vec![StructProperty::new("value", int_id.clone())])
                .build()
                .unwrap(),
        )
        .unwrap();

    let inner_id = "Inner".to_string();

    // Outer struct with a renamed field and a flattened inner struct.
    builder
        .insert(
            "Outer".to_string(),
            Struct::new()
                .name("Outer")
                .properties(vec![
                    StructProperty::new("my_field", string_id.clone())
                        .with_json_name(StructPropertySerde::Rename("my-field".to_string())),
                    StructProperty::new("inner", inner_id)
                        .with_json_name(StructPropertySerde::Flatten),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

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
    let mut builder = TypespaceBuilder::new(Settings::minimal().with_std(Std::Unqualified));

    let uuid_id = "path".to_string();
    builder
        .insert(
            uuid_id.clone(),
            Type::Native(Native::new_string_like("std::path::PathBuf")),
        )
        .unwrap();

    builder
        .insert(
            "Resource".to_string(),
            Struct::new()
                .name("Resource")
                .properties(vec![StructProperty::new("location", uuid_id)])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_native_type.rs", ts.to_codespace().into_stream())]
    fn inner() {}
}

/// A `Type::Never` field renders as `::json_serde::Absent`, the leaf
/// type's fully-qualified external path--mirroring how `Type::JsonValue`
/// renders as `::serde_json::Value`--and is unconditionally
/// `#[serde(skip)]`ped: `Absent`'s `Serialize` impl always errors, so
/// the field must never be serialized.
#[test]
fn test_never_field() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_required_trait(TypespaceTrait::Debug),
    );

    builder.insert("never".to_string(), Type::Never).unwrap();

    builder
        .insert(
            "Gone".to_string(),
            Struct::new()
                .name("Gone")
                .properties(vec![StructProperty::new("value", "never".to_string())])
                .build()
                .unwrap(),
        )
        .unwrap();

    let ts = builder.finalize(no_cycles).unwrap();

    #[check_and_include("tests/output/test_never_field.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let value = import::Gone {
            value: ::json_serde::Absent,
        };

        // The skip is unconditional: serializing produces an empty
        // object (Absent's Serialize would error if it were ever
        // invoked), and deserializing an empty object succeeds,
        // filling the field via Absent's Default.
        assert_eq!(serde_json::to_string(&value).unwrap(), "{}");
        assert!(serde_json::from_str::<import::Gone>("{}").is_ok());
        assert!(serde_json::from_str::<import::Gone>(r#"{ "value": null }"#).is_err());
    }
}

#[test]
fn test_compound_field_types() {
    let mut builder = TypespaceBuilder::new(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize)
            .with_std(Std::Unqualified),
    );

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    let bool_id = "boolean".to_string();
    builder.insert(bool_id.clone(), Type::Boolean).unwrap();

    let float_id = "float".to_string();
    builder
        .insert(float_id.clone(), Type::Float("f64".to_string()))
        .unwrap();

    let json_id = "json".to_string();
    builder.insert(json_id.clone(), Type::JsonValue).unwrap();

    let vec_id = "vec_string".to_string();
    builder
        .insert(vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    let map_id = "map".to_string();
    builder
        .insert(map_id.clone(), Type::Map(string_id.clone(), int_id.clone()))
        .unwrap();

    let set_id = "set".to_string();
    builder
        .insert(set_id.clone(), Type::Set(string_id.clone()))
        .unwrap();

    let array_id = "array".to_string();
    builder
        .insert(array_id.clone(), Type::Array(int_id.clone(), 3))
        .unwrap();

    let opt_id = "opt_string".to_string();
    builder
        .insert(opt_id.clone(), Type::Option(string_id.clone()))
        .unwrap();

    let box_string_id = "box_string".to_string();
    builder
        .insert(box_string_id.clone(), Type::Box(string_id.clone()))
        .unwrap();

    let box_vec_id = "box_vec_string".to_string();
    builder
        .insert(box_vec_id.clone(), Type::Box(vec_id.clone()))
        .unwrap();

    let tuple_id = "tuple".to_string();
    builder
        .insert(
            tuple_id.clone(),
            Type::Tuple(vec![string_id.clone(), int_id.clone()]),
        )
        .unwrap();

    builder
        .insert(
            "All".to_string(),
            Struct::new()
                .name("All")
                .properties(vec![
                    StructProperty::new("a_bool", bool_id.clone()),
                    StructProperty::new("an_int", int_id.clone()),
                    StructProperty::new("a_float", float_id.clone()),
                    StructProperty::new("a_string", string_id.clone()),
                    StructProperty::new("a_json", json_id),
                    StructProperty::new("a_vec", vec_id.clone()),
                    StructProperty::new("a_map", map_id.clone()),
                    StructProperty::new("a_set", set_id.clone()),
                    StructProperty::new("an_array", array_id.clone()),
                    StructProperty::new("a_tuple", tuple_id.clone()),
                    StructProperty::new("a_box_string", box_string_id.clone()),
                    StructProperty::new("a_box_vec", box_vec_id.clone()),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    // Exercise StructPropertyState::Default for each applicable field type.
    // JsonValue is excluded: Default is not supported for it.
    builder
        .insert(
            "Defaults".to_string(),
            Struct::new()
                .name("Defaults")
                .properties(vec![
                    StructProperty::new("a_bool", bool_id).with_state(StructPropertyState::Default),
                    StructProperty::new("an_int", int_id).with_state(StructPropertyState::Default),
                    StructProperty::new("a_float", float_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("a_string", string_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("a_vec", vec_id).with_state(StructPropertyState::Default),
                    StructProperty::new("a_map", map_id).with_state(StructPropertyState::Default),
                    StructProperty::new("a_set", set_id).with_state(StructPropertyState::Default),
                    StructProperty::new("an_array", array_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("a_tuple", tuple_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("an_option", opt_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("a_box_string", box_string_id)
                        .with_state(StructPropertyState::Default),
                    StructProperty::new("a_box_vec", box_vec_id)
                        .with_state(StructPropertyState::Default),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

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
            serde_json::json!({"an_int": 0, "a_float": 0.0, "an_array": [0, 0, 0], "a_tuple": ["", 0]})
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
            RequirementOrigin::MapKey(id) if id == "map"
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
         required because keys of map `map` must implement `Eq`"
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
    }) = builder.validate()
    else {
        panic!("expected validate to fail with a duplicate type name");
    };
    assert_eq!(name, "Twin");
    assert_eq!(first, "first");
    assert_eq!(second, "second");

    assert!(matches!(
        builder.finalize(no_cycles),
        Err(Error::DuplicateTypeName { .. })
    ));
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

// A configured map type carries its own key-trait demands: a hash map
// requires Hash and Eq of its keys--not Ord--and conflicts name exactly
// the configured traits.
#[test]
fn test_map_key_traits_override() {
    let settings = Settings::minimal().with_map_type(
        "::std::collections::HashMap",
        [
            TypespaceTrait::Hash,
            TypespaceTrait::Eq,
            TypespaceTrait::PartialEq,
        ]
        .into_iter()
        .collect(),
    );
    let mut builder = TypespaceBuilder::new(settings);

    let float_id = "float".to_string();
    builder
        .insert(float_id.clone(), Type::Float("f64".to_string()))
        .unwrap();
    builder.insert("value".to_string(), Type::String).unwrap();
    builder
        .insert("map".to_string(), Type::Map(float_id, "value".to_string()))
        .unwrap();

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
                    StructProperty::new("a", struct_a_id).with_state(StructPropertyState::Optional)
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
                    StructProperty::new("c", c_id).with_state(StructPropertyState::Optional)
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
                    StructProperty::new("b", b_id).with_state(StructPropertyState::Optional)
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
        .with_map_type(
            "::std::collections::HashMap",
            [
                TypespaceTrait::Hash,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
            ]
            .into_iter()
            .collect(),
        )
        .with_set_type(
            "::std::collections::BTreeSet",
            [
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
            ]
            .into_iter()
            .collect(),
        )
        .with_vec_type("::std::collections::VecDeque");
    let mut builder = TypespaceBuilder::new(settings);

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    let json_id = "json".to_string();
    builder.insert(json_id.clone(), Type::JsonValue).unwrap();

    let map_id = "map".to_string();
    builder
        .insert(map_id.clone(), Type::Map(string_id.clone(), int_id.clone()))
        .unwrap();

    let set_id = "set".to_string();
    builder
        .insert(set_id.clone(), Type::Set(string_id.clone()))
        .unwrap();

    let vec_id = "vec".to_string();
    builder
        .insert(vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    let obj_id = "obj".to_string();
    builder
        .insert(
            obj_id.clone(),
            Type::Map(string_id.clone(), json_id.clone()),
        )
        .unwrap();

    builder
        .insert(
            "Containers".to_string(),
            Struct::new()
                .name("Containers")
                .properties(vec![
                    StructProperty::new("a_map", map_id.clone()),
                    StructProperty::new("a_set", set_id.clone()),
                    StructProperty::new("a_vec", vec_id.clone()),
                    StructProperty::new("an_obj", obj_id.clone()),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    // Default-state fields exercise the is_empty path against the
    // overridden container types (and the ::serde_json::Map special case).
    builder
        .insert(
            "ContainerDefaults".to_string(),
            Struct::new()
                .name("ContainerDefaults")
                .properties(vec![
                    StructProperty::new("a_map", map_id).with_state(StructPropertyState::Default),
                    StructProperty::new("a_set", set_id).with_state(StructPropertyState::Default),
                    StructProperty::new("a_vec", vec_id).with_state(StructPropertyState::Default),
                    StructProperty::new("an_obj", obj_id).with_state(StructPropertyState::Default),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

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
    let mut builder = TypespaceBuilder::new(settings);

    let string_id = "string".to_string();
    builder.insert(string_id.clone(), Type::String).unwrap();

    let int_id = "integer".to_string();
    builder
        .insert(int_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    let vec_id = "vec".to_string();
    builder
        .insert(vec_id.clone(), Type::Vec(string_id.clone()))
        .unwrap();

    builder
        .insert(
            "Widget".to_string(),
            Struct::new()
                .name("Widget")
                .properties(vec![
                    StructProperty::new("name", string_id.clone()),
                    StructProperty::new("tags", vec_id.clone()),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "Gadget".to_string(),
            Enum::new()
                .name("Gadget")
                .tag_type(EnumTagType::External)
                .variants(vec![
                    EnumVariant::new("Off", VariantDetails::Unit),
                    EnumVariant::new("On", VariantDetails::Item(int_id.clone())),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "Wrapper".to_string(),
            NewtypeStruct::new(string_id.clone())
                .name("Wrapper")
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "Marker".to_string(),
            UnitStruct::new(serde_json::json!("marker"))
                .name("Marker")
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "Pair".to_string(),
            TupleStruct::new()
                .name("Pair")
                .fields(vec![string_id.clone(), int_id.clone()])
                .build()
                .unwrap(),
        )
        .unwrap();

    builder
        .insert(
            "Named".to_string(),
            TypeAlias::new(string_id.clone())
                .name("Named")
                .build()
                .unwrap(),
        )
        .unwrap();

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
    let mut builder = TypespaceBuilder::new(settings);

    let float_id = "float".to_string();
    builder
        .insert(float_id.clone(), Type::Float("f64".to_string()))
        .unwrap();

    builder
        .insert(
            "Holder".to_string(),
            Struct::new()
                .name("Holder")
                .properties(vec![StructProperty::new("value", float_id)])
                .build()
                .unwrap(),
        )
        .unwrap();

    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };

    // The requested Eq propagates into Holder's field and fails there.
    assert_eq!(conflicts.len(), 1);
    let conflict = &conflicts[0];
    assert!(matches!(conflict.required, TypespaceTrait::Eq));
    assert!(matches!(conflict.origin, RequirementOrigin::GlobalSettings));
    assert_eq!(conflict.offender, "float");
    assert_eq!(
        conflict.to_string(),
        "type `f64` (id `float`) cannot implement the required trait `Eq`\n    \
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
         required because elements of set `set` must implement `Eq`"
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

    // validate() reports the conflicts and leaves the builder usable.
    let Err(Error::TraitConflicts { conflicts }) = builder.validate() else {
        panic!("expected validate to fail with trait conflicts");
    };
    assert_eq!(conflicts.len(), 4);
    for conflict in &conflicts {
        assert_eq!(conflict.offender, "date");
        assert!(matches!(
            &conflict.origin,
            RequirementOrigin::MapKey(id) if id == "map"
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
         required because keys of map `map` must implement `Eq`"
    );

    // finalize reports the same failure.
    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected finalize to fail with trait conflicts");
    };
    assert_eq!(conflicts.len(), 4);

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

    builder.validate().expect("validate passes");
    builder.finalize(no_cycles).expect("finalize passes");
}

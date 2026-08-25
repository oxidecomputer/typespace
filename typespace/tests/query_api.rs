// Copyright 2026 Oxide Computer Company

use quote::quote;
use typespace::{
    build::{
        Enum, EnumTagType, EnumVariant, Struct, StructProperty, StructPropertyState, Type,
        VariantDetails,
    },
    no_cycles,
    settings::Settings,
    view, TypeSpaceImpl, TypespaceBuilder,
};

fn make_typespace() -> typespace::Typespace<String> {
    let mut builder = TypespaceBuilder::default();

    let str_id = "str".to_string();
    builder.insert(str_id.clone(), Type::String).unwrap();

    let u32_id = "u32".to_string();
    builder
        .insert(u32_id.clone(), Type::Integer("u32".to_string()))
        .unwrap();

    let bool_id = "bool".to_string();
    builder.insert(bool_id.clone(), Type::Boolean).unwrap();

    let opt_str_id = "opt_str".to_string();
    builder
        .insert(opt_str_id.clone(), Type::Option(str_id.clone()))
        .unwrap();

    // A struct with three properties.
    let struct_id = "MyStruct".to_string();
    builder
        .insert(
            struct_id.clone(),
            Struct::new()
                .name("MyStruct")
                .description("A sample struct".to_string())
                .properties(vec![
                    StructProperty::new("name", str_id.clone())
                        .with_description("The name field".to_string()),
                    StructProperty::new("count", u32_id.clone()),
                    StructProperty::new("label", opt_str_id.clone())
                        .with_state(StructPropertyState::Optional),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    // An enum with three variants: unit, single-item (Item), multi-item (Tuple).
    let enum_id = "MyEnum".to_string();
    builder
        .insert(
            enum_id.clone(),
            Enum::new()
                .name("MyEnum")
                .description("A sample enum".to_string())
                .tag_type(EnumTagType::External)
                .variants(vec![
                    EnumVariant::new("Nothing", VariantDetails::Unit),
                    EnumVariant::new("Single", VariantDetails::Item(str_id.clone())),
                    EnumVariant::new(
                        "Pair",
                        VariantDetails::Tuple(vec![str_id.clone(), u32_id.clone()]),
                    ),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();

    builder.finalize(no_cycles).unwrap()
}

#[test]
#[should_panic(expected = "invalid type id")]
fn get_type_panics_for_unknown_id() {
    let ts = make_typespace();
    ts.get_type(&"nonexistent".to_string());
}

#[test]
fn iter_types_covers_all_inserted_ids() {
    let ts = make_typespace();
    let names: std::collections::BTreeSet<String> = ts.iter_types().map(|t| t.name()).collect();
    assert!(names.contains("MyStruct"), "missing MyStruct in {names:?}");
    assert!(names.contains("MyEnum"), "missing MyEnum in {names:?}");
}

#[test]
fn struct_properties_via_get_type() {
    let ts = make_typespace();
    let ti = ts.get_type(&"MyStruct".to_string());

    assert_eq!(ti.name(), "MyStruct");
    assert_eq!(ti.description(), Some("A sample struct"));

    let view::TypeDetails::Struct(s) = ti.details() else {
        panic!("expected Struct, got something else");
    };

    let props: Vec<_> = s.properties_info().collect();
    assert_eq!(props.len(), 3);

    let name_prop = props.iter().find(|p| p.name == "name").unwrap();
    assert!(name_prop.required);
    assert_eq!(name_prop.description, Some("The name field"));

    let label_prop = props.iter().find(|p| p.name == "label").unwrap();
    assert!(!label_prop.required);
}

#[test]
fn enum_variants_via_get_type() {
    let ts = make_typespace();
    let ti = ts.get_type(&"MyEnum".to_string());

    assert_eq!(ti.name(), "MyEnum");
    assert_eq!(ti.description(), Some("A sample enum"));

    let view::TypeDetails::Enum(e) = ti.details() else {
        panic!("expected Enum, got something else");
    };

    let variants: Vec<_> = e.variants_info().collect();
    assert_eq!(variants.len(), 3);

    let nothing = variants.iter().find(|v| v.name == "Nothing").unwrap();
    assert!(matches!(nothing.details, view::VariantDetails::Unit));

    let single = variants.iter().find(|v| v.name == "Single").unwrap();
    assert!(
        matches!(&single.details, view::VariantDetails::Tuple(ids) if ids.len() == 1),
        "Single should be a one-element Tuple"
    );

    let pair = variants.iter().find(|v| v.name == "Pair").unwrap();
    assert!(
        matches!(&pair.details, view::VariantDetails::Tuple(ids) if ids.len() == 2),
        "Pair should be a two-element Tuple"
    );
}

#[test]
fn ident_produces_expected_tokens() {
    let ts = make_typespace();

    let struct_ti = ts.get_type(&"MyStruct".to_string());
    let struct_ident = struct_ti.ident();
    assert_eq!(struct_ident.to_string(), quote! { MyStruct }.to_string());

    let param_ident = struct_ti.parameter_ident();
    assert_eq!(param_ident.to_string(), quote! { &MyStruct }.to_string());

    let bool_ti = ts.get_type(&"bool".to_string());
    let bool_param = bool_ti.parameter_ident();
    assert_eq!(bool_param.to_string(), quote! { bool }.to_string());
}

#[test]
fn has_impl_false_for_plain_types() {
    let ts = make_typespace();
    let ti = ts.get_type(&"MyStruct".to_string());
    assert!(!ti.has_impl(TypeSpaceImpl::Display));
    assert!(!ti.has_impl(TypeSpaceImpl::FromStr));
}

// Chunk-5 query additions on the build side: names, naming contexts,
// defaults, and enum analyses.
#[test]
fn build_side_queries() {
    let str_id = "str".to_string();

    // Type::name is Some for named types, None for the rest.
    let named = Struct::<String>::new().name("Widget").build().unwrap();
    assert_eq!(named.name(), Some("Widget"));
    assert_eq!(Type::<String>::String.name(), None);
    assert_eq!(Type::Option(str_id.clone()).name(), None);

    // set_default is the post-construction counterpart of the fluent
    // default methods.
    let mut named = named;
    named.set_default(Some(typespace::build::JsonValue::new(
        serde_json::json!({}),
    )));

    // children_with_context: naming contexts per child.
    let typ = Struct::new()
        .name("S")
        .properties(vec![
            StructProperty::new("alpha", "a".to_string()),
            StructProperty::new("beta", "b".to_string()),
        ])
        .build()
        .unwrap();
    assert_eq!(
        typ.children_with_context(),
        vec![
            ("a".to_string(), "alpha".to_string()),
            ("b".to_string(), "beta".to_string()),
        ]
    );

    let typ = Enum::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![
            EnumVariant::new("Unit", VariantDetails::Unit),
            EnumVariant::new("One", VariantDetails::Item("a".to_string())),
            EnumVariant::new(
                "Pair",
                VariantDetails::Tuple(vec!["a".to_string(), "b".to_string()]),
            ),
            EnumVariant::new(
                "Named",
                VariantDetails::Struct(vec![StructProperty::new("x", "a".to_string())]),
            ),
        ])
        .build()
        .unwrap();
    assert_eq!(
        typ.children_with_context(),
        vec![
            ("a".to_string(), "One".to_string()),
            ("a".to_string(), "Pair.0".to_string()),
            ("b".to_string(), "Pair.1".to_string()),
            ("a".to_string(), "Named.x".to_string()),
        ]
    );

    assert_eq!(
        Type::Map("k".to_string(), "v".to_string()).children_with_context(),
        vec![
            ("k".to_string(), "key".to_string()),
            ("v".to_string(), "value".to_string()),
        ]
    );
    assert_eq!(
        Type::Option("o".to_string()).children_with_context(),
        vec![("o".to_string(), "".to_string())]
    );

    // json_name: the wire name of a variant.
    let plain = EnumVariant::<String>::new("Alpha", VariantDetails::Unit);
    assert_eq!(plain.json_name(), "Alpha");
    let renamed = EnumVariant::<String>::new("Alpha", VariantDetails::Unit).with_rename("alpha");
    assert_eq!(renamed.json_name(), "alpha");

    // all_simple_variants: nonempty, tagged, all-unit enums only.
    let simple = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![
            EnumVariant::new("A", VariantDetails::Unit),
            EnumVariant::new("B", VariantDetails::Unit),
        ]);
    assert!(simple.all_simple_variants());

    let untagged = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::Untagged)
        .variants(vec![EnumVariant::new("A", VariantDetails::Unit)]);
    assert!(!untagged.all_simple_variants());

    let empty = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::External);
    assert!(!empty.all_simple_variants());

    let data = Enum::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![EnumVariant::new(
            "A",
            VariantDetails::Item("a".to_string()),
        )]);
    assert!(!data.all_simple_variants());
}

// Scoped identifier rendering on the view side, and the pre-finalize
// equivalents on the builder.
#[test]
fn scoped_and_prefinalize_idents() {
    let mut builder =
        TypespaceBuilder::new(Settings::minimal().with_std(typespace::settings::Std::Unqualified));

    let str_id = "str".to_string();
    builder.insert(str_id.clone(), Type::String).unwrap();

    let vec_id = "vec".to_string();
    builder
        .insert(vec_id.clone(), Type::Vec("MyStruct".to_string()))
        .unwrap();

    let struct_id = "MyStruct".to_string();
    builder
        .insert(
            struct_id.clone(),
            Struct::new()
                .name("MyStruct")
                .properties(vec![StructProperty::new("name", str_id.clone())])
                .build()
                .unwrap(),
        )
        .unwrap();

    // Pre-finalize rendering on the builder.
    assert_eq!(
        builder.ident(&struct_id).to_string(),
        quote! { MyStruct }.to_string()
    );
    assert_eq!(
        builder.ident_in(&struct_id, "types").to_string(),
        quote! { types::MyStruct }.to_string()
    );
    assert_eq!(
        builder.ident_in(&vec_id, "types").to_string(),
        quote! { Vec<types::MyStruct> }.to_string()
    );
    assert_eq!(
        builder.parameter_ident(&struct_id).to_string(),
        quote! { &MyStruct }.to_string()
    );
    assert_eq!(
        builder.parameter_ident_in(&vec_id, "types").to_string(),
        quote! { &Vec<types::MyStruct> }.to_string()
    );
    assert_eq!(
        builder.parameter_ident(&str_id).to_string(),
        quote! { String }.to_string()
    );

    // The same queries after finalization, on the view.
    let ts = builder.finalize(no_cycles).unwrap();
    let ti = ts.get_type(&struct_id);
    assert_eq!(
        ti.ident_in("types").to_string(),
        quote! { types::MyStruct }.to_string()
    );
    let vi = ts.get_type(&vec_id);
    assert_eq!(
        vi.parameter_ident_in("types").to_string(),
        quote! { &Vec<types::MyStruct> }.to_string()
    );
}

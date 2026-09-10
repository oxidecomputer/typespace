// Copyright 2026 Oxide Computer Company

use quote::quote;
use typespace::{
    TypeSpaceImpl,
    build::{
        Enum, EnumTagType, EnumVariant, NewtypeStruct, Struct, StructProperty, TupleStruct, Type,
        TypeAlias, UnitStruct, VariantDetails,
    },
    no_cycles,
    settings::Settings,
    view,
};
use typespace_test_macro::typespace_builder;

fn make_typespace() -> typespace::Typespace<String> {
    let builder = typespace_builder!(Settings::typical(), {
        // A struct with three properties.
        /// A sample struct
        struct MyStruct {
            /// The name field
            name: String,
            count: u32,
            label: Optional<String>,
        }

        // An enum with three variants: unit, single-item (Item), multi-item (Tuple).
        /// A sample enum
        enum MyEnum {
            Nothing,
            Single(String),
            Pair(String, u32),
        }

        // A plain primitive with no named type of its own, so
        // `ident_produces_expected_tokens` has a bare type to query
        // independent of MyStruct and MyEnum.
        type Flag = bool;
    });

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

// Parameter position borrows what a caller cannot hand over cheaply
// and passes the rest by value. A String becomes a &str rather than a
// &String; an enum whose variants are all unit variants goes by value
// while one carrying a payload is borrowed; and an Option and a tuple
// keep their own syntax and apply the rule to what they hold.
#[test]
fn parameter_idents_borrow_by_rule() {
    let settings = Settings::minimal().with_std(typespace::settings::Std::Unqualified);
    let builder = typespace_builder!(settings, {
        enum UnitEnum {
            One,
            Two,
        }

        enum PayloadEnum {
            Bare,
            Wrapped(u32),
        }

        struct MyStruct {
            name: String,
        }

        struct Positions {
            flag: bool,
            vec: Vec<MyStruct>,
            opt_u32: Nullable<u32>,
            opt_str: Nullable<String>,
            opt_struct: Nullable<MyStruct>,
            tuple: (u32, String, MyStruct),
        }
    });

    let str_id = "String".to_string();
    let struct_id = "MyStruct".to_string();
    let opt_str_id = "Nullable<String>".to_string();
    let opt_struct_id = "Nullable<MyStruct>".to_string();
    let unit_enum_id = "UnitEnum".to_string();

    let cases = [
        ("u32", quote! { u32 }),
        ("bool", quote! { bool }),
        ("String", quote! { &str }),
        ("UnitEnum", quote! { UnitEnum }),
        ("PayloadEnum", quote! { &PayloadEnum }),
        ("MyStruct", quote! { &MyStruct }),
        ("Vec<MyStruct>", quote! { &Vec<MyStruct> }),
        ("Nullable<u32>", quote! { Option<u32> }),
        ("Nullable<String>", quote! { Option<&str> }),
        ("Nullable<MyStruct>", quote! { Option<&MyStruct> }),
        ("(u32, String, MyStruct)", quote! { (u32, &str, &MyStruct) }),
    ];

    // The builder answers the same question before finalization.
    for (id, expected) in &cases {
        let id = id.to_string();
        assert_eq!(
            builder.parameter_ident(&id).to_string(),
            expected.to_string(),
            "pre-finalize parameter_ident for {id}"
        );
    }

    let ts = builder.finalize(no_cycles).unwrap();

    for (id, expected) in &cases {
        let id = id.to_string();
        assert_eq!(
            ts.get_type(&id).parameter_ident().to_string(),
            expected.to_string(),
            "parameter_ident for {id}"
        );
    }

    // A named type picks up the scope; the borrow sits outside it.
    assert_eq!(
        ts.get_type(&struct_id)
            .parameter_ident_in("types")
            .to_string(),
        quote! { &types::MyStruct }.to_string()
    );
    assert_eq!(
        ts.get_type(&opt_struct_id)
            .parameter_ident_in("types")
            .to_string(),
        quote! { Option<&types::MyStruct> }.to_string()
    );
    assert_eq!(
        ts.get_type(&unit_enum_id)
            .parameter_ident_in("types")
            .to_string(),
        quote! { types::UnitEnum }.to_string()
    );

    // A lifetime names every reference the parameter introduces, and
    // only the references: a by-value parameter gains nothing.
    assert_eq!(
        ts.get_type(&struct_id)
            .parameter_ident_with_lifetime("a")
            .to_string(),
        quote! { &'a MyStruct }.to_string()
    );
    assert_eq!(
        ts.get_type(&str_id)
            .parameter_ident_with_lifetime("a")
            .to_string(),
        quote! { &'a str }.to_string()
    );
    assert_eq!(
        ts.get_type(&opt_str_id)
            .parameter_ident_with_lifetime("a")
            .to_string(),
        quote! { Option<&'a str> }.to_string()
    );
    assert_eq!(
        ts.get_type(&unit_enum_id)
            .parameter_ident_with_lifetime("a")
            .to_string(),
        quote! { UnitEnum }.to_string()
    );
}

#[test]
fn has_impl_false_for_plain_types() {
    let ts = make_typespace();
    let ti = ts.get_type(&"MyStruct".to_string());
    assert!(!ti.has_impl(TypeSpaceImpl::Display));
    assert!(!ti.has_impl(TypeSpaceImpl::FromStr));
}

/// Built-in types answer `has_impl` from what generated code really
/// gets: a String position renders as `String`, and integers and bool
/// implement everything tracked here. Floats parse and print but
/// carry no Eq, Ord, or Hash.
#[test]
fn has_impl_answers_for_builtins() {
    let builder = typespace_builder!(Settings::typical(), {
        struct Holder {
            name: String,
            count: u32,
            ratio: f64,
            flag: bool,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();

    for id in ["String", "u32", "bool"] {
        let ti = ts.get_type(&id.to_string());
        for impl_name in [
            TypeSpaceImpl::Display,
            TypeSpaceImpl::FromStr,
            TypeSpaceImpl::Eq,
            TypeSpaceImpl::Ord,
            TypeSpaceImpl::Hash,
        ] {
            assert!(ti.has_impl(impl_name), "{id} lacks {impl_name:?}");
        }
    }

    let ratio = ts.get_type(&"f64".to_string());
    assert!(ratio.has_impl(TypeSpaceImpl::Display));
    assert!(ratio.has_impl(TypeSpaceImpl::FromStr));
    assert!(!ratio.has_impl(TypeSpaceImpl::Eq));
    assert!(!ratio.has_impl(TypeSpaceImpl::Ord));
    assert!(!ratio.has_impl(TypeSpaceImpl::Hash));
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

    // all_unit_variants: nonempty, tagged, all-unit enums only.
    let simple = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![
            EnumVariant::new("A", VariantDetails::Unit),
            EnumVariant::new("B", VariantDetails::Unit),
        ]);
    assert!(simple.all_tagged_unit_variants());

    let untagged = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::Untagged)
        .variants(vec![EnumVariant::new("A", VariantDetails::Unit)]);
    assert!(!untagged.all_tagged_unit_variants());

    let empty = Enum::<String>::new()
        .name("E")
        .tag_type(EnumTagType::External);
    assert!(!empty.all_tagged_unit_variants());

    let data = Enum::new()
        .name("E")
        .tag_type(EnumTagType::External)
        .variants(vec![EnumVariant::new(
            "A",
            VariantDetails::Item("a".to_string()),
        )]);
    assert!(!data.all_tagged_unit_variants());
}

// Scoped identifier rendering on the view side, and the pre-finalize
// equivalents on the builder.
#[test]
fn scoped_and_prefinalize_idents() {
    let settings = Settings::minimal().with_std(typespace::settings::Std::Unqualified);
    let builder = typespace_builder!(settings, {
        struct MyStruct {
            name: String,
        }

        struct Positions {
            v: Vec<MyStruct>,
        }
    });

    let str_id = "String".to_string();
    let struct_id = "MyStruct".to_string();
    let vec_id = "Vec<MyStruct>".to_string();

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
        quote! { &str }.to_string()
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

// `#[deny_unknown_fields]` claimed through the macro compiles against
// the real crate and finalizes; the view layer has no accessor for
// the flag yet (that is the rendering work this attribute unblocks),
// so `get_deny_unknown_fields()`--called directly on the built type,
// the same way `build_side_queries` above exercises other build-side
// accessors--is the only way to prove it landed rather than being
// silently dropped by the macro's lowering.
#[test]
fn deny_unknown_fields_landed_via_the_macro() {
    let builder = typespace_builder!(Settings::typical(), {
        #[deny_unknown_fields]
        struct Widget {
            name: String,
        }

        #[deny_unknown_fields]
        #[untagged]
        enum Shape {
            Text(String),
        }
    });
    builder.finalize(no_cycles).unwrap();
}

#[test]
fn deny_unknown_fields_get_accessor_reflects_the_claim() {
    let Type::Struct(claimed) = Struct::<String>::new()
        .name("Widget")
        .deny_unknown_fields()
        .build()
        .unwrap()
    else {
        panic!("expected Type::Struct");
    };
    assert!(claimed.get_deny_unknown_fields());

    let Type::Struct(unclaimed) = Struct::<String>::new().name("Bare").build().unwrap() else {
        panic!("expected Type::Struct");
    };
    assert!(!unclaimed.get_deny_unknown_fields());

    let Type::Enum(claimed) = Enum::<String>::new()
        .name("Shape")
        .tag_type(EnumTagType::External)
        .deny_unknown_fields()
        .variants(vec![EnumVariant::new("Unit", VariantDetails::Unit)])
        .build()
        .unwrap()
    else {
        panic!("expected Type::Enum");
    };
    assert!(claimed.get_deny_unknown_fields());

    let Type::Enum(unclaimed) = Enum::<String>::new()
        .name("Shape")
        .tag_type(EnumTagType::External)
        .variants(vec![EnumVariant::new("Unit", VariantDetails::Unit)])
        .build()
        .unwrap()
    else {
        panic!("expected Type::Enum");
    };
    assert!(!unclaimed.get_deny_unknown_fields());
}

// The per-type `#[derive = [..]]` and `#[attr = [..]]` claimed through
// the macro compile against the real crate and finalize. Nothing
// renders either list yet, so the assertion this test can make is that
// the macro's lowering builds; that the lists survive the builder chain
// is `extra_derives_and_attrs_get_accessors_reflect_the_claim` below.
#[test]
fn extra_derives_and_attrs_landed_via_the_macro() {
    let builder = typespace_builder!(Settings::typical(), {
        #[derive = ["::std::hash::Hash", "PartialOrd"]]
        #[attr = ["serde(deny_unknown_fields)"]]
        struct Widget {
            name: String,
        }

        #[derive = ["::std::hash::Hash"]]
        struct Meters(u32);

        #[attr = ["allow(dead_code)"]]
        struct Pair(u32, String);

        #[json = null]
        #[derive = ["::std::hash::Hash"]]
        struct Nothing;

        #[derive = ["::std::hash::Hash"]]
        #[attr = ["allow(dead_code)", "non_exhaustive"]]
        enum Shape {
            Text(String),
        }

        #[attr = ["allow(dead_code)"]]
        type Label = String;
    });
    builder.finalize(no_cycles).unwrap();
}

// Every named shape stores what its `extra_derives`/`extra_attrs`
// builder methods were given, and hands it back through both the
// shape's `get_*` accessors and `Type`'s. Rendering reads neither list
// yet, so these accessors are the only proof the data landed rather
// than being dropped on the way through `build()`.
#[test]
fn extra_derives_and_attrs_get_accessors_reflect_the_claim() {
    let derives = ["::std::hash::Hash", "PartialOrd"];
    let attrs = ["allow(dead_code)"];

    let built = Struct::<String>::new()
        .name("Widget")
        .extra_derives(derives)
        .extra_attrs(attrs)
        .build()
        .unwrap();
    let Type::Struct(claimed) = &built else {
        panic!("expected Type::Struct");
    };
    assert_eq!(claimed.get_extra_derives(), derives);
    assert_eq!(claimed.get_extra_attrs(), attrs);
    assert_eq!(built.extra_derives(), derives);
    assert_eq!(built.extra_attrs(), attrs);

    let built = Enum::<String>::new()
        .name("Shape")
        .tag_type(EnumTagType::External)
        .extra_derives(derives)
        .extra_attrs(attrs)
        .variants(vec![EnumVariant::new("Unit", VariantDetails::Unit)])
        .build()
        .unwrap();
    let Type::Enum(claimed) = &built else {
        panic!("expected Type::Enum");
    };
    assert_eq!(claimed.get_extra_derives(), derives);
    assert_eq!(claimed.get_extra_attrs(), attrs);

    let built = UnitStruct::new(serde_json::Value::Null)
        .name("Nothing")
        .extra_derives(derives)
        .extra_attrs(attrs)
        .build::<String>()
        .unwrap();
    let Type::UnitStruct(claimed) = &built else {
        panic!("expected Type::UnitStruct");
    };
    assert_eq!(claimed.get_extra_derives(), derives);
    assert_eq!(claimed.get_extra_attrs(), attrs);

    let built = TupleStruct::new()
        .name("Pair")
        .extra_derives(derives)
        .extra_attrs(attrs)
        .fields(["str".to_string(), "str".to_string()])
        .build()
        .unwrap();
    let Type::TupleStruct(claimed) = &built else {
        panic!("expected Type::TupleStruct");
    };
    assert_eq!(claimed.get_extra_derives(), derives);
    assert_eq!(claimed.get_extra_attrs(), attrs);

    let built = NewtypeStruct::new("str".to_string())
        .name("Meters")
        .extra_derives(derives)
        .extra_attrs(attrs)
        .build()
        .unwrap();
    let Type::NewtypeStruct(claimed) = &built else {
        panic!("expected Type::NewtypeStruct");
    };
    assert_eq!(claimed.get_extra_derives(), derives);
    assert_eq!(claimed.get_extra_attrs(), attrs);

    // An alias renders as `type N = T;`, which carries attributes but no
    // derive (E0774), so it has attrs and nothing else.
    let built = TypeAlias::new("str".to_string())
        .name("Label")
        .extra_attrs(attrs)
        .build()
        .unwrap();
    let Type::TypeAlias(claimed) = &built else {
        panic!("expected Type::TypeAlias");
    };
    assert_eq!(claimed.get_extra_attrs(), attrs);
    assert!(built.extra_derives().is_empty());

    // A shape that claimed neither reports both as empty, as does a
    // type with no such slot at all.
    let bare = Struct::<String>::new().name("Bare").build().unwrap();
    assert!(bare.extra_derives().is_empty());
    assert!(bare.extra_attrs().is_empty());
    let unnamed = Type::<String>::String;
    assert!(unnamed.extra_derives().is_empty());
    assert!(unnamed.extra_attrs().is_empty());
}

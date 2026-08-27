fn expansion() {
    {
        let mut builder = crate::TypespaceBuilder::<String>::new(Settings::typical());
        builder
            .insert("u32".to_string(), crate::build::Type::Integer("u32".to_string()))
            .unwrap();
        builder
            .insert(
                "Inner".to_string(),
                crate::build::Struct::<String>::new()
                    .name("Inner")
                    .properties([
                        crate::build::StructProperty::new("count", "u32".to_string())
                            .with_state(crate::build::StructPropertyState::Required),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder.insert("String".to_string(), crate::build::Type::String).unwrap();
        builder
            .insert(
                "Nullable<Inner>".to_string(),
                crate::build::Type::Option("Inner".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "Widget".to_string(),
                crate::build::Struct::<String>::new()
                    .name("Widget")
                    .default(
                        ::serde_json::Value::Object(
                            [
                                ("both".to_string(), ::serde_json::Value::Null),
                                (
                                    "name".to_string(),
                                    ::serde_json::Value::String("anon".to_string()),
                                ),
                                ("nul".to_string(), ::serde_json::Value::Null),
                                ("opt".to_string(), ::serde_json::Value::Null),
                            ]
                                .into_iter()
                                .collect(),
                        ),
                    )
                    .properties([
                        crate::build::StructProperty::new("name", "String".to_string())
                            .with_state(crate::build::StructPropertyState::Required),
                        crate::build::StructProperty::new("opt", "Inner".to_string())
                            .with_state(crate::build::StructPropertyState::Optional),
                        crate::build::StructProperty::new(
                                "nul",
                                "Nullable<Inner>".to_string(),
                            )
                            .with_state(crate::build::StructPropertyState::Required),
                        crate::build::StructProperty::new(
                                "both",
                                "Nullable<Inner>".to_string(),
                            )
                            .with_state(crate::build::StructPropertyState::Optional),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

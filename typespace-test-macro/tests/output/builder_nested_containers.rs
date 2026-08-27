fn expansion() {
    {
        let mut builder = crate::TypespaceBuilder::<String>::new(Settings::typical());
        builder
            .insert("u32".to_string(), crate::build::Type::Integer("u32".to_string()))
            .unwrap();
        builder
            .insert(
                "Item".to_string(),
                crate::build::Struct::<String>::new()
                    .name("Item")
                    .properties([
                        crate::build::StructProperty::new("value", "u32".to_string())
                            .with_state(crate::build::StructPropertyState::Required),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                "Nullable<Item>".to_string(),
                crate::build::Type::Option("Item".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "Vec<Nullable<Item>>".to_string(),
                crate::build::Type::Vec("Nullable<Item>".to_string()),
            )
            .unwrap();
        builder.insert("String".to_string(), crate::build::Type::String).unwrap();
        builder
            .insert(
                "Map<String, Item>".to_string(),
                crate::build::Type::Map("String".to_string(), "Item".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "Bag".to_string(),
                crate::build::Struct::<String>::new()
                    .name("Bag")
                    .properties([
                        crate::build::StructProperty::new(
                                "items",
                                "Vec<Nullable<Item>>".to_string(),
                            )
                            .with_state(crate::build::StructPropertyState::Required),
                        crate::build::StructProperty::new(
                                "by_name",
                                "Map<String, Item>".to_string(),
                            )
                            .with_state(crate::build::StructPropertyState::Required),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

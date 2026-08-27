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
            .insert("Vec<Item>".to_string(), crate::build::Type::Vec("Item".to_string()))
            .unwrap();
        builder
            .insert(
                "ItemList".to_string(),
                crate::build::TypeAlias::<String>::new("Vec<Item>".to_string())
                    .name("ItemList")
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

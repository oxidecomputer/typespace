fn expansion() {
    {
        let mut builder = crate::TypespaceBuilder::<String>::new(Settings::typical());
        builder.insert("String".to_string(), crate::build::Type::String).unwrap();
        builder.insert("Never".to_string(), crate::build::Type::Never).unwrap();
        builder
            .insert(
                "Test".to_string(),
                crate::build::Struct::<String>::new()
                    .name("Test")
                    .properties([
                        crate::build::StructProperty::new("foo", "String".to_string())
                            .with_state(crate::build::StructPropertyState::Required),
                        crate::build::StructProperty::new("bar", "Never".to_string())
                            .with_state(crate::build::StructPropertyState::Required),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

fn expansion() {
    {
        let mut builder = ::typespace::TypespaceBuilder::<
            String,
        >::new(Settings::typical());
        builder.insert("String".to_string(), ::typespace::build::Type::String).unwrap();
        builder.insert("Never".to_string(), ::typespace::build::Type::Never).unwrap();
        builder
            .insert(
                "Test".to_string(),
                ::typespace::build::Struct::<String>::new()
                    .name("Test")
                    .properties([
                        ::typespace::build::StructProperty::new(
                                "foo",
                                "String".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "bar",
                                "Never".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

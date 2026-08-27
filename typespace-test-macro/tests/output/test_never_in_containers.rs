fn expansion() {
    {
        let mut builder = ::typespace::TypespaceBuilder::<
            String,
        >::new(Settings::typical());
        builder.insert("Never".to_string(), ::typespace::build::Type::Never).unwrap();
        builder
            .insert(
                "Vec<Never>".to_string(),
                ::typespace::build::Type::Vec("Never".to_string()),
            )
            .unwrap();
        builder.insert("String".to_string(), ::typespace::build::Type::String).unwrap();
        builder
            .insert(
                "Map<String, Never>".to_string(),
                ::typespace::build::Type::Map("String".to_string(), "Never".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "Box<Never>".to_string(),
                ::typespace::build::Type::Box("Never".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "u32".to_string(),
                ::typespace::build::Type::Integer("u32".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "(u32, Never)".to_string(),
                ::typespace::build::Type::Tuple(
                    ["u32".to_string(), "Never".to_string()].into_iter().collect(),
                ),
            )
            .unwrap();
        builder
            .insert(
                "[Never; 3]".to_string(),
                ::typespace::build::Type::Array("Never".to_string(), 3usize),
            )
            .unwrap();
        builder
            .insert(
                "Test".to_string(),
                ::typespace::build::Struct::<String>::new()
                    .name("Test")
                    .properties([
                        ::typespace::build::StructProperty::new(
                                "vec",
                                "Vec<Never>".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "map",
                                "Map<String, Never>".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "boxed",
                                "Box<Never>".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "tuple",
                                "(u32, Never)".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "array",
                                "[Never; 3]".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                        ::typespace::build::StructProperty::new(
                                "nested",
                                "Vec<Never>".to_string(),
                            )
                            .with_state(
                                ::typespace::build::StructPropertyState::Optional,
                            ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

fn expansion() {
    {
        let mut builder = ::typespace::TypespaceBuilder::<
            String,
        >::new(Settings::typical());
        builder
            .insert(
                "u32".to_string(),
                ::typespace::build::Type::Integer("u32".to_string()),
            )
            .unwrap();
        builder.insert("String".to_string(), ::typespace::build::Type::String).unwrap();
        builder
            .insert(
                "Widget".to_string(),
                ::typespace::build::TupleStruct::<String>::new()
                    .name("Widget")
                    .fields(["u32".to_string()])
                    .rest("String".to_string())
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

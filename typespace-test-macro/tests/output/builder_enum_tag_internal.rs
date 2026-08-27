fn expansion() {
    {
        let mut builder = ::typespace::TypespaceBuilder::<
            String,
        >::new(Settings::typical());
        builder
            .insert(
                "f64".to_string(),
                ::typespace::build::Type::Float("f64".to_string()),
            )
            .unwrap();
        builder
            .insert(
                "Shape".to_string(),
                ::typespace::build::Enum::<String>::new()
                    .name("Shape")
                    .tag_type(::typespace::build::EnumTagType::Internal {
                        tag: "kind".to_string(),
                    })
                    .variants([
                        ::typespace::build::EnumVariant::new(
                            "Circle",
                            ::typespace::build::VariantDetails::<
                                String,
                            >::Item("f64".to_string()),
                        ),
                        ::typespace::build::EnumVariant::new(
                            "Empty",
                            ::typespace::build::VariantDetails::<String>::Unit,
                        ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

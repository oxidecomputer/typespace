fn expansion() {
    {
        let mut builder = ::typespace::TypespaceBuilder::<
            String,
        >::new(Settings::typical());
        builder.insert("String".to_string(), ::typespace::build::Type::String).unwrap();
        builder
            .insert(
                "Widget".to_string(),
                ::typespace::build::Struct::<String>::new()
                    .name("Widget")
                    .description("A widget.\n\nHas a name.")
                    .properties([
                        ::typespace::build::StructProperty::new(
                                "name",
                                "String".to_string(),
                            )
                            .with_description("The widget's name.")
                            .with_state(
                                ::typespace::build::StructPropertyState::Required,
                            ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
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
                    .description("One of a few shapes.")
                    .tag_type(::typespace::build::EnumTagType::External)
                    .variants([
                        ::typespace::build::EnumVariant::new(
                                "Circle",
                                ::typespace::build::VariantDetails::<
                                    String,
                                >::Item("f64".to_string()),
                            )
                            .with_description("A circle."),
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
            .insert(
                "Label".to_string(),
                ::typespace::build::TypeAlias::<String>::new("String".to_string())
                    .name("Label")
                    .description(
                        "An alias for a widget's name.\n\nJust a `String` underneath.",
                    )
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

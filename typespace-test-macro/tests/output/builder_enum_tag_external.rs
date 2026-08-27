fn expansion() {
    {
        let mut builder = crate::TypespaceBuilder::<String>::new(Settings::typical());
        builder
            .insert("f64".to_string(), crate::build::Type::Float("f64".to_string()))
            .unwrap();
        builder
            .insert(
                "Shape".to_string(),
                crate::build::Enum::<String>::new()
                    .name("Shape")
                    .tag_type(crate::build::EnumTagType::External)
                    .variants([
                        crate::build::EnumVariant::new(
                            "Circle",
                            crate::build::VariantDetails::<
                                String,
                            >::Item("f64".to_string()),
                        ),
                        crate::build::EnumVariant::new(
                            "Empty",
                            crate::build::VariantDetails::<String>::Unit,
                        ),
                    ])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        builder
    };
}

pub struct MyTupleStruct(pub String, pub u32, pub Vec<String>);
impl ::std::default::Default for MyTupleStruct {
    fn default() -> Self {
        MyTupleStruct(
            "one".to_string(),
            2_u32,
            vec!["three".to_string(), "four".to_string()],
        )
    }
}
impl ::serde::Serialize for MyTupleStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        seq.serialize_element(&self.1)?;
        self.2.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for MyTupleStruct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = MyTupleStruct;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 2 or more",
                    ))?;
                let field_1 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        1usize,
                        &"a tuple of size 2 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(MyTupleStruct(field_0, field_1, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for MyTupleStruct {
    fn schema_name() -> String {
        "MyTupleStruct".to_string()
    }
    fn json_schema(
        g: &mut schemars::r#gen::SchemaGenerator,
    ) -> schemars::schema::Schema {
        let fields = [g.subschema_for::<String>(), g.subschema_for::<u32>()]
            .into_iter()
            .collect();
        schemars::schema::SchemaObject {
            metadata: Some(
                Box::new(schemars::schema::Metadata {
                    title: Some("MyTupleStruct".to_string()),
                    default: Some(
                        ::serde_json::from_str("[\"one\",2,\"three\",\"four\"]").unwrap(),
                    ),
                    ..Default::default()
                }),
            ),
            instance_type: Some(
                schemars::schema::SingleOrVec::Single(
                    Box::new(schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                Box::new(schemars::schema::ArrayValidation {
                    items: Some(schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(Box::new(g.subschema_for::<Vec<String>>())),
                    min_items: Some(2u32),
                    ..Default::default()
                }),
            ),
            ..Default::default()
        }
            .into()
    }
}

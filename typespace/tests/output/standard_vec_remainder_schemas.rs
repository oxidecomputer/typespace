#[derive(::serde::Deserialize, ::serde::Serialize, super::schema_facade::JsonSchema)]
pub struct Element {
    pub text: ::std::string::String,
}
pub struct Nested(pub u32, pub ::std::vec::Vec<::std::vec::Vec<::std::string::String>>);
impl ::serde::Serialize for Nested {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Nested {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Nested;
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
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(Nested(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl super::schema_facade::JsonSchema for Nested {
    fn schema_name() -> ::std::string::String {
        "Nested".to_string()
    }
    fn json_schema(
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("Nested".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::vec::Vec<::std::string::String>>(),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, super::schema_facade::JsonSchema)]
#[serde(transparent)]
pub struct Ordinary(pub ::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for Ordinary {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Ordinary> for ::std::vec::Vec<::std::string::String> {
    fn from(value: Ordinary) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::vec::Vec<::std::string::String>> for Ordinary {
    fn from(value: ::std::vec::Vec<::std::string::String>) -> Self {
        Self(value)
    }
}
pub struct Referenced(pub u32, pub ::std::vec::Vec<Element>);
impl ::serde::Serialize for Referenced {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Referenced {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Referenced;
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
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(Referenced(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl super::schema_facade::JsonSchema for Referenced {
    fn schema_name() -> ::std::string::String {
        "Referenced".to_string()
    }
    fn json_schema(
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("Referenced".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<Element>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Unconstrained(pub u32, pub ::std::vec::Vec<serde_json::Value>);
impl ::serde::Serialize for Unconstrained {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Unconstrained {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Unconstrained;
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
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(Unconstrained(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl super::schema_facade::JsonSchema for Unconstrained {
    fn schema_name() -> ::std::string::String {
        "Unconstrained".to_string()
    }
    fn json_schema(
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("Unconstrained".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<serde_json::Value>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}

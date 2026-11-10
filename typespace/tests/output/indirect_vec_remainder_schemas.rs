pub struct Aliased(pub u32, pub Tail);
impl ::serde::Serialize for Aliased {
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
impl<'de> ::serde::Deserialize<'de> for Aliased {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Aliased;
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
                Ok(Aliased(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Aliased {
    fn schema_name() -> ::std::string::String {
        "Aliased".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Aliased".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
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
pub struct Boxed(pub u32, pub ::std::boxed::Box<::std::vec::Vec<::std::string::String>>);
impl ::serde::Serialize for Boxed {
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
impl<'de> ::serde::Deserialize<'de> for Boxed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Boxed;
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
                Ok(Boxed(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Boxed {
    fn schema_name() -> ::std::string::String {
        "Boxed".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Boxed".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
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
pub struct Newtype(pub u32, pub Wrapped);
impl ::serde::Serialize for Newtype {
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
impl<'de> ::serde::Deserialize<'de> for Newtype {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Newtype;
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
                Ok(Newtype(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Newtype {
    fn schema_name() -> ::std::string::String {
        "Newtype".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Newtype".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
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
pub type Tail = ::std::vec::Vec<::std::string::String>;
#[derive(::serde::Deserialize, ::serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct Wrapped(pub ::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for Wrapped {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Wrapped> for ::std::vec::Vec<::std::string::String> {
    fn from(value: Wrapped) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::vec::Vec<::std::string::String>> for Wrapped {
    fn from(value: ::std::vec::Vec<::std::string::String>) -> Self {
        Self(value)
    }
}

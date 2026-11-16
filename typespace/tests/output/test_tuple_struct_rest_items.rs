pub struct Aliased(pub u32, pub Names);
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
                        ::std::boxed::Box::new(g.subschema_for::<String>()),
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
#[derive(::schemars::JsonSchema, ::serde::Deserialize, ::serde::Serialize)]
pub struct Element {
    pub text: String,
}
pub struct Flags(pub bool, pub bool);
impl ::serde::Serialize for Flags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        seq.serialize_element(&self.1)?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Flags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Flags;
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
                Ok(Flags(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Flags {
    fn schema_name() -> ::std::string::String {
        "Flags".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<bool>(), g.subschema_for::<bool>()]
            .into_iter()
            .collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Flags".to_string()),
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
                    max_items: Some(2u32),
                    min_items: Some(2u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub type Names = Vec<String>;
pub struct Nested(pub u32, pub Flags);
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
impl ::schemars::JsonSchema for Nested {
    fn schema_name() -> ::std::string::String {
        "Nested".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Nested".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<bool>()),
                    ),
                    max_items: Some(3u32),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Pair(pub u32, pub [bool; 2usize]);
impl ::serde::Serialize for Pair {
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
impl<'de> ::serde::Deserialize<'de> for Pair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Pair;
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
                Ok(Pair(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Pair {
    fn schema_name() -> ::std::string::String {
        "Pair".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Pair".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<bool>()),
                    ),
                    max_items: Some(3u32),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Referenced(pub u32, pub Vec<Element>);
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
impl ::schemars::JsonSchema for Referenced {
    fn schema_name() -> ::std::string::String {
        "Referenced".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Referenced".to_string()),
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
pub struct Rows(pub u32, pub Vec<Vec<String>>);
impl ::serde::Serialize for Rows {
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
impl<'de> ::serde::Deserialize<'de> for Rows {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Rows;
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
                Ok(Rows(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Rows {
    fn schema_name() -> ::std::string::String {
        "Rows".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Rows".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<Vec<String>>()),
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
pub struct Tags(pub u32, pub Vec<String>);
impl ::serde::Serialize for Tags {
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
impl<'de> ::serde::Deserialize<'de> for Tags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Tags;
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
                Ok(Tags(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Tags {
    fn schema_name() -> ::std::string::String {
        "Tags".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Tags".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<String>()),
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
pub struct Twice(pub u32, pub (String, String));
impl ::serde::Serialize for Twice {
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
impl<'de> ::serde::Deserialize<'de> for Twice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Twice;
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
                Ok(Twice(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Twice {
    fn schema_name() -> ::std::string::String {
        "Twice".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Twice".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<String>()),
                    ),
                    max_items: Some(3u32),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Wrapped(pub u32, pub Wrapper);
impl ::serde::Serialize for Wrapped {
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
impl<'de> ::serde::Deserialize<'de> for Wrapped {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Wrapped;
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
                Ok(Wrapped(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for Wrapped {
    fn schema_name() -> ::std::string::String {
        "Wrapped".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Wrapped".to_string()),
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
                        ::std::boxed::Box::new(g.subschema_for::<String>()),
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
#[derive(::schemars::JsonSchema, ::serde::Deserialize, ::serde::Serialize)]
#[serde(transparent)]
pub struct Wrapper(pub Vec<String>);
impl ::std::ops::Deref for Wrapper {
    type Target = Vec<String>;
    fn deref(&self) -> &Vec<String> {
        &self.0
    }
}
impl ::std::convert::From<Wrapper> for Vec<String> {
    fn from(value: Wrapper) -> Self {
        value.0
    }
}
impl ::std::convert::From<Vec<String>> for Wrapper {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

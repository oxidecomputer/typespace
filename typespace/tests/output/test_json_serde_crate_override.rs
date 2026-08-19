#[derive(::serde::Deserialize, ::serde::Serialize)]
pub struct Options {
    #[serde(
        default,
        deserialize_with = "my_json_serde::deserialize_some",
        skip_serializing_if = "Option::is_none"
    )]
    pub maybe: Option<String>,
}
#[derive(::std::clone::Clone, ::std::fmt::Debug)]
pub struct Rest(pub String, pub Vec<String>);
impl ::serde::Serialize for Rest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(my_json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Rest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Rest;
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
                    my_json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(Rest(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}

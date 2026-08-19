#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    Eq,
    PartialEq,
    ::std::hash::Hash
)]
pub enum Gadget {
    Off,
    On(u32),
}
#[derive(::std::clone::Clone, ::std::fmt::Debug, Eq, PartialEq, ::std::hash::Hash)]
pub struct Marker;
impl ::serde::Serialize for Marker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        ::serde_json::Value::String("marker".to_string()).serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for Marker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let expected = ::serde_json::Value::String("marker".to_string());
        let value: serde_json::Value = ::serde::Deserialize::deserialize(deserializer)?;
        if value != expected {
            return Err(
                ::serde::de::Error::custom(
                    format!(
                        "expected unit struct value {}, found {}", "\"marker\"",
                        ::serde_json::to_string(& value).unwrap()
                    ),
                ),
            );
        }
        Ok(Marker)
    }
}
pub type Named = String;
#[derive(::std::clone::Clone, ::std::fmt::Debug, Eq, PartialEq, ::std::hash::Hash)]
pub struct Pair(pub String, pub u32);
impl ::serde::Serialize for Pair {
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
                        &"a tuple of size 2 or more",
                    ))?;
                let field_1 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        1usize,
                        &"a tuple of size 2 or more",
                    ))?;
                Ok(Pair(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    Eq,
    PartialEq,
    ::std::hash::Hash
)]
pub struct Widget {
    pub name: String,
    pub tags: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, ::std::hash::Hash)]
pub struct Wrapper(pub String);
impl ::std::ops::Deref for Wrapper {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ::std::convert::From<Wrapper> for String {
    fn from(value: Wrapper) -> Self {
        value.0
    }
}
impl ::serde::Serialize for Wrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for Wrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Ok(Self(::serde::Deserialize::deserialize(deserializer)?))
    }
}

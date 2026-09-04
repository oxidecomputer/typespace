#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    PartialEq,
    ::std::hash::Hash,
    PartialOrd
)]
pub enum ShapeEnum {
    Only(u32),
}
impl ::std::convert::From<u32> for ShapeEnum {
    fn from(value: u32) -> Self {
        Self::Only(value)
    }
}
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    PartialEq,
    ::std::hash::Hash,
    PartialOrd
)]
#[serde(transparent)]
pub struct ShapeNewtype(pub u32);
impl ::std::ops::Deref for ShapeNewtype {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<ShapeNewtype> for u32 {
    fn from(value: ShapeNewtype) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for ShapeNewtype {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    PartialEq,
    ::std::hash::Hash,
    PartialOrd
)]
pub struct ShapeStruct {
    pub x: u32,
}
#[derive(Clone, Debug, PartialEq, ::std::hash::Hash, PartialOrd)]
pub struct ShapeTuple(pub u32, pub u32);
impl ::serde::Serialize for ShapeTuple {
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
impl<'de> ::serde::Deserialize<'de> for ShapeTuple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = ShapeTuple;
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
                Ok(ShapeTuple(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(Clone, Debug, PartialEq, ::std::hash::Hash, PartialOrd)]
pub struct ShapeUnit;
impl ::serde::Serialize for ShapeUnit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        ::serde_json::Value::String("unit-shape".to_string()).serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for ShapeUnit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let expected = ::serde_json::Value::String("unit-shape".to_string());
        let value: serde_json::Value = ::serde::Deserialize::deserialize(deserializer)?;
        if value != expected {
            return Err(
                ::serde::de::Error::custom(
                    format!(
                        "expected unit struct value {}, found {}", "\"unit-shape\"",
                        ::serde_json::to_string(& value).unwrap()
                    ),
                ),
            );
        }
        Ok(ShapeUnit)
    }
}

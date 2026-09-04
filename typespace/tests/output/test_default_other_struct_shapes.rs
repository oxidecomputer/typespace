#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(transparent)]
pub struct NewtypeShape(pub ::std::string::String);
impl ::std::ops::Deref for NewtypeShape {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<NewtypeShape> for ::std::string::String {
    fn from(value: NewtypeShape) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::string::String> for NewtypeShape {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
#[derive(Debug, PartialEq, Default)]
pub struct TupleShape(pub ::std::string::String, pub u32);
impl ::serde::Serialize for TupleShape {
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
impl<'de> ::serde::Deserialize<'de> for TupleShape {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = TupleShape;
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
                Ok(TupleShape(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(Debug, PartialEq, Default)]
pub struct UnitShape;
impl ::serde::Serialize for UnitShape {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        ::serde_json::Value::String("unit".to_string()).serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for UnitShape {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let expected = ::serde_json::Value::String("unit".to_string());
        let value: serde_json::Value = ::serde::Deserialize::deserialize(deserializer)?;
        if value != expected {
            return Err(
                ::serde::de::Error::custom(
                    format!(
                        "expected unit struct value {}, found {}", "\"unit\"",
                        ::serde_json::to_string(& value).unwrap()
                    ),
                ),
            );
        }
        Ok(UnitShape)
    }
}

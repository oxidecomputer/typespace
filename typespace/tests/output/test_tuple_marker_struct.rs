pub struct Listed(pub u32);
impl ::serde::Serialize for Listed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for Listed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = Listed;
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
                Ok(Listed(field_0))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(transparent)]
pub struct Wrapped(pub u32);
impl ::std::ops::Deref for Wrapped {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<Wrapped> for u32 {
    fn from(value: Wrapped) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for Wrapped {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

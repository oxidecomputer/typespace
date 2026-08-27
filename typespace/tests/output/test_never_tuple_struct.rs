#[derive(Debug)]
pub struct NeverTupleStruct(pub u32, pub ::json_serde::Absent);
impl ::serde::Serialize for NeverTupleStruct {
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
impl<'de> ::serde::Deserialize<'de> for NeverTupleStruct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = NeverTupleStruct;
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
                Ok(NeverTupleStruct(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}

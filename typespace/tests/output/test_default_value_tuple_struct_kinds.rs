#[derive(Debug, Default, PartialEq)]
pub struct FixedTuple(pub u32, pub ::std::string::String);
impl ::serde::Serialize for FixedTuple {
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
impl<'de> ::serde::Deserialize<'de> for FixedTuple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = FixedTuple;
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
                Ok(FixedTuple(field_0, field_1))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(Debug, Default, PartialEq)]
pub struct OpenTuple(pub u32, pub ::std::vec::Vec<u32>);
impl ::serde::Serialize for OpenTuple {
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
impl<'de> ::serde::Deserialize<'de> for OpenTuple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = OpenTuple;
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
                Ok(OpenTuple(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct TupleStructDefaults {
    #[serde(default = "defaults::tuple_struct_defaults_fixed")]
    pub fixed: FixedTuple,
    #[serde(default = "defaults::tuple_struct_defaults_open")]
    pub open: OpenTuple,
}
impl ::std::default::Default for TupleStructDefaults {
    fn default() -> Self {
        Self {
            fixed: defaults::tuple_struct_defaults_fixed(),
            open: defaults::tuple_struct_defaults_open(),
        }
    }
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn tuple_struct_defaults_fixed() -> super::FixedTuple {
        super::FixedTuple(1_u32, "a".to_string())
    }
    pub(super) fn tuple_struct_defaults_open() -> super::OpenTuple {
        super::OpenTuple(1_u32, vec![2_u32, 3_u32, 4_u32])
    }
}

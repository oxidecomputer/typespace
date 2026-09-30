#[derive(
    ::serde::Serialize,
    Clone,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
#[serde(transparent)]
pub struct Tags(::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for Tags {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Tags> for ::std::vec::Vec<::std::string::String> {
    fn from(value: Tags) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::vec::Vec<::std::string::String>> for Tags {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::vec::Vec<::std::string::String>,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.len() > 3usize {
            return Err("more than 3 items".into());
        }
        if value.len() < 1usize {
            return Err("fewer than 1 items".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Tags {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(
                <::std::vec::Vec<::std::string::String>>::deserialize(deserializer)?,
            )
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Tags {
    fn schema_name() -> ::std::string::String {
        "Tags".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <::std::vec::Vec<
            ::std::string::String,
        > as ::schemars::JsonSchema>::json_schema(g)
            .into_object();
        schema.array().min_items = ::std::option::Option::Some(1u32);
        schema.array().max_items = ::std::option::Option::Some(3u32);
        schema.into()
    }
}
/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}

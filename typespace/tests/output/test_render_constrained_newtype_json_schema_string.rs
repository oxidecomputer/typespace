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
pub struct Terse(::std::string::String);
impl ::std::ops::Deref for Terse {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Terse> for ::std::string::String {
    fn from(value: Terse) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Terse {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Terse {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <::std::string::String as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Terse {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        #[::jsonschema::validator(
            schema = "{\"anyOf\":[{\"pattern\":\"^a\",\"type\":\"string\"},{\"maxLength\":3,\"type\":\"string\"}]}"
        )]
        struct Schema;
        let json = ::serde_json::to_value(&value).map_err(|e| e.to_string())?;
        Schema::validate(&json).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Terse {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<::std::string::String>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Terse {
    fn schema_name() -> ::std::string::String {
        "Terse".to_string()
    }
    fn json_schema(
        _: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
            extensions: ::serde_json::from_str::<
                ::serde_json::Map<::std::string::String, ::serde_json::Value>,
            >(
                    "{\"anyOf\":[{\"pattern\":\"^a\",\"type\":\"string\"},{\"maxLength\":3,\"type\":\"string\"}]}",
                )
                .unwrap()
                .into_iter()
                .collect(),
            ..::std::default::Default::default()
        })
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

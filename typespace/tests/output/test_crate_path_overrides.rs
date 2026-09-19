#[derive(::serde::Serialize, Debug)]
#[serde(transparent)]
pub struct Code(::std::string::String);
impl ::std::ops::Deref for Code {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Code> for ::std::string::String {
    fn from(value: Code) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<&str> for Code {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<super::regex_engine::Regex> = ::std::sync::LazyLock::new(||
        { super::regex_engine::Regex::new("^x").unwrap() });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^x\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Code {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        ::std::convert::TryFrom::try_from(value.as_str())
    }
}
impl<'de> ::serde::Deserialize<'de> for Code {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::convert::TryFrom::try_from(
                ::std::string::String::deserialize(deserializer)?,
            )
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct Thing {
    #[serde(
        default,
        deserialize_with = "super::json_helpers::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub opt: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "super::json_helpers::always")]
    pub gone: super::json_helpers::Absent,
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

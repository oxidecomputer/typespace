#[derive(::serde::Serialize)]
#[serde(transparent)]
pub struct AnchoredUrl(::url::Url);
impl ::std::ops::Deref for AnchoredUrl {
    type Target = ::url::Url;
    fn deref(&self) -> &::url::Url {
        &self.0
    }
}
impl ::std::convert::From<AnchoredUrl> for ::url::Url {
    fn from(value: AnchoredUrl) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for AnchoredUrl {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<&str> for AnchoredUrl {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(||
        { ::regress::Regex::new("^https://").unwrap() });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^https://\"".into());
        }
        value
            .parse()
            .map(Self)
            .map_err(|e| { ::std::string::ToString::to_string(&e).into() })
    }
}
impl ::std::convert::TryFrom<::std::string::String> for AnchoredUrl {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        ::std::convert::TryFrom::try_from(value.as_str())
    }
}
impl<'de> ::serde::Deserialize<'de> for AnchoredUrl {
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

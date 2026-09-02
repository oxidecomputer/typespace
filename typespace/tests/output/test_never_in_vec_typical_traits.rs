#[derive(Clone, Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct TypicalVecHolder {
    pub values: ::std::vec::Vec<::json_serde::Absent>,
}
impl TypicalVecHolder {
    pub fn builder() -> builder::TypicalVecHolder {
        Default::default()
    }
}
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct TypicalVecHolder {
        values: ::std::result::Result<
            ::std::vec::Vec<::json_serde::Absent>,
            ::std::string::String,
        >,
    }
    impl ::std::default::Default for TypicalVecHolder {
        fn default() -> Self {
            Self {
                values: Err("no value supplied for values".to_string()),
            }
        }
    }
    impl TypicalVecHolder {
        pub fn values<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::json_serde::Absent>>,
            T::Error: ::std::fmt::Display,
        {
            self.values = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for values: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<TypicalVecHolder> for super::TypicalVecHolder {
        type Error = super::error::ConversionError;
        fn try_from(
            value: TypicalVecHolder,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self { values: value.values? })
        }
    }
    impl ::std::convert::From<super::TypicalVecHolder> for TypicalVecHolder {
        fn from(value: super::TypicalVecHolder) -> Self {
            Self { values: Ok(value.values) }
        }
    }
}
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

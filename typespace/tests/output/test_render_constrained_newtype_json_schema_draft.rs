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
pub struct Pair(::std::vec::Vec<i64>);
impl ::std::ops::Deref for Pair {
    type Target = ::std::vec::Vec<i64>;
    fn deref(&self) -> &::std::vec::Vec<i64> {
        &self.0
    }
}
impl ::std::convert::From<Pair> for ::std::vec::Vec<i64> {
    fn from(value: Pair) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::vec::Vec<i64>> for Pair {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::vec::Vec<i64>,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        #[::jsonschema::validator(
            schema = "{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"additionalItems\":false,\"items\":[{\"type\":\"integer\"},{\"type\":\"integer\"}],\"type\":\"array\"}"
        )]
        struct Schema;
        let json = ::serde_json::to_value(&value).map_err(|e| e.to_string())?;
        Schema::validate(&json).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Pair {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<::std::vec::Vec<i64>>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Pair {
    fn schema_name() -> ::std::string::String {
        "Pair".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let inner = g.subschema_for::<::std::vec::Vec<i64>>();
        let constraint = ::serde_json::from_str::<
            ::schemars::schema::Schema,
        >(
                "{\"additionalItems\":false,\"items\":[{\"type\":\"integer\"},{\"type\":\"integer\"}],\"type\":\"array\"}",
            )
            .unwrap();
        ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
            subschemas: ::std::option::Option::Some(
                ::std::boxed::Box::new(::schemars::schema::SubschemaValidation {
                    all_of: ::std::option::Option::Some(::std::vec![inner, constraint,]),
                    ..::std::default::Default::default()
                }),
            ),
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

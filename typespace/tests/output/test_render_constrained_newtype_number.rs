#[derive(::serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Count(u32);
impl ::std::ops::Deref for Count {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<Count> for u32 {
    fn from(value: Count) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Count {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Count {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <u32 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<u32> for Count {
    type Error = self::error::ConversionError;
    fn try_from(
        value: u32,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value < 5_u32 {
            return Err("less than 5".into());
        }
        if value > 100_u32 {
            return Err("greater than 100".into());
        }
        if value % 5_u32 != 0_u32 {
            return Err("not a multiple of 5".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Count {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<u32>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Count {
    fn schema_name() -> ::std::string::String {
        "Count".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <u32 as ::schemars::JsonSchema>::json_schema(g).into_object();
        schema.number().minimum = ::std::option::Option::Some(5f64);
        schema.number().maximum = ::std::option::Option::Some(100f64);
        schema.number().multiple_of = ::std::option::Option::Some(5f64);
        schema.into()
    }
}
#[derive(::serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Even(::std::num::NonZeroU32);
impl ::std::ops::Deref for Even {
    type Target = ::std::num::NonZeroU32;
    fn deref(&self) -> &::std::num::NonZeroU32 {
        &self.0
    }
}
impl ::std::convert::From<Even> for ::std::num::NonZeroU32 {
    fn from(value: Even) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Even {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Even {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <::std::num::NonZeroU32 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<::std::num::NonZeroU32> for Even {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::num::NonZeroU32,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.get() % 2_u32 != 0_u32 {
            return Err("not a multiple of 2".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Even {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<::std::num::NonZeroU32>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Even {
    fn schema_name() -> ::std::string::String {
        "Even".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <::std::num::NonZeroU32 as ::schemars::JsonSchema>::json_schema(
                g,
            )
            .into_object();
        schema.number().multiple_of = ::std::option::Option::Some(2f64);
        schema.into()
    }
}
#[derive(::serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Percent(u8);
impl ::std::ops::Deref for Percent {
    type Target = u8;
    fn deref(&self) -> &u8 {
        &self.0
    }
}
impl ::std::convert::From<Percent> for u8 {
    fn from(value: Percent) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Percent {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Percent {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <u8 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<u8> for Percent {
    type Error = self::error::ConversionError;
    fn try_from(value: u8) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value > 100_u8 {
            return Err("greater than 100".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Percent {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<u8>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Percent {
    fn schema_name() -> ::std::string::String {
        "Percent".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <u8 as ::schemars::JsonSchema>::json_schema(g).into_object();
        schema.number().minimum = ::std::option::Option::Some(0f64);
        schema.number().maximum = ::std::option::Option::Some(100f64);
        schema.into()
    }
}
#[derive(::serde::Serialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Ratio(f64);
impl ::std::ops::Deref for Ratio {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}
impl ::std::convert::From<Ratio> for f64 {
    fn from(value: Ratio) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Ratio {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Ratio {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <f64 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<f64> for Ratio {
    type Error = self::error::ConversionError;
    fn try_from(
        value: f64,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value <= 0_f64 {
            return Err("not greater than 0".into());
        }
        if value >= 1_f64 {
            return Err("not less than 1".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Ratio {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<f64>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Ratio {
    fn schema_name() -> ::std::string::String {
        "Ratio".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <f64 as ::schemars::JsonSchema>::json_schema(g).into_object();
        schema.number().exclusive_minimum = ::std::option::Option::Some(0f64);
        schema.number().exclusive_maximum = ::std::option::Option::Some(1f64);
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

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
        if value.is_nan() {
            return Err("not a number".into());
        }
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
#[derive(::serde::Serialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Scale(f32);
impl ::std::ops::Deref for Scale {
    type Target = f32;
    fn deref(&self) -> &f32 {
        &self.0
    }
}
impl ::std::convert::From<Scale> for f32 {
    fn from(value: Scale) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Scale {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Scale {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <f32 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<f32> for Scale {
    type Error = self::error::ConversionError;
    fn try_from(
        value: f32,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.is_nan() {
            return Err("not a number".into());
        }
        if value < -1_f32 {
            return Err("less than -1".into());
        }
        if value > 1_f32 {
            return Err("greater than 1".into());
        }
        let quotient = value / 0.5_f32;
        if (quotient - quotient.round()).abs()
            > quotient.abs().max(1.0) * (4.0 * f32::EPSILON)
        {
            return Err("not a multiple of 0.5".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Scale {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<f32>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Scale {
    fn schema_name() -> ::std::string::String {
        "Scale".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <f32 as ::schemars::JsonSchema>::json_schema(g).into_object();
        schema.number().minimum = ::std::option::Option::Some(-1f64);
        schema.number().maximum = ::std::option::Option::Some(1f64);
        schema.number().multiple_of = ::std::option::Option::Some(0.5f64);
        schema.into()
    }
}
#[derive(::serde::Serialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct Tenths(f64);
impl ::std::ops::Deref for Tenths {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}
impl ::std::convert::From<Tenths> for f64 {
    fn from(value: Tenths) -> Self {
        value.0
    }
}
impl ::std::fmt::Display for Tenths {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Tenths {
    type Err = self::error::ConversionError;
    fn from_str(
        value: &str,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        let value = <f64 as ::std::str::FromStr>::from_str(value)
            .map_err(|_| "could not be parsed as the inner type")?;
        ::std::convert::TryFrom::try_from(value)
    }
}
impl ::std::convert::TryFrom<f64> for Tenths {
    type Error = self::error::ConversionError;
    fn try_from(
        value: f64,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.is_nan() {
            return Err("not a number".into());
        }
        let quotient = value / 0.1_f64;
        if (quotient - quotient.round()).abs()
            > quotient.abs().max(1.0) * (4.0 * f64::EPSILON)
        {
            return Err("not a multiple of 0.1".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for Tenths {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<f64>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for Tenths {
    fn schema_name() -> ::std::string::String {
        "Tenths".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <f64 as ::schemars::JsonSchema>::json_schema(g).into_object();
        schema.number().multiple_of = ::std::option::Option::Some(0.1f64);
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

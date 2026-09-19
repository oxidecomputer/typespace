#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    schemars::JsonSchema
)]
pub struct Coords {
    pub x: i64,
    pub y: i64,
}
#[derive(::serde::Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct EvenCoords(Coords);
impl ::std::ops::Deref for EvenCoords {
    type Target = Coords;
    fn deref(&self) -> &Coords {
        &self.0
    }
}
impl ::std::convert::From<EvenCoords> for Coords {
    fn from(value: EvenCoords) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<Coords> for EvenCoords {
    type Error = self::error::ConversionError;
    fn try_from(
        value: Coords,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        #[::jsonschema::validator(
            schema = "{\"properties\":{\"x\":{\"multipleOf\":2,\"type\":\"integer\"},\"y\":{\"type\":\"integer\"}},\"required\":[\"x\",\"y\"],\"type\":\"object\"}"
        )]
        struct Schema;
        let json = ::serde_json::to_value(&value).map_err(|e| e.to_string())?;
        Schema::validate(&json).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for EvenCoords {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<Coords>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for EvenCoords {
    fn schema_name() -> ::std::string::String {
        "EvenCoords".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let inner = g.subschema_for::<Coords>();
        let constraint = ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
            extensions: ::serde_json::from_str::<
                ::serde_json::Map<::std::string::String, ::serde_json::Value>,
            >(
                    "{\"properties\":{\"x\":{\"multipleOf\":2,\"type\":\"integer\"},\"y\":{\"type\":\"integer\"}},\"required\":[\"x\",\"y\"],\"type\":\"object\"}",
                )
                .unwrap()
                .into_iter()
                .collect(),
            ..::std::default::Default::default()
        });
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

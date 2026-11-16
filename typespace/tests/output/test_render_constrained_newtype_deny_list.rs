pub struct DeniedJson(super::json_values::Value);
impl ::std::ops::Deref for DeniedJson {
    type Target = super::json_values::Value;
    fn deref(&self) -> &super::json_values::Value {
        &self.0
    }
}
impl ::std::convert::From<DeniedJson> for super::json_values::Value {
    fn from(value: DeniedJson) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<super::json_values::Value> for DeniedJson {
    type Error = self::error::ConversionError;
    fn try_from(
        value: super::json_values::Value,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if [
            super::json_values::Value::Object(
                super::json_values::Map::from_iter([
                    (
                        "a".to_string(),
                        super::json_values::Value::Array(
                            vec![
                                super::json_values::Value::Number(super::json_values::Number::from(-
                                7i64)),
                                super::json_values::Value::Number(super::json_values::Number::from(18446744073709551615u64)),
                                super::json_values::Value::Number(super::json_values::Number::from_f64(1.5f64)
                                .unwrap()), super::json_values::Value::String("quote\"\n"
                                .to_string()), super::json_values::Value::Bool(true),
                                super::json_values::Value::Bool(false),
                                super::json_values::Value::Null,
                                super::json_values::Value::Object(super::json_values::Map::from_iter([])),
                                super::json_values::Value::Array(vec![])
                            ],
                        ),
                    ),
                ]),
            ),
        ]
            .contains(&value)
        {
            Err("invalid value".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl ::schemars::JsonSchema for DeniedJson {
    fn schema_name() -> ::std::string::String {
        "DeniedJson".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <super::json_values::Value as ::schemars::JsonSchema>::json_schema(
                g,
            )
            .into_object();
        let not = ::schemars::schema::SchemaObject {
            enum_values: ::std::option::Option::Some(
                [
                    super::json_values::Value::Object(
                        super::json_values::Map::from_iter([
                            (
                                "a".to_string(),
                                super::json_values::Value::Array(
                                    vec![
                                        super::json_values::Value::Number(super::json_values::Number::from(-
                                        7i64)),
                                        super::json_values::Value::Number(super::json_values::Number::from(18446744073709551615u64)),
                                        super::json_values::Value::Number(super::json_values::Number::from_f64(1.5f64)
                                        .unwrap()), super::json_values::Value::String("quote\"\n"
                                        .to_string()), super::json_values::Value::Bool(true),
                                        super::json_values::Value::Bool(false),
                                        super::json_values::Value::Null,
                                        super::json_values::Value::Object(super::json_values::Map::from_iter([])),
                                        super::json_values::Value::Array(vec![])
                                    ],
                                ),
                            ),
                        ]),
                    ),
                ]
                    .into_iter()
                    .collect(),
            ),
            ..::std::default::Default::default()
        };
        schema.subschemas().not = Some(::std::boxed::Box::new(not.into()));
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

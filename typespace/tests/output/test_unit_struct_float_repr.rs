pub struct FloatUnitStruct;
impl ::serde::Serialize for FloatUnitStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        ::serde_json::Value::Number(::serde_json::Number::from_f64(1.5f64).unwrap())
            .serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for FloatUnitStruct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let expected = ::serde_json::Value::Number(
            ::serde_json::Number::from_f64(1.5f64).unwrap(),
        );
        let value: ::serde_json::Value = ::serde::Deserialize::deserialize(
            deserializer,
        )?;
        if value != expected {
            return Err(
                ::serde::de::Error::custom(
                    format!(
                        "expected unit struct value {}, found {}", "1.5",
                        ::serde_json::to_string(& value).unwrap()
                    ),
                ),
            );
        }
        Ok(FloatUnitStruct)
    }
}

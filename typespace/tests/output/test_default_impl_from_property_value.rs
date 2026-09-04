#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct WithDefaultValue {
    #[serde(default = "defaults::with_default_value_answer")]
    pub answer: u32,
    #[serde(default, skip_serializing_if = "::std::string::String::is_empty")]
    pub name: ::std::string::String,
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub maybe: ::std::option::Option<bool>,
}
impl ::std::default::Default for WithDefaultValue {
    fn default() -> Self {
        Self {
            answer: defaults::with_default_value_answer(),
            name: Default::default(),
            maybe: Default::default(),
        }
    }
}
pub mod defaults {
    pub fn with_default_value_answer() -> u32 {
        ::serde_json::from_value(
                ::serde_json::Value::Number(::serde_json::Number::from(42i64)),
            )
            .expect("invalid default value")
    }
}

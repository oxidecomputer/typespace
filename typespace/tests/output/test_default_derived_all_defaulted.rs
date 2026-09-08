#[derive(::serde::Deserialize, ::serde::Serialize, Debug, Default, PartialEq)]
pub struct AllDefaulted {
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub maybe: ::std::option::Option<::std::string::String>,
    #[serde(default)]
    pub count: u32,
}

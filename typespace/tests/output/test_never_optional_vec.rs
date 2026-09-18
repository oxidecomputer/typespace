#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct OptionalVecHolder {
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "Option::is_none"
    )]
    pub values: Option<Vec<::json_serde::Never>>,
}

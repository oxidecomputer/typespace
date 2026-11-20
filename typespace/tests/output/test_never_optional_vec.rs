#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct OptionalVecHolder {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<::json_serde::Never>,
}

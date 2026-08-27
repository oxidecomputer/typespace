#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct NullableHolder {
    #[serde(deserialize_with = "Option::deserialize")]
    pub value: Option<::json_serde::Absent>,
}

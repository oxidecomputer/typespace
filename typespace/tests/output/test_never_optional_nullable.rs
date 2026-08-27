#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct OptionalNullableHolder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<::json_serde::Absent>,
}

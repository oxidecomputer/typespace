#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct OptionalNullableHolder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<::json_serde::Absent>,
}

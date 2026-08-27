#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct OptionalHolder {
    #[serde(default, skip_serializing_if = "::json_serde::always")]
    pub value: ::json_serde::Absent,
}

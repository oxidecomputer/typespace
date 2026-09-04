#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct OptionalHolder {
    #[serde(default, skip_serializing_if = "::json_serde::always")]
    pub value: ::json_serde::Absent,
}

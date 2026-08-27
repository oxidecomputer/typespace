#[derive(::std::fmt::Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct Gone {
    #[serde(default, skip_serializing_if = "::json_serde::always")]
    pub value: ::json_serde::Absent,
}

#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct NullableHolder {
    #[serde(deserialize_with = "Option::deserialize")]
    pub value: Option<::json_serde::Absent>,
}

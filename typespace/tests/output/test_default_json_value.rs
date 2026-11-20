#[derive(::serde::Deserialize, ::serde::Serialize)]
pub struct Blob {
    #[serde(default)]
    pub data: ::serde_json::Value,
}

#[derive(::serde::Deserialize, ::serde::Serialize)]
pub struct Holder {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boxed: Box<Option<u32>>,
}

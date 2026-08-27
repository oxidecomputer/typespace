#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub enum ItemEnum {
    Gone(::json_serde::Absent),
    Kept(u32),
}

#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub enum StructEnum {
    Gone {
        #[serde(default, skip_serializing_if = "::json_serde::always")]
        gone: ::json_serde::Absent,
    },
    Kept(u32),
}

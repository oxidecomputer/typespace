#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub enum StructEnum {
    Gone { gone: ::json_serde::Absent },
    Kept(u32),
}

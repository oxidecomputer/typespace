#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub enum TupleEnum {
    Gone(u32, ::json_serde::Absent),
    Kept(u32),
}

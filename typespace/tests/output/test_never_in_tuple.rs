#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct TupleHolder {
    pub value: (u32, ::json_serde::Absent),
}

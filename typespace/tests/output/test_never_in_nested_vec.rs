#[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
pub struct NestedHolder {
    pub values: Vec<Vec<::json_serde::Absent>>,
}

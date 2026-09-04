#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct NestedHolder {
    pub values: Vec<Vec<::json_serde::Absent>>,
}

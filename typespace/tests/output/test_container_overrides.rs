#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct ContainerDefaults {
    #[serde(default, skip_serializing_if = ":: std :: collections :: HashMap::is_empty")]
    pub a_map: ::std::collections::HashMap<::std::string::String, u32>,
    #[serde(
        default,
        skip_serializing_if = ":: std :: collections :: BTreeSet::is_empty"
    )]
    pub a_set: ::std::collections::BTreeSet<::std::string::String>,
    #[serde(
        default,
        skip_serializing_if = ":: std :: collections :: VecDeque::is_empty"
    )]
    pub a_vec: ::std::collections::VecDeque<::std::string::String>,
    #[serde(default, skip_serializing_if = ":: serde_json :: Map::is_empty")]
    pub an_obj: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct Containers {
    pub a_map: ::std::collections::HashMap<::std::string::String, u32>,
    pub a_set: ::std::collections::BTreeSet<::std::string::String>,
    pub a_vec: ::std::collections::VecDeque<::std::string::String>,
    pub an_obj: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
}

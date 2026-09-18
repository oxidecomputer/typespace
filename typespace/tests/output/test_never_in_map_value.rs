#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub struct MapValueHolder {
    pub entries: ::std::collections::BTreeMap<String, ::json_serde::Never>,
}

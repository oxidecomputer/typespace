#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct ConfiguredContainers {
    #[serde(default = "defaults::configured_containers_numbers")]
    pub numbers: ::std::collections::VecDeque<u32>,
    #[serde(default = "defaults::configured_containers_counts")]
    pub counts: ::std::collections::HashMap<::std::string::String, u32>,
    #[serde(default = "defaults::configured_containers_tags")]
    pub tags: ::std::collections::HashSet<u32>,
}
impl ::std::default::Default for ConfiguredContainers {
    fn default() -> Self {
        Self {
            numbers: defaults::configured_containers_numbers(),
            counts: defaults::configured_containers_counts(),
            tags: defaults::configured_containers_tags(),
        }
    }
}
pub mod defaults {
    pub(super) fn configured_containers_counts() -> ::std::collections::HashMap<
        ::std::string::String,
        u32,
    > {
        [("a".to_string(), 1_u32)].into_iter().collect()
    }
    pub(super) fn configured_containers_numbers() -> ::std::collections::VecDeque<u32> {
        [8_u32, 6_u32, 7_u32].into_iter().collect()
    }
    pub(super) fn configured_containers_tags() -> ::std::collections::HashSet<u32> {
        [1_u32, 2_u32, 3_u32].into_iter().collect()
    }
}

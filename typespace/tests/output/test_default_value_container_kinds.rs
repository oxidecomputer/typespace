#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct ContainerDefaults {
    #[serde(default = "defaults::container_defaults_numbers")]
    pub numbers: ::std::vec::Vec<u32>,
    #[serde(default = "defaults::container_defaults_counts")]
    pub counts: ::std::collections::BTreeMap<::std::string::String, u32>,
    #[serde(default = "defaults::container_defaults_tags")]
    pub tags: ::std::vec::Vec<u32>,
    #[serde(default = "defaults::container_defaults_fixed")]
    pub fixed: [u32; 3usize],
    #[serde(default = "defaults::container_defaults_pair")]
    pub pair: (::std::string::String, u32),
}
impl ::std::default::Default for ContainerDefaults {
    fn default() -> Self {
        Self {
            numbers: defaults::container_defaults_numbers(),
            counts: defaults::container_defaults_counts(),
            tags: defaults::container_defaults_tags(),
            fixed: defaults::container_defaults_fixed(),
            pair: defaults::container_defaults_pair(),
        }
    }
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn container_defaults_numbers() -> ::std::vec::Vec<u32> {
        vec![8_u32, 6_u32, 7_u32]
    }
    pub(super) fn container_defaults_counts() -> ::std::collections::BTreeMap<
        ::std::string::String,
        u32,
    > {
        [("a".to_string(), 1_u32), ("b".to_string(), 2_u32)].into_iter().collect()
    }
    pub(super) fn container_defaults_tags() -> ::std::vec::Vec<u32> {
        [1_u32, 2_u32, 3_u32].into_iter().collect()
    }
    pub(super) fn container_defaults_fixed() -> [u32; 3usize] {
        [1_u32, 2_u32, 3_u32]
    }
    pub(super) fn container_defaults_pair() -> (::std::string::String, u32) {
        ("x".to_string(), 1_u32)
    }
}

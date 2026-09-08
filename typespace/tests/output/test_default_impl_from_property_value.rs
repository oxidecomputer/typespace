#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct WithDefaultValue {
    #[serde(default = "defaults::default_u64::<u32, 42>")]
    pub answer: u32,
    #[serde(default, skip_serializing_if = "::std::string::String::is_empty")]
    pub name: ::std::string::String,
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub maybe: ::std::option::Option<bool>,
}
impl ::std::default::Default for WithDefaultValue {
    fn default() -> Self {
        Self {
            answer: defaults::default_u64::<u32, 42>(),
            name: Default::default(),
            maybe: Default::default(),
        }
    }
}
pub mod defaults {
    pub(super) fn default_u64<T, const V: u64>() -> T
    where
        T: ::std::convert::TryFrom<u64>,
        <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,
    {
        T::try_from(V).unwrap()
    }
}

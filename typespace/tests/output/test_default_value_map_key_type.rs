#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Debug,
    Default,
    Eq,
    Ord,
    PartialEq,
    PartialOrd
)]
#[serde(transparent)]
pub struct Key(pub ::std::string::String);
impl ::std::ops::Deref for Key {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Key> for ::std::string::String {
    fn from(value: Key) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::string::String> for Key {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct KeyedDefaults {
    #[serde(default = "defaults::keyed_defaults_counts")]
    pub counts: ::std::collections::BTreeMap<Key, u32>,
}
impl ::std::default::Default for KeyedDefaults {
    fn default() -> Self {
        Self {
            counts: defaults::keyed_defaults_counts(),
        }
    }
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn keyed_defaults_counts() -> ::std::collections::BTreeMap<
        super::Key,
        u32,
    > {
        [(super::Key("a".to_string()), 1_u32), (super::Key("b".to_string()), 2_u32)]
            .into_iter()
            .collect()
    }
}

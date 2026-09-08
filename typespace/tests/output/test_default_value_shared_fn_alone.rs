#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Switch {
    #[serde(default = "defaults::default_bool::<true>")]
    pub on: bool,
}
impl ::std::default::Default for Switch {
    fn default() -> Self {
        Self {
            on: defaults::default_bool::<true>(),
        }
    }
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn default_bool<const V: bool>() -> bool {
        V
    }
}

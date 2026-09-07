#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct NullDefault {
    #[serde(default = "defaults::null_default_maybe")]
    pub maybe: ::std::option::Option<u32>,
}
impl ::std::default::Default for NullDefault {
    fn default() -> Self {
        Self {
            maybe: defaults::null_default_maybe(),
        }
    }
}
pub mod defaults {
    pub(super) fn null_default_maybe() -> ::std::option::Option<u32> {
        ::std::option::Option::None
    }
}

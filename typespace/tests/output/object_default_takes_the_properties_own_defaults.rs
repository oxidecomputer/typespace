#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Holder {
    #[serde(default = "defaults::holder_separator")]
    pub separator: SeparatorConfig,
}
impl ::std::default::Default for Holder {
    fn default() -> Self {
        Self {
            separator: defaults::holder_separator(),
        }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct SeparatorConfig {
    #[serde(default = "defaults::default_u64::<u32, 1>")]
    pub line_thickness: u32,
    #[serde(default = "defaults::separator_config_line_color")]
    pub line_color: ::std::string::String,
}
impl ::std::default::Default for SeparatorConfig {
    fn default() -> Self {
        Self {
            line_thickness: defaults::default_u64::<u32, 1>(),
            line_color: defaults::separator_config_line_color(),
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
    pub(super) fn holder_separator() -> super::SeparatorConfig {
        super::SeparatorConfig {
            line_thickness: 1_u32,
            line_color: "#B2000000".to_string(),
        }
    }
    pub(super) fn separator_config_line_color() -> ::std::string::String {
        "#B2000000".to_string()
    }
}

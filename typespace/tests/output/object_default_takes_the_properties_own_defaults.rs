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
    #[serde(default = "defaults::separator_config_line_thickness")]
    pub line_thickness: u32,
    #[serde(default = "defaults::separator_config_line_color")]
    pub line_color: ::std::string::String,
}
impl ::std::default::Default for SeparatorConfig {
    fn default() -> Self {
        Self {
            line_thickness: defaults::separator_config_line_thickness(),
            line_color: defaults::separator_config_line_color(),
        }
    }
}
pub mod defaults {
    pub(super) fn holder_separator() -> super::SeparatorConfig {
        super::SeparatorConfig {
            line_color: "#B2000000".to_string(),
            line_thickness: 1_u32,
        }
    }
    pub(super) fn separator_config_line_color() -> ::std::string::String {
        "#B2000000".to_string()
    }
    pub(super) fn separator_config_line_thickness() -> u32 {
        1_u32
    }
}

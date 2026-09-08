pub use conflated_as_absent::*;
pub use conflated_as_null::*;
pub use custom_type::*;
pub use double_option::*;
pub mod conflated_as_absent {
    #[derive(::serde::Deserialize, ::serde::Serialize)]
    pub struct ConflatedAsAbsent {
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_string: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub required_option: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub optional_option: Option<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub default_string: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_option: Option<String>,
        #[serde(default = "defaults::conflated_as_absent_peanut_string")]
        pub peanut_string: String,
        #[serde(default = "defaults::conflated_as_absent_peanut_option")]
        pub peanut_option: Option<String>,
    }
    /// Generation of default values for serde.
    pub mod defaults {
        pub(super) fn conflated_as_absent_peanut_option() -> Option<String> {
            Some("peanuts".to_string())
        }
        pub(super) fn conflated_as_absent_peanut_string() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod conflated_as_null {
    #[derive(::serde::Deserialize, ::serde::Serialize)]
    pub struct ConflatedAsNull {
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_string: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub required_option: Option<String>,
        pub optional_option: Option<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub default_string: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_option: Option<String>,
        #[serde(default = "defaults::conflated_as_null_peanut_string")]
        pub peanut_string: String,
        #[serde(default = "defaults::conflated_as_null_peanut_option")]
        pub peanut_option: Option<String>,
    }
    /// Generation of default values for serde.
    pub mod defaults {
        pub(super) fn conflated_as_null_peanut_option() -> Option<String> {
            Some("peanuts".to_string())
        }
        pub(super) fn conflated_as_null_peanut_string() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod custom_type {
    #[derive(::serde::Deserialize, ::serde::Serialize)]
    pub struct CustomType {
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_string: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub required_option: Option<String>,
        #[serde(default, skip_serializing_if = "super::OptionField::is_absent")]
        pub optional_option: super::OptionField<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub default_string: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_option: Option<String>,
        #[serde(default = "defaults::custom_type_peanut_string")]
        pub peanut_string: String,
        #[serde(default = "defaults::custom_type_peanut_option")]
        pub peanut_option: Option<String>,
    }
    /// Generation of default values for serde.
    pub mod defaults {
        pub(super) fn custom_type_peanut_option() -> Option<String> {
            Some("peanuts".to_string())
        }
        pub(super) fn custom_type_peanut_string() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod double_option {
    #[derive(::serde::Deserialize, ::serde::Serialize)]
    pub struct DoubleOption {
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_string: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub required_option: Option<String>,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_option: Option<Option<String>>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        pub default_string: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_option: Option<String>,
        #[serde(default = "defaults::double_option_peanut_string")]
        pub peanut_string: String,
        #[serde(default = "defaults::double_option_peanut_option")]
        pub peanut_option: Option<String>,
    }
    /// Generation of default values for serde.
    pub mod defaults {
        pub(super) fn double_option_peanut_option() -> Option<String> {
            Some("peanuts".to_string())
        }
        pub(super) fn double_option_peanut_string() -> String {
            "peanuts".to_string()
        }
    }
}

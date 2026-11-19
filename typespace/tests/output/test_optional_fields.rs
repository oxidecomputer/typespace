pub use absent_as_empty::*;
pub use absent_as_none::*;
pub mod absent_as_empty {
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
    pub struct AbsentAsEmpty {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub optional_vec: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub optional_set: Vec<String>,
        #[serde(default, skip_serializing_if = "::std::collections::BTreeMap::is_empty")]
        pub optional_map: ::std::collections::BTreeMap<String, u32>,
        #[serde(default)]
        pub optional_unit: (),
        pub required_vec: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub default_vec: Vec<String>,
        #[serde(flatten)]
        pub inner: Option<Inner>,
    }
    impl ::std::default::Default for AbsentAsEmpty {
        fn default() -> Self {
            AbsentAsEmpty {
                optional_vec: vec!["x".to_string()],
                optional_set: ["x".to_string()].into_iter().collect(),
                optional_map: [("m".to_string(), 1_u32)].into_iter().collect(),
                optional_unit: (),
                required_vec: vec!["y".to_string()],
                default_vec: vec!["z".to_string()],
                inner: Some(Inner { a: 7_u32 }),
            }
        }
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
    pub struct Inner {
        pub a: u32,
    }
}
pub mod absent_as_none {
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
    pub struct AbsentAsNone {
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_vec: Option<Vec<String>>,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_set: Option<Vec<String>>,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_map: Option<::std::collections::BTreeMap<String, u32>>,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub optional_unit: Option<()>,
        pub required_vec: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub default_vec: Vec<String>,
        #[serde(flatten)]
        pub inner: Option<Inner>,
    }
    impl ::std::default::Default for AbsentAsNone {
        fn default() -> Self {
            AbsentAsNone {
                optional_vec: Some(vec!["x".to_string()]),
                optional_set: Some(["x".to_string()].into_iter().collect()),
                optional_map: Some([("m".to_string(), 1_u32)].into_iter().collect()),
                optional_unit: Some(()),
                required_vec: vec!["y".to_string()],
                default_vec: vec!["z".to_string()],
                inner: Some(Inner { a: 7_u32 }),
            }
        }
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
    pub struct Inner {
        pub a: u32,
    }
}

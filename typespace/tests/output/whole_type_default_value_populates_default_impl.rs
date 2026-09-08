#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub b: u32,
    #[serde(default, skip_serializing_if = "::std::string::String::is_empty")]
    pub name: ::std::string::String,
}
impl ::std::default::Default for Config {
    fn default() -> Self {
        Config {
            b: 7_u32,
            name: "bob".to_string(),
        }
    }
}

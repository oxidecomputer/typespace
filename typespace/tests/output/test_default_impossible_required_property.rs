#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct HasRequired {
    pub required: ::std::string::String,
    #[serde(default)]
    pub count: u32,
}

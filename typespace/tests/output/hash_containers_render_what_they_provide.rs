#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, Eq, PartialEq)]
pub struct Holder {
    pub m: ::std::collections::HashMap<::std::string::String, u32>,
    pub s: ::std::collections::HashSet<::std::string::String>,
    pub v: ::std::vec::Vec<u32>,
}

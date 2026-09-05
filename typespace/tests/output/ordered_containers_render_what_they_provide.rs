#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub struct Holder {
    pub m: ::std::collections::BTreeMap<::std::string::String, u32>,
    pub s: ::std::collections::BTreeSet<::std::string::String>,
    pub v: ::std::vec::Vec<u32>,
}

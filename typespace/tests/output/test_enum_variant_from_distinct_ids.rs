#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Mixed {
    Listed(::std::vec::Vec<::std::string::String>),
    Bagged(::std::vec::Vec<::std::string::String>),
    Count(u32),
}
impl ::std::convert::From<u32> for Mixed {
    fn from(value: u32) -> Self {
        Self::Count(value)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Twin {
    Left(::std::vec::Vec<::std::string::String>),
    Right(::std::vec::Vec<::std::string::String>),
    Count(u32),
}
impl ::std::convert::From<u32> for Twin {
    fn from(value: u32) -> Self {
        Self::Count(value)
    }
}

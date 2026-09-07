#[derive(Debug, PartialEq)]
pub struct Wrapper(pub ::std::string::String);
impl ::std::ops::Deref for Wrapper {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Wrapper> for ::std::string::String {
    fn from(value: Wrapper) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::string::String> for Wrapper {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
impl ::std::str::FromStr for Wrapper {
    type Err = ::std::convert::Infallible;
    fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}
impl ::std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}

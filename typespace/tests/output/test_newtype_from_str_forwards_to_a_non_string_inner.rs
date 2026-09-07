#[derive(Debug, PartialEq)]
pub struct Port(pub u32);
impl ::std::ops::Deref for Port {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<Port> for u32 {
    fn from(value: Port) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for Port {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl ::std::str::FromStr for Port {
    type Err = <u32 as ::std::str::FromStr>::Err;
    fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self(value.parse()?))
    }
}
impl ::std::fmt::Display for Port {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}

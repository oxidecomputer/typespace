#[derive(Debug, PartialEq)]
pub enum Choice {
    Count(u32),
    Flag(bool),
}
impl ::std::default::Default for Choice {
    fn default() -> Self {
        Choice::Count(0_u32)
    }
}
impl ::std::convert::From<u32> for Choice {
    fn from(value: u32) -> Self {
        Self::Count(value)
    }
}
impl ::std::convert::From<bool> for Choice {
    fn from(value: bool) -> Self {
        Self::Flag(value)
    }
}

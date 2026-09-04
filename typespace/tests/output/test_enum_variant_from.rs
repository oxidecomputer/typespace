#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Collide {
    First(u32),
    Second(u32),
    Only(bool),
}
impl ::std::convert::From<bool> for Collide {
    fn from(value: bool) -> Self {
        Self::Only(value)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Label {
    Text(::std::string::String),
    Count(u32),
}
impl ::std::convert::From<u32> for Label {
    fn from(value: u32) -> Self {
        Self::Count(value)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Overlap {
    Single(u32),
    Wrapped(u32),
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum Point {
    Pair(u32, bool),
    One(u32),
}
impl ::std::convert::From<(u32, bool)> for Point {
    fn from(value: (u32, bool)) -> Self {
        Self::Pair(value.0, value.1)
    }
}
impl ::std::convert::From<(u32,)> for Point {
    fn from(value: (u32,)) -> Self {
        Self::One(value.0)
    }
}

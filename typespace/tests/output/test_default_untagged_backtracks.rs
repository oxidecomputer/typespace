#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Holder {
    #[serde(default = "defaults::holder_u")]
    pub u: U,
}
impl ::std::default::Default for Holder {
    fn default() -> Self {
        Self { u: defaults::holder_u() }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum U {
    First { a: Wrap, b: u32 },
    Second { a: Wrap },
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, Default, PartialEq)]
#[serde(transparent)]
pub struct Wrap(pub u32);
impl ::std::ops::Deref for Wrap {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<Wrap> for u32 {
    fn from(value: Wrap) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for Wrap {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
pub mod defaults {
    pub(super) fn holder_u() -> super::U {
        super::U::Second {
            a: super::Wrap(7_u32),
        }
    }
}

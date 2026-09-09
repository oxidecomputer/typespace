#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum Internal {
    Solo,
    Wrapped(Payload),
}
impl ::std::convert::From<Payload> for Internal {
    fn from(value: Payload) -> Self {
        Self::Wrapped(value)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct InternalItemDefault {
    #[serde(default = "defaults::internal_item_default_wrapped")]
    pub wrapped: Internal,
    #[serde(default = "defaults::internal_item_default_solo")]
    pub solo: Internal,
}
impl ::std::default::Default for InternalItemDefault {
    fn default() -> Self {
        Self {
            wrapped: defaults::internal_item_default_wrapped(),
            solo: defaults::internal_item_default_solo(),
        }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Payload {
    pub y: u32,
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn internal_item_default_wrapped() -> super::Internal {
        super::Internal::Wrapped(super::Payload { y: 9_u32 })
    }
    pub(super) fn internal_item_default_solo() -> super::Internal {
        super::Internal::Solo
    }
}

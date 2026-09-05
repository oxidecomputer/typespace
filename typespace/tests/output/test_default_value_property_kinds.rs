pub type Count = u32;
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct PropertyDefaults {
    #[serde(default = "defaults::property_defaults_address")]
    pub address: ::std::net::IpAddr,
    #[serde(default = "defaults::property_defaults_blob")]
    pub blob: ::serde_json::Value,
    #[serde(default = "defaults::property_defaults_weight")]
    pub weight: f64,
    #[serde(default = "defaults::property_defaults_wrapped")]
    pub wrapped: Wrapped,
    #[serde(default = "defaults::property_defaults_count")]
    pub count: Count,
    #[serde(default = "defaults::property_defaults_nz")]
    pub nz: ::std::num::NonZeroU64,
    #[serde(default = "defaults::property_defaults_maybe")]
    pub maybe: ::std::option::Option<u32>,
}
impl ::std::default::Default for PropertyDefaults {
    fn default() -> Self {
        Self {
            address: defaults::property_defaults_address(),
            blob: defaults::property_defaults_blob(),
            weight: defaults::property_defaults_weight(),
            wrapped: defaults::property_defaults_wrapped(),
            count: defaults::property_defaults_count(),
            nz: defaults::property_defaults_nz(),
            maybe: defaults::property_defaults_maybe(),
        }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq, Default)]
#[serde(transparent)]
pub struct Wrapped(pub u32);
impl ::std::ops::Deref for Wrapped {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<Wrapped> for u32 {
    fn from(value: Wrapped) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for Wrapped {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
pub mod defaults {
    pub(super) fn property_defaults_address() -> ::std::net::IpAddr {
        ::serde_json::from_str::<::std::net::IpAddr>("\"127.0.0.1\"").unwrap()
    }
    pub(super) fn property_defaults_blob() -> ::serde_json::Value {
        ::serde_json::from_str::<::serde_json::Value>("{\"a\":[8,6,7]}").unwrap()
    }
    pub(super) fn property_defaults_count() -> super::Count {
        7_u32
    }
    pub(super) fn property_defaults_maybe() -> ::std::option::Option<u32> {
        ::std::option::Option::Some(5_u32)
    }
    pub(super) fn property_defaults_nz() -> ::std::num::NonZeroU64 {
        ::std::num::NonZeroU64::new(1).unwrap()
    }
    pub(super) fn property_defaults_weight() -> f64 {
        1.5_f64
    }
    pub(super) fn property_defaults_wrapped() -> super::Wrapped {
        super::Wrapped(7_u32)
    }
}

#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Inner {
    pub x: u32,
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct PropertyStructDefault {
    #[serde(default = "defaults::property_struct_default_inner")]
    pub inner: Inner,
}
impl ::std::default::Default for PropertyStructDefault {
    fn default() -> Self {
        Self {
            inner: defaults::property_struct_default_inner(),
        }
    }
}
pub mod defaults {
    pub(super) fn property_struct_default_inner() -> super::Inner {
        super::Inner { x: 5_u32 }
    }
}

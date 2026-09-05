#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(tag = "t", content = "c")]
pub enum Adjacent {
    Solo,
    Newtype(u32),
    Duo(u32, ::std::string::String),
    Trio { x: u32 },
}
impl ::std::convert::From<u32> for Adjacent {
    fn from(value: u32) -> Self {
        Self::Newtype(value)
    }
}
impl ::std::convert::From<(u32, ::std::string::String)> for Adjacent {
    fn from(value: (u32, ::std::string::String)) -> Self {
        Self::Duo(value.0, value.1)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct EnumDefaults {
    #[serde(default = "defaults::enum_defaults_external_item")]
    pub external_item: External,
    #[serde(default = "defaults::enum_defaults_external_tuple")]
    pub external_tuple: External,
    #[serde(default = "defaults::enum_defaults_external_struct")]
    pub external_struct: External,
    #[serde(default = "defaults::enum_defaults_internal_struct")]
    pub internal_struct: Internal,
    #[serde(default = "defaults::enum_defaults_adjacent_item")]
    pub adjacent_item: Adjacent,
    #[serde(default = "defaults::enum_defaults_adjacent_tuple")]
    pub adjacent_tuple: Adjacent,
    #[serde(default = "defaults::enum_defaults_adjacent_struct")]
    pub adjacent_struct: Adjacent,
    #[serde(default = "defaults::enum_defaults_untagged_item")]
    pub untagged_item: Untagged,
    #[serde(default = "defaults::enum_defaults_untagged_tuple")]
    pub untagged_tuple: Untagged,
}
impl ::std::default::Default for EnumDefaults {
    fn default() -> Self {
        Self {
            external_item: defaults::enum_defaults_external_item(),
            external_tuple: defaults::enum_defaults_external_tuple(),
            external_struct: defaults::enum_defaults_external_struct(),
            internal_struct: defaults::enum_defaults_internal_struct(),
            adjacent_item: defaults::enum_defaults_adjacent_item(),
            adjacent_tuple: defaults::enum_defaults_adjacent_tuple(),
            adjacent_struct: defaults::enum_defaults_adjacent_struct(),
            untagged_item: defaults::enum_defaults_untagged_item(),
            untagged_tuple: defaults::enum_defaults_untagged_tuple(),
        }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum External {
    Solo,
    Newtype(u32),
    Duo(u32, ::std::string::String),
    Trio { x: u32 },
}
impl ::std::convert::From<u32> for External {
    fn from(value: u32) -> Self {
        Self::Newtype(value)
    }
}
impl ::std::convert::From<(u32, ::std::string::String)> for External {
    fn from(value: (u32, ::std::string::String)) -> Self {
        Self::Duo(value.0, value.1)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(tag = "t")]
pub enum Internal {
    Solo,
    Trio { x: u32 },
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Untagged {
    AsNewtype(u32),
    AsDuo(u32, ::std::string::String),
}
impl ::std::convert::From<u32> for Untagged {
    fn from(value: u32) -> Self {
        Self::AsNewtype(value)
    }
}
impl ::std::convert::From<(u32, ::std::string::String)> for Untagged {
    fn from(value: (u32, ::std::string::String)) -> Self {
        Self::AsDuo(value.0, value.1)
    }
}
pub mod defaults {
    pub(super) fn enum_defaults_adjacent_item() -> super::Adjacent {
        super::Adjacent::Newtype(7_u32)
    }
    pub(super) fn enum_defaults_adjacent_struct() -> super::Adjacent {
        super::Adjacent::Trio { x: 5_u32 }
    }
    pub(super) fn enum_defaults_adjacent_tuple() -> super::Adjacent {
        super::Adjacent::Duo(3_u32, "hi".to_string())
    }
    pub(super) fn enum_defaults_external_item() -> super::External {
        super::External::Newtype(7_u32)
    }
    pub(super) fn enum_defaults_external_struct() -> super::External {
        super::External::Trio { x: 5_u32 }
    }
    pub(super) fn enum_defaults_external_tuple() -> super::External {
        super::External::Duo(3_u32, "hi".to_string())
    }
    pub(super) fn enum_defaults_internal_struct() -> super::Internal {
        super::Internal::Trio { x: 5_u32 }
    }
    pub(super) fn enum_defaults_untagged_item() -> super::Untagged {
        super::Untagged::AsNewtype(9_u32)
    }
    pub(super) fn enum_defaults_untagged_tuple() -> super::Untagged {
        super::Untagged::AsDuo(3_u32, "hi".to_string())
    }
}

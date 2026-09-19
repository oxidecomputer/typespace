#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(transparent)]
pub struct Addressed(pub ::std::net::IpAddr);
impl ::std::ops::Deref for Addressed {
    type Target = ::std::net::IpAddr;
    fn deref(&self) -> &::std::net::IpAddr {
        &self.0
    }
}
impl ::std::convert::From<Addressed> for ::std::net::IpAddr {
    fn from(value: Addressed) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::net::IpAddr> for Addressed {
    fn from(value: ::std::net::IpAddr) -> Self {
        Self(value)
    }
}
impl ::std::default::Default for Addressed {
    fn default() -> Self {
        Addressed(
            ::serde_json::from_str::<::std::net::IpAddr>("\"127.0.0.1\"")
                .expect("invalid default provided"),
        )
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(transparent)]
pub struct Blob(pub ::serde_json::Value);
impl ::std::ops::Deref for Blob {
    type Target = ::serde_json::Value;
    fn deref(&self) -> &::serde_json::Value {
        &self.0
    }
}
impl ::std::convert::From<Blob> for ::serde_json::Value {
    fn from(value: Blob) -> Self {
        value.0
    }
}
impl ::std::convert::From<::serde_json::Value> for Blob {
    fn from(value: ::serde_json::Value) -> Self {
        Self(value)
    }
}
impl ::std::default::Default for Blob {
    fn default() -> Self {
        Blob(
            ::serde_json::from_str::<::serde_json::Value>("{\"a\":[8,6,7]}")
                .expect("invalid default provided"),
        )
    }
}
pub type Count = u32;
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(transparent)]
pub struct Counted(pub Count);
impl ::std::ops::Deref for Counted {
    type Target = Count;
    fn deref(&self) -> &Count {
        &self.0
    }
}
impl ::std::convert::From<Counted> for Count {
    fn from(value: Counted) -> Self {
        value.0
    }
}
impl ::std::convert::From<Count> for Counted {
    fn from(value: Count) -> Self {
        Self(value)
    }
}
impl ::std::default::Default for Counted {
    fn default() -> Self {
        Counted(3_u32)
    }
}
#[derive(Debug, Default, PartialEq)]
pub struct Marker;
impl ::serde::Serialize for Marker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        ::serde_json::Value::String("marker".to_string()).serialize(serializer)
    }
}
impl<'de> ::serde::Deserialize<'de> for Marker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let expected = ::serde_json::Value::String("marker".to_string());
        let value: ::serde_json::Value = ::serde::Deserialize::deserialize(
            deserializer,
        )?;
        if value != expected {
            return Err(
                ::serde::de::Error::custom(
                    format!(
                        "expected unit struct value {}, found {}", "\"marker\"",
                        ::serde_json::to_string(& value).unwrap()
                    ),
                ),
            );
        }
        Ok(Marker)
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
#[serde(transparent)]
pub struct Weight(pub f64);
impl ::std::ops::Deref for Weight {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}
impl ::std::convert::From<Weight> for f64 {
    fn from(value: Weight) -> Self {
        value.0
    }
}
impl ::std::convert::From<f64> for Weight {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl ::std::default::Default for Weight {
    fn default() -> Self {
        Weight(1.5_f64)
    }
}

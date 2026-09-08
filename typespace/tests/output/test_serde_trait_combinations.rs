pub mod both {
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    #[serde(tag = "t", content = "c", deny_unknown_fields)]
    pub enum Adjacent {
        P(String),
        Q(u32),
    }
    impl ::std::convert::From<u32> for Adjacent {
        fn from(value: u32) -> Self {
            Self::Q(value)
        }
    }
    pub type Alias = Vec<String>;
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    pub enum External {
        Unit,
        Payload(String),
        Fields { #[serde(rename = "cee")] c: u32 },
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    pub struct Inner {
        pub value: u32,
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    #[serde(tag = "type")]
    pub enum Internal {
        X { x: u32 },
        Y { y: u32 },
    }
    #[derive(Debug)]
    pub struct Marker;
    impl ::serde::Serialize for Marker {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            ::serde_json::Value::String("<<marker>>".to_string()).serialize(serializer)
        }
    }
    impl<'de> ::serde::Deserialize<'de> for Marker {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            let expected = ::serde_json::Value::String("<<marker>>".to_string());
            let value: serde_json::Value = ::serde::Deserialize::deserialize(
                deserializer,
            )?;
            if value != expected {
                return Err(
                    ::serde::de::Error::custom(
                        format!(
                            "expected unit struct value {}, found {}", "\"<<marker>>\"",
                            ::serde_json::to_string(& value).unwrap()
                        ),
                    ),
                );
            }
            Ok(Marker)
        }
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    pub struct Outer {
        #[serde(rename = "my-field")]
        pub my_field: String,
        #[serde(flatten)]
        pub inner: Inner,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub maybe: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub nullable: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub maybe_nullable: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub tags: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub flag: bool,
        #[serde(default)]
        pub nothing: (),
        #[serde(default = "defaults::outer_peanut")]
        pub peanut: String,
        #[serde(default, skip_serializing_if = "::json_serde::always")]
        pub never: ::json_serde::Absent,
    }
    #[derive(Debug)]
    pub struct Pair(pub String, pub u32);
    impl ::serde::Serialize for Pair {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            use ::serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(None)?;
            seq.serialize_element(&self.0)?;
            seq.serialize_element(&self.1)?;
            seq.end()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for Pair {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            struct Visitor;
            impl<'de> ::serde::de::Visitor<'de> for Visitor {
                type Value = Pair;
                fn expecting(
                    &self,
                    formatter: &mut ::std::fmt::Formatter,
                ) -> ::std::fmt::Result {
                    formatter.write_str("a sequence")
                }
                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: ::serde::de::SeqAccess<'de>,
                {
                    let field_0 = seq
                        .next_element()?
                        .ok_or_else(|| ::serde::de::Error::invalid_length(
                            0usize,
                            &"a tuple of size 2 or more",
                        ))?;
                    let field_1 = seq
                        .next_element()?
                        .ok_or_else(|| ::serde::de::Error::invalid_length(
                            1usize,
                            &"a tuple of size 2 or more",
                        ))?;
                    Ok(Pair(field_0, field_1))
                }
            }
            deserializer.deserialize_seq(Visitor)
        }
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    pub enum Renamed {
        #[serde(rename = "one")]
        One,
        #[serde(rename = "two")]
        Two,
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct Strict {
        pub name: String,
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    #[serde(untagged)]
    pub enum Untagged {
        S(String),
        N(u32),
    }
    impl ::std::convert::From<u32> for Untagged {
        fn from(value: u32) -> Self {
            Self::N(value)
        }
    }
    #[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
    #[serde(transparent)]
    pub struct Wrapper(pub String);
    impl ::std::ops::Deref for Wrapper {
        type Target = String;
        fn deref(&self) -> &String {
            &self.0
        }
    }
    impl ::std::convert::From<Wrapper> for String {
        fn from(value: Wrapper) -> Self {
            value.0
        }
    }
    impl ::std::convert::From<String> for Wrapper {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
    pub mod defaults {
        pub(super) fn outer_peanut() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod deserialize_only {
    #[derive(::serde::Deserialize, Debug)]
    #[serde(tag = "t", content = "c", deny_unknown_fields)]
    pub enum Adjacent {
        P(String),
        Q(u32),
    }
    impl ::std::convert::From<u32> for Adjacent {
        fn from(value: u32) -> Self {
            Self::Q(value)
        }
    }
    pub type Alias = Vec<String>;
    #[derive(::serde::Deserialize, Debug)]
    pub enum External {
        Unit,
        Payload(String),
        Fields { #[serde(rename = "cee")] c: u32 },
    }
    #[derive(::serde::Deserialize, Debug)]
    pub struct Inner {
        pub value: u32,
    }
    #[derive(::serde::Deserialize, Debug)]
    #[serde(tag = "type")]
    pub enum Internal {
        X { x: u32 },
        Y { y: u32 },
    }
    #[derive(Debug)]
    pub struct Marker;
    impl<'de> ::serde::Deserialize<'de> for Marker {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            let expected = ::serde_json::Value::String("<<marker>>".to_string());
            let value: serde_json::Value = ::serde::Deserialize::deserialize(
                deserializer,
            )?;
            if value != expected {
                return Err(
                    ::serde::de::Error::custom(
                        format!(
                            "expected unit struct value {}, found {}", "\"<<marker>>\"",
                            ::serde_json::to_string(& value).unwrap()
                        ),
                    ),
                );
            }
            Ok(Marker)
        }
    }
    #[derive(::serde::Deserialize, Debug)]
    pub struct Outer {
        #[serde(rename = "my-field")]
        pub my_field: String,
        #[serde(flatten)]
        pub inner: Inner,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub maybe: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub nullable: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub maybe_nullable: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub tags: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub flag: bool,
        #[serde(default)]
        pub nothing: (),
        #[serde(default = "defaults::outer_peanut")]
        pub peanut: String,
        #[serde(default, skip_serializing_if = "::json_serde::always")]
        pub never: ::json_serde::Absent,
    }
    #[derive(Debug)]
    pub struct Pair(pub String, pub u32);
    impl<'de> ::serde::Deserialize<'de> for Pair {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            struct Visitor;
            impl<'de> ::serde::de::Visitor<'de> for Visitor {
                type Value = Pair;
                fn expecting(
                    &self,
                    formatter: &mut ::std::fmt::Formatter,
                ) -> ::std::fmt::Result {
                    formatter.write_str("a sequence")
                }
                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: ::serde::de::SeqAccess<'de>,
                {
                    let field_0 = seq
                        .next_element()?
                        .ok_or_else(|| ::serde::de::Error::invalid_length(
                            0usize,
                            &"a tuple of size 2 or more",
                        ))?;
                    let field_1 = seq
                        .next_element()?
                        .ok_or_else(|| ::serde::de::Error::invalid_length(
                            1usize,
                            &"a tuple of size 2 or more",
                        ))?;
                    Ok(Pair(field_0, field_1))
                }
            }
            deserializer.deserialize_seq(Visitor)
        }
    }
    #[derive(::serde::Deserialize, Debug)]
    pub enum Renamed {
        #[serde(rename = "one")]
        One,
        #[serde(rename = "two")]
        Two,
    }
    #[derive(::serde::Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct Strict {
        pub name: String,
    }
    #[derive(::serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Untagged {
        S(String),
        N(u32),
    }
    impl ::std::convert::From<u32> for Untagged {
        fn from(value: u32) -> Self {
            Self::N(value)
        }
    }
    #[derive(::serde::Deserialize, Debug)]
    #[serde(transparent)]
    pub struct Wrapper(pub String);
    impl ::std::ops::Deref for Wrapper {
        type Target = String;
        fn deref(&self) -> &String {
            &self.0
        }
    }
    impl ::std::convert::From<Wrapper> for String {
        fn from(value: Wrapper) -> Self {
            value.0
        }
    }
    impl ::std::convert::From<String> for Wrapper {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
    pub mod defaults {
        pub(super) fn outer_peanut() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod neither {
    #[derive(Debug)]
    pub enum Adjacent {
        P(String),
        Q(u32),
    }
    impl ::std::convert::From<u32> for Adjacent {
        fn from(value: u32) -> Self {
            Self::Q(value)
        }
    }
    pub type Alias = Vec<String>;
    #[derive(Debug)]
    pub enum External {
        Unit,
        Payload(String),
        Fields { c: u32 },
    }
    #[derive(Debug)]
    pub struct Inner {
        pub value: u32,
    }
    #[derive(Debug)]
    pub enum Internal {
        X { x: u32 },
        Y { y: u32 },
    }
    #[derive(Debug)]
    pub struct Marker;
    #[derive(Debug)]
    pub struct Outer {
        pub my_field: String,
        pub inner: Inner,
        pub maybe: Option<String>,
        pub nullable: Option<String>,
        pub maybe_nullable: Option<String>,
        pub tags: Vec<String>,
        pub flag: bool,
        pub nothing: (),
        pub peanut: String,
        pub never: ::json_serde::Absent,
    }
    #[derive(Debug)]
    pub struct Pair(pub String, pub u32);
    #[derive(Debug)]
    pub enum Renamed {
        One,
        Two,
    }
    #[derive(Debug)]
    pub struct Strict {
        pub name: String,
    }
    #[derive(Debug)]
    pub enum Untagged {
        S(String),
        N(u32),
    }
    impl ::std::convert::From<u32> for Untagged {
        fn from(value: u32) -> Self {
            Self::N(value)
        }
    }
    #[derive(Debug)]
    pub struct Wrapper(pub String);
    impl ::std::ops::Deref for Wrapper {
        type Target = String;
        fn deref(&self) -> &String {
            &self.0
        }
    }
    impl ::std::convert::From<Wrapper> for String {
        fn from(value: Wrapper) -> Self {
            value.0
        }
    }
    impl ::std::convert::From<String> for Wrapper {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
    pub mod defaults {
        pub(super) fn outer_peanut() -> String {
            "peanuts".to_string()
        }
    }
}
pub mod serialize_only {
    #[derive(::serde::Serialize, Debug)]
    #[serde(tag = "t", content = "c")]
    pub enum Adjacent {
        P(String),
        Q(u32),
    }
    impl ::std::convert::From<u32> for Adjacent {
        fn from(value: u32) -> Self {
            Self::Q(value)
        }
    }
    pub type Alias = Vec<String>;
    #[derive(::serde::Serialize, Debug)]
    pub enum External {
        Unit,
        Payload(String),
        Fields { #[serde(rename = "cee")] c: u32 },
    }
    #[derive(::serde::Serialize, Debug)]
    pub struct Inner {
        pub value: u32,
    }
    #[derive(::serde::Serialize, Debug)]
    #[serde(tag = "type")]
    pub enum Internal {
        X { x: u32 },
        Y { y: u32 },
    }
    #[derive(Debug)]
    pub struct Marker;
    impl ::serde::Serialize for Marker {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            ::serde_json::Value::String("<<marker>>".to_string()).serialize(serializer)
        }
    }
    #[derive(::serde::Serialize, Debug)]
    pub struct Outer {
        #[serde(rename = "my-field")]
        pub my_field: String,
        #[serde(flatten)]
        pub inner: Inner,
        #[serde(
            default,
            deserialize_with = "::json_serde::deserialize_some",
            skip_serializing_if = "Option::is_none"
        )]
        pub maybe: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        pub nullable: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub maybe_nullable: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub tags: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pub flag: bool,
        #[serde(default)]
        pub nothing: (),
        #[serde(default = "defaults::outer_peanut")]
        pub peanut: String,
        #[serde(default, skip_serializing_if = "::json_serde::always")]
        pub never: ::json_serde::Absent,
    }
    #[derive(Debug)]
    pub struct Pair(pub String, pub u32);
    impl ::serde::Serialize for Pair {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            use ::serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(None)?;
            seq.serialize_element(&self.0)?;
            seq.serialize_element(&self.1)?;
            seq.end()
        }
    }
    #[derive(::serde::Serialize, Debug)]
    pub enum Renamed {
        #[serde(rename = "one")]
        One,
        #[serde(rename = "two")]
        Two,
    }
    #[derive(::serde::Serialize, Debug)]
    pub struct Strict {
        pub name: String,
    }
    #[derive(::serde::Serialize, Debug)]
    #[serde(untagged)]
    pub enum Untagged {
        S(String),
        N(u32),
    }
    impl ::std::convert::From<u32> for Untagged {
        fn from(value: u32) -> Self {
            Self::N(value)
        }
    }
    #[derive(::serde::Serialize, Debug)]
    #[serde(transparent)]
    pub struct Wrapper(pub String);
    impl ::std::ops::Deref for Wrapper {
        type Target = String;
        fn deref(&self) -> &String {
            &self.0
        }
    }
    impl ::std::convert::From<Wrapper> for String {
        fn from(value: Wrapper) -> Self {
            value.0
        }
    }
    impl ::std::convert::From<String> for Wrapper {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
    pub mod defaults {
        pub(super) fn outer_peanut() -> String {
            "peanuts".to_string()
        }
    }
}

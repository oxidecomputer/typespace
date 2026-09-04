#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Debug,
    ::schemars::JsonSchema,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    ::std::default::Default
)]
pub struct MyStruct {
    pub a: ::std::string::String,
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub b: ::std::option::Option<u32>,
    #[serde(deserialize_with = "::std::option::Option::deserialize")]
    pub c: ::std::option::Option<::std::string::String>,
    #[serde(default = "defaults::my_struct_d")]
    pub d: u32,
}
impl MyStruct {
    pub fn builder() -> builder::MyStruct {
        Default::default()
    }
}
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct MyStruct {
        a: ::std::result::Result<::std::string::String, ::std::string::String>,
        b: ::std::result::Result<::std::option::Option<u32>, ::std::string::String>,
        c: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        d: ::std::result::Result<u32, ::std::string::String>,
    }
    impl ::std::default::Default for MyStruct {
        fn default() -> Self {
            Self {
                a: Err("no value supplied for a".to_string()),
                b: Ok(Default::default()),
                c: Err("no value supplied for c".to_string()),
                d: Ok(super::defaults::my_struct_d()),
            }
        }
    }
    impl MyStruct {
        pub fn a<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.a = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for a: {e}"));
            self
        }
        pub fn b<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<u32>>,
            T::Error: ::std::fmt::Display,
        {
            self.b = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for b: {e}"));
            self
        }
        pub fn c<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.c = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for c: {e}"));
            self
        }
        pub fn d<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u32>,
            T::Error: ::std::fmt::Display,
        {
            self.d = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for d: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<MyStruct> for super::MyStruct {
        type Error = super::error::ConversionError;
        fn try_from(
            value: MyStruct,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                a: value.a?,
                b: value.b?,
                c: value.c?,
                d: value.d?,
            })
        }
    }
    impl ::std::convert::From<super::MyStruct> for MyStruct {
        fn from(value: super::MyStruct) -> Self {
            Self {
                a: Ok(value.a),
                b: Ok(value.b),
                c: Ok(value.c),
                d: Ok(value.d),
            }
        }
    }
}
pub mod defaults {
    pub fn my_struct_d() -> u32 {
        ::serde_json::from_value(
                ::serde_json::Value::Number(::serde_json::Number::from(42i64)),
            )
            .expect("invalid default value")
    }
}
/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}

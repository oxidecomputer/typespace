#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Alpha {
    #[serde(default = "defaults::default_u64::<u32, 7>")]
    pub count: u32,
    #[serde(default = "defaults::default_i64::<i32, -3>")]
    pub offset: i32,
    #[serde(default = "defaults::default_bool::<true>")]
    pub flag: bool,
    #[serde(default = "defaults::default_nzu64::<::std::num::NonZeroU8, 2>")]
    pub little: ::std::num::NonZeroU8,
    #[serde(default = "defaults::alpha_dip")]
    pub dip: ::std::num::NonZeroI32,
    #[serde(default = "defaults::alpha_label")]
    pub label: ::std::string::String,
}
impl ::std::default::Default for Alpha {
    fn default() -> Self {
        Self {
            count: defaults::default_u64::<u32, 7>(),
            offset: defaults::default_i64::<i32, -3>(),
            flag: defaults::default_bool::<true>(),
            little: defaults::default_nzu64::<::std::num::NonZeroU8, 2>(),
            dip: defaults::alpha_dip(),
            label: defaults::alpha_label(),
        }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct Beta {
    #[serde(default = "defaults::default_u64::<u32, 7>")]
    pub count: u32,
    #[serde(default = "defaults::default_u64::<u32, 9>")]
    pub other: u32,
    #[serde(default = "defaults::default_bool::<false>")]
    pub flag: bool,
    #[serde(default = "defaults::beta_weight")]
    pub weight: f64,
}
impl ::std::default::Default for Beta {
    fn default() -> Self {
        Self {
            count: defaults::default_u64::<u32, 7>(),
            other: defaults::default_u64::<u32, 9>(),
            flag: defaults::default_bool::<false>(),
            weight: defaults::beta_weight(),
        }
    }
}
pub mod defaults {
    pub(super) fn default_bool<const V: bool>() -> bool {
        V
    }
    pub(super) fn default_i64<T, const V: i64>() -> T
    where
        T: ::std::convert::TryFrom<i64>,
        <T as ::std::convert::TryFrom<i64>>::Error: ::std::fmt::Debug,
    {
        T::try_from(V).unwrap()
    }
    pub(super) fn default_u64<T, const V: u64>() -> T
    where
        T: ::std::convert::TryFrom<u64>,
        <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,
    {
        T::try_from(V).unwrap()
    }
    pub(super) fn default_nzu64<T, const V: u64>() -> T
    where
        T: ::std::convert::TryFrom<::std::num::NonZeroU64>,
        <T as ::std::convert::TryFrom<::std::num::NonZeroU64>>::Error: ::std::fmt::Debug,
    {
        T::try_from(::std::num::NonZeroU64::try_from(V).unwrap()).unwrap()
    }
    pub(super) fn alpha_dip() -> ::std::num::NonZeroI32 {
        ::std::num::NonZeroI32::new(-3).unwrap()
    }
    pub(super) fn alpha_label() -> ::std::string::String {
        "hi".to_string()
    }
    pub(super) fn beta_weight() -> f64 {
        1.5_f64
    }
}

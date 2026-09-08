#[derive(::serde::Deserialize, ::serde::Serialize, Debug, Default, PartialEq)]
pub struct A {
    #[serde(
        default,
        deserialize_with = "::json_serde::deserialize_some",
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub a: ::std::option::Option<::std::boxed::Box<A>>,
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct B {
    #[serde(default = "defaults::b_a")]
    pub a: A,
}
impl ::std::default::Default for B {
    fn default() -> Self {
        Self { a: defaults::b_a() }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct C {
    #[serde(default = "defaults::c_c")]
    pub c: ::std::option::Option<::std::boxed::Box<C>>,
    pub x: u32,
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct D {
    #[serde(default = "defaults::d_c")]
    pub c: ::std::option::Option<::std::boxed::Box<C>>,
}
impl ::std::default::Default for D {
    fn default() -> Self {
        Self { c: defaults::d_c() }
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct E {
    #[serde(default = "defaults::e_d")]
    pub d: D,
}
impl ::std::default::Default for E {
    fn default() -> Self {
        Self { d: defaults::e_d() }
    }
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn b_a() -> super::A {
        super::A {
            a: ::std::option::Option::Some(
                Box::new(super::A {
                    a: ::std::option::Option::Some(
                        Box::new(super::A { a: Default::default() }),
                    ),
                }),
            ),
        }
    }
    pub(super) fn c_c() -> ::std::option::Option<::std::boxed::Box<super::C>> {
        ::std::option::Option::Some(
            Box::new(super::C {
                c: ::std::option::Option::Some(
                    Box::new(super::C {
                        c: ::std::option::Option::None,
                        x: 2_u32,
                    }),
                ),
                x: 1_u32,
            }),
        )
    }
    pub(super) fn d_c() -> ::std::option::Option<::std::boxed::Box<super::C>> {
        ::std::option::Option::Some(
            Box::new(super::C {
                c: ::std::option::Option::Some(
                    Box::new(super::C {
                        c: ::std::option::Option::Some(
                            Box::new(super::C {
                                c: ::std::option::Option::None,
                                x: 2_u32,
                            }),
                        ),
                        x: 1_u32,
                    }),
                ),
                x: 100_u32,
            }),
        )
    }
    pub(super) fn e_d() -> super::D {
        super::D {
            c: ::std::option::Option::Some(
                Box::new(super::C {
                    c: ::std::option::Option::Some(
                        Box::new(super::C {
                            c: ::std::option::Option::Some(
                                Box::new(super::C {
                                    c: ::std::option::Option::None,
                                    x: 2_u32,
                                }),
                            ),
                            x: 1_u32,
                        }),
                    ),
                    x: 100_u32,
                }),
            ),
        }
    }
}

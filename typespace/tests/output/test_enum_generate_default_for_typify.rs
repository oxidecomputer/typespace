pub enum EnumAdjacent {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumAdjacent {
    fn default() -> Self {
        EnumAdjacent::Foo
    }
}
pub enum EnumExternal {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumExternal {
    fn default() -> Self {
        EnumExternal::Foo
    }
}
pub enum EnumInternal {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumInternal {
    fn default() -> Self {
        EnumInternal::Foo
    }
}
pub enum EnumUntagged {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumUntagged {
    fn default() -> Self {
        EnumUntagged::Foo
    }
}

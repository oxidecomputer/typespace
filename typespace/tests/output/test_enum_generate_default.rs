pub enum EnumAdjacent {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumAdjacent {
    fn default() -> Self {
        EnumAdjacent::Bar("None".to_string())
    }
}
pub enum EnumExternal {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumExternal {
    fn default() -> Self {
        EnumExternal::Bar("None".to_string())
    }
}
pub enum EnumInternal {
    Foo,
    Bar { value: ::std::string::String },
    Baz,
}
impl ::std::default::Default for EnumInternal {
    fn default() -> Self {
        EnumInternal::Bar {
            value: "None".to_string(),
        }
    }
}
pub enum EnumUntagged {
    Foo,
    Bar(::std::string::String),
    Baz,
}
impl ::std::default::Default for EnumUntagged {
    fn default() -> Self {
        EnumUntagged::Bar("None".to_string())
    }
}

#[derive(Default)]
pub enum EnumAdjacent {
    #[default]
    Foo,
    Bar(::std::string::String),
    Baz,
}
#[derive(Default)]
pub enum EnumExternal {
    #[default]
    Foo,
    Bar(::std::string::String),
    Baz,
}
#[derive(Default)]
pub enum EnumInternal {
    #[default]
    Foo,
    Bar(::std::string::String),
    Baz,
}
#[derive(Default)]
pub enum EnumUntagged {
    #[default]
    Foo,
    Bar(::std::string::String),
    Baz,
}

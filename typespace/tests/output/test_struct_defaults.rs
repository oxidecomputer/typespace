pub struct StructAllDefault {
    pub a: ::std::string::String,
    pub b: ::std::option::Option<::std::string::String>,
}
impl ::std::default::Default for StructAllDefault {
    fn default() -> Self {
        StructAllDefault {
            a: "x".to_string(),
            b: Default::default(),
        }
    }
}

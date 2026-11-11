pub struct Aliased(pub u32, pub Tail);
impl ::schemars::JsonSchema for Aliased {
    fn schema_name() -> ::std::string::String {
        "Aliased".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Aliased".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<Tail>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Boxed(pub u32, pub ::std::boxed::Box<::std::vec::Vec<::std::string::String>>);
impl ::schemars::JsonSchema for Boxed {
    fn schema_name() -> ::std::string::String {
        "Boxed".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Boxed".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g
                                .subschema_for::<
                                    ::std::boxed::Box<::std::vec::Vec<::std::string::String>>,
                                >(),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct Newtype(pub u32, pub Wrapped);
impl ::schemars::JsonSchema for Newtype {
    fn schema_name() -> ::std::string::String {
        "Newtype".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Newtype".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<Wrapped>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub type Tail = ::std::vec::Vec<::std::string::String>;
#[derive(schemars::JsonSchema)]
#[schemars(transparent)]
pub struct Wrapped(pub ::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for Wrapped {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Wrapped> for ::std::vec::Vec<::std::string::String> {
    fn from(value: Wrapped) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::vec::Vec<::std::string::String>> for Wrapped {
    fn from(value: ::std::vec::Vec<::std::string::String>) -> Self {
        Self(value)
    }
}

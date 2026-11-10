#[derive(schemars::JsonSchema)]
#[schemars(transparent)]
pub struct Cycle(pub ::std::boxed::Box<OtherCycle>);
impl ::std::ops::Deref for Cycle {
    type Target = ::std::boxed::Box<OtherCycle>;
    fn deref(&self) -> &::std::boxed::Box<OtherCycle> {
        &self.0
    }
}
impl ::std::convert::From<Cycle> for ::std::boxed::Box<OtherCycle> {
    fn from(value: Cycle) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::boxed::Box<OtherCycle>> for Cycle {
    fn from(value: ::std::boxed::Box<OtherCycle>) -> Self {
        Self(value)
    }
}
pub struct Cyclic(pub u32, pub Cycle);
impl ::schemars::JsonSchema for Cyclic {
    fn schema_name() -> ::std::string::String {
        "Cyclic".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("Cyclic".to_string()),
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
                        ::std::boxed::Box::new(::schemars::schema::Schema::Bool(true)),
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
#[derive(schemars::JsonSchema)]
#[schemars(transparent)]
pub struct OtherCycle(pub ::std::boxed::Box<Cycle>);
impl ::std::ops::Deref for OtherCycle {
    type Target = ::std::boxed::Box<Cycle>;
    fn deref(&self) -> &::std::boxed::Box<Cycle> {
        &self.0
    }
}
impl ::std::convert::From<OtherCycle> for ::std::boxed::Box<Cycle> {
    fn from(value: OtherCycle) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::boxed::Box<Cycle>> for OtherCycle {
    fn from(value: ::std::boxed::Box<Cycle>) -> Self {
        Self(value)
    }
}

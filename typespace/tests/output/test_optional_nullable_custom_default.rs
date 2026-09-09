#[derive(Debug, schemars::JsonSchema)]
pub struct Piggies {
    #[schemars(
        default,
        skip_serializing_if = "::json_serde::OptionalNullable::is_absent"
    )]
    pub piggy_a: super::OptionField<::std::string::String>,
    #[schemars(
        default,
        skip_serializing_if = "::json_serde::OptionalNullable::is_absent"
    )]
    pub piggy_b: super::OptionField<::std::string::String>,
    #[schemars(
        default,
        skip_serializing_if = "::json_serde::OptionalNullable::is_absent"
    )]
    pub piggy_c: super::OptionField<::std::string::String>,
}
impl ::std::default::Default for Piggies {
    fn default() -> Self {
        Piggies {
            piggy_a: ::json_serde::OptionalNullable::value("roast beef".to_string()),
            piggy_b: Default::default(),
            piggy_c: ::json_serde::OptionalNullable::null(),
        }
    }
}

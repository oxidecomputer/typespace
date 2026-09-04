#[derive(::serde::Deserialize, ::serde::Serialize, Debug)]
pub enum OptionalStructEnum {
    Gone {
        #[serde(default, skip_serializing_if = "::json_serde::always")]
        gone: ::json_serde::Absent,
    },
    Kept(u32),
}
impl ::std::convert::From<u32> for OptionalStructEnum {
    fn from(value: u32) -> Self {
        Self::Kept(value)
    }
}

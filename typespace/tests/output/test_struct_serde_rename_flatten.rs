#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct Inner {
    pub value: u32,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
pub struct Outer {
    #[serde(rename = "my-field")]
    pub my_field: String,
    #[serde(flatten)]
    pub inner: Inner,
}

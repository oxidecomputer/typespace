#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(transparent)]
pub struct MyInt(pub u32);
impl ::std::ops::Deref for MyInt {
    type Target = u32;
    fn deref(&self) -> &u32 {
        &self.0
    }
}
impl ::std::convert::From<MyInt> for u32 {
    fn from(value: MyInt) -> Self {
        value.0
    }
}
impl ::std::convert::From<u32> for MyInt {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
///A newtype wrapping String.
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(transparent)]
pub struct MyString(pub String);
impl ::std::ops::Deref for MyString {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}
impl ::std::convert::From<MyString> for String {
    fn from(value: MyString) -> Self {
        value.0
    }
}
impl ::std::convert::From<String> for MyString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

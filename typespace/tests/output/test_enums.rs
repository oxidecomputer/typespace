#[derive(::serde::Serialize, ::serde::Deserialize)]
pub enum External {
    Unit,
    Item(String),
    Named { x: u32 },
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(tag = "type")]
pub enum Internal {
    Unit,
    Named { x: u32 },
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum Adjacent {
    Unit,
    Item(String),
    Named { x: u32 },
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(untagged)]
pub enum Untagged {
    Unit,
    Item(String),
    Named { x: u32 },
}

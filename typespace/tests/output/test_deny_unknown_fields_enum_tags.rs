#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(tag = "t", content = "c", deny_unknown_fields)]
pub enum DenyAdjacent {
    Only { x: u32 },
}
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(deny_unknown_fields)]
pub enum DenyExternal {
    Only { x: u32 },
}
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum DenyInternal {
    Only { x: u32 },
}
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum DenyUntagged {
    Only { x: u32 },
}

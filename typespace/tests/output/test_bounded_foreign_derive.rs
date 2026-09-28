#[derive(
    ::typespace_test_macro::ForeignDerive,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd
)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

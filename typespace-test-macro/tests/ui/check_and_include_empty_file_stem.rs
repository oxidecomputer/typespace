// A snapshot path with a directory but no file name (the shape a
// mid-edit path like "tests/output/.rs" takes) is rejected.

#[typespace_test_macro::check_and_include("tests/output/.rs", ())]
fn inner() {}

fn main() {}

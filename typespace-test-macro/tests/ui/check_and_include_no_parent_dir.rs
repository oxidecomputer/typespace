// A bare file name with no directory component is not a snapshot
// path: every real snapshot lives under a directory like
// "tests/output".

#[typespace_test_macro::check_and_include("my_test.rs", ())]
fn inner() {}

fn main() {}

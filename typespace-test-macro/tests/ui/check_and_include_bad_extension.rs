// A snapshot path must end in ".rs"; this is checked before the path
// is ever turned into a filesystem path.

#[typespace_test_macro::check_and_include("tests/output/my_test.txt", ())]
fn inner() {}

fn main() {}

// The snapshot path must be relative to the crate; an absolute path
// would have the macro write outside the crate.

#[typespace_test_macro::check_and_include("/tmp/output/my_test.rs", ())]
fn inner() {}

fn main() {}

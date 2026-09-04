// A type alias renders as `type N = T;`, and Rust allows no derive
// there (E0774). Attributes are legal on an alias, so `#[attr]` is not
// restricted the same way.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[derive = ["::std::hash::Hash"]]
        type Handle = String;
    });
}

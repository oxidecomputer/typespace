// `#[derive]` is type-level; a field is not a named type.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[derive = ["::std::hash::Hash"]]
            name: String,
        }
    });
}

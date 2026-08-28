// `#[flatten]` is field-level; a struct item is not a field.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[flatten]
        struct Widget {
            name: String,
        }
    });
}

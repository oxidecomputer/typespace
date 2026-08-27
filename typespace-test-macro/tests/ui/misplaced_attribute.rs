// `#[untagged]` is real, but only valid on an enum, not a struct.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[untagged]
        struct Widget {
            name: String,
        }
    });
}

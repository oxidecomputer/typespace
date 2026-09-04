// An `#[attr]` element is an opaque attribute, written as a string.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[attr = ["allow(dead_code)", 3]]
        struct Widget {
            name: String,
        }
    });
}

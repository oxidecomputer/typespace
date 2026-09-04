// An empty `#[derive]` list claims the attribute and asks for nothing.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[derive = []]
        struct Widget {
            name: String,
        }
    });
}

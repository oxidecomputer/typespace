// `#[rename]` names a wire name, so it requires one.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[rename]
            name: String,
        }
    });
}

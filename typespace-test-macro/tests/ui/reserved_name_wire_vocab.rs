// Nor after a wire-vocabulary spelling.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Optional {
            value: u32,
        }
    });
}

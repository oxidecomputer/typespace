// An item can't reuse a container's name either.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Vec {
            value: u32,
        }
    });
}

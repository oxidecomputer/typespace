// An item can't be named after a container spelling either.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Vec {
            value: u32,
        }
    });
}

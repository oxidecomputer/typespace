// An item can't be named after a primitive spelling: it would collide
// with the anonymous node the same name already identifies.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct String {
            value: u32,
        }
    });
}

// An item can't reuse a primitive's name: it would collide with the
// anonymous node that name already identifies.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct String {
            value: u32,
        }
    });
}

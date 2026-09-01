// A tuple struct can pull at most one trailing field into `rest`.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget(#[flatten] u32, #[flatten] String);
    });
}

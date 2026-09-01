// In a tuple struct, `#[flatten]` is only valid on the last field.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget(#[flatten] u32, String);
    });
}

// A single-field tuple struct is a newtype, which has no `rest` for
// `#[flatten]` to fill.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget(#[flatten] Vec<u32>);
    });
}

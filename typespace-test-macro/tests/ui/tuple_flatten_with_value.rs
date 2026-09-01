// `#[flatten]` is a bare marker on a tuple field too; there is nothing
// to give it.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget(u32, #[flatten = "inner"] String);
    });
}

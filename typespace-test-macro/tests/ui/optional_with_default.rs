// `Optional<T>` is redundant on a field that also claims `#[default]`:
// the default already makes the field absent-able, and it wins.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[default = 1]
            count: Optional<u32>,
        }
    });
}

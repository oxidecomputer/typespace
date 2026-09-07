// `OptionalNullable<T>` is redundant on a field that also claims
// `#[default]`: the default already makes the field absent-able, and
// it wins, so `Nullable<T>` with the default is what this means.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[default = 1]
            count: OptionalNullable<u32>,
        }
    });
}

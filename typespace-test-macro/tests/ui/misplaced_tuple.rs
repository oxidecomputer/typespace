// `#[tuple]` marks an enum variant or a tuple struct; a struct with
// named fields is neither.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[tuple]
        struct Widget {
            name: String,
        }
    });
}

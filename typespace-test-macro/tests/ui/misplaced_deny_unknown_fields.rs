// `#[deny_unknown_fields]` is item-level; a field is not a struct or an enum.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[deny_unknown_fields]
            name: String,
        }
    });
}

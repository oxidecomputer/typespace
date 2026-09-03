// `#[deny_unknown_fields]` is a bare marker; there is nothing to give it.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[deny_unknown_fields = true]
        struct Widget {
            name: String,
        }
    });
}

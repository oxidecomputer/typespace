// The duplicate-attribute rule covers the bare `#[flatten]` marker too.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[flatten]
            #[flatten]
            inner: String,
        }
    });
}

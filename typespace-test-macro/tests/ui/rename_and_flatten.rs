// A property carries one serde name treatment, not both.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[rename = "inner"]
            #[flatten]
            inner: String,
        }
    });
}

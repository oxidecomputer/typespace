// Bare `Option<T>` is ambiguous: absent, null, or either?

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: Option<String>,
        }
    });
}

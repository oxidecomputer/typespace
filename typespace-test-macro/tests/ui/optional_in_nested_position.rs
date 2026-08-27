// `Optional<T>` is a field-top-level-only marker; nested, it's an error.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            items: Vec<Optional<String>>,
        }
    });
}

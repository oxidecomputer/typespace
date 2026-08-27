// A leading `::` is rejected, not silently treated as a plain path
// (which would otherwise let `::Optional<T>` alias the wire keyword).

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: ::String,
        }
    });
}

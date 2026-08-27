// One segment behind a leading `::` is neither a plain type name (which
// takes no leading `::`) nor a native type's path (which needs two or
// more segments).

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: ::String,
        }
    });
}

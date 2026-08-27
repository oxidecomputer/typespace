// `Nullable` with no type argument isn't a valid reference to
// anything--it needs the `T` it wraps.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: Nullable,
        }
    });
}

// A qualified self-type names no type in this grammar, in a field or
// anywhere else a type appears.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: <u32 as Iterator>::Item,
        }
    });
}

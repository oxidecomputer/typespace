// `!` parses as a type, but typespace has no never-type model yet.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: !,
        }
    });
}

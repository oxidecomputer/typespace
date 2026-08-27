// The same attribute can't be specified twice on one field.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[default = 1]
            #[default = 2]
            count: u32,
        }
    });
}

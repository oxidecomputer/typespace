// A wire name is a string, not arbitrary JSON.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            #[rename = 1]
            name: String,
        }
    });
}

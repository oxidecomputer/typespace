// A typo'd attribute name is not recognized anywhere.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[dfeault = 1]
        struct Widget {
            name: String,
        }
    });
}

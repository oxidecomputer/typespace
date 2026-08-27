// A native type is named by a Rust path, which needs two or more
// segments; one segment is a plain item name, not a native type.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        native NaiveDate: Clone;

        struct Event {
            when: NaiveDate,
        }
    });
}

// A `native` type has no description slot; a doc comment there is
// misplaced, the same as any other attribute would be.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        /// A date.
        native chrono::NaiveDate;

        struct Event {
            when: chrono::NaiveDate,
        }
    });
}

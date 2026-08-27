// A native type's bounds are drawn from the traits typespace tracks;
// anything else names the offending trait and lists the vocabulary.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        native ::chrono::NaiveDate: Clone + Copy;

        struct Event {
            when: ::chrono::NaiveDate,
        }
    });
}

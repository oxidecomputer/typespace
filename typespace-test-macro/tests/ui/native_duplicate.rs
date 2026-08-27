// One path can only be declared native once in an invocation.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        native ::chrono::NaiveDate: Clone;
        native ::chrono::NaiveDate: Clone + Debug;

        struct Event {
            when: ::chrono::NaiveDate,
        }
    });
}

// Using a path no `native` item declared fails at the use, rather than
// conjuring a trait-less native that would only fail later.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Event {
            when: chrono::NaiveDate,
        }
    });
}

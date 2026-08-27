// A unit struct's wire representation must be stated explicitly.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Marker;
    });
}

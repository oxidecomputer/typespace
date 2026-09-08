// A unit struct has one possible value, so `#[default]` carries no
// information there.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[json = "marker"]
        #[default = "marker"]
        struct Marker;
    });
}

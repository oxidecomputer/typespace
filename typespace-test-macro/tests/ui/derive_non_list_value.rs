// `#[derive]` takes a list, not a bare string: a type commonly wants
// several derives, and the list is how it says so.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[derive = "::std::hash::Hash"]
        struct Widget {
            name: String,
        }
    });
}

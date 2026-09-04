// `#[derive]` follows the one-claim-per-name rule like every other
// attribute; several derives go in one list.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[derive = ["::std::hash::Hash"]]
        #[derive = ["PartialOrd"]]
        struct Widget {
            name: String,
        }
    });
}

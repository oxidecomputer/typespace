// A token that isn't the start of any JSON-ish value falls through to
// the Lookahead1 fallback, which enumerates what was expected.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        #[default = @]
        struct Widget {
            name: String,
        }
    });
}

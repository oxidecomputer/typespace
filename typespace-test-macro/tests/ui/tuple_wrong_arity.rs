// `#[tuple]` keeps a single-type payload from collapsing into the
// newtype form; a two-type payload is already a tuple.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        enum Choice {
            #[tuple]
            Pair(u32, bool),
        }
    });
}

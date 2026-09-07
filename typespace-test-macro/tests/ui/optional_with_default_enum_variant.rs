// The same `Optional<T>`/`#[default]` redundancy rejected on a struct
// field is rejected on an enum's struct-shaped variant fields too: both
// go through the same field lowering.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        enum Widget {
            Gone {
                #[default = 1]
                count: Optional<u32>,
            },
            Kept(u32),
        }
    });
}

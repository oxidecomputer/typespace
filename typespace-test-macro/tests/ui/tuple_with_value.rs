// `#[tuple]` is a bare marker; there is nothing to give it.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        enum Choice {
            #[tuple = "yes"]
            One(u32),
        }
    });
}

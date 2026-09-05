// A `NonZero` integer name is grammar too, so an item can't reuse one
// either: it would collide with the anonymous node that name already
// identifies.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct NonZeroU64 {
            value: u32,
        }
    });
}

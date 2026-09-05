// An item can't use a name that conflicts with the macro syntax (do we don't
// screw up by accient).

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Optional {
            value: u32,
        }
    });
}

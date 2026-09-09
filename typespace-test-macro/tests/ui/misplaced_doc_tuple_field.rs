// A tuple struct's individual positional fields carry no metadata of
// their own (`TupleStruct::fields` is a plain list of ids), so a doc
// comment on one is misplaced; only the tuple struct itself has a
// description slot.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget(
            /// The width.
            u32,
            String,
        );
    });
}

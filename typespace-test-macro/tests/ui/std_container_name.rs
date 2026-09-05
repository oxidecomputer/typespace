// `HashMap`/`BTreeMap` are Rust types; typespace_builder! names its
// map marker `Map<K, V>` instead, since the rendered container is a
// settings decision.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            by_name: HashMap<String, u32>,
        }
    });
}

// Two items can't declare the same name in one invocation.

fn main() {
    let _ = typespace_test_macro::typespace_builder!(Settings::typical(), {
        struct Widget {
            name: String,
        }

        enum Widget {
            On,
            Off,
        }
    });
}

fn wrapper() {
    {
        let _ = include_str!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", "tests/output/my_type.rs")
        );
        let __snapshot_path = ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/output/my_type.rs");
        let __content: ::std::string::String = {
            let __output_tokens = ts.render();
            let __file: ::syn::File = ::syn::parse2(__output_tokens)
                .expect("failed to parse rendered output as Rust file");
            ::prettyplease::unparse(&__file)
        };
        let __existing = ::std::fs::read_to_string(&__snapshot_path).unwrap_or_default();
        if ::newline_converter::dos2unix(&__existing)
            != ::newline_converter::dos2unix(&__content)
        {
            ::expectorate::assert_contents(&__snapshot_path, &__content);
            panic!("snapshot updated, run tests again: {}", __snapshot_path.display());
        }
        mod import {
            use super::*;
            pub struct MyType(pub String);
        }
        let value = import::MyType("hello".to_string());
        assert_eq!(value.0, "hello");
    }
}

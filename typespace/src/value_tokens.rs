// Copyright 2026 Oxide Computer Company

use proc_macro2::TokenStream;
use quote::quote;

/// Emit a `serde_json::Value` as a token stream that constructs the same value
/// at runtime.
pub fn value_tokens(value: &serde_json::Value) -> TokenStream {
    match value {
        serde_json::Value::Null => quote! {
            ::serde_json::Value::Null
        },
        serde_json::Value::Bool(b) => {
            quote! {
                ::serde_json::Value::Bool(#b)
            }
        }
        serde_json::Value::Number(number) => {
            if let Some(n) = number.as_i64() {
                quote! {
                    ::serde_json::Value::Number(::serde_json::Number::from(#n))
                }
            } else if let Some(n) = number.as_u64() {
                quote! {
                    ::serde_json::Value::Number(::serde_json::Number::from(#n))
                }
            } else if let Some(n) = number.as_f64() {
                // The unwrap in the emitted code cannot fire: from_f64
                // returns None only for a non-finite input, and a
                // serde_json::Number cannot hold one, so any f64 read
                // out of a Number converts back.
                quote! {
                    ::serde_json::Value::Number(
                        ::serde_json::Number::from_f64(#n).unwrap()
                    )
                }
            } else {
                // A number, the fourth kind! Practically this happens if
                // serde_json's arbitrary_precision feature is enabled.
                let value_as_str = number.to_string();
                quote! {
                    ::serde_json::from_str::<::serde_json::Value>(
                        #value_as_str
                    ).unwrap()
                }
            }
        }
        serde_json::Value::String(s) => quote! {
            ::serde_json::Value::String(#s.to_string())
        },
        serde_json::Value::Array(values) => {
            let elems = values.iter().map(value_tokens);
            quote! {
                ::serde_json::Value::Array(vec![#(#elems),*])
            }
        }
        serde_json::Value::Object(map) => {
            let entries = map.iter().map(|(k, v)| {
                let value = value_tokens(v);
                quote! {
                    (#k.to_string(), #value)
                }
            });
            quote! {
                ::serde_json::Value::Object(
                    ::serde_json::Map::from_iter([#(#entries),*])
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::value_tokens;

    /// The three number kinds that have a Rust literal form.
    ///
    /// The fourth kind, a number that is none of i64, u64, or f64,
    /// exists only when serde_json's `arbitrary_precision` feature is
    /// on. That feature belongs to the consumer's build rather than to
    /// typespace, so no test here can construct one; see the comment
    /// on that arm.
    #[test]
    fn each_number_kind_renders() {
        let cases = [
            ("-7", "from (- 7i64)"),
            // Above i64::MAX, so as_i64 declines and as_u64 answers.
            ("18446744073709551615", "from (18446744073709551615u64)"),
            ("1.5", "from_f64 (1.5f64)"),
        ];
        for (literal, expected) in cases {
            let value = serde_json::from_str::<serde_json::Value>(literal).unwrap();
            let rendered = value_tokens(&value).to_string();
            assert!(
                rendered.contains(expected),
                "{literal} rendered as {rendered}, wanted {expected}",
            );
        }
    }
}

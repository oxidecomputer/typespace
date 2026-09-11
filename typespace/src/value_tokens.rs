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
                    ::serde_json::Value::Number(::serde_json::Number::from_f64(#n).unwrap())
                }
            } else {
                // A number that is none of i64, u64, or f64 (possible
                // only under serde_json's arbitrary_precision feature)
                // has no literal form here.
                panic!("Invalid number")
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

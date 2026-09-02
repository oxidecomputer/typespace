// Copyright 2026 Oxide Computer Company

//! Implementation of the `check_and_include` attribute macro; see its
//! rustdoc in `lib.rs` for the user-facing contract.

use std::str::FromStr;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ItemFn, LitStr, Token,
};

/// Panic message used when a snapshot file did not exist (or was empty)
/// and the macro wrote a fresh one.
const MISSING_FILE_MESSAGE: &str = "snapshot file created, run tests again";

struct MacroArgs {
    filename: LitStr,
    output_expr: Expr,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let filename: LitStr = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let output_expr: Expr = input.parse()?;
        Ok(MacroArgs {
            filename,
            output_expr,
        })
    }
}

fn pretty_tokens(output_expr: &Expr) -> proc_macro2::TokenStream {
    quote! {
        {
            let __output_tokens = #output_expr;
            let __file: ::syn::File = ::syn::parse2(__output_tokens)
                .expect("failed to parse rendered output as Rust file");
            ::prettyplease::unparse(&__file)
        }
    }
}

fn expand_missing_file(
    filename: &str,
    output_expr: &Expr,
    message: &str,
) -> proc_macro2::TokenStream {
    let pretty = pretty_tokens(output_expr);
    // Fold `message` into the format string here, at macro-authoring time,
    // rather than passing it as a separate panic! argument: that keeps the
    // expanded tokens for the default caller identical to a plain literal
    // panic, which is what the expectorate fixture for this path pins down.
    let panic_fmt = format!("{message}: {{}}");
    quote! {
        {
            let __snapshot_path = ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(#filename);
            let __content: ::std::string::String = #pretty;
            if let Some(parent) = __snapshot_path.parent() {
                ::std::fs::create_dir_all(parent).ok();
            }
            ::std::fs::write(&__snapshot_path, &__content)
                .expect("failed to write snapshot");
            // This forces re-evaluation of the macro if the snapshot file
            // changes.
            let _ = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #filename));
            panic!(#panic_fmt, __snapshot_path.display());
        }
    }
}

fn expand_inner(
    filename: &str,
    output_expr: &Expr,
    file_tokens: proc_macro2::TokenStream,
    body_stmts: &[syn::Stmt],
) -> proc_macro2::TokenStream {
    let pretty = pretty_tokens(output_expr);
    quote! {
        {
            // This forces re-evaluation of the macro if the snapshot file
            // changes.
            let _ = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/",
            #filename));

            let __snapshot_path = ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(#filename);
            let __content: ::std::string::String = #pretty;
            let __needs_update = match ::std::fs::read_to_string(&__snapshot_path) {
                Ok(ref existing) => existing != &__content,
                Err(_) => true,
            };
            if __needs_update {
                if let Some(parent) = __snapshot_path.parent() {
                    ::std::fs::create_dir_all(parent).ok();
                }
                ::std::fs::write(&__snapshot_path, __content)
                    .expect("failed to write snapshot");
                panic!(
                    "snapshot updated, run tests again: {}",
                    __snapshot_path.display()
                );
            }
            mod import {
                use super::*;
                #file_tokens
            }
            #( #body_stmts )*
        }
    }
}

/// Entry point for the `#[check_and_include]` attribute; see its
/// rustdoc in `lib.rs`.
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MacroArgs);
    let func = parse_macro_input!(item as ItemFn);

    if !func.sig.inputs.is_empty() {
        return syn::Error::new_spanned(
            &func.sig.inputs,
            "check_and_include: function must have no parameters",
        )
        .to_compile_error()
        .into();
    }

    let filename_str = args.filename.value();
    let output_expr = &args.output_expr;
    let body_stmts = &func.block.stmts;

    // `std::env::var` here is not tracked by cargo the way `env!` is; the
    // `build.rs` rerun-if-env-changed directive is what makes toggling this
    // variable re-run the macro.
    if std::env::var("TYPESPACE_SNAPSHOT_NO_INCLUDE").is_ok_and(|v| !v.is_empty()) {
        return expand_missing_file(
            &filename_str,
            output_expr,
            "snapshot include skipped because TYPESPACE_SNAPSHOT_NO_INCLUDE is \
             set, run tests again",
        )
        .into();
    }

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return syn::Error::new_spanned(
                &args.filename,
                "CARGO_MANIFEST_DIR is not set; cannot resolve snapshot path",
            )
            .to_compile_error()
            .into();
        }
    };

    let snapshot_path = std::path::Path::new(&manifest_dir).join(&filename_str);

    let file_tokens: proc_macro2::TokenStream = match std::fs::read_to_string(&snapshot_path) {
        Err(_) => {
            // We create a file so that our later use of `include_str!` will
            // succeed.
            let _ = std::fs::write(&snapshot_path, "");
            return expand_missing_file(&filename_str, output_expr, MISSING_FILE_MESSAGE).into();
        }

        // If the file is zero-length, we assume that we made it in a previous
        // run to satisfy the condition above, but something went wrong. We'll
        // treat this as a file that needs to be created.
        Ok(content) if content.trim().is_empty() => {
            return expand_missing_file(&filename_str, output_expr, MISSING_FILE_MESSAGE).into();
        }

        Ok(content) => match proc_macro2::TokenStream::from_str(&content) {
            Ok(ts) => ts,
            Err(e) => {
                return syn::Error::new_spanned(
                    &args.filename,
                    format!(
                        "snapshot file {} contains invalid Rust ({}): \
                         fix or delete it to regenerate",
                        snapshot_path.display(),
                        e
                    ),
                )
                .to_compile_error()
                .into();
            }
        },
    };

    expand_inner(&filename_str, output_expr, file_tokens, body_stmts).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_expansion_missing_file() {
        let output_expr: Expr = parse_quote! { ts.render() };
        let expanded = expand_missing_file(
            "tests/output/missing.rs",
            &output_expr,
            MISSING_FILE_MESSAGE,
        );
        let wrapped: syn::File = parse_quote! { fn wrapper() { #expanded } };
        let out = prettyplease::unparse(&wrapped);
        expectorate::assert_contents("tests/output/test_expansion_missing_file.rs", &out);
    }

    #[test]
    fn test_expansion() {
        let output_expr: Expr = parse_quote! { ts.render() };
        let file_tokens: proc_macro2::TokenStream = parse_quote! {
            pub struct MyType(pub String);
        };
        let body_stmts: Vec<syn::Stmt> = parse_quote! {
            let value = import::MyType("hello".to_string());
            assert_eq!(value.0, "hello");
        };

        let expanded = expand_inner(
            "tests/output/my_type.rs",
            &output_expr,
            file_tokens,
            &body_stmts,
        );

        let wrapped: syn::File = parse_quote! { fn wrapper() { #expanded } };
        let out = prettyplease::unparse(&wrapped);
        expectorate::assert_contents("tests/output/test_expansion.rs", &out);
    }
}

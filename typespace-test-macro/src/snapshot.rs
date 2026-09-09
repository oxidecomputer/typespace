// Copyright 2026 Oxide Computer Company

//! Implementation of the `check_and_include` attribute macro; see its
//! rustdoc in `lib.rs` for the user-facing contract.

use std::str::FromStr;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, ItemFn, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Panic message used when a snapshot file did not exist (or was empty)
/// and the macro wrote a fresh one.
const MISSING_FILE_MESSAGE: &str = "snapshot file created, run tests again";

/// Text rust-analyzer splices into the source at the cursor position
/// when it computes completions: it reparses the edited source with
/// the marker inserted and re-expands proc macros so it can see what
/// is available at that point. While someone is typing the filename
/// argument of `check_and_include`, this fires on every keystroke,
/// so `filename_str` momentarily contains a nonsense partial path with
/// this marker embedded in it. A real `cargo build` never inserts this
/// text anywhere, so seeing it here means we are being asked for
/// completions, not compiling for real.
const RA_COMPLETION_MARKER: &str = "raCompletionMarker";

/// Example snapshot path used in guard error messages below.
const EXAMPLE_SNAPSHOT_PATH: &str = "tests/output/my_test.rs";

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

/// Expansion used when `filename_str` contains [`RA_COMPLETION_MARKER`]:
/// a stub that only needs to parse and compile. It must not touch the
/// filesystem or reference the (garbage) path in any way, since the
/// path is a nonsense partial string mid-edit, not a real snapshot
/// path.
fn expand_ra_completion_stub() -> proc_macro2::TokenStream {
    quote! {
        {
            unimplemented!(
                "check_and_include: stub expansion for a rust-analyzer \
                 completion request"
            )
        }
    }
}

/// Checks that `filename_str` is a well-formed relative snapshot path,
/// before it is turned into a filesystem path anywhere in `expand`.
/// Returns `Err` describing what is wrong and what a valid snapshot
/// path looks like.
fn validate_snapshot_path(filename_str: &str) -> Result<(), String> {
    if !filename_str.ends_with(".rs") {
        return Err(format!(
            "snapshot path {filename_str:?} must end in \".rs\"; a \
             snapshot path looks like {EXAMPLE_SNAPSHOT_PATH:?}"
        ));
    }

    let file_name = filename_str.rsplit('/').next().unwrap_or(filename_str);
    let stem = &file_name[..file_name.len() - ".rs".len()];
    if stem.is_empty() {
        return Err(format!(
            "snapshot path {filename_str:?} has no file name before \
             \".rs\"; a snapshot path looks like {EXAMPLE_SNAPSHOT_PATH:?}"
        ));
    }

    let parent = filename_str
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    if parent.is_empty() {
        return Err(format!(
            "snapshot path {filename_str:?} has no parent directory; a \
             snapshot path looks like {EXAMPLE_SNAPSHOT_PATH:?}, not a bare \
             file name at the crate root"
        ));
    }

    if filename_str.starts_with('/') || filename_str.split('/').any(|c| c == "..") {
        return Err(format!(
            "snapshot path {filename_str:?} must be a relative path with \
             no leading \"/\" and no \"..\" component; a snapshot path \
             looks like {EXAMPLE_SNAPSHOT_PATH:?}"
        ));
    }

    Ok(())
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

    // See `RA_COMPLETION_MARKER`: rust-analyzer re-expands this macro on
    // every keystroke while the filename argument is being typed, with
    // the marker embedded in `filename_str` at the cursor position. We
    // skip silently -- no `compile_error!` -- rather than flooding the
    // editor with an error on every keystroke; this can never occur in
    // a real cargo build, so nothing is lost. Checked first, before any
    // of the path guards below, so a marker-bearing path never reaches
    // (and never fails) those checks.
    if filename_str.contains(RA_COMPLETION_MARKER) {
        return expand_ra_completion_stub().into();
    }

    if let Err(message) = validate_snapshot_path(&filename_str) {
        return syn::Error::new_spanned(&args.filename, message)
            .to_compile_error()
            .into();
    }

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
            // `validate_snapshot_path` only checks that the path has a
            // parent directory component syntactically; it does not
            // check that the directory exists on disk. If it does not,
            // `std::fs::write` below returns an error, which we already
            // discard, so this is already a silent no-op: no file gets
            // created and expansion falls through to
            // `expand_missing_file` below as it would for any other
            // missing snapshot file.
            //
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

    #[test]
    fn test_expansion_ra_completion_stub() {
        // Not an expectorate golden: this pins the invariants that
        // matter (it parses, and it never mentions the snapshot path
        // or `include_str!`) rather than the exact rendered text, so
        // there is no fixture file for this test to write.
        let expanded = expand_ra_completion_stub();
        let wrapped: syn::File = parse_quote! { fn wrapper() { #expanded } };
        let out = prettyplease::unparse(&wrapped);
        assert!(!out.contains("include_str"), "{out}");
        assert!(!out.contains("raCompletionMarker"), "{out}");
    }

    #[test]
    fn test_validate_snapshot_path_accepts_call_site_shape() {
        assert!(validate_snapshot_path("tests/output/my_test.rs").is_ok());
        assert!(validate_snapshot_path("tests/output/nested/dir/my_test.rs").is_ok());
    }

    #[test]
    fn test_validate_snapshot_path_rejects_non_rs_extension() {
        let err = validate_snapshot_path("tests/output/my_test.txt").unwrap_err();
        assert!(err.contains("must end in \".rs\""), "{err}");
    }

    #[test]
    fn test_validate_snapshot_path_rejects_empty_stem() {
        // This is the shape that a mid-edit path like
        // `tests/output/.rs` produces: a directory but no file name.
        let err = validate_snapshot_path("tests/output/.rs").unwrap_err();
        assert!(err.contains("no file name before"), "{err}");
    }

    #[test]
    fn test_validate_snapshot_path_rejects_missing_parent() {
        let err = validate_snapshot_path("my_test.rs").unwrap_err();
        assert!(err.contains("no parent directory"), "{err}");
    }

    #[test]
    fn test_validate_snapshot_path_rejects_absolute_path() {
        let err = validate_snapshot_path("/tmp/output/my_test.rs").unwrap_err();
        assert!(err.contains("must be a relative path"), "{err}");
    }

    #[test]
    fn test_validate_snapshot_path_rejects_parent_dir_component() {
        let err = validate_snapshot_path("tests/../output/my_test.rs").unwrap_err();
        assert!(err.contains("must be a relative path"), "{err}");
    }
}

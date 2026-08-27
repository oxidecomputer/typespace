// Copyright 2026 Oxide Computer Company

//! The error-path oracle for `typespace_builder!`: every UI case in
//! `tests/ui/` pins both the message and the span rustc reports for
//! one error condition. Each fixture never gets far enough to emit any
//! `crate::...` reference (see `builder::expand`: an error discards the
//! whole input and emits only `compile_error!`), so these fixtures need
//! no dependency beyond this crate itself.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

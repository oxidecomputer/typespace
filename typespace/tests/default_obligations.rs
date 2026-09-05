// Copyright 2026 Oxide Computer Company

//! Tests pinning two `Default`-obligation bugs in `feasibility` for
//! `Type::Struct` in `typespace/src/trait_resolution.rs`.
//!
//! An obligation should exist exactly where the rendered `Default`
//! impl calls `Default::default()` on a property's type. Each test
//! below asserts the outcome that follows from that rule; today
//! `feasibility` computes the wrong obligation set, so finalization
//! fails where it should succeed. These tests are expected to fail
//! until the obligation computation is fixed; do not update them to
//! match the current (wrong) behavior.

use typespace::{no_cycles, settings::Settings, TypespaceTrait};
use typespace_test_macro::typespace_builder;

/// A property with its own attached default value is never filled with
/// `Default::default()`: the rendered impl takes its value from a
/// generated `defaults::` function instead (see
/// `tests/output/test_default_impl_from_property_value.rs`, where
/// `answer` comes from `defaults::with_default_value_answer()` while
/// `name` and `maybe` come from `Default::default()`). So a struct with
/// no whole-type default, and only a property in that state, should be
/// able to implement `Default` even though that property's type
/// (`::std::net::IpAddr`) has no `Default` of its own.
///
/// `feasibility`'s no-whole-type-default branch instead collects an
/// obligation for exactly the properties in
/// `StructPropertyState::DefaultValue` state--the ones that need
/// nothing--and none for the properties that actually call
/// `Default::default()`. That obligates `IpAddr` for `Default`, so
/// finalize rejects a struct that could otherwise implement it.
#[test]
fn default_value_property_is_not_obligated_for_default() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Default),
        {
            native ::std::net::IpAddr: Clone + Debug + PartialEq + Serialize + Deserialize;

            struct WithNonDefaultProperty {
                #[default = "127.0.0.1"]
                address: ::std::net::IpAddr,
            }
        }
    );

    builder.finalize(no_cycles).expect(
        "WithNonDefaultProperty's Default impl fills `address` from a \
         generated defaults:: function, never IpAddr::default(), so \
         IpAddr's missing Default impl should not block finalization",
    );
}

/// For a struct with a whole-type default value, the rendered impl
/// takes each property the default value names from that value and
/// fills every other property with `Default::default()`
/// (`default_impl_struct` in `typespace/src/default.rs`). A property
/// absent from the default value and not in the `Optional` state
/// should therefore be filled with `Default::default()`, not rejected:
/// `feasibility` agrees, and reports exactly that property as an
/// obligation.
///
/// But `default_impl_struct` itself errors on any such property
/// instead of filling it: it only special-cases `Optional`, and
/// returns `Error::InvalidDefault` (reason "missing required property
/// {key}") for every other state, `count`'s `Default` state included.
/// `feasibility`'s promise and the walk's actual behavior disagree, so
/// finalize fails on a struct it should accept--and the error message
/// calls `count` "required" even though its state is `Default`, not
/// `Required`.
#[test]
fn whole_type_default_fills_absent_default_state_property() {
    let builder = typespace_builder!(Settings::minimal(), {
        #[default = {}]
        struct WithWholeDefault {
            #[default]
            count: u32,
        }
    });

    builder.finalize(no_cycles).expect(
        "count is absent from the struct's whole-type default value but \
         is not Optional; feasibility says its obligation is met and \
         the impl should fill it with Default::default()",
    );
}

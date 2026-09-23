// Copyright 2026 Oxide Computer Company

//! Tests pinning which properties a `Default` impl obligates, for
//! `Type::Struct` in `typespace/src/trait_resolution.rs`.
//!
//! An obligation exists exactly where the rendered `Default` impl
//! calls `Default::default()` on a property's type. Each test below
//! asserts the outcome that follows from that rule, so a change that
//! widens or narrows the obligation set fails here.

use typespace::{TypespaceTrait, no_cycles, settings::Settings};
use typespace_test_macro::typespace_builder;

mod common;

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
/// absent from the default value and in the `Default` state is
/// therefore filled, not rejected: the walk emits
/// `Default::default()` for it, and in check mode collects the
/// matching obligation on the property's type, so `feasibility` and
/// the walk agree.
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

/// A generated property default asks nothing of the property's type.
///
/// `Outer.inner` is `#[serde(default)]`, so `Inner` is required to
/// implement `Default`. `Inner` can: its only property has its own
/// default value, so the impl is `color: defaults::inner_color()`,
/// which needs no `Default` from `Color`. Finalization must succeed.
#[test]
fn generated_property_default_needs_nothing_of_its_type() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Debug),
        {
            enum Color {
                Red,
                Green,
            }

            struct Inner {
                #[default = "Red"]
                color: Color,
            }

            struct Outer {
                #[default]
                inner: Inner,
            }
        }
    );

    let ts = builder.finalize(no_cycles).expect(
        "a property with its own default value obligates nothing of its \
         type",
    );

    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream())
        .expect("rendered output parses as a Rust file");
    assert!(
        common::has_impl(&file, "Default", "Inner"),
        "no Default impl for Inner"
    );
}

// A flattened property's fields render inside the OUTER type's
// `Default` impl, so the outer type owes `Default` of what they hold
// and the flattened type itself owes nothing. The conflict has to say
// so by the path it walked: `Foo` reaches `Thing` through `inner`,
// and `Bar` is only a step along the way.
//
// The default walk raises the obligation from inside `Bar`, one hop
// below where it started, so an obligation carries the whole path it
// walked rather than the relation where it was raised.
//
// `Bar` fails here too, twice over and for its own reasons: it has a
// required property, which rules `Default` out for it entirely, and
// its `#[serde(default)]` property charges `Thing` through the seed
// in `required_resolution` that runs for every type. Neither reaches
// `Foo`, and neither is what this test reads. They are why the error
// carries three conflicts rather than one.
#[test]
fn a_flattened_obligation_names_the_path_it_walked() {
    let builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Default),
        {
            native ::ext::Thing: Clone + !Default;

            #[default = { "bar": 1 }]
            struct Foo {
                #[flatten]
                inner: Bar,
            }
            struct Bar {
                bar: u32,
                #[default]
                thing: ::ext::Thing,
            }
        }
    );

    let Err(error) = builder.finalize(no_cycles) else {
        panic!("Thing cannot implement Default");
    };

    assert!(
        error.to_string().contains(
            "    required because `Bar` passes the requirement to its field `thing`\n\
             \x20   required because `Foo` passes the requirement to its field `inner`\n"
        ),
        "the path from Foo to Thing runs through `inner` and then \
         `thing`:\n{error}"
    );
}

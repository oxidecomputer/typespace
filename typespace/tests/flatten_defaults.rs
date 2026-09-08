// Copyright 2026 Oxide Computer Company

//! Default values for a struct that flattens one of its properties.
//!
//! A flattened property has no wire name of its own: its fields appear
//! among the outer object's keys. So a default value written for the
//! outer struct names the inner struct's fields directly, and the walk
//! in `default.rs` has to route each key to whichever property claims
//! it. Today it does neither, ending in two asserts that panic during
//! `finalize`:
//!
//! ```ignore
//! assert!(extra_keys.is_empty());
//! assert!(flattened_properties.is_empty());
//! ```
//!
//! typify 1 implements this in `value_for_struct_props`
//! (typify-impl/src/value.rs): it collects every key no ordinary
//! property claims into one object and hands that object to each
//! flattened property in turn, requiring the flattened type to be a
//! struct, an option of one, or a map.
//!
//! Every test here is red until that lands.

use typespace::{
    TypespaceTrait,
    error::Error,
    no_cycles,
    settings::{Settings, Std},
};
use typespace_test_macro::typespace_builder;

fn settings() -> Settings {
    Settings::minimal()
        .with_std(Std::Unqualified)
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::PartialEq)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_desired_trait(TypespaceTrait::Default)
}

/// The rendered file, from `impl Default for #name` onward.
fn default_body(ts: &typespace::Typespace<String>, name: &str) -> String {
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);
    let needle = format!("impl ::std::default::Default for {name} {{");
    let start = rendered
        .find(&needle)
        .unwrap_or_else(|| panic!("no Default impl for {name} in:\n{rendered}"));
    rendered[start..].to_string()
}

/// A key the outer struct does not claim belongs to the flattened
/// property, and the value walk builds the inner struct from it.
#[test]
fn whole_type_default_fills_a_flattened_struct() {
    let builder = typespace_builder!(settings(), {
        #[default = { "b": 22, "c": "xx" }]
        struct Inner {
            b: u32,
            c: String,
        }

        #[default = { "a": 1, "b": 2, "c": "x" }]
        struct Outer {
            a: u32,
            #[flatten]
            inner: Inner,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Outer");
    assert!(body.contains("a: 1"), "{body}");
    assert!(body.contains("inner: Inner {"), "{body}");
    assert!(body.contains("b: 2"), "{body}");
    assert!(body.contains("c: \"x\".to_string()"), "{body}");
}

/// A flattened map absorbs every unclaimed key, not just the ones some
/// named property happens to match.
#[test]
fn whole_type_default_fills_a_flattened_map() {
    let builder = typespace_builder!(settings(), {
        #[default = { "a": 1, "x": 9, "y": 8 }]
        struct Outer {
            a: u32,
            #[flatten]
            rest: Map<String, u32>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Outer");
    assert!(body.contains("a: 1"), "{body}");
    assert!(body.contains("\"x\""), "{body}");
    assert!(body.contains("\"y\""), "{body}");
}

/// A flattened option of a struct: the unclaimed keys build the struct
/// and it lands wrapped in `Some`.
#[test]
fn whole_type_default_fills_a_flattened_optional_struct() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[default = { "a": 1, "b": 2 }]
        struct Outer {
            a: u32,
            #[flatten]
            inner: Optional<Inner>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Outer");
    assert!(body.contains("inner: Some(Inner {"), "{body}");
    assert!(body.contains("b: 2"), "{body}");
}

/// Two flattened properties, each claiming a different key.
///
/// This is the case that decides the design. typify hands the SAME
/// object of unclaimed keys to every flattened property, so each one
/// sees keys meant for the other and has to ignore them. A rule that
/// rejects an unclaimed key everywhere cannot also serve this case, so
/// the rejection has to be scoped to the outermost walk, where nothing
/// downstream can claim the key.
#[test]
fn two_flattened_properties_each_claim_their_own_keys() {
    let builder = typespace_builder!(settings(), {
        #[default = { b: 111 }]
        struct First {
            b: u32,
        }

        struct Second {
            c: Optional<u32>,
        }

        #[default = { "a": 1, "b": 2, "c": 3 }]
        struct Outer {
            a: u32,
            #[flatten]
            first: First,
            #[flatten]
            second: Second,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Outer");
    assert!(body.contains("first: First {"), "{body}");
    assert!(body.contains("b: 2"), "{body}");
    assert!(body.contains("second: Second {"), "{body}");
    assert!(body.contains("c: Some(3_u32)"), "{body}");
}

/// A key no property claims, with no flattened property to absorb it,
/// is a mistake in the default value and `finalize` says so.
///
/// typify silently drops such a key. Rejecting it is the stricter
/// reading, and it matches how the walk already treats a required
/// property the value fails to name.
#[test]
fn unclaimed_key_without_a_flattened_property_is_rejected() {
    let builder = typespace_builder!(settings(), {
        #[default = { "a": 1, "typo": 2 }]
        #[deny_unknown_fields]
        struct Outer {
            a: u32,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a default value naming `typo`");
    };
    assert!(matches!(err, Error::InvalidDefault { .. }), "{err:?}");
}

/// The same routing applies when the value arrives through a property's
/// own default rather than the type's, since both go through one walk.
#[test]
fn property_level_default_fills_a_flattened_struct() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        struct Flattening {
            a: u32,
            #[flatten]
            inner: Inner,
        }

        struct Holder {
            #[default = { "a": 1, "b": 2 }]
            held: Flattening,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let file = syn::parse2::<syn::File>(ts.to_codespace().into_stream()).unwrap();
    let rendered = prettyplease::unparse(&file);
    assert!(rendered.contains("mod defaults"), "{rendered}");
    assert!(rendered.contains("inner: super::Inner {"), "{rendered}");
}

/// A struct cannot both deny unknown fields and flatten a property.
///
/// serde decides `deny_unknown_fields` in the outer struct's
/// deserializer, which meets a key the flattened type may claim and
/// has no way to ask, so serde documents the pair as unsupported.
/// `finalize` refuses it rather than emitting code whose behavior
/// cannot be read off the source. The default-value walk relies on
/// this: its flattened branch asserts the flag is clear.
#[test]
fn deny_unknown_fields_with_a_flattened_property_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[default = { "a": 1, "b": 2 }]
        #[deny_unknown_fields]
        struct Outer {
            a: u32,
            #[flatten]
            inner: Inner,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject deny_unknown_fields with a flattened property");
    };
    assert!(
        matches!(err, Error::FlattenWithDenyUnknownFields { .. }),
        "{err:?}"
    );
}

/// The rule holds with no default value in sight: the combination is
/// refused for what it would mean at deserialization, not for what it
/// does to the value walk.
#[test]
fn deny_unknown_fields_with_a_flattened_property_is_rejected_without_a_default() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[deny_unknown_fields]
        struct Outer {
            a: u32,
            #[flatten]
            inner: Inner,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject deny_unknown_fields with a flattened property");
    };
    assert!(
        matches!(err, Error::FlattenWithDenyUnknownFields { .. }),
        "{err:?}"
    );
}

/// A value the flattened type cannot accept leaves an OPTIONAL
/// flattened property absent, and the field is still initialized.
///
/// `Inner` requires `b`, which the default value never names, so the
/// walk into the flattened property fails. Because the property is
/// optional, that failure means "absent" rather than propagating: an
/// optional property has a legal empty state and the value walk uses
/// it. The field must still appear in the struct literal, since
/// omitting it would leave code that does not compile.
///
/// One consequence worth stating: a malformed value for an optional
/// flattened property is indistinguishable from a deliberately absent
/// one. That matches how an unclaimed key is tolerated outside
/// `deny_unknown_fields`.
#[test]
fn unusable_value_leaves_an_optional_flattened_property_absent() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[default = { "a": 1 }]
        struct Outer {
            a: u32,
            #[flatten]
            inner: Optional<Inner>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Outer");
    assert!(body.contains("a: 1"), "{body}");
    assert!(body.contains("inner: Default::default()"), "{body}");
}

/// A flattened map takes only the keys no named property claimed.
///
/// A map claims every key it is handed, so handing it the whole value
/// would fold the outer struct's own properties into it. serde routes
/// a key to the named property first and offers the flattened type
/// only what is left; the value walk matches that.
#[test]
fn a_flattened_map_leaves_claimed_keys_to_their_properties() {
    let builder = typespace_builder!(settings(), {
        #[default = { "something": "s", "x": "1", "y": "2" }]
        struct Foo {
            something: String,
            #[flatten]
            rest: Map<String, String>,
        }
    });

    let ts = builder.finalize(no_cycles).unwrap();
    let body = default_body(&ts, "Foo");
    assert!(body.contains("something: \"s\".to_string()"), "{body}");
    assert!(body.contains("\"x\""), "{body}");
    assert!(body.contains("\"y\""), "{body}");
    assert_eq!(
        body.matches("something").count(),
        1,
        "`something` is claimed by its own property and must not also \
         land in the flattened map: {body}"
    );
}

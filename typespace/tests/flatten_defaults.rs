// Copyright 2026 Oxide Computer Company

//! Default values for a struct that flattens one of its properties.
//!
//! A flattened property has no wire name of its own: its fields appear
//! among the outer object's keys. So a default value written for the
//! outer struct names the inner struct's fields directly, and the walk
//! in `default.rs` routes each key to whichever property claims it.
//!
//! The rules these tests validate:
//!
//! - A named property claims its own key first. A flattened property receives
//!   only what is left unclaimed, which is what serde does at deserialization
//!   and what keeps a flattened map from swallowing the outer struct's own
//!   properties.
//! - Every flattened property receives the same leftover keys and takes what
//!   it recognizes, so two flattened properties each end up with their own.
//! - A key nothing claims is tolerated, matching serde, except under
//!   `deny_unknown_fields`, where it is an error. The strictness of the
//!   default-value check follows the strictness the struct itself declares.
//!   Note that `deny_unknown_fields` is incompatible with `flatten`.
//! - If a flattened property is optional, then not satisfying that properties
//!   own required values simply leaves the value as absent rather than
//!   resulting in an error.
//! - `deny_unknown_fields` alongside a flattened property is refused by
//!   `finalize`, since serde cannot honor the pair.
//! - A flattened property's type must serialize as an object, which
//!   `finalize` checks through any stack of options, boxes, aliases,
//!   and newtype structs. A scalar, a sequence, or a tuple is refused.
//!
//! typify 1 implements the routing in `value_for_struct_props`
//! (typify-impl/src/value.rs), requiring the flattened type to be a
//! struct, an option of one, or a map. typespace enforces the
//! equivalent rule in `check_type_structure`, differing on two points.
//! It looks through the option rather than admitting any option, so a
//! flattened `Optional<u32>` is refused here and admitted there, even
//! though serde fails on it as surely as on a bare `u32`. And it admits
//! an enum, which serializes as an object under each of serde's three
//! tagged representations; an untagged enum is judged one variant at a
//! time.

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

/// A key no property claims is not allowed with `deny_unknown_fields`.
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

/// serde can only flatten a value that serializes as an object, so a
/// property whose type is a scalar is refused.
///
/// Nothing in the generated code would say so: `#[serde(flatten)]`
/// compiles against any `Serialize` type and fails only when a value
/// reaches the wire, with "can only flatten structs and maps".
#[test]
fn a_flattened_scalar_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Outer {
            a: u32,
            #[flatten]
            count: u32,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened scalar");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// An option is judged by what it wraps.
///
/// typify 1 admits any option at all, which lets a flattened
/// `Option<u32>` through: it serializes cleanly while it holds `None`
/// and fails the moment it holds a value. The option is transparent to
/// flattening, so the answer is the inner type's.
#[test]
fn a_flattened_option_of_a_scalar_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Outer {
            a: u32,
            #[flatten]
            count: Optional<u32>,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened option of a scalar");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// A sequence serializes as an array, so flattening one is refused.
#[test]
fn a_flattened_sequence_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Outer {
            a: u32,
            #[flatten]
            items: Vec<u32>,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened sequence");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// The error names the struct and the offending property.
#[test]
fn the_flatten_error_names_the_struct_and_the_property() {
    let builder = typespace_builder!(settings(), {
        struct Outer {
            a: u32,
            #[flatten]
            label: String,
        }
    });

    let Err(Error::FlattenNonObject {
        type_name,
        property,
    }) = builder.finalize(no_cycles)
    else {
        panic!("expected finalize to reject a flattened string");
    };
    assert_eq!(type_name, "Outer");
    assert_eq!(property, "label");
}

/// A box, an alias, and a newtype struct each pass the flatten through
/// to what they wrap, so each answers as its inner type does.
#[test]
fn flattening_reaches_through_transparent_wrappers() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        type AliasOfInner = Inner;

        struct NewtypeOfInner(AliasOfInner);

        struct Outer {
            a: u32,
            #[flatten]
            inner: Box<NewtypeOfInner>,
        }
    });

    builder.finalize(no_cycles).unwrap();
}

/// A newtype struct wrapping a scalar is refused for the same reason,
/// reached the same way.
#[test]
fn a_flattened_newtype_of_a_scalar_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Count(u32);

        struct Outer {
            a: u32,
            #[flatten]
            count: Count,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened newtype of a scalar");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// A tagged enum serializes as an object whatever its variants hold,
/// because the tag is itself a key. An externally tagged unit variant
/// is the case worth naming: unflattened it serializes as a bare
/// string, and flattened serde writes it as the variant name keying a
/// `null`.
#[test]
fn a_flattened_tagged_enum_is_allowed() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        enum Tagged {
            Unit,
            Payload(Inner),
            Scalar(u32),
        }

        struct Outer {
            a: u32,
            #[flatten]
            tagged: Tagged,
        }
    });

    builder.finalize(no_cycles).unwrap();
}

/// An untagged enum writes whatever its selected variant writes, so
/// every variant has to serialize as an object.
#[test]
fn a_flattened_untagged_enum_of_objects_is_allowed() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[untagged]
        enum Either {
            Named { c: u32 },
            Wrapped(Inner),
        }

        struct Outer {
            a: u32,
            #[flatten]
            either: Either,
        }
    });

    builder.finalize(no_cycles).unwrap();
}

/// One variant that serializes as something other than an object is
/// enough to refuse the whole enum: the value that selects it is the
/// value that fails.
#[test]
fn a_flattened_untagged_enum_with_a_scalar_variant_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[untagged]
        enum Either {
            Wrapped(Inner),
            Bare(u32),
        }

        struct Outer {
            a: u32,
            #[flatten]
            either: Either,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened untagged enum with a scalar variant");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// An untagged unit variant is refused too. It serializes as nothing,
/// which leaves deserialization no key to recognize the variant by, so
/// the round trip fails rather than the serialization.
#[test]
fn a_flattened_untagged_enum_with_a_unit_variant_is_rejected() {
    let builder = typespace_builder!(settings(), {
        struct Inner {
            b: u32,
        }

        #[untagged]
        enum Either {
            Wrapped(Inner),
            Nothing,
        }

        struct Outer {
            a: u32,
            #[flatten]
            either: Either,
        }
    });

    let Err(err) = builder.finalize(no_cycles) else {
        panic!("expected finalize to reject a flattened untagged enum with a unit variant");
    };
    assert!(matches!(err, Error::FlattenNonObject { .. }), "{err:?}");
}

/// A flattened map is admitted, which the default-value tests above
/// already rely on; this states it as a rule of its own.
#[test]
fn a_flattened_map_is_allowed() {
    let builder = typespace_builder!(settings(), {
        struct Outer {
            a: u32,
            #[flatten]
            rest: Map<String, u32>,
        }
    });

    builder.finalize(no_cycles).unwrap();
}

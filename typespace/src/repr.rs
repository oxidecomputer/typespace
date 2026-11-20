// Copyright 2026 Oxide Computer Company

//! How a struct property is rendered in a Rust struct.
//!
//! Each property has a state (required, optional, default, or a default
//! value), a type, and a wire name or `flatten`. Those three, with the
//! settings, determine how the field is rendered: the type itself or an
//! `Option` around it, the associated serde attributes, and how it is assigned a value
//! when the key is absent. [`PropertyRepr::of`] makes this determination
//! in a single place, and the various code paths that depend on it process the result.
//!
//! Three callers use it:
//! - The renderer emits the field's type and attributes from it.
//! - The default value walk renders a supplied value in order to construct a
//!   corresponding instance of the generated type: the value itself for a bare
//!   field, `Some` for an `Option`-wrapped one, the custom container's
//!   constructor for a consumer's optional-nullable type.
//! - Trait resolution calls [`PropertyRepr::fills_from_default`] to find the
//!   fields that `Default::default()` fills when absent. This is both where
//!   `#[serde(default)]` goes and what a generated `Default` impl calls, so
//!   the types for those fields must implement the `Default` trait.
//!
//! Each variant is a distinct way a field can be rendered. The confluence
//! of state, type, and name (and settings) determine the appropriate variant;
//! the same variant may be reached via different combinations.
//! - A required scalar and an optional `Option<T>` under `ConflateAsNull` are
//!   both `Value` with no attributes, and no caller distinguishes them.
//! - A `Default`-state field and an optional container under `AbsentAsEmpty`
//!   are both `Value` with `serde(default)` and an `is_empty` skip, and both
//!   require `Default` of their type.
//!
//! Two cases are decided before the settings are consulted.
//! - A flattened property has no key, so the settings for how a keyed field
//!   represents absence do not apply. A flattened property translates to
//!   either Value of its type, or FlattenedOption when optional.
//! - A required `Option<T>` (i.e. a field that *may* be `null` but *must* be
//!   present) specifies `Option::deserialize` explicitly, since serde's default
//!   handling would also accept an absent key.
//!
//! The skip predicate for a `Default`-state field depends on the type:
//! containers and strings have `is_empty`, an `Option` has `is_none`,
//! and named types, natives, arrays, tuples, numbers, and booleans have
//! nothing a generated attribute can name. [`Skip::for_default`] holds
//! that per-type table; the typify compatibility cases and the TODOs are
//! on its arms.

use std::collections::BTreeMap;

use crate::build::{
    StructProperty, StructPropertySerde, StructPropertyState, Type, all_named_types,
};
use crate::settings::{ContainerType, OptionalContainer, OptionalNullable, OptionalUnit, Settings};

/// The predicate a field's `skip_serializing_if` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Skip<'a, Id> {
    /// `Option::is_none`.
    IsNone,
    /// `is_empty` of the type with this id, a container or a string. A
    /// field that holds it in a `Box` names the boxed type here.
    IsEmpty(&'a Id),
    /// `OptionalNullable::is_absent` of the custom optional-nullable type.
    IsAbsent,
    /// The field is never serialized.
    Always,
}

/// How a struct property is held in the Rust struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropertyRepr<'a, Id> {
    /// The type itself. `default` is whether the `#[serde(default)]` attribute
    /// is present; `skip` is the predicate, if any, that leaves the field out
    /// of serialization.
    ///
    /// The type may itself be an `Option`, meaning the value is either
    /// present or `null`. The `Option` variants below wrap a type in an
    /// `Option` because the field may be absent.
    Value {
        default: bool,
        skip: Option<Skip<'a, Id>>,
    },
    /// The type, an `Option`, which must be present but may be `null`:
    /// `deserialize_with = Option::deserialize`, since without it serde
    /// accepts an absent field.
    RequiredOption,
    /// `Option<T>` with `default`, `deserialize_some`, and the `is_none`
    /// skip: absent is `None` and `null` is an error.
    NonNullOption,
    // `Option<T>` with only the `is_none` skip: absent and `null` both
    // read as `None`, which is serde's own handling of an `Option`. This
    // is the rendering of an optional property under the `AllowNull`
    // setting proposed in typespace #68; it is here as a note until that
    // setting exists.
    //
    // NullableOption,
    /// `Option<T>` with no attribute, for a flattened property only: serde
    /// reads it as `Some` when the leftover keys parse as `T` and `None`
    /// otherwise, and serializes `None` as no additional keys.
    FlattenedOption,
    /// The consumer's optional-nullable `container` specified by
    /// [`Settings::optional_nullable`].
    CustomOption {
        container: &'a ContainerType,
        inner: &'a Id,
    },
    /// `::json_serde::Absent`: a `Never` that may be absent (and, in fact,
    /// *must* be absent).
    Absent,
}

impl<'a, Id: Ord> PropertyRepr<'a, Id> {
    /// Decide `prop`'s representation from its state, its type, and the
    /// settings.
    pub(crate) fn of(
        prop: &'a StructProperty<Id>,
        types: &'a BTreeMap<Id, Type<Id>>,
        settings: &'a Settings,
    ) -> Self {
        let ty = types.get(&prop.type_id).unwrap();
        match (&prop.json_name, &prop.state, ty) {
            // A flattened property has no key, so the settings for how a keyed
            // field represents absence do not apply.
            (StructPropertySerde::Flatten, StructPropertyState::Optional, _) => {
                Self::FlattenedOption
            }
            (StructPropertySerde::Flatten, StructPropertyState::Required, _) => Self::Value {
                default: false,
                skip: None,
            },
            (StructPropertySerde::Flatten, _, _) => unreachable!(),

            (_, StructPropertyState::Optional, Type::Never) => Self::Absent,
            (
                _,
                StructPropertyState::Required
                | StructPropertyState::Default
                | StructPropertyState::DefaultValue(_),
                Type::Never,
            ) => unreachable!("finalization rejects a Never property that requires a value"),

            // A required field that is an Option<T> needs a custom
            // deserializer so that the field must be present, but may be null.
            (_, StructPropertyState::Required, Type::Option(_)) => Self::RequiredOption,

            // A required field has no serde annotations.
            (_, StructPropertyState::Required, _) => Self::Value {
                default: false,
                skip: None,
            },

            // An explicit default value for when the field is absent. Note
            // that serialization is not impacted. The renderer specifies the
            // `defaults` function in the attribute.
            (_, StructPropertyState::DefaultValue(_), _) => Self::Value {
                default: false,
                skip: None,
            },

            // A default field that is an Option<T> explicitly conflates absent
            // and null, so we ignore the optional_nullable setting. Note that
            // #[serde(default)] is a no-op for Option<T> so we omit it.
            (_, StructPropertyState::Default, Type::Option(_)) => Self::Value {
                default: false,
                skip: Some(Skip::IsNone),
            },

            // A default field that's not an Option<T> explicitly conflates
            // absence and the default value.
            (_, StructPropertyState::Default, _) => Self::Value {
                default: true,
                skip: Skip::for_default(&prop.type_id, types, settings),
            },

            // An optional field that is also an Option<T> may be the type
            // value, null, or absent. The settings determine its handling.
            // As above, default is a no-op for Option<T>.
            (_, StructPropertyState::Optional, Type::Option(inner)) => {
                match &settings.optional_nullable {
                    OptionalNullable::ConflateAsAbsent => Self::Value {
                        default: false,
                        skip: Some(Skip::IsNone),
                    },
                    OptionalNullable::ConflateAsNull => Self::Value {
                        default: false,
                        skip: None,
                    },
                    // Wrap the type (an Option) in another Option.
                    OptionalNullable::DoubleOption => Self::NonNullOption,
                    OptionalNullable::CustomType(container) => {
                        Self::CustomOption { container, inner }
                    }
                }
            }

            // For optional containers, settings specify the handling.
            (_, StructPropertyState::Optional, Type::Vec(_) | Type::Set(_) | Type::Map(..)) => {
                match settings.optional_container {
                    OptionalContainer::AbsentAsEmpty => Self::Value {
                        default: true,
                        skip: Some(Skip::IsEmpty(&prop.type_id)),
                    },
                    OptionalContainer::AbsentAsNone => Self::NonNullOption,
                }
            }

            // For an optional unit, settings specify the handling.
            (_, StructPropertyState::Optional, Type::Unit) => match settings.optional_unit {
                OptionalUnit::AbsentAsUnit => Self::Value {
                    default: true,
                    skip: None,
                },
                OptionalUnit::AbsentAsNone => Self::NonNullOption,
            },

            // An optional typed field may not be null; we use the
            // json_serde::deserialize_some function to enforce this.
            (_, StructPropertyState::Optional, _) => Self::NonNullOption,
        }
    }

    /// Whether an absent field is filled with `Default::default()`: by
    /// serde through `#[serde(default)]`, and by a generated `Default`
    /// impl.
    pub(crate) fn fills_from_default(&self) -> bool {
        matches!(self, Self::Value { default: true, .. })
    }
}

impl<'a, Id: Ord> Skip<'a, Id> {
    /// The skip for a field in the `Default` state whose type is the one
    /// with this id.
    fn for_default(
        id: &'a Id,
        types: &'a BTreeMap<Id, Type<Id>>,
        settings: &Settings,
    ) -> Option<Self> {
        match types.get(id).unwrap() {
            // Here we assume that the generated type for the field has an
            // implementation of Default. There isn't a simple "is_default()"
            // that we can presume... so we'll just leave it.
            all_named_types!(_) => None,

            // The same applies to external types.
            Type::Native(_) => None,

            // We have a property whose type is an Option meaning that it
            // may have a value of null. The "Default" state means that it
            // takes on its "intrinsic" default value if the field is
            // absent. This means we can ignore any of the
            // optional/nullable settings as they don't particularly apply
            // here.
            //
            // Note that #[serde(default)] is a no-op for Option<T>.
            Type::Option(_) => Some(Skip::IsNone),

            Type::Box(boxed_id) => Self::for_default(boxed_id, types, settings),

            // TYPIFY COMPAT. typify never skips serialization of a
            // default-state String, so imitation withholds the skip;
            // the skip below returns when the flag goes.
            Type::String if settings.typify_compat => None,

            Type::Vec(_) | Type::Map(_, _) | Type::Set(_) | Type::String => Some(Skip::IsEmpty(id)),

            // As above, sure--there might be a Default impl--but we don't have
            // a way to check if the value matches that value, so... whatever.
            Type::Array(_, _) | Type::Tuple(_) => None,

            Type::Unit => {
                // TODO 9.7.2026
                // It's possible that we could add skip here, but typify
                // doesn't do it, and it could also screw up JsonSchema
                // generation.
                None
            }

            Type::Boolean => {
                // Congratulation! You found an external expression of my
                // insanity. I am genuinely curious if anyone will ever
                // encounter this via a generated type. Note that this may
                // cause invalid code to be generated e.g. if the type is
                // Box<bool>, and I'm fine with that.

                // I've had second thoughts and am leaving this out of the
                // code, but in as a reminder of crazier times...

                // serde_options.push(quote! {
                //     skip_serializing_if = "std::ops::Not::not"
                // });
                None
            }

            // There isn't an "is_zero()" so... we'll just leave it be.
            Type::Integer(_) | Type::Float(_) => None,

            // This isn't a runtime error that could be handled; it's a
            // programming error.
            Type::JsonValue => panic!("Default value for JsonValue is not supported"),

            // A Never property reaches this function in no state:
            // finalization rejects every state that would, and the
            // Optional state renders without a skip of this kind.
            Type::Never => unreachable!("Never properties add no skip attribute"),
        }
    }
}

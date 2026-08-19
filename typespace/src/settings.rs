// Copyright 2026 Oxide Computer Company

//! Settings that govern how types are processed and rendered.
//!
//! [`Settings`] is the root: start from [`Settings::default`] and
//! adjust with the `with_` methods, or deserialize one from
//! configuration data.

use serde::Deserialize;

// TODO 7/18/2025
// I wanted to get this started to think through various settings that we might
// eventually want...
/// Modify how types are processed and generated.
///
/// Settings are supplied to
/// [`TypespaceBuilder::finalize`](crate::TypespaceBuilder::finalize) and
/// govern rendering. Start from [`Settings::default`] and adjust with
/// the `with_` methods; the type also implements `Deserialize` so
/// settings can come from configuration data. Two axes exist today: how
/// `std` prelude types are spelled ([`Std`]) and how
/// optional-and-nullable values are modeled ([`OptionalNullable`]).
#[derive(Debug, Default, Deserialize)]
pub struct Settings {
    /// When set to `FullyQualified`, (the default), types in the `std` crate's
    /// prelude are fully qualified. For example, the `Option` type is rendered
    /// as `::std::option::Option`. When set to `Unqualified`, these types
    /// appear in their more typical, auto-imported form. The latter is useful
    /// if one intends to use type generation as a starting point for
    /// manually-edited code. Note that this is relevant only to types in the
    /// `std` crate's prelude such as `Option`, `Vec`, and `String`; types such
    /// as `std::collections::BTreeMap` are always fully qualified since they
    /// are not in the prelude.
    #[serde(default)]
    pub(crate) std: Std,

    /// Specify the modeling of values that may be either `null` or optional
    /// (i.e. absent). The default is `ConflateAsAbsent`, which models `null`
    /// and `optional` as equivalent by using the `std::option::Option<T>` type
    /// and skipping serialization of `None` values. While imprecise, this is
    /// typical of Rust code.
    #[serde(default)]
    pub(crate) optional_nullable: OptionalNullable,
    // map_type: Option<()>,
    // set_type: Option<()>,
}

impl Settings {
    /// Set how types from the `std` prelude are spelled in generated
    /// code; see [`Std`]. The default is [`Std::FullyQualified`].
    pub fn with_std(mut self, std: Std) -> Self {
        self.std = std;
        self
    }

    /// Set how values that may be either `null` or absent are modeled;
    /// see [`OptionalNullable`]. The default is
    /// [`OptionalNullable::ConflateAsAbsent`].
    pub fn with_optional_nullable(mut self, optional_nullable: OptionalNullable) -> Self {
        self.optional_nullable = optional_nullable;
        self
    }
}

/// Specify how types in the `std` crate's prelude are spelled in
/// generated code. Types outside the prelude, such as
/// `std::collections::BTreeMap`, are always fully qualified.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Std {
    /// Fully qualify prelude types: `Option` renders as
    /// `::std::option::Option`. This is the default.
    #[default]
    FullyQualified,
    /// Render prelude types in their typical, auto-imported form. Useful
    /// if generated code is a starting point for manually-edited code.
    Unqualified,
}

/// Specify the modeling of values that may be either 'null' or 'optional'
/// (i.e. absent).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionalNullable {
    /// Model `null` and `optional` as equivalent by using the
    /// `std::option::Option<T>` type. Skip serialization of `None` values.
    /// This is the default.
    #[default]
    ConflateAsAbsent,

    /// Model `null` and `optional` as equivalent by using the
    /// `std::option::Option<T>` type. `None` values are serialized as `null`.
    ConflateAsNull,

    /// Use a "double `Option`" of the form
    /// `std::option::Option<std::option::Option<T>>`. A `None` indicates that
    /// the value is absent; `Some(None)` indicates that the value is present
    /// and null; and `Some(Some(_))` indicates that the value is present
    /// and non-null.
    DoubleOption,

    /// Use a custom type `Opt` where `Opt: std::default::Default +
    /// serde::Deserialize + serde::Serialize`. The `Default` implementation
    /// specifies the value for a field when absent; the `Deserialize`
    /// implementation produces a value otherwise (null or a non-null value of
    /// T). In addition, `Opt` must implement `is_absent(&self) -> bool` which
    /// is used with the serde `skip_serializing_if` attribute to omit the
    /// field.
    CustomType(String),
}

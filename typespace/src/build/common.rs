// Copyright 2026 Oxide Computer Company

use proc_macro2::{TokenStream, TokenTree};

use crate::error::Error;
use crate::TypespaceTraitSet;

/// A JSON value used as a default, with total (if vacuous) ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonValue(pub serde_json::Value);
impl JsonValue {
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl Ord for JsonValue {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}
impl PartialOrd for JsonValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The metadata shared by every named type.
///
/// While a shape is under construction the name may be unset; the
/// shape's `build()` (and, defensively, insertion into the builder)
/// rejects a shape whose name is missing or not a valid identifier.
#[derive(Debug, Clone, Default)]
pub struct TypeCommon {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) default: Option<JsonValue>,
    pub(crate) extra_derives: Vec<String>,
    pub(crate) extra_attrs: Vec<String>,
    pub(crate) built: Option<TypeCommonBuilt>,
}

impl TypeCommon {
    /// The type's name, if one has been set.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.default.as_ref().map(|JsonValue(value)| value)
    }

    /// Opaque derive paths for this type alone.
    ///
    /// These are additional to
    /// [`Settings::with_derive`](crate::settings::Settings::with_derive),
    /// whose paths apply to every type; a type's derive attribute names
    /// both sets.
    pub fn extra_derives(&self) -> &[String] {
        &self.extra_derives
    }

    /// Opaque attributes for this type alone.
    ///
    /// These are additional to
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr),
    /// whose attributes apply to every type.
    pub fn extra_attrs(&self) -> &[String] {
        &self.extra_attrs
    }

    /// The name of a validated shape.
    ///
    /// # Panics
    ///
    /// Panics if the name is unset; shapes reachable through a
    /// [`TypespaceBuilder`](crate::TypespaceBuilder) or a finalized
    /// typespace have passed validation, so a panic here is a typespace
    /// bug.
    pub(crate) fn built_name(&self) -> &str {
        self.name.as_deref().expect("validated type has a name")
    }

    /// Check that a nonempty, identifier-safe name is present.
    ///
    /// An unset name is [`Error::MissingTypeName`]; a set name goes
    /// through [`validate_ident`], where the empty string fails like
    /// any other garbage.
    pub(crate) fn validate_name<Id>(&self, kind: &'static str) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Some(name) = self.name.as_deref() else {
            return Err(Error::MissingTypeName { kind });
        };
        validate_ident(kind, name)
    }
}

/// Check that `name` is usable as a plain Rust identifier.
///
/// Rejects anything that does not lex as an identifier, keywords, raw
/// identifiers, and `gen`. One routine covers type names, property
/// names, and variant names; `kind` labels the error.
pub(crate) fn validate_ident<Id>(kind: &'static str, name: &str) -> Result<(), Error<Id>>
where
    Id: std::fmt::Debug + std::fmt::Display,
{
    // syn must accept `gen` (an identifier before edition 2024), but our
    // generated code may land in a 2024 crate, so we reject it ourselves.
    if name == "gen" {
        return Err(Error::InvalidName {
            kind,
            name: name.to_string(),
            message: "is a Rust keyword",
        });
    }
    if !name.starts_with("r#") && syn::parse_str::<syn::Ident>(name).is_ok() {
        return Ok(());
    }
    // Distinguish keywords from lexically invalid names for the error
    // message: a keyword lexes as a lone identifier even though syn's
    // `Ident` parser rejects it.
    let is_keyword = !name.starts_with("r#")
        && name.parse::<TokenStream>().is_ok_and(|ts| {
            let mut trees = ts.into_iter();
            matches!(
                (trees.next(), trees.next()),
                (Some(TokenTree::Ident(ident)), None) if ident == name
            )
        });
    if is_keyword {
        return Err(Error::InvalidName {
            kind,
            name: name.to_string(),
            message: "is a Rust keyword",
        });
    }
    Err(Error::InvalidName {
        kind,
        name: name.to_string(),
        message: "is not a valid Rust identifier (raw identifiers are not supported)",
    })
}

#[derive(Debug, Clone)]
pub(crate) struct TypeCommonBuilt {
    /// Computed set of traits required of this type.
    // TODO 3/25/2026
    // This definitely needs more consideration after I start feeling it out.
    pub traits: TypespaceTraitSet,

    /// Whether the type's `FromStr` returns `Ok` for every `&str`.
    ///
    /// True when the value is stored verbatim with no validation step
    /// between the `&str` and the constructed value, which makes the
    /// type useless as anything but the last arm of an untagged enum's
    /// first-match-wins `FromStr` chain. The property is syntactic: a
    /// constraint counts as validation even where it happens to accept
    /// every string.
    ///
    /// Only the recursive answers land here (an unconstrained newtype
    /// struct's and a type alias's). `Type::String` is answered by
    /// matching the variant, and every other kind of type is `false`
    /// outright, so neither wants a cache.
    ///
    /// Filled in by
    /// `resolve_from_string_irrefutable`, which runs after
    /// `break_cycles` and before trait resolution.
    pub from_string_irrefutable: bool,
}

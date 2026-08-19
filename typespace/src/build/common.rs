// Copyright 2026 Oxide Computer Company

use crate::{TypespaceError, TypespaceTraitSet};

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
/// Constructed only through the shape builders, which guarantee a
/// nonempty name.
#[derive(Debug, Clone)]
pub struct TypeCommon {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) default: Option<JsonValue>,
    pub(crate) built: Option<TypeCommonBuilt>,
}

impl TypeCommon {
    /// The type's name, always nonempty.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The default value, if any.
    pub fn default(&self) -> Option<&serde_json::Value> {
        self.default.as_ref().map(|JsonValue(value)| value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TypeCommonBuilt {
    // TODO 3/25/2026
    // This definitely needs more consideration after I start feeling it out.
    pub traits: TypespaceTraitSet,
}

/// Accumulates the name, description, and default shared by the shape
/// builders.
#[derive(Debug, Default, Clone)]
pub(crate) struct CommonBuilder {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) default: Option<JsonValue>,
}

impl CommonBuilder {
    /// Produce the common metadata for a `kind` of shape ("struct",
    /// "enum", ...), failing with
    /// [`TypespaceError::MissingTypeName`] unless a nonempty name was
    /// provided.
    pub(crate) fn build<Id>(self, kind: &'static str) -> Result<TypeCommon, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self {
            name,
            description,
            default,
        } = self;
        let Some(name) = name.filter(|name| !name.is_empty()) else {
            return Err(TypespaceError::MissingTypeName { kind });
        };
        Ok(TypeCommon {
            name,
            description,
            default,
            built: None,
        })
    }
}

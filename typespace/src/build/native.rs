// Copyright 2026 Oxide Computer Company

use crate::{TypespaceTrait, TypespaceTraitSet};

/// An externally defined type emitted by its Rust path; construct one
/// with [`Native::new`] or [`Native::new_string_like`].
#[derive(Debug, Clone)]
pub struct Native<Id> {
    pub(crate) name: String,

    pub(crate) impls: TypespaceTraitSet,

    // TODO from typify 1: in order to support const generics, this could be a
    // TypeOrConst enum, but note that we may some day need to disambiguate
    // char and &'static str since schemars represents a char as a string of
    // length 1.
    pub(crate) parameters: Vec<Id>,

    /// Whether the type's `FromStr` accepts every input string.
    ///
    /// This is an analysis fact tracked for internal use (deciding, for
    /// example, whether an untagged enum with a string variant can
    /// implement `FromStr` without a failure case), not a requestable
    /// trait; it is fed by [`Native::new_string_like`] and is not part
    /// of the public trait vocabulary.
    #[allow(dead_code)]
    pub(crate) from_string_irrefutable: bool,
}

impl<Id> Native<Id> {
    /// The Rust type path emitted verbatim into generated code.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The traits the type is known to implement.
    pub fn impls(&self) -> &TypespaceTraitSet {
        &self.impls
    }

    /// The IDs of the type's generic type parameters.
    pub fn parameters(&self) -> &[Id] {
        &self.parameters
    }
    /// Create a native type. `name` is the Rust type path emitted
    /// verbatim into generated code; `impls` is the set of traits the
    /// type is known to implement, consulted when trait requirements
    /// propagate to it during finalization; `parameters` are the IDs of
    /// its generic type parameters, if any.
    pub fn new(name: impl ToString, impls: TypespaceTraitSet, parameters: Vec<Id>) -> Self {
        Self {
            name: name.to_string(),
            impls,
            parameters,
            from_string_irrefutable: false,
        }
    }

    /// Create a native type that behaves like a string: it takes no type
    /// parameters and implements the full complement of traits that
    /// `String` does, including `Display` and `FromStr`.
    pub fn new_string_like(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            impls: [
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
                TypespaceTrait::JsonSchema,
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Hash,
                TypespaceTrait::Display,
                TypespaceTrait::FromStr,
            ]
            .into_iter()
            .collect(),
            parameters: Default::default(),
            from_string_irrefutable: true,
        }
    }
}

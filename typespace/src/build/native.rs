// Copyright 2026 Oxide Computer Company

use crate::{TraitDisposition, TypespaceTrait, TypespaceTraitSet, ALL_TRAITS};

/// An externally defined type emitted by its Rust path; construct one
/// with [`Native::new`] or [`Native::new_string_like`].
#[derive(Debug, Clone)]
pub struct Native<Id> {
    pub(crate) name: String,

    /// The traits the type is known to implement.
    pub(crate) impls: TypespaceTraitSet,

    /// The traits the declaration cannot answer for.
    ///
    /// Disjoint from `impls`, which the constructors maintain. A trait
    /// in neither set is one the type is known not to implement.
    pub(crate) unknown: TypespaceTraitSet,

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

    /// The traits the declaration cannot answer for.
    pub fn unknown(&self) -> &TypespaceTraitSet {
        &self.unknown
    }

    /// What the declaration says about `trait_`.
    pub fn disposition(&self, trait_: TypespaceTrait) -> TraitDisposition {
        match (self.impls.contains(&trait_), self.unknown.contains(&trait_)) {
            (true, _) => TraitDisposition::Yes,
            (false, true) => TraitDisposition::Unknown,
            (false, false) => TraitDisposition::No,
        }
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
    ///
    /// Every trait outside `impls` is one the type is known not to
    /// implement. A declarer that cannot answer for a trait says so
    /// with [`with_unknown`](Self::with_unknown) or
    /// [`with_rest_unknown`](Self::with_rest_unknown).
    pub fn new(name: impl ToString, impls: TypespaceTraitSet, parameters: Vec<Id>) -> Self {
        Self {
            name: name.to_string(),
            impls,
            unknown: TypespaceTraitSet::empty(),
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
            unknown: TypespaceTraitSet::empty(),
            parameters: Default::default(),
            from_string_irrefutable: true,
        }
    }

    /// Mark each of `traits` as one the declaration cannot answer for.
    pub fn with_unknown(self, traits: TypespaceTraitSet) -> Self {
        traits.into_iter().fold(self, |native, trait_| {
            native.with_disposition(trait_, TraitDisposition::Unknown)
        })
    }

    /// Mark every trait the type does not declare as one the
    /// declaration cannot answer for.
    ///
    /// This is the shape a source like typify's `x-rust-type` schema
    /// extension has: it names a Rust type and the little it knows
    /// about it, and has no way to state anything further.
    pub fn with_rest_unknown(self) -> Self {
        let rest = ALL_TRAITS
            .into_iter()
            .filter(|trait_| !self.impls.contains(trait_))
            .collect::<TypespaceTraitSet>();
        self.with_unknown(rest)
    }

    /// Set what the declaration says about one trait, replacing
    /// whatever it said before.
    pub fn with_disposition(
        mut self,
        trait_: TypespaceTrait,
        disposition: TraitDisposition,
    ) -> Self {
        self.impls.remove(trait_);
        self.unknown.remove(trait_);
        match disposition {
            TraitDisposition::Yes => self.impls.add(trait_),
            TraitDisposition::No => {}
            TraitDisposition::Unknown => self.unknown.add(trait_),
        }
        self
    }
}

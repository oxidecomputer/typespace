// Copyright 2026 Oxide Computer Company

use crate::error::Error;
use crate::settings::ContainerType;
use crate::{ALL_TRAITS, TraitProvision, TypespaceTrait, TypespaceTraitSet};

/// An externally defined type emitted by its Rust path; construct one
/// with [`Native::new`] or [`Native::new_string_like`].
///
/// A native is, structurally, exactly the container-side vocabulary
/// [`ContainerType`] already states: a path emitted verbatim, what it
/// demands of its type parameters (its obligations, empty by default),
/// and what it implements for every trait typespace tracks (its
/// provisions). A native composes one rather than growing a parallel
/// set of fields, so the two only differ in what they ARE--a
/// consumer-supplied type referenced verbatim versus a container
/// [`Settings`](crate::settings::Settings) is configured to render
/// as--not in how either is described.
///
/// The one place a native's vocabulary is strictly larger is
/// [`TraitProvision::Unknown`]: a container is hand-authored, so its
/// declaration is expected to answer for every trait, but a native
/// sometimes comes from a machine source--typify's `x-rust-type` schema
/// extension, say--that names a Rust type and the little it knows about
/// it, with no way to state more. [`with_unknown`](Self::with_unknown)
/// and [`with_rest_unknown`](Self::with_rest_unknown) mark those traits;
/// finalization rejects a configured container that does the same (see
/// `check_containers`).
#[derive(Debug, Clone)]
pub struct Native<Id> {
    // Boxed so that `Type::Native`, which holds a `Native` inline,
    // stays close in size to `Type`'s other variants: `ContainerType`
    // carries two `syn::Type` fields, and `syn::Type` alone is large
    // enough that clippy's `large_enum_variant` flags `Type` without
    // this indirection.
    pub(crate) container: Box<ContainerType>,

    // TODO from typify 1: in order to support const generics, this could be a
    // TypeOrConst enum, but note that we may some day need to disambiguate
    // char and &'static str since schemars represents a char as a string of
    // length 1.
    pub(crate) parameters: Vec<Id>,
}

impl<Id> Native<Id> {
    /// The Rust type path emitted verbatim into generated code.
    pub fn path(&self) -> &syn::Type {
        self.container.path()
    }

    /// The IDs of the type's generic type parameters.
    pub fn parameters(&self) -> &[Id] {
        &self.parameters
    }

    /// What the native demands of each of its type parameters, in
    /// parameter order, independent of any trait a caller requires or
    /// desires of the native itself.
    ///
    /// Empty by default--the common case of a native with no such
    /// requirement--and set with
    /// [`with_obligations`](Self::with_obligations). Finalization
    /// rejects a native whose obligation count does not match its
    /// parameter count.
    pub fn obligations(&self) -> &[TypespaceTraitSet] {
        self.container.obligations()
    }

    /// What the declaration says about `trait_`.
    pub fn disposition(&self, trait_: TypespaceTrait) -> TraitProvision {
        self.container.provision(trait_)
    }

    /// What the declaration says about every trait typespace tracks.
    pub fn dispositions(&self) -> impl Iterator<Item = (TypespaceTrait, TraitProvision)> + '_ {
        self.container.provisions()
    }

    /// Create a native type. `path` is the Rust type path emitted verbatim
    /// into generated code (it should typically start with `::` to avoid
    /// conflicts); `impls` is the set of traits the type is known to implement
    /// unconditionally, consulted when trait requirements propagate to it
    /// during finalization; `parameters` are the IDs of its generic type
    /// parameters, if any.
    ///
    /// Every trait outside `impls` is one the type is known not to
    /// implement, which [`with_disposition`](Self::with_disposition) can be
    /// used to express explicitly. A declarer that cannot answer for a trait
    /// says so with [`with_unknown`](Self::with_unknown) or
    /// [`with_rest_unknown`](Self::with_rest_unknown); one that implements a
    /// trait only when a type parameter does says so with
    /// [`with_disposition`](Self::with_disposition) and
    /// [`TraitProvision::IfParameters`].
    ///
    /// # Panics
    ///
    /// Panics if `path` cannot be parsed as a Rust type.
    pub fn new(path: &str, impls: TypespaceTraitSet, parameters: Vec<Id>) -> Self {
        let always = impls.iter().copied().collect::<Vec<_>>();
        let never = ALL_TRAITS
            .into_iter()
            .filter(|trait_| !impls.contains(trait_))
            .collect::<Vec<_>>();
        let obligations = vec![TypespaceTraitSet::empty(); parameters.len()];
        Self {
            container: Box::new(
                ContainerType::new(path, obligations).with_provisions(&always, &never),
            ),
            parameters,
        }
    }

    /// Create a native type that behaves like a string: it takes no type
    /// parameters and implements the full complement of traits that
    /// `String` does, including `Display` and `FromStr`.
    pub fn new_string_like(path: &str) -> Self {
        Self::new(
            path,
            [
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
            Vec::new(),
        )
    }

    /// Mark each of `traits` as one the declaration cannot answer for.
    pub fn with_unknown(self, traits: TypespaceTraitSet) -> Self {
        traits.into_iter().fold(self, |native, trait_| {
            native.with_disposition(trait_, TraitProvision::Unknown)
        })
    }

    /// Mark every trait the type does not positively declare--neither
    /// unconditionally nor conditioned on a type parameter--as one the
    /// declaration cannot answer for.
    ///
    /// This is the shape a source like typify's `x-rust-type` schema
    /// extension has: it names a Rust type and the little it knows
    /// about it, and has no way to state anything further.
    pub fn with_rest_unknown(self) -> Self {
        let rest = ALL_TRAITS
            .into_iter()
            .filter(|trait_| {
                matches!(
                    self.disposition(*trait_),
                    TraitProvision::Never | TraitProvision::Unknown
                )
            })
            .collect::<TypespaceTraitSet>();
        self.with_unknown(rest)
    }

    /// Set what the declaration says about one trait, replacing
    /// whatever it said before.
    pub fn with_disposition(mut self, trait_: TypespaceTrait, disposition: TraitProvision) -> Self {
        self.container = Box::new(self.container.with_provision(trait_, disposition));
        self
    }

    /// Set what the native demands of each of its type parameters,
    /// replacing the default (empty for every parameter). There is one
    /// entry per type parameter the native takes; finalization rejects
    /// a mismatch.
    pub fn with_obligations(
        mut self,
        obligations: impl IntoIterator<Item = TypespaceTraitSet>,
    ) -> Self {
        self.container = Box::new(self.container.with_obligations(obligations));
        self
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    ///
    /// A native's obligation count must match its parameter count,
    /// exactly as `check_containers` verifies for a configured
    /// container's obligations against the parameter count its
    /// position renders.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let declared = self.container.obligations().len();
        let parameters = self.parameters.len();
        if declared != parameters {
            return Err(Error::NativeParameterCount {
                path: crate::settings::path_text(self.container.path()),
                declared,
                parameters,
            });
        }
        Ok(())
    }
}

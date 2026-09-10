// Copyright 2026 Oxide Computer Company

//! Settings that govern how types are processed and rendered.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{ALL_TRAITS, TypespaceTrait, TypespaceTraitSet};

// TODO 7/18/2025
// I wanted to get this started to think through various settings that we might
// eventually want...
/// Modify how types are processed and generated.
///
/// Settings are supplied to
/// [`TypespaceBuilder::new`](crate::TypespaceBuilder::new) and govern
/// finalization and rendering. Start from [`Settings::default`] and
/// adjust with the `with_` methods; the type also implements
/// `Deserialize` so settings can come from configuration data.
#[derive(Debug, Deserialize)]
pub struct Settings {
    /// How types in the `std` prelude are rendered; see [`Std`].
    #[serde(default)]
    pub(crate) std: Std,

    /// How values that may be `null` or absent are represented; see
    /// [`OptionalNullable`]. The default is `ConflateAsAbsent`.
    #[serde(default)]
    pub(crate) optional_nullable: OptionalNullable,

    /// The container [`Type::Map`](crate::build::Type) renders as: its
    /// path, what it demands of its key and value types, and what it
    /// implements.
    #[serde(default = "Settings::default_map_type")]
    pub(crate) map_type: ContainerType,

    /// The container [`Type::Set`](crate::build::Type) renders as: its
    /// path, what it demands of its element type, and what it
    /// implements.
    #[serde(default = "Settings::default_set_type")]
    pub(crate) set_type: ContainerType,

    /// The container [`Type::Vec`](crate::build::Type) renders as: its
    /// path, what it demands of its element type, and what it
    /// implements.
    #[serde(default = "Settings::default_vec_type")]
    pub(crate) vec_type: ContainerType,

    /// Traits every named type is required to implement.
    #[serde(default)]
    pub(crate) required_traits: TypespaceTraitSet,

    /// Traits desired for every type that's able to implement each.
    #[serde(default)]
    pub(crate) desired_traits: TypespaceTraitSet,

    /// Opaque derive paths included in every derive attribute.
    #[serde(default)]
    pub(crate) extra_derives: Vec<String>,

    /// Opaque attributes included in every type.
    #[serde(default)]
    pub(crate) extra_attrs: Vec<String>,

    /// Generate builder for struct types.
    #[serde(default)]
    pub(crate) struct_builder: bool,

    // TYPIFY COMPAT ANCHOR. This setting exists so typespace can
    // imitate typify's renderer while typify moves onto it, and it goes
    // away when that finishes. Every site whose behavior changes under
    // it carries a TYPIFY COMPAT marker; grep the marker to find all of
    // them, and removing the setting means removing every one.
    #[doc(hidden)]
    #[serde(default)]
    pub(crate) typify_compat: bool,
}

/// The traits an ordered-lookup container (`BTreeMap`, or the ordered
/// treatment of sets) demands of its key or element type. This is the
/// default requirement set for maps and sets; container overrides
/// supply their own.
fn ordered_lookup_traits() -> TypespaceTraitSet {
    [
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Ord,
        TypespaceTrait::PartialOrd,
    ]
    .into_iter()
    .collect()
}

/// The traits a hash-lookup container (`HashMap`, `HashSet`) demands of
/// its key or element type.
fn hash_lookup_traits() -> TypespaceTraitSet {
    [
        TypespaceTrait::Eq,
        TypespaceTrait::PartialEq,
        TypespaceTrait::Hash,
    ]
    .into_iter()
    .collect()
}

/// Traits no owning std container (`BTreeMap`, `BTreeSet`, `Vec`)
/// implements at any parameter: `Display` and `FromStr`, which none of
/// them render, and `Copy`, since each owns a heap-allocated buffer.
const OWNING_NEVER: [TypespaceTrait; 3] = [
    TypespaceTrait::Display,
    TypespaceTrait::FromStr,
    TypespaceTrait::Copy,
];

/// `OWNING_NEVER` plus the ordering traits: what a hash std container
/// (`HashMap`, `HashSet`) has no impl for at any parameter.
const HASHING_NEVER: [TypespaceTrait; 6] = [
    TypespaceTrait::Display,
    TypespaceTrait::FromStr,
    TypespaceTrait::Copy,
    TypespaceTrait::Ord,
    TypespaceTrait::PartialOrd,
    TypespaceTrait::Hash,
];

impl Settings {
    /// The container a map renders as absent configuration.
    fn default_map_type() -> ContainerType {
        ContainerType::btree_map()
    }

    /// The container a set renders as absent configuration: a `Vec`,
    /// which demands nothing of its elements, carrying the
    /// ordered-lookup obligation that set deduplication policy imposes.
    fn default_set_type() -> ContainerType {
        ContainerType::vec().with_obligations([ordered_lookup_traits()])
    }

    /// The container a vec renders as absent configuration.
    fn default_vec_type() -> ContainerType {
        ContainerType::vec()
    }

    pub fn minimal() -> Self {
        Self {
            std: Std::FullyQualified,
            optional_nullable: OptionalNullable::default(),
            map_type: Self::default_map_type(),
            set_type: Self::default_set_type(),
            vec_type: Self::default_vec_type(),
            required_traits: TypespaceTraitSet::empty(),
            desired_traits: TypespaceTraitSet::empty(),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
            struct_builder: false,
            typify_compat: false,
        }
    }

    pub fn typical() -> Self {
        Self {
            required_traits: [
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
            ]
            .into_iter()
            .collect(),
            struct_builder: true,
            ..Self::minimal()
        }
    }

    pub fn maximal() -> Self {
        Self {
            required_traits: [
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
                TypespaceTrait::JsonSchema,
            ]
            .into_iter()
            .collect(),
            desired_traits: [
                TypespaceTrait::Display,
                TypespaceTrait::FromStr,
                TypespaceTrait::Eq,
                TypespaceTrait::PartialEq,
                TypespaceTrait::Ord,
                TypespaceTrait::PartialOrd,
                TypespaceTrait::Hash,
                TypespaceTrait::Default,
                TypespaceTrait::Copy,
            ]
            .into_iter()
            .collect(),
            ..Self::typical()
        }
    }

    /// Set the [`Std`] syntax used to render types from the `std`
    /// prelude. The default is [`Std::FullyQualified`].
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

    /// The modeling of values that may be absent, null, or a value.
    pub fn optional_nullable(&self) -> &OptionalNullable {
        &self.optional_nullable
    }

    /// Set the container type used to render
    /// [`Type::Map`](crate::build::Type).
    ///
    /// The default is [`ContainerType::btree_map`]. Finalization
    /// imposes the container's key obligation on every map key.
    ///
    /// The path is emitted verbatim with the key and value types as its
    /// generic arguments, so it must take two parameters and have an
    /// `is_empty` method returning a boolean.
    ///
    /// A string-keyed map of [`Type::JsonValue`](crate::build::Type)
    /// renders as `::serde_json::Map` whatever this is set to.
    ///
    /// ```
    /// # use typespace::settings::{ContainerType, Settings};
    /// let settings = Settings::minimal()
    ///     .with_map_type(ContainerType::hash_map());
    /// ```
    pub fn with_map_type(mut self, map_type: ContainerType) -> Self {
        self.map_type = map_type;
        self
    }

    /// The container a map renders as, and what it demands and
    /// implements.
    pub fn map_type(&self) -> &ContainerType {
        &self.map_type
    }

    /// Set the container type used to render
    /// [`Type::Set`](crate::build::Type).
    ///
    /// The default is a `Vec`, which does not enforce deduplication
    /// but still demands the `Ord` family of its elements.
    /// Finalization imposes the container's element obligation on every
    /// set element.
    ///
    /// The path is emitted verbatim with the element type as its single
    /// generic argument, so it must take one parameter and have an
    /// `is_empty` method returning a boolean.
    ///
    /// ```
    /// # use typespace::settings::{ContainerType, Settings};
    /// let settings = Settings::minimal()
    ///     .with_set_type(ContainerType::hash_set());
    /// ```
    pub fn with_set_type(mut self, set_type: ContainerType) -> Self {
        self.set_type = set_type;
        self
    }

    /// The container a set renders as, and what it demands and
    /// implements.
    pub fn set_type(&self) -> &ContainerType {
        &self.set_type
    }

    /// Set the container type used to render
    /// [`Type::Vec`](crate::build::Type).
    ///
    /// The default is [`ContainerType::vec`], which demands nothing of
    /// its elements.
    ///
    /// The path is emitted verbatim with the element type as its single
    /// generic argument, so it must take one parameter and have an
    /// `is_empty` method returning a boolean.
    pub fn with_vec_type(mut self, vec_type: ContainerType) -> Self {
        self.vec_type = vec_type;
        self
    }

    /// The container a vec renders as, and what it demands and
    /// implements.
    pub fn vec_type(&self) -> &ContainerType {
        &self.vec_type
    }

    /// Require every generated type to implement the given trait.
    ///
    /// Traits may be derived or implemented with custom code generated by
    /// `typespace`. If any type cannot implement the trait (e.g. `Eq` cannot
    /// be applied to a type that contains an `f64`), finalization fails with a
    /// [`Error`](crate::error::Error).
    ///
    /// See [`Settings::with_desired_trait`].
    pub fn with_required_trait(mut self, trait_impl: TypespaceTrait) -> Self {
        self.required_traits.add(trait_impl);
        self
    }

    /// Request every generated type implement the given trait if it's able to.
    ///
    /// Traits may be derived or implemented with custom code generated by
    /// `typespace`. If any type cannot implement the trait (e.g. `Eq` cannot
    /// be applied to a type that contains an `f64`), it is simply omitted for
    /// that type.
    ///
    /// See [`Settings::with_required_trait`].
    pub fn with_desired_trait(mut self, trait_impl: TypespaceTrait) -> Self {
        self.desired_traits.add(trait_impl);
        self
    }

    /// Add a derived trait to every named type.
    ///
    /// The path is emitted verbatim. Callers should take care not to specify
    /// traits present in [`TypespaceTrait`], as those may require special
    /// handling. Callers should also take care not to specify two aliases of
    /// the same trait. Traits that cannot be derived for a type may cause
    /// generated code to fail to compile; `typespace` has no way to validate
    /// the validity or applicability of a derive path.
    pub fn with_derive(mut self, derive: impl Into<String>) -> Self {
        self.extra_derives.push(derive.into());
        self
    }

    /// Specify an attribute that will precede every generated type.
    pub fn with_attr(mut self, attr: impl Into<String>) -> Self {
        self.extra_attrs.push(attr.into());
        self
    }

    /// Specify whether struct types should include an associated builder.
    pub fn with_struct_builder(mut self, struct_builder: bool) -> Self {
        self.struct_builder = struct_builder;
        self
    }

    // TYPIFY COMPAT
    #[doc(hidden)]
    pub fn with_typify_compat(mut self, typify_compat: bool) -> Self {
        self.typify_compat = typify_compat;
        self
    }
}

/// A container type, what it demands of its parameters, and what it
/// implements.
///
/// The path is emitted verbatim, with the contained types as its generic
/// arguments. The obligations are what the container demands of those
/// parameters--irrespective of what traits the container itself is expected to
/// implement: `K: Eq + Hash` for a `HashMap`, `K: Ord` for `BTreeMap`, nothing
/// of a value or an element type (for those types). The [`TraitProvision`]
/// entries say, for each trait typespace tracks, whether the container never
/// implements it, implements it unconditionally, or implements it only when
/// every parameter does.
///
/// The number of obligations must match the number of type parameters the
/// type expects (two for a map, one for a set or a vec). Finalization produces
/// an error if that's not the case.
///
/// Finalization imposes an obligations on type parameters according to these
/// settings (and this may cascade to dependent traits i.e. `Ord` implies
/// `PartialOrd`; `Eq`, and `PartialEq`).
///
/// There are presets for common modalities:
///
/// ```
/// # use typespace::{
/// #     settings::{ContainerType, TraitProvision},
/// #     TypespaceTrait, TypespaceTraitSet,
/// # };
/// // An ordered map, at another path, that clones its keys and
/// // values, and that has no `Hash` impl of its own.
/// let container = ContainerType::btree_map()
///     .with_path("::im::OrdMap")
///     .with_obligations([
///         [TypespaceTrait::Ord, TypespaceTrait::Clone]
///             .into_iter()
///             .collect::<TypespaceTraitSet>(),
///         [TypespaceTrait::Clone].into_iter().collect(),
///     ])
///     .with_provision(TypespaceTrait::Hash, TraitProvision::Never);
/// ```
///
/// A container with no matching preset can state its whole table in
/// one expression with [`ContainerType::new`] and
/// [`with_provisions`](Self::with_provisions), rather than building it
/// up one [`with_provision`](Self::with_provision) call at a time.
#[derive(Clone, Deserialize)]
#[serde(try_from = "ContainerTypeRepr")]
pub struct ContainerType {
    /// The path the container renders as.
    path: syn::Type,

    /// The prelude shortcut, rendered when [`Std::Unqualified`] is configured;
    /// since `Vec` is the only container typespace names that has one, and
    /// it's the default, this can't be set by consumers.
    prelude_path: Option<syn::Type>,

    /// What the container demands of each of its type parameters, in
    /// parameter order.
    obligations: Vec<TypespaceTraitSet>,

    /// What the container implements for each trait typespace tracks.
    provisions: ProvisionTable,
}

impl ContainerType {
    /// A container at `path` demanding `obligations` of its type
    /// parameters, whose behavior is otherwise unknown beyond what
    /// rendering assumes: `Default` unconditional, `Clone`, `Debug`,
    /// `Serialize`, and `Deserialize` following the parameters, and no
    /// other trait implemented.
    ///
    /// This is the declaration for a container that arrives as a bare
    /// path with nothing said about it. Every trait it does in fact
    /// implement is added with
    /// [`with_provision`](Self::with_provision); until then a
    /// requirement for one is a conflict rather than generated code
    /// that does not compile. When more is known about the container
    /// up front, [`with_provisions`](Self::with_provisions) states the
    /// whole table as exceptions to forwarding.
    ///
    /// # Panics
    ///
    /// Panics if `path` cannot be parsed as a Rust type.
    pub fn new(path: &str, obligations: impl IntoIterator<Item = TypespaceTraitSet>) -> Self {
        Self::opaque(parse_path(path), obligations.into_iter().collect())
    }

    /// [`ContainerType::new`] with the path already parsed.
    fn opaque(path: syn::Type, obligations: Vec<TypespaceTraitSet>) -> Self {
        Self::from_parts(path, obligations, ProvisionTable::opaque())
    }

    /// Assemble a container from its full parts, with no prelude-path
    /// shortcut; [`with_prelude`](Self::with_prelude) adds one
    /// afterward for the containers that have one.
    fn from_parts(
        path: syn::Type,
        obligations: Vec<TypespaceTraitSet>,
        provisions: ProvisionTable,
    ) -> Self {
        Self {
            path,
            prelude_path: None,
            obligations,
            provisions,
        }
    }

    /// Set the prelude-path shortcut, rendered under [`Std::Unqualified`].
    fn with_prelude(mut self, prelude_path: syn::Type) -> Self {
        self.prelude_path = Some(prelude_path);
        self
    }

    /// `::std::collections::BTreeMap` and containers that behave as it
    /// does: keys must be ordered, and every trait but `Display` and
    /// `FromStr` follows the parameters, with `Default` unconditional.
    pub fn btree_map() -> Self {
        Self::new(
            "::std::collections::BTreeMap",
            [ordered_lookup_traits(), TypespaceTraitSet::empty()],
        )
        .with_provisions(&[TypespaceTrait::Default], &OWNING_NEVER)
    }

    /// `::std::collections::HashMap` and containers that behave as it
    /// does: keys must be hashable, and the container has no `Ord`,
    /// `PartialOrd`, or `Hash` impl at any key or value type.
    pub fn hash_map() -> Self {
        Self::new(
            "::std::collections::HashMap",
            [hash_lookup_traits(), TypespaceTraitSet::empty()],
        )
        .with_provisions(&[TypespaceTrait::Default], &HASHING_NEVER)
    }

    /// `Vec` and containers that behave as it does: nothing is demanded
    /// of the element type, and every trait but `Display` and `FromStr`
    /// follows the element, with `Default` unconditional.
    ///
    /// This is the container the set position renders by default as
    /// well; the ordered-lookup obligation the default set carries is
    /// deduplication policy, not the container's need, and is stated
    /// with [`with_obligations`](Self::with_obligations).
    pub fn vec() -> Self {
        Self::new("::std::vec::Vec", [TypespaceTraitSet::empty()])
            .with_provisions(&[TypespaceTrait::Default], &OWNING_NEVER)
            .with_prelude(parse_path("Vec"))
    }

    /// `Option` and similar wrappers: nothing is demanded of the parameter,
    /// `Default` is unconditional, and every trait but `Display` and `FromStr`
    /// follows the parameter, `Copy` included.
    ///
    /// This is the intended base for an
    /// [`OptionalNullable::CustomType`]
    /// declaration: point it at the wrapper with
    /// [`with_path`](Self::with_path), then narrow any provision the
    /// wrapper lacks, or state the wrapper's own table with
    /// [`ContainerType::new`] and
    /// [`with_provisions`](Self::with_provisions) when several
    /// provisions need narrowing at once.
    pub fn option() -> Self {
        Self::new("::std::option::Option", [TypespaceTraitSet::empty()])
            .with_provisions(
                &[TypespaceTrait::Default],
                &[TypespaceTrait::Display, TypespaceTrait::FromStr],
            )
            .with_prelude(parse_path("Option"))
    }

    /// `::std::collections::BTreeSet` and containers that behave as it
    /// does: elements must be ordered, and the provisions are `Vec`'s.
    pub fn btree_set() -> Self {
        Self::new("::std::collections::BTreeSet", [ordered_lookup_traits()])
            .with_provisions(&[TypespaceTrait::Default], &OWNING_NEVER)
    }

    /// `::std::collections::HashSet` and containers that behave as it
    /// does: elements must be hashable, and the container has no `Ord`,
    /// `PartialOrd`, or `Hash` impl at any element type.
    pub fn hash_set() -> Self {
        Self::new("::std::collections::HashSet", [hash_lookup_traits()])
            .with_provisions(&[TypespaceTrait::Default], &HASHING_NEVER)
    }

    /// The path the container renders as.
    pub fn path(&self) -> &syn::Type {
        &self.path
    }

    /// What the container demands of each of its type parameters, in
    /// parameter order.
    pub fn obligations(&self) -> &[TypespaceTraitSet] {
        &self.obligations
    }

    /// What the container demands of the type parameter in position
    /// `index`.
    ///
    /// # Panics
    ///
    /// Panics if the container has no such parameter; finalization
    /// checks a declaration's arity against the position it is
    /// configured for.
    pub fn obligation(&self, index: usize) -> &TypespaceTraitSet {
        &self.obligations[index]
    }

    /// What the container implements for `trait_`.
    pub fn provision(&self, trait_: TypespaceTrait) -> TraitProvision {
        self.provisions.get(trait_)
    }

    /// What the container implements for each trait typespace tracks.
    pub fn provisions(&self) -> impl Iterator<Item = (TypespaceTrait, TraitProvision)> + '_ {
        self.provisions.iter()
    }

    /// Set the path the container renders as, keeping what it demands
    /// and implements.
    ///
    /// # Panics
    ///
    /// Panics if `path` cannot be parsed as a Rust type.
    pub fn with_path(self, path: &str) -> Self {
        self.with_parsed_path(parse_path(path))
    }

    /// [`ContainerType::with_path`] with the path already parsed.
    fn with_parsed_path(mut self, path: syn::Type) -> Self {
        self.path = path;
        // A configured path is emitted verbatim; only the containers
        // typespace names for itself have a prelude path.
        self.prelude_path = None;
        self
    }

    /// Set what the container demands of each of its type parameters,
    /// replacing the preset's obligations.
    ///
    /// The number of entries is the container's arity.
    pub fn with_obligations(
        mut self,
        obligations: impl IntoIterator<Item = TypespaceTraitSet>,
    ) -> Self {
        self.obligations = obligations.into_iter().collect();
        self
    }

    /// Set what the container implements for one trait, replacing the
    /// preset's answer.
    pub fn with_provision(mut self, trait_: TypespaceTrait, provision: TraitProvision) -> Self {
        self.provisions.set(trait_, provision);
        self
    }

    /// Replace what the container implements, stated as exceptions:
    /// traits in `always` have an impl whatever the parameters are,
    /// traits in `never` have no impl at any parameter, and every
    /// other trait typespace tracks follows the parameters.
    ///
    /// This replaces the whole table, so narrow individual traits with
    /// [`with_provision`](Self::with_provision) after it, not before.
    ///
    /// ```
    /// # use typespace::{
    /// #     settings::ContainerType,
    /// #     TypespaceTrait, TypespaceTraitSet,
    /// # };
    /// // A custom optional/nullable wrapper: `Default` always,
    /// // `Display` and `FromStr` never, every other trait follows
    /// // the parameter.
    /// let wrapper = ContainerType::new("::my::Opt", [TypespaceTraitSet::empty()])
    ///     .with_provisions(
    ///         &[TypespaceTrait::Default],
    ///         &[TypespaceTrait::Display, TypespaceTrait::FromStr],
    ///     );
    /// ```
    pub fn with_provisions(mut self, always: &[TypespaceTrait], never: &[TypespaceTrait]) -> Self {
        self.provisions = ProvisionTable::new(never, always);
        self
    }

    /// Merge per-trait overrides into the table, for deserialization.
    fn with_provision_overrides(
        mut self,
        provides: BTreeMap<TypespaceTrait, TraitProvision>,
    ) -> Self {
        self.provisions = self.provisions.overridden(provides);
        self
    }

    /// The path to render under the configured [`Std`] syntax.
    pub(crate) fn rendered_path(&self, std: &Std) -> &syn::Type {
        match (std, &self.prelude_path) {
            (Std::Unqualified, Some(path)) => path,
            _ => &self.path,
        }
    }
}

/// Parse a container path, panicking on invalid input.
fn parse_path(path: &str) -> syn::Type {
    syn::parse_str::<syn::Type>(path).expect("valid type path")
}

/// The token text of a path.
///
/// `syn::Type` implements neither `PartialEq` nor `Debug` without syn's
/// `extra-traits` feature; equality and debug output go through the
/// tokens instead.
pub(crate) fn path_text(path: &syn::Type) -> String {
    use quote::ToTokens;
    path.to_token_stream().to_string()
}

impl PartialEq for ContainerType {
    fn eq(&self, other: &Self) -> bool {
        path_text(&self.path) == path_text(&other.path)
            && self.prelude_path.as_ref().map(path_text)
                == other.prelude_path.as_ref().map(path_text)
            && self.obligations == other.obligations
            && self.provisions == other.provisions
    }
}

impl Eq for ContainerType {}

impl std::fmt::Debug for ContainerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerType")
            .field("path", &path_text(&self.path))
            .field("prelude_path", &self.prelude_path.as_ref().map(path_text))
            .field("obligations", &self.obligations)
            .field("provisions", &self.provisions)
            .finish()
    }
}

/// The deserialized form of [`ContainerType`].
///
/// A declaration names the preset it behaves as, in which case every
/// other key adjusts that preset, or states its own `path` and
/// `obligations` and claims only what rendering assumes.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ContainerTypeRepr {
    /// The preset the container starts from.
    #[serde(default)]
    like: Option<ContainerFamily>,
    /// The path the container renders as, replacing the preset's.
    #[serde(default)]
    path: Option<String>,
    /// What the container demands of each parameter, replacing the
    /// preset's.
    #[serde(default)]
    obligations: Option<Vec<TypespaceTraitSet>>,
    /// Per-trait answers replacing the preset's.
    #[serde(default)]
    provides: BTreeMap<TypespaceTrait, TraitProvision>,
}

/// The [`ContainerType`] presets, by name.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ContainerFamily {
    BtreeMap,
    HashMap,
    Vec,
    Option,
    BtreeSet,
    HashSet,
}

impl ContainerFamily {
    /// The declaration this name selects.
    fn preset(&self) -> ContainerType {
        match self {
            Self::BtreeMap => ContainerType::btree_map(),
            Self::HashMap => ContainerType::hash_map(),
            Self::Vec => ContainerType::vec(),
            Self::Option => ContainerType::option(),
            Self::BtreeSet => ContainerType::btree_set(),
            Self::HashSet => ContainerType::hash_set(),
        }
    }
}

impl TryFrom<ContainerTypeRepr> for ContainerType {
    type Error = String;

    fn try_from(repr: ContainerTypeRepr) -> Result<Self, Self::Error> {
        let ContainerTypeRepr {
            like,
            path,
            obligations,
            provides,
        } = repr;

        let path = path
            .map(|path| {
                syn::parse_str::<syn::Type>(&path)
                    .map_err(|err| format!("invalid container type {path:?}: {err}"))
            })
            .transpose()?;

        let declared = match (like, path, obligations) {
            (Some(chosen), path, obligations) => {
                let with_path = match path {
                    Some(path) => chosen.preset().with_parsed_path(path),
                    None => chosen.preset(),
                };
                match obligations {
                    Some(obligations) => with_path.with_obligations(obligations),
                    None => with_path,
                }
            }
            (None, Some(path), Some(obligations)) => Self::opaque(path, obligations),
            (None, _, _) => {
                return Err("a container declaration states the preset it behaves \
                            as with `like`, or states its own `path` and \
                            `obligations`"
                    .to_string());
            }
        };

        Ok(declared.with_provision_overrides(provides))
    }
}

/// What a container implements for one trait.
///
/// A container declaration answers this for every trait typespace
/// tracks. The same three answers describe the containers a consumer
/// cannot configure: `Option` implements `Default` whatever its
/// parameter does ([`Always`](Self::Always)), `Box` implements it only
/// when its parameter does ([`IfParameters`](Self::IfParameters)), and
/// neither implements `FromStr` at any parameter
/// ([`Never`](Self::Never)).
///
/// Nothing verifies a declaration against the container it describes.
/// Claiming more than the container implements yields generated code
/// that does not compile; claiming less yields a trait conflict naming
/// the trait and the container, which is diagnosable, so the presets
/// and any hand-written declaration should lean toward claiming less.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraitProvision {
    /// No impl exists, whatever the parameters implement.
    Never,
    /// The container implements the trait whatever its parameters do.
    Always,
    /// The container implements the trait when every parameter does.
    IfParameters,
}

/// One [`TraitProvision`] for every trait typespace tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProvisionTable(BTreeMap<TypespaceTrait, TraitProvision>);

impl ProvisionTable {
    /// A table naming the traits a container never provides and those
    /// it provides unconditionally; every other trait is conditional on
    /// the parameters.
    fn new(never: &[TypespaceTrait], always: &[TypespaceTrait]) -> Self {
        Self(
            ALL_TRAITS
                .into_iter()
                .map(|trait_| {
                    let provision = match (never.contains(&trait_), always.contains(&trait_)) {
                        (true, _) => TraitProvision::Never,
                        (false, true) => TraitProvision::Always,
                        (false, false) => TraitProvision::IfParameters,
                    };
                    (trait_, provision)
                })
                .collect(),
        )
    }

    /// The table claiming only what rendering assumes of any container:
    /// `Default` whatever the parameters do, and `Clone`, `Debug`,
    /// `Serialize`, and `Deserialize` when the parameters have them.
    fn opaque() -> Self {
        let forwarded = [
            TypespaceTrait::Clone,
            TypespaceTrait::Debug,
            TypespaceTrait::Serialize,
            TypespaceTrait::Deserialize,
            TypespaceTrait::Default,
        ];
        let never = ALL_TRAITS
            .into_iter()
            .filter(|trait_| !forwarded.contains(trait_))
            .collect::<Vec<_>>();
        Self::new(&never, &[TypespaceTrait::Default])
    }

    fn get(&self, trait_: TypespaceTrait) -> TraitProvision {
        self.0[&trait_]
    }

    fn set(&mut self, trait_: TypespaceTrait, provision: TraitProvision) {
        self.0.insert(trait_, provision);
    }

    fn iter(&self) -> impl Iterator<Item = (TypespaceTrait, TraitProvision)> + '_ {
        self.0
            .iter()
            .map(|(trait_, provision)| (*trait_, *provision))
    }

    /// Apply the per-trait entries of a deserialized declaration.
    fn overridden(self, provides: BTreeMap<TypespaceTrait, TraitProvision>) -> Self {
        provides
            .into_iter()
            .fold(self, |mut table, (trait_, provision)| {
                table.set(trait_, provision);
                table
            })
    }
}

/// Specify the syntax used to render types in the `std` crate's
/// prelude. Types outside the prelude, such as
/// `std::collections::BTreeMap`, are always fully qualified.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Std {
    /// Fully qualify prelude types: `Option` renders as
    /// `::std::option::Option`.
    #[default]
    FullyQualified,
    /// Render prelude types in their typical, auto-imported form. Useful
    /// if generated code is a starting point for manually-edited code.
    Unqualified,
}

/// Specify the modeling of values that may be either 'null' or 'optional'
/// (i.e. absent).
// One of these lives in each Settings; boxing CustomType's declaration
// to shrink the enum would tax every construction site instead.
#[allow(clippy::large_enum_variant)]
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
    /// This is the default behavior of `serde` absent any additional
    /// attributes on a field.
    ConflateAsNull,

    /// Use a "double `Option`" of the form
    /// `std::option::Option<std::option::Option<T>>`. A `None` indicates that
    /// the value is absent; `Some(None)` indicates that the value is present
    /// and null; and `Some(Some(_))` indicates that the value is present
    /// and non-null.
    DoubleOption,

    /// Use a custom tri-state, type `Opt` where `Opt:
    /// json_serde::OptionalNullable` (note that `OptionalNullable` implies
    /// `Default`). It should typically be an enum, generic over `T`, with
    /// variants for absent, null, and a `T` value.
    ///
    /// The [`ContainerType`] specifies the type's path, the traits it
    /// requires of `T` (typically none), and--for each trait--whether `Opt<T>`
    /// implements it never, always, or (typically) when `T` does.
    /// [`ContainerType::option`] is the intended base for a declaration.
    CustomType(ContainerType),
}

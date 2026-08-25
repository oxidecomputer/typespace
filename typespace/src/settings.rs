// Copyright 2026 Oxide Computer Company

//! Settings that govern how types are processed and rendered.

use serde::Deserialize;

use crate::{TypespaceTrait, TypespaceTraitSet};

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

    /// The container type used to render [`Type::Map`](crate::build::Type),
    /// in place of the default `::std::collections::BTreeMap`.
    #[serde(default)]
    pub(crate) map_type: Option<ContainerType>,

    /// The traits the configured map type requires of its key type.
    #[serde(default = "ordered_lookup_traits")]
    pub(crate) map_key_traits: TypespaceTraitSet,

    /// The container type used to render [`Type::Set`](crate::build::Type),
    /// in place of the default `Vec`.
    #[serde(default)]
    pub(crate) set_type: Option<ContainerType>,

    /// The traits the configured set type requires of its element type.
    #[serde(default = "ordered_lookup_traits")]
    pub(crate) set_element_traits: TypespaceTraitSet,

    /// The container type used to render [`Type::Vec`](crate::build::Type),
    /// in place of the default `Vec`.
    #[serde(default)]
    pub(crate) vec_type: Option<ContainerType>,

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

impl Settings {
    pub fn minimal() -> Self {
        Self {
            std: Std::FullyQualified,
            optional_nullable: OptionalNullable::default(),
            map_type: None,
            map_key_traits: ordered_lookup_traits(),
            set_type: None,
            set_element_traits: ordered_lookup_traits(),
            vec_type: None,
            required_traits: TypespaceTraitSet::empty(),
            desired_traits: TypespaceTraitSet::empty(),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }

    pub fn typical() -> Self {
        Self {
            std: Std::FullyQualified,
            optional_nullable: OptionalNullable::default(),
            map_type: None,
            map_key_traits: ordered_lookup_traits(),
            set_type: None,
            set_element_traits: ordered_lookup_traits(),
            vec_type: None,
            required_traits: [
                TypespaceTrait::Clone,
                TypespaceTrait::Debug,
                TypespaceTrait::Serialize,
                TypespaceTrait::Deserialize,
            ]
            .into_iter()
            .collect(),
            desired_traits: TypespaceTraitSet::empty(),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }

    pub fn all_traits() -> Self {
        Self {
            std: Std::FullyQualified,
            optional_nullable: OptionalNullable::default(),
            map_type: None,
            map_key_traits: ordered_lookup_traits(),
            set_type: None,
            set_element_traits: ordered_lookup_traits(),
            vec_type: None,
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
            ]
            .into_iter()
            .collect(),
            extra_derives: Default::default(),
            extra_attrs: Default::default(),
        }
    }

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

    /// Set the container type used to render [`Type::Map`](crate::build::Type).
    ///
    /// The default is `::std::collections::BTreeMap`, which requires
    /// its keys to implement the `Ord` family. `key_traits` states what
    /// the configured container requires of its key type instead (for a
    /// hash map: `Hash`, `Eq`, and `PartialEq`); finalization imposes
    /// exactly those requirements on every map key. The type named by
    /// `map_type` is emitted verbatim with the key and value types as
    /// its two generic arguments, so it must:
    ///
    /// - take two generic parameters, `K` and `V`;
    /// - have an `is_empty` method that returns a boolean;
    /// - implement `Default`, `Clone`, `Debug`,
    ///   [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html),
    ///   and
    ///   [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html).
    ///
    /// A map whose key type is a string and whose value type is
    /// [`Type::JsonValue`](crate::build::Type) is rendered as
    /// `::serde_json::Map` regardless of this setting, matching the map
    /// type inside `::serde_json::Value` itself.
    pub fn with_map_type<T: Into<ContainerType>>(
        mut self,
        map_type: T,
        key_traits: TypespaceTraitSet,
    ) -> Self {
        self.map_type = Some(map_type.into());
        self.map_key_traits = key_traits;
        self
    }

    /// Set the container type used to render [`Type::Set`](crate::build::Type).
    ///
    /// The default is `Vec` (deduplication is not enforced), though set
    /// elements are still required to be comparable: the `Ord` family
    /// by default. `element_traits` states what the configured
    /// container requires of its element type instead (for a hash set:
    /// `Hash`, `Eq`, and `PartialEq`); finalization imposes exactly
    /// those requirements on every set element. The type named by
    /// `set_type` is emitted verbatim with the element type as its
    /// single generic argument, so it must:
    ///
    /// - take one generic parameter, `T`;
    /// - have an `is_empty` method that returns a boolean;
    /// - implement `Default`, `Clone`, `Debug`,
    ///   [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html),
    ///   and
    ///   [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html).
    pub fn with_set_type<T: Into<ContainerType>>(
        mut self,
        set_type: T,
        element_traits: TypespaceTraitSet,
    ) -> Self {
        self.set_type = Some(set_type.into());
        self.set_element_traits = element_traits;
        self
    }

    /// Set the container type used to render [`Type::Vec`](crate::build::Type).
    ///
    /// The default is `Vec`. The type named by `vec_type` is emitted
    /// verbatim with the element type as its single generic argument,
    /// so it must:
    ///
    /// - take one generic parameter, `T`;
    /// - have an `is_empty` method that returns a boolean;
    /// - implement `Default`, `Clone`, `Debug`,
    ///   [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html),
    ///   and
    ///   [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html).
    pub fn with_vec_type<T: Into<ContainerType>>(mut self, vec_type: T) -> Self {
        self.vec_type = Some(vec_type.into());
        self
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

    pub fn with_attr(mut self, attr: impl Into<String>) -> Self {
        self.extra_attrs.push(attr.into());
        self
    }
}

/// A container type path used in place of a built-in container render.
///
/// Wraps the Rust path of a container type (`::std::collections::HashMap`,
/// say). Rendering emits the path verbatim and appends the generic
/// arguments appropriate to the container being rendered; the
/// `Settings::with_*_type` methods document what each container
/// requires of the type.
#[derive(Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct ContainerType(pub(crate) syn::Type);

impl ContainerType {
    /// Create a new ContainerType from a [`str`].
    ///
    /// # Panics
    ///
    /// Panics if `s` cannot be parsed as a Rust type. Prefer
    /// [`str::parse`] (via the [`FromStr`](std::str::FromStr)
    /// implementation) to handle invalid input without panicking.
    pub fn new(s: &str) -> Self {
        let container_type = syn::parse_str::<syn::Type>(s).expect("valid type path");
        Self(container_type)
    }
}

impl std::str::FromStr for ContainerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let container_type = syn::parse_str::<syn::Type>(s)
            .map_err(|err| format!("invalid container type {s:?}: {err}"))?;
        Ok(Self(container_type))
    }
}

impl TryFrom<String> for ContainerType {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<syn::Type> for ContainerType {
    fn from(t: syn::Type) -> Self {
        Self(t)
    }
}

impl From<&str> for ContainerType {
    /// Parse a container type from a string path.
    ///
    /// # Panics
    ///
    /// Panics if the string cannot be parsed as a Rust type; see
    /// [`ContainerType::new`].
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Debug for ContainerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use quote::ToTokens;
        write!(f, "ContainerType({})", self.0.to_token_stream())
    }
}

/// Specify how types in the `std` crate's prelude are spelled in
/// generated code. Types outside the prelude, such as
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
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionalNullable {
    /// Model `null` and `optional` as equivalent by using the
    /// `std::option::Option<T>` type. Skip serialization of `None` values.
    /// This is the default--it's a typical configuration where `None` values
    /// are omitted.
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

    /// Use a custom type `Opt` where `Opt: std::default::Default +
    /// serde::Deserialize + serde::Serialize`. The `Default` implementation
    /// specifies the value for a field when absent; the `Deserialize`
    /// implementation produces a value otherwise (null or a non-null value of
    /// T). In addition, `Opt` must implement `is_absent(&self) -> bool` which
    /// is used with the serde `skip_serializing_if` attribute to omit the
    /// field.
    CustomType(String),
    // 8/21/2026
    // At the fringes to consider. Should we have a AbsentOptional traits that
    // requires Default + Deserialize + Serialize, and requires an is_absent()
    // method? We could shove it into json-serde, and we could impl it for
    // Option<Option<T>>.
}

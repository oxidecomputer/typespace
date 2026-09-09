// Copyright 2026 Oxide Computer Company

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::TypespaceRenderer;
use crate::build::{Type, TypeCommon};
use crate::error::Error;

/// A type alias (`pub type Name = Target;`).
///
/// A `TypeAlias` is its own builder: [`TypeAlias::new`] starts one
/// under construction around its required target, the fluent methods
/// fill it in, and [`TypeAlias::build`] validates it and produces the
/// finished [`Type::TypeAlias`] value. An alias introduces no value of
/// its own, so it carries no default slot.
#[derive(Debug, Clone)]
pub struct TypeAlias<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) target: Id,
}

impl<Id> TypeAlias<Id> {
    /// Start a type alias for the type `target`.
    pub fn new(target: Id) -> Self {
        Self {
            common: Default::default(),
            target,
        }
    }

    /// Set the alias's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.common.name = Some(name.into());
        self
    }

    /// Set the description (doc comment source).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.common.description = Some(description.into());
        self
    }

    /// Add opaque attributes applied to this type alone.
    ///
    /// These are additional to the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn extra_attrs(mut self, attrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.common
            .extra_attrs
            .extend(attrs.into_iter().map(Into::into));
        self
    }

    /// Validate the type alias and produce it as a [`Type`] value.
    ///
    /// Fails if the name is missing or not a valid identifier.
    pub fn build(self) -> Result<Type<Id>, Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.validate()?;
        Ok(Type::TypeAlias(self))
    }

    /// The checks `build()` applies; also run at insertion as
    /// defense-in-depth.
    pub(crate) fn validate(&self) -> Result<(), Error<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        self.common.validate_name("type alias")
    }

    /// The alias's name, if one has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn get_description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The opaque attributes applied to this type alone, additional to
    /// the crate-wide attributes from
    /// [`Settings::with_attr`](crate::settings::Settings::with_attr).
    pub fn get_extra_attrs(&self) -> &[String] {
        self.common.extra_attrs()
    }

    /// The ID of the aliased type.
    pub fn get_target(&self) -> &Id {
        &self.target
    }
}

impl<Id: Clone + Ord + std::fmt::Debug + std::fmt::Display> TypeAlias<Id> {
    pub(crate) fn children(&self) -> Vec<Id> {
        vec![self.target.clone()]
    }

    pub(crate) fn render(&self, typespace: &TypespaceRenderer<'_, Id>) -> TokenStream {
        let Self {
            common:
                TypeCommon {
                    name,
                    description,
                    built: _,
                    default: _,
                    extra_derives: _,
                    extra_attrs: _,
                },
            target: type_id,
        } = self;
        let name = name.as_deref().expect("validated type has a name");
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        let target_ident = typespace.render_ident(type_id);

        quote! {
            #description
            pub type #name_ident = #target_ident;
        }
    }
}

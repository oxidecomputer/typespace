// Copyright 2026 Oxide Computer Company

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::build::{CommonBuilder, Type, TypeCommon};
use crate::{TypespaceError, TypespaceRenderer};

/// A type alias (`pub type Name = Target;`); construct one with
/// [`TypeAlias::builder`].
#[derive(Debug, Clone)]
pub struct TypeAlias<Id> {
    pub(crate) common: TypeCommon,
    pub(crate) target: Id,
}

impl<Id> TypeAlias<Id> {
    /// Start building a type alias for the type `target`; see
    /// [`TypeAliasBuilder`].
    pub fn builder(target: Id) -> TypeAliasBuilder<Id> {
        TypeAliasBuilder {
            common: CommonBuilder::default(),
            target,
        }
    }

    /// The alias's name, always nonempty.
    pub fn name(&self) -> &str {
        self.common.name()
    }

    /// The description (doc comment source), if any.
    pub fn description(&self) -> Option<&str> {
        self.common.description()
    }

    /// The ID of the aliased type.
    pub fn target(&self) -> &Id {
        &self.target
    }
}

/// Assembles a [`TypeAlias`]; created by [`TypeAlias::builder`].
///
/// The name is the one required ingredient and may be supplied at any
/// point before [`TypeAliasBuilder::build`], which produces the
/// finished [`Type::TypeAlias`] value.
#[derive(Debug, Clone)]
pub struct TypeAliasBuilder<Id> {
    common: CommonBuilder,
    target: Id,
}

impl<Id> TypeAliasBuilder<Id> {
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

    /// Produce the type alias as a [`Type`] value.
    ///
    /// Fails with [`TypespaceError::MissingTypeName`] unless a nonempty
    /// name was provided.
    pub fn build(self) -> Result<Type<Id>, TypespaceError<Id>>
    where
        Id: std::fmt::Debug + std::fmt::Display,
    {
        let Self { common, target } = self;
        Ok(Type::TypeAlias(TypeAlias {
            common: common.build("type alias")?,
            target,
        }))
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
                },
            target: type_id,
        } = self;
        let description = description.as_ref().map(|desc| quote! { #[doc = #desc ]});
        let name_ident = format_ident!("{name}");

        let target_ident = typespace.render_ident(type_id);

        quote! {
            #description
            pub type #name_ident = #target_ident;
        }
    }
}

// Copyright 2026 Oxide Computer Company

//! The gate for every `#[serde(..)]` attribute typespace emits.
//!
//! serde's derive macros are what read `#[serde(..)]`. An attribute on
//! an item that derives neither `Serialize` nor `Deserialize` has
//! nothing to consume it, and the compiler rejects it as an unknown
//! attribute. The rule for renderers, then: never write the tokens
//! `#[serde(..)]`. Push each option into a [`SerdeAttrs`], which
//! carries the item's [`SerdeDerives`] and renders to nothing when no
//! derive would read what it holds. This module is the one place that
//! writes the attribute, so a new option is gated by having been
//! pushed.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{TypespaceTrait, TypespaceTraitSet};

/// The serde derives an item carries.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SerdeDerives {
    serialize: bool,
    deserialize: bool,
    jsonschema: bool,
}

impl SerdeDerives {
    /// Read the serde derives from the trait set a renderer passes to
    /// `render_derives`.
    ///
    /// Read them from that set and no other: a renderer that writes its
    /// own `Serialize` or `Deserialize` impl takes the trait out of the
    /// set first, and a hand-written impl reads no attributes.
    pub(crate) fn new(traits: &TypespaceTraitSet) -> Self {
        Self {
            serialize: traits.contains(&TypespaceTrait::Serialize),
            deserialize: traits.contains(&TypespaceTrait::Deserialize),
            jsonschema: traits.contains(&TypespaceTrait::JsonSchema),
        }
    }

    /// Whether the item derives `Deserialize`.
    ///
    /// serde parses the whole attribute in either derive and ignores
    /// what the other one reads, so an option rarely wants a finer gate
    /// than [`SerdeAttrs`] already applies. `deny_unknown_fields` takes
    /// this one: it says nothing on a `Serialize`-only item.
    pub(crate) fn deserialize(self) -> bool {
        self.deserialize
    }

    /// An empty option list for an item with these derives.
    pub(crate) fn attrs(self) -> SerdeAttrs {
        SerdeAttrs {
            derives: self,
            options: Vec::new(),
        }
    }
}

/// The `#[serde(..)]` options collected for one item or one field.
///
/// Renders as the attribute holding every option pushed into it, or as
/// nothing when it is empty or the item has no serde derive to read it.
#[derive(Debug)]
pub(crate) struct SerdeAttrs {
    derives: SerdeDerives,
    options: Vec<TokenStream>,
}

impl SerdeAttrs {
    /// Add one option, such as `rename = "my-field"`.
    pub(crate) fn push(&mut self, option: TokenStream) {
        self.options.push(option);
    }
}

impl Extend<TokenStream> for SerdeAttrs {
    fn extend<T: IntoIterator<Item = TokenStream>>(&mut self, options: T) {
        self.options.extend(options);
    }
}

impl ToTokens for SerdeAttrs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { derives, options } = self;
        let serde = derives.serialize || derives.deserialize;
        if !options.is_empty() {
            if serde {
                tokens.extend(quote! {
                    #[serde(
                        #( #options ),*
                    )]
                });
            } else if derives.jsonschema {
                tokens.extend(quote! {
                    #[schemars(
                        #( #options ),*
                    )]
                });
            }
        }
    }
}

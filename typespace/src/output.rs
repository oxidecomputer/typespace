// Copyright 2026 Oxide Computer Company

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::ToTokens;

use crate::default::DefaultHelper;
use crate::settings::{ContainerType, GeneratedCrate, Settings};

/// The destination a render pass writes into.
///
/// A single `&mut Outputspace` travels the render call chain: items go
/// into the codespace it holds, and anything a rendered type needs the
/// whole output to carry accumulates alongside them.
pub(crate) struct Outputspace<'a> {
    settings: &'a Settings,
    cs: codespace::Codespace,

    /// Shared `defaults` functions that the rendered properties call.
    ///
    /// Rendering a property records the function it wants here, and
    /// [`Outputspace::into_codespace`] defines each one once every type
    /// is rendered.
    default_helpers: BTreeSet<DefaultHelper>,

    /// The crates the rendered code refers to, recorded as each path is
    /// written through [`Outputspace::crate_path`].
    crates: BTreeSet<GeneratedCrate>,

    /// The crates the paths in the settings are written from, containers
    /// and foreign derives, recorded by root as each is written.
    foreign: BTreeSet<String>,
}

impl<'a> Outputspace<'a> {
    pub(crate) fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            cs: codespace::Codespace::default(),
            default_helpers: BTreeSet::new(),
            crates: BTreeSet::new(),
            foreign: BTreeSet::new(),
        }
    }

    pub(crate) fn settings(&self) -> &'a Settings {
        self.settings
    }

    /// The codespace that rendered items go into.
    pub(crate) fn cs(&mut self) -> &mut codespace::Codespace {
        &mut self.cs
    }

    /// Record a shared `defaults` function a rendered property calls.
    pub(crate) fn add_default_helper(&mut self, helper: DefaultHelper) {
        self.default_helpers.insert(helper);
    }

    /// The path generated code refers to `krate` by, recording that the
    /// output depends on it.
    ///
    /// Call this where the code naming the crate is emitted, inside
    /// whatever gate decides that it is (a granted trait, a present
    /// remainder, a non-empty pattern list), never above it: a lookup
    /// hoisted over the gate reports a crate the output may not use.
    pub(crate) fn crate_path(&mut self, krate: GeneratedCrate) -> TokenStream {
        self.crates.insert(krate);
        self.settings.crate_paths.tokens(krate)
    }

    /// [`Outputspace::crate_path`] as the unspaced string a serde attribute
    /// value takes.
    pub(crate) fn crate_path_text(&mut self, krate: GeneratedCrate) -> String {
        self.crates.insert(krate);
        self.settings.crate_paths.text(krate)
    }

    /// The path generated code refers to a configured container by,
    /// recording the crate it is written from.
    pub(crate) fn container<'c>(&mut self, container: &'c ContainerType) -> &'c syn::Type {
        if let syn::Type::Path(type_path) = container.path() {
            self.record_root(&type_path.path);
        }
        container.rendered_path(&self.settings.std)
    }

    /// A foreign derive's path as written, recording the crate it is
    /// written from.
    pub(crate) fn foreign_path(&mut self, path: &syn::Path) -> TokenStream {
        self.record_root(path);
        path.to_token_stream()
    }

    /// Record the crate a path is written from: its first segment when
    /// the path starts with `::`. A bare or relative path names nothing
    /// the consumer has not already brought into scope, and the
    /// toolchain's own crates are not dependencies.
    fn record_root(&mut self, path: &syn::Path) {
        if let (Some(_), Some(segment)) = (path.leading_colon, path.segments.first()) {
            let root = segment.ident.to_string();
            if !["std", "core", "alloc"].contains(&root.as_str()) {
                self.foreign.insert(root);
            }
        }
    }

    /// Finish the output, yielding the codespace it accumulated, with the
    /// crates the code refers to registered as its dependencies: each of
    /// typespace's own under its registry name, or under the root of its
    /// override when the settings point it at another crate (an override
    /// naming a module of the consumer's own registers nothing), and
    /// each container or foreign derive crate under its root.
    pub(crate) fn into_codespace(self) -> codespace::Codespace {
        let Self {
            settings,
            mut cs,
            default_helpers,
            crates,
            foreign,
        } = self;

        for dep in crates
            .into_iter()
            .filter_map(|krate| settings.crate_paths.dependency(krate))
            .chain(foreign.into_iter().map(codespace::Dependency::new))
        {
            cs.add_dependency(dep)
                .expect("each crate is registered once at any version, so none can conflict");
        }

        // Every property whose default value a shared function
        // produces recorded that function as it rendered; define each
        // of them once. The empty key sorts them ahead of the
        // per-property functions, which are keyed by name. Naming the
        // module creates it, so ask for it only when something goes in
        // it.
        if !default_helpers.is_empty() {
            let defaults_mod = cs.get_root_mod().get_mod("defaults");
            for helper in default_helpers {
                defaults_mod.add_item("", helper.definition());
            }
        }

        // A per-property function creates the module as it renders and
        // a shared helper creates it just above, so document it here:
        // the one point both paths pass through.
        if cs.get_root_mod().has_mod("defaults") {
            cs.get_root_mod()
                .get_mod("defaults")
                .add_docs(" Generation of default values for serde.");
        }

        cs
    }
}

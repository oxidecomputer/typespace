// Copyright 2026 Oxide Computer Company

use std::collections::BTreeSet;

use crate::default::DefaultHelper;

/// The destination a render pass writes into.
///
/// A single `&mut Outputspace` travels the render call chain: items go
/// into the codespace it holds, and anything a rendered type needs the
/// whole output to carry accumulates alongside them.
#[derive(Default)]
pub(crate) struct Outputspace {
    cs: codespace::Codespace,

    /// Shared `defaults` functions that the rendered properties call.
    ///
    /// Rendering a property records the function it wants here, and
    /// [`Outputspace::into_codespace`] defines each one once every type
    /// is rendered.
    default_helpers: BTreeSet<DefaultHelper>,
}

impl Outputspace {
    /// The codespace that rendered items go into.
    pub(crate) fn cs(&mut self) -> &mut codespace::Codespace {
        &mut self.cs
    }

    /// Record a shared `defaults` function a rendered property calls.
    pub(crate) fn add_default_helper(&mut self, helper: DefaultHelper) {
        self.default_helpers.insert(helper);
    }

    /// Finish the output, yielding the codespace it accumulated.
    pub(crate) fn into_codespace(self) -> codespace::Codespace {
        let Self {
            mut cs,
            default_helpers,
        } = self;

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

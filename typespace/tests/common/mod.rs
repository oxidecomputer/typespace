// Copyright 2026 Oxide Computer Company

//! Helpers for reading generated code back as syntax.
//!
//! A test that cares what typespace emitted asks these rather than
//! matching on rendered characters, which change with formatting.

// Each integration test binary compiles this module on its own, so a
// helper only some of them use looks dead to the rest.
#![allow(dead_code)]

/// The names a type's `derive` attributes carry.
pub fn derives_of(file: &syn::File, type_name: &str) -> Vec<String> {
    attrs_of(file, type_name)
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .flat_map(|attr| {
            attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .unwrap()
        })
        .map(|path| path.segments.last().unwrap().ident.to_string())
        .collect()
}

/// The attributes on a named type.
pub fn attrs_of<'a>(file: &'a syn::File, type_name: &str) -> &'a [syn::Attribute] {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == type_name => Some(&item.attrs),
            syn::Item::Enum(item) if item.ident == type_name => Some(&item.attrs),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{type_name} is rendered"))
}

/// Whether the file carries `impl <trait_name> for <type_name>`.
pub fn has_impl(file: &syn::File, trait_name: &str, type_name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item) => {
            let Some((path, _for)) = &item.trait_ else {
                return false;
            };
            let names_trait = path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == trait_name);
            let syn::Type::Path(self_ty) = item.self_ty.as_ref() else {
                return false;
            };
            let names_type = self_ty
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == type_name);
            names_trait && names_type
        }
        _ => false,
    })
}

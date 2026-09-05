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

/// Whether the rendered `type_name` carries `trait_name`, either as a
/// derive or as a generated impl.
pub fn implements(file: &syn::File, type_name: &str, trait_name: &str) -> bool {
    derives_of(file, type_name)
        .iter()
        .any(|name| name == trait_name)
        || has_impl(file, trait_name, type_name)
}

/// Ask the compiler whether a concrete Rust type implements one trait.
///
/// The inherent const applies only where the bound holds; the blanket
/// trait const answers for every other type, and resolution prefers the
/// inherent one. So `Probe::<T>::IMPLS` is true exactly when the impl
/// exists, in both directions: a false answer is the compiler saying
/// that code deriving the trait would not build.
macro_rules! trait_probe {
    ($name:ident, $($bound:tt)*) => {
        pub mod $name {
            pub struct Probe<T>(::std::marker::PhantomData<T>);
            pub trait Fallback {
                const IMPLS: bool = false;
            }
            impl<T> Fallback for Probe<T> {}
            impl<T: $($bound)*> Probe<T> {
                pub const IMPLS: bool = true;
            }
        }
    };
}

/// One [`trait_probe`] per trait typespace tracks.
pub mod probes {
    trait_probe!(clone, ::std::clone::Clone);
    trait_probe!(copy, ::std::marker::Copy);
    trait_probe!(debug, ::std::fmt::Debug);
    trait_probe!(serialize, ::serde::Serialize);
    trait_probe!(deserialize, for<'de> ::serde::Deserialize<'de>);
    trait_probe!(json_schema, ::schemars::JsonSchema);
    trait_probe!(display, ::std::fmt::Display);
    trait_probe!(from_str, ::std::str::FromStr);
    trait_probe!(eq, ::std::cmp::Eq);
    trait_probe!(partial_eq, ::std::cmp::PartialEq);
    trait_probe!(ord, ::std::cmp::Ord);
    trait_probe!(partial_ord, ::std::cmp::PartialOrd);
    trait_probe!(hash, ::std::hash::Hash);
    trait_probe!(default, ::std::default::Default);
}

/// The traits a concrete Rust type implements, as a
/// [`TypespaceTraitSet`](typespace::TypespaceTraitSet).
///
/// The answer comes from the compiler, so a test can hold a container
/// declaration up against the container it describes.
#[macro_export]
macro_rules! implemented_traits {
    ($ty:ty) => {{
        use ::typespace::TypespaceTrait as T;
        // Where every probe's inherent const applies, no fallback is
        // consulted and the imports look unused.
        #[allow(unused_imports)]
        use $crate::common::probes::{
            clone::Fallback as _, copy::Fallback as _, debug::Fallback as _,
            default::Fallback as _, deserialize::Fallback as _, display::Fallback as _,
            eq::Fallback as _, from_str::Fallback as _, hash::Fallback as _,
            json_schema::Fallback as _, ord::Fallback as _, partial_eq::Fallback as _,
            partial_ord::Fallback as _, serialize::Fallback as _, *,
        };
        [
            (T::Clone, <clone::Probe<$ty>>::IMPLS),
            (T::Copy, <copy::Probe<$ty>>::IMPLS),
            (T::Debug, <debug::Probe<$ty>>::IMPLS),
            (T::Serialize, <serialize::Probe<$ty>>::IMPLS),
            (T::Deserialize, <deserialize::Probe<$ty>>::IMPLS),
            (T::JsonSchema, <json_schema::Probe<$ty>>::IMPLS),
            (T::Display, <display::Probe<$ty>>::IMPLS),
            (T::FromStr, <from_str::Probe<$ty>>::IMPLS),
            (T::Eq, <eq::Probe<$ty>>::IMPLS),
            (T::PartialEq, <partial_eq::Probe<$ty>>::IMPLS),
            (T::Ord, <ord::Probe<$ty>>::IMPLS),
            (T::PartialOrd, <partial_ord::Probe<$ty>>::IMPLS),
            (T::Hash, <hash::Probe<$ty>>::IMPLS),
            (T::Default, <default::Probe<$ty>>::IMPLS),
        ]
        .into_iter()
        .filter(|(_, implemented)| *implemented)
        .map(|(trait_, _)| trait_)
        .collect::<::typespace::TypespaceTraitSet>()
    }};
}

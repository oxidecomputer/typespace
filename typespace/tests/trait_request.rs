// Copyright 2026 Oxide Computer Company

//! Traits requested by name: `ForeignTrait`, `TraitRequest`, and the
//! settings methods that take them.

use typespace::error::{Error, OffenderReason, RequirementOrigin};
use typespace::settings::{ForeignTrait, Settings, TraitSpec};
use typespace::{TypespaceTrait, no_cycles};
use typespace_test_macro::{check_and_include, typespace_builder};

mod common;

const FOREIGN: &str = "::typespace_test_macro::ForeignDerive";

fn foreign() -> ForeignTrait {
    ForeignTrait::new(FOREIGN).unwrap()
}

fn path_of(derive: &ForeignTrait) -> String {
    use quote::ToTokens;
    derive.path().to_token_stream().to_string().replace(' ', "")
}

/// A trait typespace models cannot be a foreign derive, however it is
/// spelled; the refusal names the typed method.
#[test]
fn a_modeled_trait_is_not_foreign() {
    for spelling in [
        "Hash",
        "::std::hash::Hash",
        "std::hash::Hash",
        "core::hash::Hash",
    ] {
        let err = ForeignTrait::new(spelling).unwrap_err();
        assert_eq!(err.text, spelling);
        assert!(
            err.reason
                .contains("with_required_trait(TypespaceTrait::Hash)"),
            "{err}"
        );
    }
}

/// A bare identifier has no crate path for generated code to resolve.
#[test]
fn a_bare_identifier_is_not_foreign() {
    let err = ForeignTrait::new("Deftly").unwrap_err();
    assert!(err.reason.contains("crate path"), "{err}");
}

/// Anything syn cannot read as a path is refused.
#[test]
fn a_non_path_is_not_foreign() {
    let err = ForeignTrait::new("not a path!").unwrap_err();
    assert!(err.reason.contains("not a Rust path"), "{err}");
}

/// A foreign path is accepted with or without its leading `::`.
#[test]
fn a_foreign_path_is_accepted() {
    assert_eq!(path_of(&foreign()), FOREIGN);
    assert_eq!(
        path_of(&ForeignTrait::new("deftly::Deftly").unwrap()),
        "deftly::Deftly"
    );
}

/// `parse` recognizes a modeled trait by bare name or canonical path.
#[test]
fn parse_recognizes_a_modeled_trait() {
    for spelling in [
        "Ord",
        "::std::cmp::Ord",
        "std::cmp::Ord",
        "::serde::Serialize",
        "JsonSchema",
        "::schemars::JsonSchema",
    ] {
        assert!(
            matches!(
                TraitSpec::parse(spelling),
                Ok(TraitSpec::Known(trait_))
                    if trait_ == TypespaceTrait::from_path(spelling).unwrap()
            ),
            "{spelling}"
        );
    }
}

/// `parse` reads bounds after a colon, each a modeled trait.
#[test]
fn parse_reads_bounds() {
    let Ok(TraitSpec::Foreign(derive)) =
        TraitSpec::parse(&format!("{FOREIGN}: Ord + ::std::hash::Hash"))
    else {
        panic!("expected a foreign derive with bounds");
    };
    assert_eq!(path_of(&derive), FOREIGN);
    assert_eq!(
        derive.bounds().iter().copied().collect::<Vec<_>>(),
        vec![TypespaceTrait::Hash, TypespaceTrait::Ord]
    );

    let Ok(TraitSpec::Foreign(derive)) = TraitSpec::parse(FOREIGN) else {
        panic!("expected a foreign derive");
    };
    assert!(derive.bounds().is_empty());
}

/// A bound must be a modeled trait; a modeled path takes no bounds; the
/// path itself is checked as `ForeignTrait::new` checks it.
#[test]
fn parse_refuses_bad_input() {
    let err = TraitSpec::parse(&format!("{FOREIGN}: Deftly")).unwrap_err();
    assert!(err.reason.contains("`Deftly` is not a trait"), "{err}");

    let err = TraitSpec::parse("Ord: Hash").unwrap_err();
    assert!(err.reason.contains("takes no bounds"), "{err}");

    let err = TraitSpec::parse("Deftly").unwrap_err();
    assert!(err.reason.contains("crate path"), "{err}");

    let err = TraitSpec::parse("not a path!").unwrap_err();
    assert_eq!(err.text, "not a path!");
}

/// A known request lands in the required set; a foreign one in the
/// derive list.
#[test]
fn extra_required_trait_routes_by_kind() {
    let settings = Settings::minimal()
        .with_extra_required_trait(TraitSpec::parse("Ord").unwrap())
        .with_extra_required_trait(TraitSpec::parse(FOREIGN).unwrap());
    assert!(settings.required_traits.contains(&TypespaceTrait::Ord));
    assert_eq!(settings.extra_derives.len(), 1);
    assert_eq!(path_of(&settings.extra_derives[0]), FOREIGN);
}

/// A foreign derive with bounds is emitted on every named type, and its
/// bounds are required of them, here satisfiable everywhere.
#[test]
fn a_bounded_foreign_derive_is_emitted_with_its_bounds() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_derive(foreign().requires([TypespaceTrait::Ord, TypespaceTrait::Hash]));
    let builder = typespace_builder!(settings, {
        struct Point {
            x: u32,
            y: u32,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    assert!(
        ts.get_type(&"Point".to_string())
            .has_impl(TypespaceTrait::Ord)
    );

    #[check_and_include("tests/output/test_bounded_foreign_derive.rs", ts.to_codespace().into_stream())]
    fn inner() {
        let a = import::Point { x: 1, y: 2 };
        let b = import::Point { x: 1, y: 3 };
        assert!(a < b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
    }
}

/// A type that cannot satisfy a foreign derive's bound fails finalize,
/// and the conflict names the derive as the origin.
#[test]
fn a_bounded_foreign_derive_conflicts_naming_the_derive() {
    let settings = Settings::minimal().with_derive(foreign().requires([TypespaceTrait::Ord]));
    let builder = typespace_builder!(settings, {
        struct Reading {
            value: f64,
        }
    });
    let Err(Error::TraitConflicts { conflicts }) = builder.finalize(no_cycles) else {
        panic!("expected a trait conflict");
    };
    // `Ord` brings `Eq` along, and a float can be neither.
    assert!(!conflicts.is_empty());
    for conflict in &conflicts {
        assert!(
            matches!(&conflict.reason, OffenderReason::Primitive { type_name } if type_name == "f64"),
            "{conflict:?}"
        );
        assert!(
            matches!(&conflict.origin, RequirementOrigin::ForeignDerive { derive } if derive == FOREIGN),
            "{:?}",
            conflict.origin
        );
    }
    let ord = conflicts
        .iter()
        .find(|conflict| conflict.required == TypespaceTrait::Ord)
        .expect("Ord itself conflicts");
    assert!(
        ord.to_string()
            .contains(&format!("the derive `{FOREIGN}` requires `Ord`")),
        "{ord}"
    );
}

/// Settings data lists foreign derives as text, read through `parse`.
#[test]
fn foreign_derives_deserialize() {
    let settings = serde_json::from_str::<Settings>(&format!(
        r#"{{ "extra_derives": ["{FOREIGN}", "{FOREIGN}: Ord"] }}"#
    ))
    .unwrap();
    assert_eq!(settings.extra_derives.len(), 2);
    assert!(settings.extra_derives[0].bounds().is_empty());
    assert!(
        settings.extra_derives[1]
            .bounds()
            .contains(&TypespaceTrait::Ord)
    );

    serde_json::from_str::<Settings>(r#"{ "extra_derives": ["Hash"] }"#).unwrap_err();
    serde_json::from_str::<Settings>(r#"{ "extra_derives": ["Deftly"] }"#).unwrap_err();
    serde_json::from_str::<Settings>(&format!(r#"{{ "extra_derives": ["{FOREIGN}: Deftly"] }}"#))
        .unwrap_err();
}

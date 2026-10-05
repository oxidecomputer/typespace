// Copyright 2026 Oxide Computer Company

//! The dependency list a rendered typespace reports.

use typespace::build::{NewtypeConstraints, NewtypeStruct, Type};
use typespace::settings::{ContainerType, ForeignTrait, GeneratedCrate, Settings};
use typespace::{TypespaceBuilder, TypespaceTrait, TypespaceTraitSet, no_cycles};
use typespace_test_macro::typespace_builder;

mod common;

fn settings() -> Settings {
    Settings::minimal()
        .with_required_trait(TypespaceTrait::Debug)
        .with_required_trait(TypespaceTrait::Serialize)
        .with_required_trait(TypespaceTrait::Deserialize)
        .with_required_trait(TypespaceTrait::JsonSchema)
}

/// A pattern-constrained newtype, inserted raw because the macro has no
/// NewtypeConstraints syntax.
fn insert_code(builder: &mut TypespaceBuilder<String>) {
    builder
        .insert("code string".to_string(), Type::String)
        .unwrap();
    builder
        .insert(
            "code".to_string(),
            Type::NewtypeStruct(
                NewtypeStruct::new("code string".to_string())
                    .name("Code")
                    .constraints(NewtypeConstraints::String {
                        min: None,
                        max: None,
                        patterns: vec!["^x".to_string()],
                    }),
            ),
        )
        .unwrap();
}

fn names(ts: &typespace::Typespace<String>) -> Vec<String> {
    common::codespace(ts)
        .dependencies()
        .map(|dep| dep.name.clone())
        .collect()
}

/// Each of typespace's own crates is reported when the rendered code
/// refers to it, under its registry name; a native's crate is the
/// consumer's to report.
#[test]
fn rendered_code_reports_typespace_crates() {
    let mut builder = typespace_builder!(settings(), {
        native ::std::path::PathBuf: Clone + Debug + Serialize + Deserialize + JsonSchema;

        struct Thing {
            at: ::std::path::PathBuf,
            gone: Optional<!>,
            any: JsonValue,
        }
    });
    insert_code(&mut builder);

    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(
        names(&ts),
        ["json-serde", "regress", "schemars", "serde", "serde_json"]
    );
}

/// A typespace whose code needs none of the optional crates reports
/// only serde.
#[test]
fn plain_types_need_only_serde() {
    let builder = typespace_builder!(
        Settings::minimal()
            .with_required_trait(TypespaceTrait::Serialize)
            .with_required_trait(TypespaceTrait::Deserialize),
        {
            struct Thing {
                name: String,
                count: u32,
            }
        }
    );
    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(names(&ts), ["serde"]);
}

/// With the path to json-serde overridden to another crate, that crate
/// is reported in its place.
#[test]
fn an_overridden_crate_path_reports_its_root() {
    let settings = settings().with_crate_path(GeneratedCrate::JsonSerde, "::my_js");
    let builder = typespace_builder!(settings, {
        struct Thing {
            gone: Optional<!>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(names(&ts), ["my_js", "schemars", "serde"]);
}

/// A crate the consumer records for a native type is reported with
/// typespace's own, carrying what the consumer said about it.
#[test]
fn recorded_native_crates_are_reported() {
    let mut builder = typespace_builder!(
        Settings::minimal().with_required_trait(TypespaceTrait::Serialize),
        {
            native ::chrono::NaiveDate: Clone + Debug + Serialize;

            struct Thing {
                when: ::chrono::NaiveDate,
            }
        }
    );
    builder.add_dependency(typespace::codespace::Dependency {
        version: "0.4".parse().unwrap(),
        features: vec!["serde".to_string()],
        ..typespace::codespace::Dependency::new("chrono")
    });
    let ts = builder.finalize(no_cycles).unwrap();
    let cs = common::codespace(&ts);
    let deps = cs.dependencies().collect::<Vec<_>>();
    let names = deps.iter().map(|dep| dep.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, ["chrono", "serde"]);
    assert_eq!(deps[0].version.to_string(), "^0.4");
    assert_eq!(deps[0].features, ["serde"]);
}

/// With the path overridden to a module of the consumer's own, nothing
/// is reported for it.
#[test]
fn a_module_override_reports_nothing() {
    let settings = settings().with_crate_path(GeneratedCrate::JsonSerde, "super::json_helpers");
    let builder = typespace_builder!(settings, {
        struct Thing {
            gone: Optional<!>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(names(&ts), ["schemars", "serde"]);
}

/// A container configured from another crate reports that crate when a
/// type uses the container; the std containers report nothing.
#[test]
fn a_configured_container_reports_its_crate() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Serialize)
        .with_map_type(ContainerType::new(
            "::indexmap::IndexMap",
            [TypespaceTraitSet::empty(), TypespaceTraitSet::empty()],
        ))
        .with_set_type(ContainerType::btree_set());
    let builder = typespace_builder!(settings, {
        struct Thing {
            by_name: Map<String, u32>,
            names: Set<String>,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(names(&ts), ["indexmap", "serde"]);
}

/// A foreign derive written from a crate reports that crate, whether
/// it applies crate-wide or to one type; a bare derive reports nothing.
#[test]
fn a_foreign_derive_reports_its_crate() {
    let settings = Settings::minimal()
        .with_required_trait(TypespaceTrait::Serialize)
        .with_derive(ForeignTrait::new("::deftly::Deftly").unwrap());
    let builder = typespace_builder!(settings, {
        struct Thing {
            name: String,
        }

        #[derive = ["::educe::Educe", "Bare"]]
        struct Other {
            name: String,
        }
    });
    let ts = builder.finalize(no_cycles).unwrap();
    assert_eq!(names(&ts), ["deftly", "educe", "serde"]);
}

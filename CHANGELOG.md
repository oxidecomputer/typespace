# Changelog

## Next

## [0.1.4] - 2026-11-23

* Lower the MSRV to 1.88: the newest compiler the crate itself needs (for
  `let` chains), and what its `regress` dependency needs.
* `TypespaceTrait::from_path` compares parsed paths, so a derive written with
  spaced separators (`schemars :: JsonSchema`) is recognized as the trait.
* An `Optional` whose type is a container (`Vec`, `Set`, `Map`) or the
  unit type (`()`) have settings (`optional_container` and `optional_unit`--to
  join `optional_nullable` for `Optional` `Option` types) that govern whether
  they are rendered as raw types with `#[serde(default)]` or with an `Option`
  wrapper.
* `Optional` more formally allows for absent to be distinguished from the
  default value for the type depending on settings; `Default` or `DefaultValue`
  for a struct field expressly means that the default value is equivalent to
  the absence of the field.
* A flattened property renders as its type, or as a plain `Option` of that type
  when `Optional`, with `flatten` as its only serde attribute; the settings
  for absent keyed fields do not apply.
* A flattened property in the `Default` or `DefaultValue` state is refused at
  finalize (`Error::FlattenWithDefault`): serde never finds a flattened
  property missing, so a default would never apply.
* A generated `Default` impl requires `Default` of every field it assigns
  `Default::default()`, which includes an `Optional` container or unit left
  unwrapped by `optional_container` or `optional_unit`. A `vec_type`, `set_type`,
  or `map_type` declared without `Default` now refuses `Default` on such a struct
  rather than deriving an impl that does not compile.
* A `Default`-state property of type `JsonValue` renders with `#[serde(default)]`
  (an absent key reads as `null`) instead of panicking.
* A default value for a constrained newtype, whether the type's own or a
  property's, is checked against the constraints at `finalize`, as the
  rendered `TryFrom` would check it.

## [0.1.3] - 2026-11-18

* String-constrained newtypes gets hand-written `JsonSchema` impls
* An allow-list or deny-list newtype whose inner type implements `FromStr`
  implements it too; `Display` forwards to the inner type.
* `#[serde(default)]` properties and property default values put their
  `Default` and `Deserialize` requirements on other types only when the owning
  type itself deserializes. A type nothing deserializes no longer fails
  `finalize` over them
* `Box<T>` implements `Display` when `T` does, so a newtype that contains a
  `Box` can satisfy that requirement.
* Default values and allow/deny list values are constructed directly rather
  than deserialized at runtime.
* An `i128` default value accepts anything up to `u64::MAX`.
* Generated code for a struct fields of type `()` no longer trip
  `clippy::unit_arg`

## [0.1.2] - 2026-11-16

* The default set container is a plain `Vec` with no element obligation. A
  set whose elements cannot be compared no longer fails at `finalize` unless
  the settings name a real set container such as `BTreeSet` or `HashSet`
* A tuple struct's schema describes its rest's elements as `additionalItems`
  for a `Vec`, `Set`, fixed array, or one-type tuple or tuple struct rest,
  through any alias, newtype, or `Box`; elements it cannot describe are left
  unconstrained
* A tuple struct whose rest is not a sequence of one element type is rejected
  at `finalize` (`Error::InvalidRest`)

## [0.1.1] - 2026-11-15

* Fixes `TryFrom<&str>` for a string-constrained newtype over a string-like native type
* A string constraint's patterns are compiled with the ECMA-262 `u` flag, as
  JSON Schema specifies, so `\p{..}` and `\u{..}` escapes work; a pattern with
  an identity escape the `u` flag forbids now fails in the consumer's build
* A string-constrained newtype whose inner type is neither `String` nor
  `FromStr` is rejected at `finalize` rather than in the consumer's build
* Schemas for tuple structs with a direct standard `Vec` remainder now describe
  each trailing element instead of the vector container. This changes schema
  validation without changing the generated Rust representation or Serde behavior.

## [0.1.0] - 2026-11-07

* Initial stable release
* `Type::details()` identifies a `Never` as `::json_serde::Never` rather than
  `::json_serde::Absent`
* codespace to 0.1 and json-serde to 1.0

## [0.0.1-alpha.4] - 2026-10-14

* Under `typify_compat`, a comparison trait the consumer required explicitly, or
  specified in a type's extra derives, is derived retained; only the derives
  that typify chose on its own are excluded (#26)

## [0.0.1-alpha.3] - 2026-10-04

* Adds `NewtypeConstraints` for `Array`, `Integer`, and `Number` (#19)
* A newtype's constraints are checked against the type it wraps at
  finalization, reported as `Error::InvalidConstraints` (#19)
* Allow a foreign derive to be a bare (unscoped) name (#18)
* The codespace from `Typespace::to_codespace` tracks the crates its code
  depends on (`Codespace::dependencies`), including those a consumer records
  for its native types with `TypespaceBuilder::add_dependency` (#22)
* Re-export `codespace` (#22)

## [0.0.1-alpha.2] - 2026-09-28

* `Settings::with_derive` takes a `ForeignTrait`; a derive that names a
  trait typespace models is a required trait, not a derive. Consumers with a
  trait name in hand use `TraitSpec::parse` and
  `Settings::with_extra_required_trait`, which routes either way. A foreign
  derive must carry its crate path and may declare the traits it requires (#16)
* `Settings::with_crate_path` overrides the path by which generated code refers
  to a particular crate (#12)
* `Settings` derives `Clone` (#11)
* `NewtypeConstraints::JsonSchema` renders: values are validated
  against the schema at construction and deserialization, and a `JsonSchema`
  impl reports the constraint through schemars's typed fields, so the trait is
  refused for a schema schemars cannot hold. A schema that names no draft is
  validated as draft-07 (#14)
* A flattened property's type must be one serde can flatten (a struct, an enum,
  or a map); finalization rejects the rest, on a struct or a struct-style enum
  variant (#13)
* Allow and deny list values are checked against the inner type at
  finalization (#9)
* Integer and float default values are checked against the type's range (#8)
* `make_box_id` is called at most once for any id during finalization (#10)
* A tuple struct's description no longer leaks its doc attribute into the
  reported schema (#7)

## [0.0.1-alpha.1] - 2026-09-19

* Initial release

[0.1.4]: https://github.com/oxidecomputer/typespace/releases/v0.1.4
[0.1.3]: https://github.com/oxidecomputer/typespace/releases/v0.1.3
[0.1.2]: https://github.com/oxidecomputer/typespace/releases/v0.1.2
[0.1.1]: https://github.com/oxidecomputer/typespace/releases/v0.1.1
[0.1.0]: https://github.com/oxidecomputer/typespace/releases/v0.1.0
[0.0.1-alpha.4]: https://github.com/oxidecomputer/typespace/releases/v0.0.1-alpha.4
[0.0.1-alpha.3]: https://github.com/oxidecomputer/typespace/releases/v0.0.1-alpha.3
[0.0.1-alpha.2]: https://github.com/oxidecomputer/typespace/releases/v0.0.1-alpha.2
[0.0.1-alpha.1]: https://github.com/oxidecomputer/typespace/releases/v0.0.1-alpha.1

# Changelog

## Next

* `NewtypeConstraints::Number` constrains a newtype over an integer or
  floating-point type: an inclusive and an exclusive bound at each end and a
  multiple-of, each optional, checked at construction and deserialization and
  reported keyword by keyword in the `JsonSchema` impl. Bounds are
  `NumericBound` values, an integer or a float
* `NewtypeConstraints::Array` renders: the length of the wrapped sequence is
  checked at construction and deserialization, and the `JsonSchema` impl
  reports `minItems` and `maxItems`
* A newtype's constraints are checked against the type it wraps at
  finalization, reported as `Error::InvalidConstraints`

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

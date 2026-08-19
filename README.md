# typespace

[![typespace on crates.io](https://img.shields.io/crates/v/typespace)](https://crates.io/crates/typespace)
[![Documentation (latest release)](https://img.shields.io/badge/docs-latest%20version-brightgreen.svg)](https://docs.rs/typespace)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE)

Semantic model of Rust types for code generation

## Overview

A code generator that turns schemas into Rust types needs a
representation for those types while it works--not source text, which is
too late to analyze, and not a general Rust AST, which is far more than
it needs. `typespace` models just the vocabulary a schema-driven
generator uses: structs, enums, newtype structs, tuple structs (with an
optional "rest" field), unit structs, and type aliases, along with the
built-in, container, and native (externally defined) types they refer
to.

Types form a graph of `Type<Id>` values keyed by a caller-chosen `Id`.
Names come from outside: typespace never invents identifiers, applies
casing, or resolves collisions. The caller inserts fully-named types
into a `TypespaceBuilder` and calls `finalize`, which validates the
graph (dangling references are errors), breaks containment cycles by
inserting `Box` types, and propagates trait requirements--a type used as
a map key must be `Ord`, and so must everything it contains.

```rust
use quote::format_ident;
use typespace::{
    no_cycles, StructProperty, StructPropertySerde,
    StructPropertyState, Type, TypeStruct, TypespaceBuilder,
    TypespaceSettings,
};

let mut builder = TypespaceBuilder::default();
builder.insert("string".to_string(), Type::String).unwrap();
builder
    .insert(
        "Thing".to_string(),
        Type::Struct(TypeStruct::new(
            "Thing",
            Some("A named thing.".to_string()),
            vec![StructProperty::new(
                format_ident!("name"),
                StructPropertySerde::None,
                StructPropertyState::Required,
                None,
                "string".to_string(),
            )],
            false,
        )),
    )
    .unwrap();

let ts = builder
    .finalize(TypespaceSettings::default(), no_cycles)
    .unwrap();
let tokens = ts.to_codespace().into_stream();
```

Caller-input problems--duplicate IDs, dangling references, impossible
trait requirements--are reported as `TypespaceError`; any panic is a
typespace bug.

## Output

A finalized `Typespace` renders with `to_codespace` into a
[codespace](https://github.com/oxidecomputer/codespace) `Codespace`--
token emission is codespace's job--with one item per named type and
helper functions (serde default functions, for example) routed to their
own modules. From there the caller flattens to a single `TokenStream` or
splits into per-module files. Output is deterministic and unformatted.
Generated code is faithful to JSON semantics: absent vs `null` fields
(with several configurable modelings, including double-`Option` and
custom wrapper types), required-but-nullable fields, and tuple rest
fields all round-trip.

Generated code has runtime dependencies of its own: serde always, and
serde_json or json-serde for particular constructs, plus whatever
crates back the native type paths the caller supplied. Cargo cannot
surface these; the crate docs section "Dependencies of generated code"
gives the exact conditions.

Finalized types can also be inspected without rendering: `get_type` and
`iter_types` return `TypeInfo` views exposing names, identifiers,
structural details, and trait impls--for consumers like progenitor that
generate code referring to the generated types.

typespace has no JSON Schema, OpenAPI, or IDL awareness; mapping a
schema onto these types is the caller's job (typify's, for instance). It
emits no files and runs no formatter. See the crate docs for details.

## Alternatives

[codegen](https://docs.rs/codegen) is a builder API for Rust items--
modules, structs, enums, functions--rendered to strings. It competes at
the same semantic item-builder altitude, but has no notion of type
identity or a type graph: no references between types, no cycle breaking
via boxing, no trait-requirement propagation, and no JSON/serde fidelity
(attribute selection, absent-vs-null handling). It is also effectively
dormant.

## Future direction

- Trait/derive configurability: `#[derive(...)]` sets are hardcoded in
  most render methods today. `TypespaceTrait` and `TypespaceTraitSet`
  exist and should be wired into `TypespaceSettings` so callers control
  both the traits *required* of generated types (driving propagation at
  finalize time) and the traits *emitted* (at render time); how those
  two axes relate is deliberately unresolved. Until then the trait-set
  API stays minimal (no `remove`, no set operations, no `Display`).
- `push_traits` is incomplete: trait requirements that route through
  `UnitStruct`, `TupleStruct`, or `TypeAlias`, or `Display`/`FromStr`
  requirements on container types, hit `todo!()`.
- Newtype constraints: `TypeNewtypeConstraints` is accepted but not yet
  rendered; constraints should be enforced at deserialization.
- Concrete map/set types: `Type::Map` always renders as `BTreeMap` and
  `Type::Set` as `Vec`; both should be `TypespaceSettings` fields.
- `TypeCommon::default` is never rendered; it should drive a generated
  `Default` impl.
- Type-to-module layout as a settings axis with multiple strategies
  (everything in one mod; custom impls routed to submods such as a
  serde mod; ...).
- Whether generated code keeps the json-serde runtime dep or inlines
  helpers--to be settled before crates.io publish.
- Test coverage for trait validation error cases beyond
  float-as-map-key.

## Status

- Pre-publication; API unstable.
- Part of the typify/progenitor code-generation stack being extracted
  from oxidecomputer/typify.

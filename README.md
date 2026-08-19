# typespace

[![typespace on crates.io](https://img.shields.io/crates/v/typespace)](https://crates.io/crates/typespace)
[![Documentation (latest release)](https://img.shields.io/badge/docs-latest%20version-brightgreen.svg)](https://docs.rs/typespace)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE)

Semantic model of Rust types for code generation

## Overview

`typespace` allows consumers to model complex Rust types and render them as
code. It not only handles the basic construction of types, it also implements
desired traits, deals with breaking containment cycles via boxing, propagates
required traits, and identifies unsatisfiable constructions. Its intermediate
representation of types can be queried ("does this type implement this
trait?") or rendered as text or a `TokenStream` (via the `codespace` crate).

It fell out of the `typify` and `progenitor` crates. The former converts JSON
Schema into Rust types; the latter generates SDKs from OpenAPI documents--for
which JSON Schema is a subset (more or less...). `typespace` has been made
more generic, and individually testable to both better serve those code
generators and for use by other code generation libraries.

Types form a graph of `Type<Id>` values keyed by a caller-chosen `Id` type.
Names come from the consumer: `typespace` never invents identifiers, applies
casing, or resolves collisions. Consumers insert types
into a `TypespaceBuilder` and call `finalize()`. This validates the
type graph (dangling references are errors), breaks containment cycles by
inserting `Box` types, and propagates trait requirements (e.g. a type used as
a `BTreeMap` key must be `Ord`).

```rust
use quote::format_ident;
use typespace::{
    build::{
        Struct, StructProperty, StructPropertySerde,
        StructPropertyState, Type,
    },
    no_cycles,
    settings::Settings,
    TypespaceBuilder,
};

let mut builder = TypespaceBuilder::default();
builder.insert("string".to_string(), Type::String).unwrap();
builder
    .insert(
        "Thing".to_string(),
        Type::Struct(Struct::new(
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
    .finalize(Settings::default(), no_cycles)
    .unwrap();
let tokens = ts.to_codespace().into_stream();
```

The tokens render (roughly) to:

```rust
/// A named thing.
#[derive(::serde::Deserialize, ::serde::Serialize)]
pub struct Thing {
    pub name: ::std::string::String,
}
```

Caller-input problems--duplicate IDs, dangling references, impossible
trait requirements--are reported as `TypespaceError`; any panic is a
`typespace` bug (please file an issue!).

## Output

Consumers render a finalized `Typespace` via the
[`codespace`](https://github.com/oxidecomputer/codespace) crate. Generated code
includes runtime dependencies on crates; the crate docs section "Dependencies
of generated code" gives the exact conditions.

Finalized types can also be inspected without rendering: `get_type` and
`iter_types` return `view::Type` views exposing names, identifiers,
structural details, and trait impls. This allows additional code generation
to properly interact with these generated types.

`typespace` has no JSON Schema, OpenAPI, or IDL awareness; mapping a
schema onto these types is the caller's job (`typify`'s, for instance). It
emits no files (that's `codespace`'s job) and runs no formatter (such as the
`prettyplease` crate). See the crate docs for details.

## Alternatives

[codegen](https://docs.rs/codegen) is a builder API for Rust items--
modules, structs, enums, functions--rendered to strings. It also operates at
the semantic item-builder level, but has no notion of type
identity or a type graph: no references between types, no cycle breaking
via boxing, no trait-requirement propagation, and no JSON/serde fidelity
(attribute selection, absent-vs-null handling).

## Future direction

- Trait/derive configurability: `#[derive(...)]` sets are hardcoded in
  most render methods today. `TypespaceTrait` and `TypespaceTraitSet`
  exist and should be wired into `settings::Settings` so callers control
  both the traits *required* of generated types (driving propagation at
  finalize time) and the traits *emitted* (at render time); how those
  two axes relate is deliberately unresolved. Until then the trait-set
  API stays minimal (no `remove`, no set operations, no `Display`).
- `push_traits` is incomplete: trait requirements that route through
  `UnitStruct`, `TupleStruct`, or `TypeAlias`, or `Display`/`FromStr`
  requirements on container types, hit `todo!()`.
- Newtype constraints: `build::NewtypeConstraints` is accepted but not yet
  rendered; constraints should be enforced at deserialization.
- Concrete map/set types: `Type::Map` always renders as `BTreeMap` and
  `Type::Set` as `Vec`; both should be `settings::Settings` fields.
- `build::TypeCommon::default` is never rendered; it should drive a generated
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
- Part of the typify/progenitor code-generation stack.

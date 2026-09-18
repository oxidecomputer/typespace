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
trait?") or rendered as source code or a `TokenStream` (via the `codespace`
crate).

It fell out of the `typify` and `progenitor` crates. The former converts JSON
Schema into Rust types; the latter generates SDKs from OpenAPI documents--for
which JSON Schema is a subset (more or less...). `typespace` has been made
more generic, and individually testable to both better serve those code
generators and for use by other code generation libraries.

Types form a graph of `Type<Id>` values keyed by a caller-chosen `Id` type.
Names come from the consumer: `typespace` never invents identifiers, applies
casing, or resolves collisions. Consumers create a `TypespaceBuilder` from
`settings::Settings`, insert types, and call `finalize()`. This validates the
type graph (dangling references are errors), breaks containment cycles by
inserting `Box` types, and propagates trait requirements (e.g. a type used as
a map key must be `Ord`).

```rust
use typespace::{
    build::{Struct, StructProperty, Type},
    no_cycles,
    TypespaceBuilder,
};

let mut builder = TypespaceBuilder::default();
builder.insert("string".to_string(), Type::String).unwrap();
builder
    .insert(
        "Thing".to_string(),
        Struct::new()
            .name("Thing")
            .description("A named thing.")
            .properties([StructProperty::new(
                "name",
                "string".to_string(),
            )])
            .build()
            .unwrap(),
    )
    .unwrap();

let ts = builder.finalize(no_cycles).unwrap();
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
trait requirements--are reported as `error::Error`; any panic is a
`typespace` bug (please file an issue!).

## Output

Consumers render a finalized `Typespace` via the
[`codespace`](https://github.com/oxidecomputer/codespace) crate. Generated code
includes runtime dependencies on crates; the crate docs section "Dependencies
of generated code" gives the exact conditions.

Generated code needs `serde_json` 1.0.145 or later if it holds a JSON
value (`build::Type::JsonValue`) as a `HashSet` element or `HashMap`
key: that release added `Hash` for `serde_json::Value`. This crate's
own `serde_json` dependency already floors at 1.0.148, so building
`typespace` satisfies it.

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

- Trait conflicts are reported exhaustively but without root-cause
  deduplication: one offending type reachable along many requirement
  paths produces one conflict per path.

## Status

- Pre-publication; API unstable.
- Part of the typify/progenitor code-generation stack.

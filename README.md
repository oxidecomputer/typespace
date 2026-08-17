# typespace

Semantic model of Rust types for code generation.

## What it is

- Models Rust types semantically: structs, enums, newtypes, tuple structs, type aliases, native/container types.
- NOT a general Rust AST - just the vocabulary a schema-driven generator needs.

## Workflow

- Construct `Type<Id>` values; the caller picks the `Id` type.
- Insert fully-named types into a `TypespaceBuilder`.
- `finalize(settings, make_box_id)` validates the graph, breaks containment cycles by boxing, propagates trait requirements.
- `to_codespace()` emits into a `codespace::Codespace`; from there, tokens or files.

## Principles

- Names come from outside; typespace never invents identifiers.
- The only output is a `Codespace` - token emission is codespace's job.
- Type-to-module layout is a settings axis with multiple strategies (everything in one mod; custom impls routed to submods such as a serde mod; ...).
- Generated code is faithful to JSON semantics: absent vs `null`, tuple rest fields, newtype constraints enforced at deserialization.
- Caller-input problems are `TypespaceError`; panics are typespace bugs.
- Deterministic output.

## Boundaries

- No JSON Schema / OpenAPI / IDL awareness - that belongs to callers like typify.
- No identifier generation, casing, or collision resolution.
- No formatting.

## Status

- Pre-publication; API unstable.
- Part of the typify/progenitor code-generation stack being extracted from oxidecomputer/typify.

## Open questions

- Trait/derive configurability: required-of-types vs emitted-derives - deliberately unresolved.
- Whether generated code keeps the json-serde runtime dep or inlines helpers - to be settled before crates.io publish.

## TODO

### Correctness

- **`push_traits` incomplete** - `UnitStruct`, `TupleStruct`, and `TypeAlias`
  all hit `todo!()`, so any type graph that routes through those during
  finalization will panic. `Display`/`FromStr` requirements on container types
  (`Vec`, `Array`, `Tuple`) also hit `todo!()`.

- **`TypeNewtypeConstraints` is unrendered** - the enum is accepted by
  constructors but ignored entirely by `render()`; callers passing real
  constraints get no effect and no error.

### Settings / Configurability

- **Trait derivation** - `#[derive(Serialize, Deserialize)]` (and `Clone`,
  `Debug` on unit structs) are hardcoded in every render method with
  inconsistent strategies across variants. `TypespaceTrait` and
  `TypespaceTraitSet` already exist; wire them into `TypespaceSettings` so
  callers control which traits appear in the output.

- **Map / Set concrete types** - `Type::Map` always renders as `BTreeMap` and
  `Type::Set` as `Vec` with a TODO. Add a `TypespaceSettings` field to choose
  between `BTreeMap`/`HashMap` and `BTreeSet`/`HashSet`.

- **Finalize-time trait configuration** - trait inclusion and propagation
  (which traits are *required* of generated types, driving the push/poison walk
  in `finalize`) should be configurable at finalize time, distinct from the
  render-time setting of which traits to emit.

### Cleanup

- **`TypeCommon::default`** - the `default: Option<JsonValue>` field on
  `TypeCommon` is never rendered; it should drive a generated `Default` impl.

- **`TypespaceTrait` / `TypespaceTraitSet` in lib.rs** - currently defined but
  only used inside `push_traits`. The API is incomplete (no `remove`, no set
  operations, no `Display`). Once trait configurability is added these become
  part of the public settings surface; until then, consider keeping them
  `pub(crate)`.

### Test coverage

- Additional trait validation error cases beyond Float-as-map-key

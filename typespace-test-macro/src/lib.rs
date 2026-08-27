// Copyright 2026 Oxide Computer Company

use proc_macro::TokenStream;

mod builder;
mod snapshot;

/// Attribute macro for snapshot-testing rendered Rust code.
///
/// Usage:
/// ```ignore
/// #[check_and_include("tests/output/test_unit_struct.rs", ts.render())]
/// fn inner() {
///     let value = import::MyUnitStruct;
///     assert_eq!(serde_json::to_string(&value).unwrap(), "\"<<+>>\"");
/// }
/// ```
///
/// The annotated function is replaced by an inline block that:
/// 1. Evaluates the expression, pretty-prints it, and compares against
///    the snapshot file; updates + panics if different.
/// 2. Embeds the snapshot file content as `mod import { ... }` (read at
///    macro expansion time).
/// 3. Runs the original function body.
///
/// The annotated function must have no parameters.
#[proc_macro_attribute]
pub fn check_and_include(attr: TokenStream, item: TokenStream) -> TokenStream {
    snapshot::expand(attr, item)
}

/// Build a `TypespaceBuilder<String>` from Rust-like type descriptions.
///
/// ```ignore
/// let builder = typespace_builder!(Settings::typical(), {
///     struct Inner {
///         count: u32,
///     }
///
///     #[default = { name: "anon", inner: { count: 0 } }]
///     struct Outer {
///         name: String,
///         inner: Optional<Inner>,
///     }
///
///     enum Color { Red, Green }
/// });
/// ```
///
/// Expands to `{ let mut builder = TypespaceBuilder::<String>::new(S); ...;
/// builder }`: every described type is inserted, plus every anonymous
/// node its types imply; call `.finalize(...)` yourself on the result.
/// Usable from any crate that depends on `typespace` (generated code
/// is qualified as `::typespace::...`).
///
/// # Ids
///
/// `TypespaceBuilder` is generic over an Id; this macro fixes it to
/// `String` and assigns one to each type as follows:
///
/// - Named item (`struct`/`enum`/`type`): its name verbatim, e.g.
///   `"Outer"`.
/// - Anonymous node (container, primitive, `Nullable` wrapper): an id
///   reconstructed from its parsed type, not lifted from source text,
///   e.g. `"Vec<u32>"`, `"Map<KeyStruct, String>"`. Because it's
///   reconstructed rather than copied, different ways of writing the
///   same type collapse to one id: source whitespace is never
///   part of it, and an array length written in hex normalizes to
///   decimal. Inserted once even if referenced repeatedly.
/// - `native` item: its path verbatim, keeping the leading `::` (if one
///   was written), including any generic arguments, e.g.
///   `"::foo::Wrapper<Inner>"`.
/// - `Nullable<T>` and `OptionalNullable<T>` both wrap `T` in the same
///   anonymous `Option` node, id `"Nullable<T>"`: referencing a given
///   `T` through either form reuses one node instead of inserting
///   two `Option`-shaped types for it.
///
/// An item's name can't be one the macro's own vocabulary already uses
/// as an id (`String`, `Vec`, `Optional`, ...), and can't repeat a name
/// already declared in the same invocation; both are spanned compile
/// errors rather than the runtime `DuplicateTypeId` they'd otherwise
/// surface as.
///
/// # Item forms
///
/// | Syntax                               | Builds                    |
/// |--------------------------------------|---------------------------|
/// | `struct N { f: Ty, .. }`             | `Struct`                  |
/// | `struct N(Ty);`                      | `NewtypeStruct`           |
/// | `struct N(Ty, Ty, ..);`              | `TupleStruct`             |
/// | `struct N;` (requires `#[json = V]`) | `UnitStruct::new(V)`      |
/// | `enum N { .. }`                      | `Enum`                    |
/// | `type N = Ty;`                       | `TypeAlias`               |
/// | `native P;` / `native P: Tr + Tr;`   | `Native`                  |
///
/// Enum variants: `V` unit; `V(Ty)` single payload (`VariantDetails::Item`);
/// `V(Ty, Ty, ..)` tuple payload; `V { f: Ty, .. }` struct payload. A
/// struct-shaped variant's fields follow the same rules as a struct's.
///
/// # Types
///
/// A bare name not listed below is a named reference to that item's id.
/// A path of two or more segments (`chrono::NaiveDate`,
/// `::std::path::PathBuf`) names a native type, which a `native` item
/// has to declare first. Two other paths are rejected everywhere a type
/// appears: `::Bar`, one segment behind a leading `::`, is not a Rust
/// type path, and `<T as Trait>::Bar` names nothing this grammar can
/// resolve.
///
/// Primitives (each an anonymous node): `String`, `bool`,
/// `u8..=usize`/`i8..=isize`, `f32`/`f64`, `()`, `JsonValue`. Containers
/// (each an anonymous node): `Vec<T>`, `Box<T>`, `Map<K, V>`, `Set<T>`,
/// `[T; N]`, `(A, B, ..)`. `Map`/`Set` are typespace markers, not Rust
/// types--the rendered container is a settings decision, so
/// `HashMap`/`BTreeMap`/`HashSet`/`BTreeSet` are rejected with an error
/// pointing at `Map`/`Set` instead. `!` is an unsatisfiable type; use it
/// for a struct property that must be absent or an array that must be empty.
///
/// Non-Rust vocabulary for optionality and nullability:
///
/// | Syntax                 | Where           | Meaning              |
/// |------------------------|-----------------|-----------------------|
/// | `Optional<T>`          | field top level | may be omitted        |
/// | `Nullable<T>`          | anywhere        | may be `T` or `null`  |
/// | `OptionalNullable<T>`  | field top level | omitted or `null`     |
///
/// `Option<T>`, and the bare (argument-less) forms `Optional`,
/// `Nullable`, and `OptionalNullable`, are compile errors--each names
/// a real ambiguity or an incomplete type, not a valid reference.
///
/// # Native types
///
/// `native P;` declares an externally defined type that generated code
/// emits as the Rust path `P`; `native P: Tr + Tr;` also states the
/// traits it implements.
///
/// ```ignore
/// native chrono::NaiveDate;
/// native ::std::path::PathBuf: Clone + Debug + Display + FromStr;
/// native ::foo::Wrapper<Inner>: Clone + Debug;
/// ```
///
/// `P` needs two or more segments, and its leading `::` is part of it:
/// the path is emitted verbatim, so `chrono::NaiveDate` and
/// `::chrono::NaiveDate` are two different native types, with different
/// ids and different generated text. Generic arguments become the
/// type's parameters, each lowered like any other type, so
/// `::foo::Wrapper<Inner>` emits the name `::foo::Wrapper` alongside the
/// single parameter id `Inner`.
///
/// Once declared, `P` goes wherever a type goes: a field's type, a
/// variant payload, an alias target, a container's element, key, or
/// value, a tuple component, and inside
/// `Optional`/`Nullable`/`OptionalNullable`. Using a path no `native`
/// item declared is an error at the use rather than a trait-less native
/// conjured on the spot, and declaring one path twice is an error at
/// the second declaration. Native paths are their own namespace, so no
/// `native` item can collide with a `struct`/`enum`/`type` name.
///
/// A native declares only the traits it lists: `native P: Ord;` declares
/// `Ord`, and neither `PartialOrd` nor `Eq`. The names it accepts are
/// typespace's trait vocabulary: `Clone`, `Debug`, `Serialize`,
/// `Deserialize`, `JsonSchema`, `Display`, `FromStr`, `Eq`, `PartialEq`,
/// `Ord`, `PartialOrd`, `Hash`, and `Default`.
///
/// # Attributes
///
/// - Field `#[default]`: `StructPropertyState::Default`.
/// - Field `#[default = V]`: `StructPropertyState::DefaultValue(V)`.
/// - Type-level `#[default = V]`: the type's `.default(V)`.
/// - Unit struct `#[json = V]`: its wire repr (required; any JSON).
/// - Unit variant `#[json = "name"]`: its serde rename (string only).
/// - Enum tagging: `EnumTagType::External` (default), `#[untagged]`,
///   `#[tag = "t"]` (internal), `#[tag = "t", content = "c"]`
///   (adjacent).
///
/// An attribute used somewhere other than the list above--an unknown
/// name, or a real one in the wrong place (`#[untagged]` on a struct) --
/// is a compile error naming the mistake, as is repeating one.
///
/// `V` is JSON-ish: objects (`{ k: v, .. }`, unquoted-ident or
/// string-literal keys), arrays, strings, numbers,
/// `true`, `false`, `null`, nested arbitrarily, trailing commas permitted.
#[proc_macro]
pub fn typespace_builder(input: TokenStream) -> TokenStream {
    builder::expand(input.into()).into()
}

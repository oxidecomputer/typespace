// Copyright 2026 Oxide Computer Company

//! Implementation of the `typespace_builder!` macro.
//!
//! The contract (item forms, type vocabulary, id conventions,
//! attributes) is documented on the macro itself; this module is the
//! parser and lowering pass behind it.
//!
//! The input is Rust-like, but not quite Rust: the custom attributes
//! (`#[default = { .. }]`, `#[tag = "t", content = "c"]`, ...) are not
//! legal attribute grammar (`syn::Attribute`'s `meta` field requires a
//! `Meta`, and a brace-delimited key/value literal is not an `Expr`).
//! So this parses the whole input by hand with `syn::parse::Parse`,
//! using `bracketed!`/`braced!`/`parenthesized!` to step into delimited
//! groups without ever routing through `syn::Attribute` or `syn::Meta`.
//! Field and variant payload types, which *are* ordinary Rust type
//! grammar (`Vec<T>`, `Optional<T>`, `[T; N]`, ...), are parsed with
//! plain `syn::Type`.
//!
//! Lowering is single-pass: each item is turned directly into a
//! `builder.insert(id, ...).unwrap();` statement, threading a
//! [`Lowering`] accumulator that also collects the anonymous nodes
//! (containers, primitives, `Nullable` wrappers) that item's types
//! imply, de-duplicated by id so a repeated `Vec<u32>` or `u32` is only
//! inserted once, and the named items declared so far (catching a
//! reserved or duplicate item name at the point it's declared). A
//! `native` item's path is tracked the same way, in its own namespace:
//! using a path no `native` item has declared is an error (we don't create
//! a trait-less node conjured into the graph).
//!
//! The generated code always qualifies typespace's own vocabulary as
//! `::typespace::...`. It also resolves inside the `typespace` crate's own
//! tests, because that crate's root declares `extern crate self as
//! typespace;`. Ids are always `String` (`Id = String`), so builder calls that
//! would otherwise leave `Id` ambiguous (`Enum::new()`,
//! `VariantDetails::Unit`, ...) are turbofished to `::<String>` rather than
//! relying on inference through the whole chain.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, ExprLit, Ident, Lit, LitFloat, LitInt, LitStr, Token, Type,
};

/// Entry point shared by the `#[proc_macro]` wrapper and by tests: parse
/// `input`, lower it, and return the builder-construction expression --
/// or a block of `compile_error!`s standing in its place if anything
/// about the input is invalid.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let parsed = match syn::parse2::<BuilderInput>(input) {
        Ok(parsed) => parsed,
        Err(err) => return wrap_compile_error(err),
    };
    match parsed.lower() {
        Ok(tokens) => tokens,
        Err(err) => wrap_compile_error(err),
    }
}

/// Render `err` as `compile_error!` invocations wrapped in a block.
///
/// `syn::Error::combine` (used for a duplicate item or attribute, to
/// also point at the first occurrence) makes `err` expand to more than
/// one `compile_error!` in sequence; typespace_builder! is always
/// invoked in expression position, where two sequential macro calls
/// with nothing joining them is not valid syntax. The block makes the
/// combined output one expression regardless of how many messages
/// `err` carries.
fn wrap_compile_error(err: syn::Error) -> TokenStream {
    let compile_errors = err.to_compile_error();
    quote! { { #compile_errors } }
}

// ---------------------------------------------------------------------
// Input grammar
// ---------------------------------------------------------------------

struct BuilderInput {
    settings: Expr,
    items: Vec<Item>,
}

impl Parse for BuilderInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let settings: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        braced!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(content.parse()?);
        }
        Ok(BuilderInput { settings, items })
    }
}

enum Item {
    Struct(StructItem),
    Enum(EnumItem),
    Alias(AliasItem),
    Native(NativeItem),
}

struct StructItem {
    name: Ident,
    attrs: Vec<AttrEntry>,
    body: StructBody,
}

enum StructBody {
    Fields(Vec<FieldItem>),
    Tuple(Vec<TupleField>),
    Unit,
}

struct FieldItem {
    attrs: Vec<AttrEntry>,
    name: Ident,
    ty: Type,
}

/// One element of a tuple struct's body: a type, with the attributes
/// (currently only `#[flatten]`, and only on the last element) that
/// give it non-default treatment.
struct TupleField {
    attrs: Vec<AttrEntry>,
    ty: Type,
}

struct EnumItem {
    name: Ident,
    attrs: Vec<AttrEntry>,
    variants: Vec<VariantItem>,
}

struct VariantItem {
    attrs: Vec<AttrEntry>,
    name: Ident,
    payload: VariantPayload,
}

enum VariantPayload {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<FieldItem>),
}

struct AliasItem {
    name: Ident,
    attrs: Vec<AttrEntry>,
    target: Type,
}

/// A `native P;` or `native P: Trait1 + Trait;` declaration: an externally
/// defined type, named by the Rust path `P` generated code emits
/// for it, declaring what it implements.
struct NativeItem {
    attrs: Vec<AttrEntry>,
    /// The declared path, kept as a `Type` so the one path grammar in
    /// [`name_or_native`] judges a declaration and a use alike.
    ty: Type,
    bounds: Vec<NativeBound>,
}

/// One entry in a `native` item's bound list.
enum NativeBound {
    /// `Tr`: the type implements the trait.
    Yes(syn::Path),
    /// `!Tr`: the type does not implement the trait.
    No(syn::Path),
    /// `?Tr`: the declaration cannot answer for the trait.
    Unknown(syn::Path),
    /// `..`: every trait the list does not name is unknown.
    Rest,
}

impl Parse for NativeBound {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![..]) {
            input.parse::<Token![..]>()?;
            Ok(NativeBound::Rest)
        } else if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            Ok(NativeBound::No(input.parse()?))
        } else if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            Ok(NativeBound::Unknown(input.parse()?))
        } else {
            Ok(NativeBound::Yes(input.parse()?))
        }
    }
}

/// Parse a brace-delimited, comma-terminated list of `name: Type`
/// fields: a struct's own fields, or a struct-shaped variant's.
fn parse_fields(input: ParseStream) -> syn::Result<Vec<FieldItem>> {
    let content;
    braced!(content in input);
    let mut fields = Vec::new();
    while !content.is_empty() {
        let attrs = parse_attrs(&content)?;
        let name: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let ty: Type = content.parse()?;
        fields.push(FieldItem { attrs, name, ty });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    Ok(fields)
}

/// Parse a parenthesized, comma-separated list of types: a variant's
/// tuple payload. Arity is classified later, at lowering
/// (`VariantPayload::Tuple`'s doc comment explains the split).
fn parse_tuple_payload(input: ParseStream) -> syn::Result<Vec<Type>> {
    let content;
    parenthesized!(content in input);
    let types = Punctuated::<Type, Token![,]>::parse_terminated(&content)?;
    Ok(types.into_iter().collect())
}

/// Parse a parenthesized, comma-separated list of a struct's tuple
/// fields, each an optionally-attributed type. Arity is classified
/// later, at lowering (`StructBody::Tuple`'s doc comment explains the
/// split).
fn parse_struct_tuple_payload(input: ParseStream) -> syn::Result<Vec<TupleField>> {
    let content;
    parenthesized!(content in input);
    let mut fields = Vec::new();
    while !content.is_empty() {
        let attrs = parse_attrs(&content)?;
        let ty: Type = content.parse()?;
        fields.push(TupleField { attrs, ty });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    Ok(fields)
}

/// Make a custom keyword for `native`
mod keyword {
    syn::custom_keyword!(native);
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = parse_attrs(input)?;
        if input.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            let name: Ident = input.parse()?;
            let body = if input.peek(syn::token::Brace) {
                StructBody::Fields(parse_fields(input)?)
            } else if input.peek(syn::token::Paren) {
                let fields = parse_struct_tuple_payload(input)?;
                input.parse::<Token![;]>()?;
                StructBody::Tuple(fields)
            } else {
                input.parse::<Token![;]>()?;
                StructBody::Unit
            };
            Ok(Item::Struct(StructItem { name, attrs, body }))
        } else if input.peek(Token![enum]) {
            input.parse::<Token![enum]>()?;
            let name: Ident = input.parse()?;
            let content;
            braced!(content in input);
            let mut variants = Vec::new();
            while !content.is_empty() {
                let variant_attrs = parse_attrs(&content)?;
                let variant_name: Ident = content.parse()?;
                let payload = if content.peek(syn::token::Paren) {
                    VariantPayload::Tuple(parse_tuple_payload(&content)?)
                } else if content.peek(syn::token::Brace) {
                    VariantPayload::Struct(parse_fields(&content)?)
                } else {
                    VariantPayload::Unit
                };
                variants.push(VariantItem {
                    attrs: variant_attrs,
                    name: variant_name,
                    payload,
                });
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }
            Ok(Item::Enum(EnumItem {
                name,
                attrs,
                variants,
            }))
        } else if input.peek(Token![type]) {
            input.parse::<Token![type]>()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let target: Type = input.parse()?;
            input.parse::<Token![;]>()?;
            Ok(Item::Alias(AliasItem {
                name,
                attrs,
                target,
            }))
        } else if input.peek(keyword::native) {
            input.parse::<keyword::native>()?;
            let ty: Type = input.parse()?;
            let bounds = if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                Punctuated::<NativeBound, Token![+]>::parse_separated_nonempty(input)?
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            input.parse::<Token![;]>()?;
            Ok(Item::Native(NativeItem { attrs, ty, bounds }))
        } else {
            Err(input.error("expected `struct`, `enum`, `type`, or `native`"))
        }
    }
}

// ---------------------------------------------------------------------
// Custom attributes: `#[name]` / `#[name = json-ish, name = json-ish]`
// ---------------------------------------------------------------------

/// One `name` or `name = value` entry from a `#[...]` group. A single
/// bracket may hold several comma-separated entries (adjacent tagging
/// is `#[tag = "t", content = "c"]`).
struct AttrEntry {
    name: Ident,
    value: Option<serde_json::Value>,
}

/// Every attribute name typespace_builder! recognizes, paired with a
/// short description of where it's valid. Backs the distinction
/// [`claim_attrs`] draws between an unknown attribute (a typo) and one
/// that exists but is used in the wrong place.
const KNOWN_ATTRS: &[(&str, &str)] = &[
    ("default", "a struct, an enum, or a field"),
    ("deny_unknown_fields", "a struct or an enum"),
    ("json", "a unit struct or an enum variant"),
    ("rename", "a field"),
    ("flatten", "a field"),
    ("tuple", "an enum variant or a tuple struct"),
    ("untagged", "an enum"),
    ("tag", "an enum"),
    ("content", "an enum"),
];

/// The attributes one context (an item, a field, a variant) has
/// claimed, keyed by name; see [`claim_attrs`].
type Claims<'a> = BTreeMap<&'static str, &'a AttrEntry>;

/// Claim exactly the attributes named in `allowed` out of `attrs`,
/// erroring on anything else: a [`KNOWN_ATTRS`] name used somewhere
/// other than `allowed` names where it *is* valid; any other name is
/// unknown; a name repeated in `attrs` errors at the second occurrence,
/// pointing back at the first via [`syn::Error::combine`].
fn claim_attrs<'a>(
    attrs: &'a [AttrEntry],
    allowed: &'static [&'static str],
) -> syn::Result<Claims<'a>> {
    let mut claimed = Claims::new();
    for attr in attrs {
        let name = attr.name.to_string();
        let Some(&key) = allowed.iter().find(|candidate| **candidate == name) else {
            return Err(match KNOWN_ATTRS.iter().find(|(known, _)| *known == name) {
                Some((_, home)) => syn::Error::new_spanned(
                    &attr.name,
                    format!("`#[{name}]` is only valid on {home}"),
                ),
                None => {
                    syn::Error::new_spanned(&attr.name, format!("unknown attribute `#[{name}]`"))
                }
            });
        };
        if let Some(first) = claimed.insert(key, attr) {
            let mut err =
                syn::Error::new_spanned(&attr.name, format!("duplicate `#[{name}]` attribute"));
            err.combine(syn::Error::new_spanned(
                &first.name,
                "previously specified here",
            ));
            return Err(err);
        }
    }
    Ok(claimed)
}

/// Consume every leading `#[...]` group before an item, field, or
/// variant.
fn parse_attrs(input: ParseStream) -> syn::Result<Vec<AttrEntry>> {
    let mut entries = Vec::new();
    while input.peek(Token![#]) {
        entries.extend(parse_one_attr_group(input)?);
    }
    Ok(entries)
}

fn parse_one_attr_group(input: ParseStream) -> syn::Result<Vec<AttrEntry>> {
    input.parse::<Token![#]>()?;
    let content;
    bracketed!(content in input);
    let mut entries = Vec::new();
    loop {
        let name: Ident = content.parse()?;
        let value = if content.peek(Token![=]) {
            content.parse::<Token![=]>()?;
            Some(parse_json_value(&content)?)
        } else {
            None
        };
        entries.push(AttrEntry { name, value });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            if content.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
    if !content.is_empty() {
        return Err(content.error("unexpected token in attribute"));
    }
    Ok(entries)
}

/// Require that `entry`'s value is a JSON string, for the attributes
/// (`#[tag = "t"]`, `#[rename = "n"]`, a unit variant's
/// `#[json = "name"]`) whose model only supports a string, not
/// arbitrary JSON.
fn expect_string_value(entry: &AttrEntry) -> syn::Result<String> {
    match &entry.value {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(syn::Error::new_spanned(
            &entry.name,
            format!("`#[{}]` requires a string value", entry.name),
        )),
        None => Err(syn::Error::new_spanned(
            &entry.name,
            format!(
                "`#[{}]` requires a value: #[{} = \"...\"]",
                entry.name, entry.name
            ),
        )),
    }
}

/// A type-level `#[default = <json-ish>]`, rendered as the type
/// builder's `.default(...)` call (or nothing, if unclaimed).
fn claimed_default(claims: &Claims) -> syn::Result<TokenStream> {
    match claims.get("default").copied() {
        None => Ok(TokenStream::new()),
        Some(entry) => {
            let value = entry.value.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(
                    &entry.name,
                    "type-level #[default] requires a value: #[default = <json-ish>]",
                )
            })?;
            let tokens = value_tokens(value);
            Ok(quote! { .default(#tokens) })
        }
    }
}

/// A claimed `#[deny_unknown_fields]`, rendered as the type builder's
/// `.deny_unknown_fields()` call (or nothing, if unclaimed). The
/// attribute is a bare marker.
fn claimed_deny_unknown_fields(claims: &Claims) -> syn::Result<TokenStream> {
    match claims.get("deny_unknown_fields").copied() {
        None => Ok(TokenStream::new()),
        Some(entry) => {
            require_bare(entry)?;
            Ok(quote! { .deny_unknown_fields() })
        }
    }
}

/// A field's `#[default]` / `#[default = <json-ish>]`, overriding
/// `base` (the state implied by the field's wire-vocabulary type, e.g.
/// `Optional<T>`) when claimed.
fn field_state_override(claims: &Claims, base: TokenStream) -> syn::Result<TokenStream> {
    match claims.get("default").copied() {
        None => Ok(base),
        Some(entry) => match &entry.value {
            None => Ok(quote! { ::typespace::build::StructPropertyState::Default }),
            Some(value) => {
                let tokens = value_tokens(value);
                Ok(quote! {
                    ::typespace::build::StructPropertyState::DefaultValue(
                        ::typespace::build::JsonValue::new(#tokens)
                    )
                })
            }
        },
    }
}

/// A claimed bare-marker attribute's value, if any, is an error: an
/// attribute like `#[flatten]`, `#[deny_unknown_fields]`, or
/// `#[tuple]` carries no value at all. Shared by every bare-marker
/// attribute's claim site: a named property's `#[flatten]` (in
/// [`field_json_name`]), a tuple struct's `#[flatten]` on its last
/// field (in [`claimed_tuple_flatten`]), `#[deny_unknown_fields]` (in
/// [`claimed_deny_unknown_fields`]), and `#[tuple]` (in
/// [`claimed_tuple_marker`]).
fn require_bare(entry: &AttrEntry) -> syn::Result<()> {
    match &entry.value {
        None => Ok(()),
        Some(_) => Err(syn::Error::new_spanned(
            &entry.name,
            format!("#[{}] takes no value", entry.name),
        )),
    }
}

/// A claimed `#[tuple]`, validated against the payload arity it
/// applies to. The attribute exists to keep a single-type payload from
/// collapsing into the `Item` form, so it is meaningful only there:
/// `arity` of anything but 1 means there is no collapse to prevent.
fn claimed_tuple_marker(claims: &Claims, arity: usize) -> syn::Result<bool> {
    match claims.get("tuple").copied() {
        None => Ok(false),
        Some(entry) => {
            require_bare(entry)?;
            match arity {
                1 => Ok(true),
                _ => Err(syn::Error::new_spanned(
                    &entry.name,
                    "#[tuple] is only valid on a single-type payload, \
                     `(Ty)`, which it keeps from collapsing into the \
                     newtype form; every other arity is already a tuple",
                )),
            }
        }
    }
}

/// A field's `#[rename = "name"]` / `#[flatten]`, rendered as the
/// property's `.with_json_name(...)` call (or nothing, if neither was
/// claimed). `StructPropertySerde` holds one treatment of a property's
/// name, so claiming both is a contradiction.
fn field_json_name(claims: &Claims) -> syn::Result<Option<TokenStream>> {
    let rename = claims.get("rename").copied();
    let flatten = claims.get("flatten").copied();
    match (rename, flatten) {
        (None, None) => Ok(None),
        (Some(rename), Some(_)) => Err(syn::Error::new_spanned(
            &rename.name,
            "#[rename] cannot be combined with #[flatten]",
        )),
        (Some(entry), None) => {
            let json_name = expect_string_value(entry)?;
            Ok(Some(quote! {
                .with_json_name(
                    ::typespace::build::StructPropertySerde::Rename(#json_name.to_string())
                )
            }))
        }
        (None, Some(entry)) => {
            require_bare(entry)?;
            Ok(Some(quote! {
                .with_json_name(::typespace::build::StructPropertySerde::Flatten)
            }))
        }
    }
}

/// A tuple struct's `#[flatten]`, validated across all its fields:
/// claimed on at most one, only the last, holding no value. Returns
/// the claimed entry, if any.
fn claimed_tuple_flatten(fields: &[TupleField]) -> syn::Result<Option<&AttrEntry>> {
    let flattens = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let claims = claim_attrs(&field.attrs, &["flatten"])?;
            Ok(claims.get("flatten").copied().map(|entry| (index, entry)))
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    match flattens.as_slice() {
        [] => Ok(None),
        [(index, entry)] => {
            require_bare(entry)?;
            match *index == fields.len() - 1 {
                true => Ok(Some(*entry)),
                false => Err(syn::Error::new_spanned(
                    &entry.name,
                    "#[flatten] is only valid on a tuple struct's last field",
                )),
            }
        }
        [(_, first), (_, second), ..] => {
            let mut err = syn::Error::new_spanned(&second.name, "duplicate `#[flatten]` attribute");
            err.combine(syn::Error::new_spanned(
                &first.name,
                "previously specified here",
            ));
            Err(err)
        }
    }
}

/// The enum's tag type from its `#[untagged]` / `#[tag = ..]` /
/// `#[tag = .., content = ..]` claims; `EnumTagType::External` if none
/// were claimed.
fn enum_tag_type(claims: &Claims) -> syn::Result<TokenStream> {
    let untagged = claims.get("untagged").copied();
    let tag = claims.get("tag").copied();
    let content = claims.get("content").copied();
    match (untagged, tag, content) {
        (Some(entry), None, None) => match entry.value {
            Some(_) => Err(syn::Error::new_spanned(
                &entry.name,
                "#[untagged] takes no value",
            )),
            None => Ok(quote! { ::typespace::build::EnumTagType::Untagged }),
        },
        (Some(entry), _, _) => Err(syn::Error::new_spanned(
            &entry.name,
            "#[untagged] cannot be combined with #[tag]/#[content]",
        )),
        (None, None, None) => Ok(quote! { ::typespace::build::EnumTagType::External }),
        (None, Some(tag), None) => {
            let tag = expect_string_value(tag)?;
            Ok(quote! { ::typespace::build::EnumTagType::Internal { tag: #tag.to_string() } })
        }
        (None, Some(tag), Some(content)) => {
            let tag = expect_string_value(tag)?;
            let content = expect_string_value(content)?;
            Ok(quote! {
                ::typespace::build::EnumTagType::Adjacent {
                    tag: #tag.to_string(),
                    content: #content.to_string(),
                }
            })
        }
        (None, None, Some(content)) => Err(syn::Error::new_spanned(
            &content.name,
            "#[content] requires #[tag] (adjacent tagging needs both)",
        )),
    }
}

// ---------------------------------------------------------------------
// JSON-ish value grammar
// ---------------------------------------------------------------------

/// Parse the JSON-ish grammar: objects with unquoted-ident or
/// string-literal keys, arrays, strings, numbers (negative allowed),
/// `true`/`false`/`null`, nested arbitrarily, trailing commas tolerated.
fn parse_json_value(input: ParseStream) -> syn::Result<serde_json::Value> {
    if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        let mut map = serde_json::Map::new();
        while !content.is_empty() {
            // Allow strings or idents for convenience.
            let key = if content.peek(LitStr) {
                content.parse::<LitStr>()?.value()
            } else {
                content.parse::<Ident>()?.to_string()
            };
            content.parse::<Token![:]>()?;
            let value = parse_json_value(&content)?;
            map.insert(key, value);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        if !content.is_empty() {
            return Err(content.error("expected `,` or `}`"));
        }
        Ok(serde_json::Value::Object(map))
    } else if input.peek(syn::token::Bracket) {
        let content;
        bracketed!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(parse_json_value(&content)?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        if !content.is_empty() {
            return Err(content.error("expected `,` or `]`"));
        }
        Ok(serde_json::Value::Array(items))
    } else if input.peek(LitStr) {
        Ok(serde_json::Value::String(input.parse::<LitStr>()?.value()))
    } else if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        parse_json_number(input, true)
    } else if input.peek(LitInt) || input.peek(LitFloat) {
        parse_json_number(input, false)
    } else if input.peek(syn::LitBool) {
        Ok(serde_json::Value::Bool(
            input.parse::<syn::LitBool>()?.value,
        ))
    } else if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        if ident == "null" {
            Ok(serde_json::Value::Null)
        } else {
            Err(syn::Error::new_spanned(
                ident,
                "expected a JSON-ish value (object, array, string, \
                 number, true, false, or null)",
            ))
        }
    } else {
        // None of the peeks above matched; ask a Lookahead1 to name
        // every alternative it was offered, so the error enumerates
        // what was actually expected instead of a generic message.
        let lookahead = input.lookahead1();
        lookahead.peek(syn::token::Brace);
        lookahead.peek(syn::token::Bracket);
        lookahead.peek(LitStr);
        lookahead.peek(Token![-]);
        lookahead.peek(LitInt);
        lookahead.peek(LitFloat);
        lookahead.peek(syn::LitBool);
        lookahead.peek(Ident);
        Err(lookahead.error())
    }
}

fn parse_json_number(input: ParseStream, negative: bool) -> syn::Result<serde_json::Value> {
    if input.peek(LitFloat) {
        let lit: LitFloat = input.parse()?;
        let value: f64 = lit.base10_parse()?;
        let value = if negative { -value } else { value };
        let number = serde_json::Number::from_f64(value)
            .ok_or_else(|| syn::Error::new_spanned(lit, "invalid JSON number"))?;
        Ok(serde_json::Value::Number(number))
    } else if input.peek(LitInt) {
        let lit: LitInt = input.parse()?;
        let value: i64 = lit.base10_parse()?;
        let value = if negative { -value } else { value };
        Ok(serde_json::Value::Number(value.into()))
    } else {
        Err(input.error("expected a number"))
    }
}

// ---------------------------------------------------------------------
// Lowering: items and types -> insert statements
// ---------------------------------------------------------------------

/// Names reserved for typespace_builder!'s own type vocabulary: an
/// item can't take one of these names, because it would collide with
/// the anonymous node the same identifier already names (a `struct
/// String` would fight the primitive id `"String"`).
const RESERVED_NAMES: &[&str] = &[
    "String",
    "bool",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f32",
    "f64",
    "JsonValue",
    "Vec",
    "Box",
    "Map",
    "Set",
    "Option",
    "Optional",
    "Never",
    "Nullable",
    "OptionalNullable",
];

/// Accumulates the `builder.insert(...)` statements as items and their
/// types are lowered, de-duplicating anonymous nodes (containers,
/// primitives, `Nullable` wrappers) by id so a repeated reference to
/// the same anonymous type is only inserted once, and tracking the
/// named items and native paths declared so far to catch a reserved or
/// repeated name, or a use of an undeclared native path.
#[derive(Default)]
struct Lowering {
    inserts: Vec<TokenStream>,
    anon_ids: BTreeSet<String>,
    named_ids: BTreeMap<String, Ident>,
    /// Every `native` item declared so far, keyed by its id, holding
    /// the path it was declared with so a second declaration of the
    /// same path can point back at the first.
    native_ids: BTreeMap<String, syn::Path>,
}

impl Lowering {
    /// Insert the anonymous node `id` (built by `type_expr`) unless an
    /// equal id has already been inserted.
    fn ensure_anon(&mut self, id: &str, type_expr: TokenStream) {
        if self.anon_ids.insert(id.to_string()) {
            self.inserts.push(quote! {
                builder.insert(#id.to_string(), #type_expr).unwrap();
            });
        }
    }

    /// Claim `name` as a named item's id, and return it as a `String`.
    /// Errors, spanned at `name`, if it collides with the reserved type
    /// vocabulary ([`RESERVED_NAMES`]) or with an item already declared
    /// in this invocation (pointing back at that first declaration via
    /// [`syn::Error::combine`])--both are graph-id collisions that
    /// `builder.insert` would otherwise only catch at test run time.
    fn claim_named_item(&mut self, name: &Ident) -> syn::Result<String> {
        let text = name.to_string();
        if RESERVED_NAMES.contains(&text.as_str()) {
            return Err(syn::Error::new_spanned(
                name,
                format!(
                    "`{text}` is reserved for typespace_builder!'s own \
                     type vocabulary and can't name an item"
                ),
            ));
        }
        if let Some(first) = self.named_ids.insert(text.clone(), name.clone()) {
            let mut err = syn::Error::new_spanned(name, format!("duplicate item `{text}`"));
            err.combine(syn::Error::new_spanned(first, "previously defined here"));
            return Err(err);
        }
        Ok(text)
    }

    /// Claim `path` as a native type's id, and return it as a `String`.
    /// Errors, spanned at `path`, if the same path was already declared
    /// in this invocation, pointing back at that first declaration via
    /// [`syn::Error::combine`]. Native paths are their own namespace:
    /// every one has two or more segments, so none can collide with a
    /// named item's single identifier or with [`RESERVED_NAMES`].
    fn claim_native(&mut self, path: &syn::Path) -> syn::Result<String> {
        let id = native_id(path)?;
        match self.native_ids.insert(id.clone(), path.clone()) {
            None => Ok(id),
            Some(first) => {
                let mut err =
                    syn::Error::new_spanned(path, format!("duplicate native type `{id}`"));
                err.combine(syn::Error::new_spanned(first, "previously declared here"));
                Err(err)
            }
        }
    }
}

impl BuilderInput {
    fn lower(&self) -> syn::Result<TokenStream> {
        let mut lowering = Lowering::default();
        for item in &self.items {
            match item {
                Item::Struct(item) => lower_struct(item, &mut lowering)?,
                Item::Enum(item) => lower_enum(item, &mut lowering)?,
                Item::Alias(item) => lower_alias(item, &mut lowering)?,
                Item::Native(item) => lower_native(item, &mut lowering)?,
            }
        }
        let settings = &self.settings;
        let inserts = &lowering.inserts;
        Ok(quote! {
            {
                let mut builder = ::typespace::TypespaceBuilder::<String>::new(#settings);
                #( #inserts )*
                builder
            }
        })
    }
}

/// Lower a struct's (or a struct-shaped variant's) fields to
/// `StructProperty` expressions.
fn lower_struct_properties(
    fields: &[FieldItem],
    lowering: &mut Lowering,
) -> syn::Result<Vec<TokenStream>> {
    fields
        .iter()
        .map(|field| {
            let (type_id, base_state) = lower_field_type(&field.ty, lowering)?;
            let claims = claim_attrs(&field.attrs, &["default", "rename", "flatten"])?;
            let state = field_state_override(&claims, base_state)?;
            let json_name = field_json_name(&claims)?;
            let field_name = field.name.to_string();
            Ok(quote! {
                ::typespace::build::StructProperty::new(#field_name, #type_id.to_string())
                    .with_state(#state)
                    #json_name
            })
        })
        .collect::<syn::Result<Vec<_>>>()
}

fn lower_struct(item: &StructItem, lowering: &mut Lowering) -> syn::Result<()> {
    let name = lowering.claim_named_item(&item.name)?;
    let allowed: &'static [&'static str] = match &item.body {
        StructBody::Unit => &["default", "json"],
        // `deny_unknown_fields` is only valid on the fields form:
        // `typespace::build::TupleStruct`/`NewtypeStruct`/`UnitStruct`
        // have no such builder method.
        StructBody::Fields(_) => &["default", "deny_unknown_fields"],
        StructBody::Tuple(_) => &["default", "tuple"],
    };
    let claims = claim_attrs(&item.attrs, allowed)?;
    let default_tokens = claimed_default(&claims)?;
    match &item.body {
        StructBody::Fields(fields) => {
            let props = lower_struct_properties(fields, lowering)?;
            let deny_unknown_fields_tokens = claimed_deny_unknown_fields(&claims)?;
            lowering.inserts.push(quote! {
                builder.insert(
                    #name.to_string(),
                    ::typespace::build::Struct::<String>::new()
                        .name(#name)
                        #default_tokens
                        #deny_unknown_fields_tokens
                        .properties([ #(#props),* ])
                        .build()
                        .unwrap(),
                ).unwrap();
            });
        }

        // One type becomes a newtype struct unless `#[tuple]` asks for
        // a one-field tuple struct; two or more, a tuple struct.
        // `#[flatten]` on the last field of a tuple struct pulls it
        // into `.rest(...)` instead of `.fields(...)`; a newtype's sole
        // field has no `rest` to pull it into.
        StructBody::Tuple(fields) => {
            let flatten = claimed_tuple_flatten(fields)?;
            let force_tuple = claimed_tuple_marker(&claims, fields.len())?;
            match (fields.len(), flatten) {
                (1, Some(entry)) => {
                    return Err(syn::Error::new_spanned(
                        &entry.name,
                        "#[flatten] requires a positional field before it: a \
                         single-field tuple struct is a newtype, which has \
                         no `rest`",
                    ));
                }
                (1, None) if force_tuple => {
                    let field_id = lower_type(&fields[0].ty, lowering)?;
                    lowering.inserts.push(quote! {
                        builder.insert(
                            #name.to_string(),
                            ::typespace::build::TupleStruct::<String>::new()
                                .name(#name)
                                #default_tokens
                                .fields([ #field_id.to_string() ])
                                .build()
                                .unwrap(),
                        ).unwrap();
                    });
                }
                (1, None) => {
                    let inner_id = lower_type(&fields[0].ty, lowering)?;
                    lowering.inserts.push(quote! {
                        builder.insert(
                            #name.to_string(),
                            ::typespace::build::NewtypeStruct::new(#inner_id.to_string())
                                .name(#name)
                                #default_tokens
                                .build()
                                .unwrap(),
                        ).unwrap();
                    });
                }
                (_, flatten) => {
                    let rest_len = match flatten {
                        Some(_) => fields.len() - 1,
                        None => fields.len(),
                    };
                    let ids = fields[..rest_len]
                        .iter()
                        .map(|field| lower_type(&field.ty, lowering))
                        .collect::<syn::Result<Vec<_>>>()?;
                    let rest_tokens = match flatten {
                        None => TokenStream::new(),
                        Some(_) => {
                            let rest_id = lower_type(&fields[rest_len].ty, lowering)?;
                            quote! { .rest(#rest_id.to_string()) }
                        }
                    };
                    lowering.inserts.push(quote! {
                        builder.insert(
                            #name.to_string(),
                            ::typespace::build::TupleStruct::<String>::new()
                                .name(#name)
                                #default_tokens
                                .fields([ #(#ids.to_string()),* ])
                                #rest_tokens
                                .build()
                                .unwrap(),
                        ).unwrap();
                    });
                }
            }
        }
        StructBody::Unit => {
            let json = claims.get("json").copied().ok_or_else(|| {
                syn::Error::new_spanned(
                    &item.name,
                    "unit struct requires #[json = <repr>]: the wire form \
                     must be stated explicitly",
                )
            })?;
            let value = json.value.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(&json.name, "#[json] requires a value: #[json = <repr>]")
            })?;
            let repr_tokens = value_tokens(value);
            lowering.inserts.push(quote! {
                builder.insert(
                    #name.to_string(),
                    ::typespace::build::UnitStruct::new(#repr_tokens)
                        .name(#name)
                        #default_tokens
                        .build::<String>()
                        .unwrap(),
                ).unwrap();
            });
        }
    }
    Ok(())
}

fn lower_enum(item: &EnumItem, lowering: &mut Lowering) -> syn::Result<()> {
    let name = lowering.claim_named_item(&item.name)?;
    let claims = claim_attrs(
        &item.attrs,
        &[
            "default",
            "untagged",
            "tag",
            "content",
            "deny_unknown_fields",
        ],
    )?;
    let default_tokens = claimed_default(&claims)?;
    let tag_type_tokens = enum_tag_type(&claims)?;
    let deny_unknown_fields_tokens = claimed_deny_unknown_fields(&claims)?;
    let variant_tokens = item
        .variants
        .iter()
        .map(|variant| lower_variant(variant, lowering))
        .collect::<syn::Result<Vec<_>>>()?;
    lowering.inserts.push(quote! {
        builder.insert(
            #name.to_string(),
            ::typespace::build::Enum::<String>::new()
                .name(#name)
                .tag_type(#tag_type_tokens)
                #default_tokens
                #deny_unknown_fields_tokens
                .variants([ #(#variant_tokens),* ])
                .build()
                .unwrap(),
        ).unwrap();
    });
    Ok(())
}

fn lower_variant(variant: &VariantItem, lowering: &mut Lowering) -> syn::Result<TokenStream> {
    let claims = claim_attrs(&variant.attrs, &["json", "tuple"])?;

    // `#[tuple]` applies to the payload, so its arity check needs the
    // payload; a variant that has none cannot carry the attribute at
    // all, which arity 0 reports.
    let payload_arity = match &variant.payload {
        VariantPayload::Tuple(types) => types.len(),
        VariantPayload::Unit | VariantPayload::Struct(_) => 0,
    };
    let force_tuple = claimed_tuple_marker(&claims, payload_arity)?;

    let details_tokens = match &variant.payload {
        VariantPayload::Unit => quote! { ::typespace::build::VariantDetails::<String>::Unit },
        VariantPayload::Tuple(types) if types.len() == 1 && !force_tuple => {
            let id = lower_type(&types[0], lowering)?;
            quote! { ::typespace::build::VariantDetails::<String>::Item(#id.to_string()) }
        }
        VariantPayload::Tuple(types) => {
            let ids = types
                .iter()
                .map(|ty| lower_type(ty, lowering))
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                ::typespace::build::VariantDetails::<String>::Tuple(
                    [ #(#ids.to_string()),* ].into_iter().collect()
                )
            }
        }
        VariantPayload::Struct(fields) => {
            let props = lower_struct_properties(fields, lowering)?;
            quote! {
                ::typespace::build::VariantDetails::<String>::Struct(
                    [ #(#props),* ].into_iter().collect()
                )
            }
        }
    };
    let variant_name = variant.name.to_string();
    let with_rename = match claims.get("json").copied() {
        Some(entry) => {
            let rename = expect_string_value(entry)?;
            Some(quote! { .with_rename(#rename) })
        }
        None => None,
    };
    // `with_rename` is `Option<TokenStream>`: quote!'s `ToTokens` impl
    // for `Option<T: ToTokens>` interpolates the contents when `Some`
    // and emits nothing at all when `None`, so this needs no branch.
    Ok(quote! {
        ::typespace::build::EnumVariant::new(#variant_name, #details_tokens)
            #with_rename
    })
}

fn lower_alias(item: &AliasItem, lowering: &mut Lowering) -> syn::Result<()> {
    let name = lowering.claim_named_item(&item.name)?;
    claim_attrs(&item.attrs, &[])?;
    let target_id = lower_type(&item.target, lowering)?;
    lowering.inserts.push(quote! {
        builder.insert(
            #name.to_string(),
            ::typespace::build::TypeAlias::<String>::new(#target_id.to_string())
                .name(#name)
                .build()
                .unwrap(),
        ).unwrap();
    });
    Ok(())
}

fn lower_native(item: &NativeItem, lowering: &mut Lowering) -> syn::Result<()> {
    // No attributes are currently permitted for a native type.
    let _ = claim_attrs(&item.attrs, &[])?;
    let path = native_path(&item.ty)?;
    let id = lowering.claim_native(path)?;
    let name = path_text(path);
    let impls = native_trait_group(&item.bounds, |bound| match bound {
        NativeBound::Yes(path) => Some(path),
        _ => None,
    })?;
    let unknown = native_trait_group(&item.bounds, |bound| match bound {
        NativeBound::Unknown(path) => Some(path),
        _ => None,
    })?;
    let known_not = native_trait_group(&item.bounds, |bound| match bound {
        NativeBound::No(path) => Some(path),
        _ => None,
    })?;
    // An empty array literal would leave the element type unresolved,
    // so the trait-less case names the empty set directly.
    let impls_tokens = if impls.is_empty() {
        quote! { ::typespace::TypespaceTraitSet::empty() }
    } else {
        quote! { [ #(#impls),* ].into_iter().collect::<::typespace::TypespaceTraitSet>() }
    };
    // `..` is the background statement, so it is applied first and the
    // bounds that name a trait outright override it.
    let rest_tokens = item
        .bounds
        .iter()
        .any(|bound| matches!(bound, NativeBound::Rest))
        .then(|| quote! { .with_rest_unknown() });
    let unknown_tokens = (!unknown.is_empty()).then(|| {
        quote! {
            .with_unknown(
                [ #(#unknown),* ].into_iter().collect::<::typespace::TypespaceTraitSet>()
            )
        }
    });
    let known_not_tokens = known_not
        .iter()
        .map(|trait_tokens| {
            quote! {
                .with_disposition(#trait_tokens, ::typespace::TraitDisposition::No)
            }
        })
        .collect::<Vec<_>>();
    let parameter_ids = native_args(path)?
        .into_iter()
        .map(|arg| lower_type(arg, lowering))
        .collect::<syn::Result<Vec<_>>>()?;
    lowering.inserts.push(quote! {
        builder.insert(
            #id.to_string(),
            ::typespace::build::Type::Native(
                ::typespace::build::Native::new(
                    #name,
                    #impls_tokens,
                    [ #(#parameter_ids.to_string()),* ].into_iter().collect(),
                )
                #rest_tokens
                #unknown_tokens
                #(#known_not_tokens)*
            ),
        ).unwrap();
    });
    Ok(())
}

/// The `TypespaceTrait` tokens for the bounds `select` picks out.
fn native_trait_group<'a>(
    bounds: &'a [NativeBound],
    select: impl Fn(&'a NativeBound) -> Option<&'a syn::Path>,
) -> syn::Result<Vec<TokenStream>> {
    bounds
        .iter()
        .filter_map(select)
        .map(native_trait_tokens)
        .collect()
}

// ---------------------------------------------------------------------
// Native types
// ---------------------------------------------------------------------

/// Every trait name a `native` item's bound list accepts: the
/// `TypespaceTrait` variants, in declaration order, which is also the
/// order the error for an unrecognized name lists them in.
const NATIVE_TRAITS: &[&str] = &[
    "Clone",
    "Debug",
    "Serialize",
    "Deserialize",
    "JsonSchema",
    "Display",
    "FromStr",
    "Eq",
    "PartialEq",
    "Ord",
    "PartialOrd",
    "Hash",
    "Default",
];

/// The `TypespaceTrait` a `native` item's bound names.
///
/// A native declares only the traits it lists: `native P: Ord;` gets no
/// `PartialOrd` or `Eq` added, so trait propagation can catch an
/// incomplete claim about someone else's type.
fn native_trait_tokens(bound: &syn::Path) -> syn::Result<TokenStream> {
    match bound.get_ident() {
        Some(ident) if NATIVE_TRAITS.contains(&ident.to_string().as_str()) => {
            Ok(quote! { ::typespace::TypespaceTrait::#ident })
        }
        _ => {
            let accepted = NATIVE_TRAITS
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            Err(syn::Error::new_spanned(
                bound,
                format!(
                    "`{}` is not a trait typespace tracks; a native \
                     type's bounds are drawn from {accepted}",
                    path_text(bound)
                ),
            ))
        }
    }
}

/// The path a `native` item declares: anything else is rejected with
/// the same message a use of it in a type position would get.
fn native_path(ty: &Type) -> syn::Result<&syn::Path> {
    match ty {
        Type::Path(type_path) => match name_or_native(type_path, ty)? {
            NameOrNative::Native(path) => Ok(path),
            NameOrNative::Name(_) => Err(syn::Error::new_spanned(
                ty,
                "a native type's path needs two or more segments, e.g. \
                 `native ::std::path::PathBuf;`",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            ty,
            "`native` needs a Rust type path, e.g. \
             `native ::std::path::PathBuf;`",
        )),
    }
}

/// The Rust path text a native type is emitted as: its segment names
/// joined by `::`, keeping the leading `::` if the input contains one.
///
/// Generic arguments are left out: they become `Native::parameters`,
/// which the renderer emits after the name, so `::foo::Wrapper<Inner>`
/// is the name `::foo::Wrapper` plus the one parameter id `Inner`.
fn path_text(path: &syn::Path) -> String {
    // Printing the path's tokens instead would give both the generic
    // arguments and one space per token: `:: foo :: Wrapper < Inner >`.
    let leading = if path.leading_colon.is_some() {
        "::"
    } else {
        ""
    };
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    format!("{leading}{}", segments.join("::"))
}

/// A native path's generic arguments, which belong to its last segment
/// (`::foo::Wrapper<Inner>`); each becomes one `Native::parameters`
/// entry, lowered like any other type.
fn native_args(path: &syn::Path) -> syn::Result<Vec<&Type>> {
    let last = path.segments.len() - 1;
    let misplaced = path.segments.iter().enumerate().any(|(index, segment)| {
        index != last && !matches!(segment.arguments, syn::PathArguments::None)
    });
    if misplaced {
        return Err(syn::Error::new_spanned(
            path,
            "a native type's generic arguments belong to its last path \
             segment, e.g. `::foo::Wrapper<Inner>`",
        ));
    }
    match &path.segments[last].arguments {
        syn::PathArguments::None => Ok(Vec::new()),
        syn::PathArguments::AngleBracketed(args) => Ok(generic_type_args(&args.args)),
        syn::PathArguments::Parenthesized(_) => Err(syn::Error::new_spanned(
            path,
            "unsupported type syntax in typespace_builder!",
        )),
    }
}

/// The graph id a native type is inserted under: its path text plus its
/// argument ids, so `::foo::Wrapper<Vec<u32>>` is the id
/// `"::foo::Wrapper<Vec<u32>>"`.
///
/// Like every other id here this is reconstructed from the parsed path
/// rather than lifted from the source span (see [`anon_id`]), but the
/// leading `::` is part of it: it appears in the emitted output, so
/// `chrono::NaiveDate` and `::chrono::NaiveDate` are two different
/// native types, not one path written two ways.
fn native_id(path: &syn::Path) -> syn::Result<String> {
    let name = path_text(path);
    let parts = native_args(path)?
        .into_iter()
        .map(anon_id)
        .collect::<syn::Result<Vec<_>>>()?;
    if parts.is_empty() {
        Ok(name)
    } else {
        Ok(format!("{name}<{}>", parts.join(", ")))
    }
}

/// The id a use of a native path refers to, requiring that some
/// `native` item earlier in the same invocation declared it.
///
/// An undeclared path is an error where it is written rather than a
/// trait-less native inserted on the spot: a mistyped path should fail
/// as a mistyped path, not later as a trait conflict against a type
/// nobody declared.
fn lower_native_use(path: &syn::Path, lowering: &Lowering) -> syn::Result<String> {
    let id = native_id(path)?;
    if lowering.native_ids.contains_key(&id) {
        Ok(id)
    } else {
        Err(syn::Error::new_spanned(
            path,
            format!(
                "`{id}` is not a declared native type; declare it first \
                 with `native {id};`, or `native {id}: Clone + Debug;` \
                 to state the traits it implements"
            ),
        ))
    }
}

// ---------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------

/// Lower a struct field's declared type, handling the field-top-level-only
/// wire vocabulary (`Optional<T>`, `OptionalNullable<T>`).
///
/// Returns the id the field's `StructProperty` should reference, and the
/// baseline `StructPropertyState` that type implies (`Required` unless
/// the field type is `Optional<T>` or `OptionalNullable<T>`); a
/// field-level `#[default]` attribute overrides this baseline
/// separately, in [`field_state_override`].
fn lower_field_type(ty: &Type, lowering: &mut Lowering) -> syn::Result<(String, TokenStream)> {
    match bare_generic_path(ty) {
        Some((ident, args)) if ident == "Optional" => {
            let type_args = generic_type_args(args);
            let inner = expect_one_arg(&type_args, ty)?;
            let id = lower_type(inner, lowering)?;
            Ok((
                id,
                quote! { ::typespace::build::StructPropertyState::Optional },
            ))
        }
        Some((ident, args)) if ident == "OptionalNullable" => {
            let type_args = generic_type_args(args);
            let inner = expect_one_arg(&type_args, ty)?;
            let id = ensure_nullable(inner, lowering)?;
            Ok((
                id,
                quote! { ::typespace::build::StructPropertyState::Optional },
            ))
        }
        _ => {
            let id = lower_type(ty, lowering)?;
            Ok((
                id,
                quote! { ::typespace::build::StructPropertyState::Required },
            ))
        }
    }
}

/// Lower a type appearing anywhere other than a field's top level:
/// container elements, array elements, tuple components, enum variant
/// payloads, and alias targets. `Optional<T>`/`OptionalNullable<T>` are
/// rejected here ("may be absent" is a property of a field, not a
/// type); `Nullable<T>` is fine anywhere.
fn lower_type(ty: &Type, lowering: &mut Lowering) -> syn::Result<String> {
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => {
            lowering.ensure_anon("()", quote! { ::typespace::build::Type::Unit });
            Ok("()".to_string())
        }
        Type::Tuple(tuple) => {
            let ids = tuple
                .elems
                .iter()
                .map(|elem| lower_type(elem, lowering))
                .collect::<syn::Result<Vec<_>>>()?;
            let id = anon_id(ty)?;
            lowering.ensure_anon(
                &id,
                quote! { ::typespace::build::Type::Tuple([ #(#ids.to_string()),* ].into_iter().collect()) },
            );
            Ok(id)
        }
        Type::Array(array) => {
            let elem_id = lower_type(&array.elem, lowering)?;
            let len = array_len_usize(&array.len)?;
            let id = anon_id(ty)?;
            lowering.ensure_anon(
                &id,
                quote! { ::typespace::build::Type::Array(#elem_id.to_string(), #len) },
            );
            Ok(id)
        }
        Type::Never(_) => {
            lowering.ensure_anon(
                "Never",
                quote! {
                    ::typespace::build::Type::Never
                },
            );
            Ok("Never".to_string())
        }

        Type::Path(type_path) => lower_path_type(type_path, ty, lowering),
        _ => Err(syn::Error::new_spanned(
            ty,
            "unsupported type syntax in typespace_builder!",
        )),
    }
}

/// If `ty` is a single-segment, unqualified path with angle-bracketed
/// generic arguments (`Foo<..>`), return its name and argument list.
/// Used to recognize the wire-vocabulary wrappers (`Optional`,
/// `OptionalNullable`) at a field's top level regardless of what else
/// `lower_type` would make of them. A native path (`foo::Optional<T>`)
/// and a qualified self-type never match, so neither can reach the wire
/// vocabulary through this door; see [`name_or_native`].
fn bare_generic_path(ty: &Type) -> Option<(&Ident, &Punctuated<syn::GenericArgument, Token![,]>)> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let NameOrNative::Name(segment) = name_or_native(type_path, ty).ok()? else {
        return None;
    };
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => Some((&segment.ident, &args.args)),
        _ => None,
    }
}

/// A type path names either the macro's own vocabulary or a native type.
enum NameOrNative<'a> {
    /// A plain, unqualified name: the macro's own vocabulary (`String`,
    /// `Vec<T>`, `Optional<T>`, ...) or a reference to a named item.
    Name(&'a syn::PathSegment),
    /// A native type's Rust path, which some `native` item declares.
    Native(&'a syn::Path),
}

/// Read a type path as a plain name or as a native type's path.
///
/// One segment and no leading `::` is a plain name; two or more
/// segments, with or without a leading `::`, is a native type's path.
/// The `::` is part of that path's identity rather than punctuation to
/// be normalized away, because the path is emitted verbatim.
///
/// Anything else is rejected here, so no later step has to consider it:
/// a lone segment behind a leading `::` (`::String`, which is neither a
/// plain name nor a Rust type path), and a qualified self-type
/// (`<T as Trait>::Foo`), which names nothing this grammar can
/// resolve.
fn name_or_native<'a>(type_path: &'a syn::TypePath, ty: &Type) -> syn::Result<NameOrNative<'a>> {
    match (
        type_path.qself.is_some(),
        type_path.path.leading_colon.is_some(),
        type_path.path.segments.len(),
    ) {
        (true, _, _) => Err(syn::Error::new_spanned(
            ty,
            "unsupported type syntax in typespace_builder!: a qualified \
             self-type (`<T as Trait>::Foo`) names no type here",
        )),
        (false, false, 1) => Ok(NameOrNative::Name(&type_path.path.segments[0])),
        (false, true, 1) => Err(syn::Error::new_spanned(
            ty,
            "unsupported type syntax in typespace_builder!: a plain type \
             name takes no leading `::`, and a native type's path needs \
             two or more segments (`::std::path::PathBuf`)",
        )),
        (false, _, _) => Ok(NameOrNative::Native(&type_path.path)),
    }
}

fn lower_path_type(
    type_path: &syn::TypePath,
    ty: &Type,
    lowering: &mut Lowering,
) -> syn::Result<String> {
    let segment = match name_or_native(type_path, ty)? {
        NameOrNative::Native(path) => return lower_native_use(path, lowering),
        NameOrNative::Name(segment) => segment,
    };
    let name = segment.ident.to_string();

    // Scalars: no generics, one Type variant apiece.
    match name.as_str() {
        "String" => {
            lowering.ensure_anon("String", quote! { ::typespace::build::Type::String });
            return Ok("String".to_string());
        }
        "bool" => {
            lowering.ensure_anon("bool", quote! { ::typespace::build::Type::Boolean });
            return Ok("bool".to_string());
        }
        "JsonValue" => {
            lowering.ensure_anon("JsonValue", quote! { ::typespace::build::Type::JsonValue });
            return Ok("JsonValue".to_string());
        }
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" => {
            lowering.ensure_anon(
                &name,
                quote! { ::typespace::build::Type::Integer(#name.to_string()) },
            );
            return Ok(name);
        }
        "f32" | "f64" => {
            lowering.ensure_anon(
                &name,
                quote! { ::typespace::build::Type::Float(#name.to_string()) },
            );
            return Ok(name);
        }
        "Option" => {
            return Err(syn::Error::new_spanned(
                &segment.ident,
                "`Option` is ambiguous here: use `Optional<T>` (may be \
                 absent), `Nullable<T>` (may be null), or \
                 `OptionalNullable<T>` (either)",
            ));
        }
        "Optional" | "OptionalNullable" => {
            return Err(match &segment.arguments {
                syn::PathArguments::AngleBracketed(_) => syn::Error::new_spanned(
                    &segment.ident,
                    format!(
                        "`{name}` may only appear as a field's top-level \
                         type: \"may be absent\" is a property of a \
                         field, not a type"
                    ),
                ),
                _ => syn::Error::new_spanned(
                    &segment.ident,
                    format!("`{name}` needs a type argument: `{name}<T>`"),
                ),
            });
        }
        "Nullable" if matches!(segment.arguments, syn::PathArguments::None) => {
            return Err(syn::Error::new_spanned(
                &segment.ident,
                "`Nullable` needs a type argument: `Nullable<T>`",
            ));
        }
        "HashMap" | "BTreeMap" => {
            return Err(syn::Error::new_spanned(
                &segment.ident,
                format!(
                    "`{name}` is a Rust type; typespace_builder! expects \
                     `Map<K, V>` (the rendered container is a \
                     settings decision)"
                ),
            ));
        }
        "HashSet" | "BTreeSet" => {
            return Err(syn::Error::new_spanned(
                &segment.ident,
                format!(
                    "`{name}` is a Rust type; typespace_builder! expects \
                     `Set<T>` (the rendered container is a settings \
                     decision)"
                ),
            ));
        }
        _ => {}
    }

    match &segment.arguments {
        syn::PathArguments::None => {
            // A bare, unparameterized identifier: a reference to a
            // user-defined type, by name.
            Ok(name)
        }
        syn::PathArguments::AngleBracketed(args) => {
            let type_args = generic_type_args(&args.args);
            match name.as_str() {
                "Vec" => {
                    let inner = expect_one_arg(&type_args, ty)?;
                    let inner_id = lower_type(inner, lowering)?;
                    let id = anon_id(ty)?;
                    lowering.ensure_anon(
                        &id,
                        quote! { ::typespace::build::Type::Vec(#inner_id.to_string()) },
                    );
                    Ok(id)
                }
                "Box" => {
                    let inner = expect_one_arg(&type_args, ty)?;
                    let inner_id = lower_type(inner, lowering)?;
                    let id = anon_id(ty)?;
                    lowering.ensure_anon(
                        &id,
                        quote! { ::typespace::build::Type::Box(#inner_id.to_string()) },
                    );
                    Ok(id)
                }
                "Set" => {
                    let inner = expect_one_arg(&type_args, ty)?;
                    let inner_id = lower_type(inner, lowering)?;
                    let id = anon_id(ty)?;
                    lowering.ensure_anon(
                        &id,
                        quote! { ::typespace::build::Type::Set(#inner_id.to_string()) },
                    );
                    Ok(id)
                }
                "Map" => {
                    let (key, value) = expect_two_args(&type_args, ty)?;
                    let key_id = lower_type(key, lowering)?;
                    let value_id = lower_type(value, lowering)?;
                    let id = anon_id(ty)?;
                    lowering.ensure_anon(
                        &id,
                        quote! {
                            ::typespace::build::Type::Map(#key_id.to_string(), #value_id.to_string())
                        },
                    );
                    Ok(id)
                }
                "Nullable" => {
                    let inner = expect_one_arg(&type_args, ty)?;
                    ensure_nullable(inner, lowering)
                }
                _ => Err(syn::Error::new_spanned(
                    ty,
                    format!("unknown parameterized type `{name}` in typespace_builder!"),
                )),
            }
        }
        syn::PathArguments::Parenthesized(_) => Err(syn::Error::new_spanned(
            ty,
            "unsupported type syntax in typespace_builder!",
        )),
    }
}

/// Insert (if not already present) the anonymous `Option` node that
/// `Nullable<T>`/`OptionalNullable<T>` both normalize to, with id
/// `"Nullable<T>"` regardless of which input produced it: both wrap
/// `T` in the same node, so referencing a given `T` through either
/// mechanism reuses one node instead of inserting two `Option`-shaped
/// types for it.
fn ensure_nullable(inner: &Type, lowering: &mut Lowering) -> syn::Result<String> {
    let inner_id = lower_type(inner, lowering)?;
    let inner_anon_id = anon_id(inner)?;
    let id = format!("Nullable<{inner_anon_id}>");
    lowering.ensure_anon(
        &id,
        quote! { ::typespace::build::Type::Option(#inner_id.to_string()) },
    );
    Ok(id)
}

fn generic_type_args(args: &Punctuated<syn::GenericArgument, Token![,]>) -> Vec<&Type> {
    args.iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect()
}

fn expect_one_arg<'a>(args: &'a [&'a Type], whole: &Type) -> syn::Result<&'a Type> {
    match args {
        [one] => Ok(one),
        _ => Err(syn::Error::new_spanned(
            whole,
            "expected exactly one type parameter",
        )),
    }
}

fn expect_two_args<'a>(args: &'a [&'a Type], whole: &Type) -> syn::Result<(&'a Type, &'a Type)> {
    match args {
        [a, b] => Ok((a, b)),
        _ => Err(syn::Error::new_spanned(
            whole,
            "expected exactly two type parameters",
        )),
    }
}

/// The id an anonymous node (a container, a primitive, a `Nullable`
/// wrapper) is inserted under.
///
/// This is reconstructed from the parsed `syn::Type`, not lifted
/// verbatim from the source span: it is built fragment by fragment
/// from each segment's name and its recursively computed argument ids
/// (see the call sites in `lower_path_type`, `lower_type`). That is
/// what makes different ways of writing the same type collapse to
/// one id--source whitespace never enters the id, and an array
/// length written in hex normalizes to decimal (`base10_digits`) the
/// same as `Nullable<T>` and `OptionalNullable<T>` normalize to one
/// `"Nullable<T>"` node.
fn anon_id(ty: &Type) -> syn::Result<String> {
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Ok("()".to_string()),
        // `!` names the same anonymous node wherever it appears, so this
        // is the id `lower_type` registers it under.
        Type::Never(_) => Ok("Never".to_string()),
        Type::Tuple(tuple) => {
            let parts = tuple
                .elems
                .iter()
                .map(anon_id)
                .collect::<syn::Result<Vec<_>>>()?;
            Ok(format!("({})", parts.join(", ")))
        }
        Type::Array(array) => {
            let elem = anon_id(&array.elem)?;
            let len = array_len_anon_id(&array.len)?;
            Ok(format!("[{elem}; {len}]"))
        }
        Type::Path(type_path) => match name_or_native(type_path, ty)? {
            NameOrNative::Native(path) => native_id(path),
            NameOrNative::Name(segment) => {
                let name = segment.ident.to_string();
                match &segment.arguments {
                    syn::PathArguments::None => Ok(name),
                    syn::PathArguments::AngleBracketed(args) => {
                        let parts = generic_type_args(&args.args)
                            .into_iter()
                            .map(anon_id)
                            .collect::<syn::Result<Vec<_>>>()?;
                        Ok(format!("{name}<{}>", parts.join(", ")))
                    }
                    syn::PathArguments::Parenthesized(_) => Err(syn::Error::new_spanned(
                        ty,
                        "unsupported type syntax in typespace_builder!",
                    )),
                }
            }
        },
        _ => Err(syn::Error::new_spanned(
            ty,
            "unsupported type syntax in typespace_builder!",
        )),
    }
}

fn array_len_anon_id(expr: &Expr) -> syn::Result<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => Ok(lit.base10_digits().to_string()),
        _ => Err(syn::Error::new_spanned(
            expr,
            "array length must be an integer literal",
        )),
    }
}

fn array_len_usize(expr: &Expr) -> syn::Result<usize> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => lit.base10_parse(),
        _ => Err(syn::Error::new_spanned(
            expr,
            "array length must be an integer literal",
        )),
    }
}

// ---------------------------------------------------------------------
// JSON-ish value -> tokens
// ---------------------------------------------------------------------

/// Render a parsed JSON-ish value as an expression building the
/// equivalent `serde_json::Value`, fully qualified so it does not
/// depend on what the call site has imported.
fn value_tokens(value: &serde_json::Value) -> TokenStream {
    match value {
        serde_json::Value::Null => quote! { ::serde_json::Value::Null },
        serde_json::Value::Bool(b) => quote! { ::serde_json::Value::Bool(#b) },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                quote! { ::serde_json::Value::Number(::serde_json::Number::from(#i)) }
            } else if let Some(u) = n.as_u64() {
                quote! { ::serde_json::Value::Number(::serde_json::Number::from(#u)) }
            } else {
                let f = n.as_f64().expect("serde_json::Number is int or float");
                quote! {
                    ::serde_json::Value::Number(
                        ::serde_json::Number::from_f64(#f).unwrap()
                    )
                }
            }
        }
        serde_json::Value::String(s) => quote! { ::serde_json::Value::String(#s.to_string()) },
        serde_json::Value::Array(items) => {
            let items = items.iter().map(value_tokens);
            quote! {
                ::serde_json::Value::Array(
                    [ #(#items),* ].into_iter().collect()
                )
            }
        }
        serde_json::Value::Object(map) => {
            let entries = map.iter().map(|(key, value)| {
                let value = value_tokens(value);
                quote! { (#key.to_string(), #value) }
            });
            quote! {
                ::serde_json::Value::Object(
                    [ #(#entries),* ].into_iter().collect()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::expand;
    use quote::quote;

    /// Pretty-print `expand(input)`'s result as a standalone item so it
    /// reads like ordinary source in the golden file, rather than as
    /// one long token stream.
    fn expand_pretty(input: proc_macro2::TokenStream) -> String {
        let expanded = expand(input);
        let wrapped: syn::File = syn::parse2(quote! {
            fn expansion() {
                #expanded;
            }
        })
        .expect("expansion should parse as a Rust file (wrap failed)");
        prettyplease::unparse(&wrapped)
    }

    // Golden tests: each captures the exact generated code for one
    // representative input, so the expansion is visible and reviewable
    // in the diff of `tests/output/`. These complement, not replace,
    // the behavioral tests in typespace/src/trait_resolution.rs, which
    // check that the generated code actually builds and finalizes the
    // graph it claims to, and the trybuild UI tests in
    // `tests/ui/`, which pin the error paths' messages and spans.

    #[test]
    fn golden_optional_nullable_default() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Inner {
                    count: u32,
                }

                #[default = { name: "anon", opt: null, nul: null, both: null }]
                struct Widget {
                    name: String,
                    opt: Optional<Inner>,
                    nul: Nullable<Inner>,
                    both: OptionalNullable<Inner>,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_optional_nullable_default.rs", &out);
    }

    #[test]
    fn golden_enum_tag_external() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                enum Shape {
                    Circle(f64),
                    Empty,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_enum_tag_external.rs", &out);
    }

    #[test]
    fn golden_enum_tag_untagged() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                #[untagged]
                enum Shape {
                    Circle(f64),
                    Empty,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_enum_tag_untagged.rs", &out);
    }

    #[test]
    fn golden_enum_tag_internal() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                #[tag = "kind"]
                enum Shape {
                    Circle(f64),
                    Empty,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_enum_tag_internal.rs", &out);
    }

    #[test]
    fn golden_enum_tag_adjacent() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                #[tag = "kind", content = "value"]
                enum Shape {
                    Circle(f64),
                    #[json = "nothing"]
                    Empty,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_enum_tag_adjacent.rs", &out);
    }

    #[test]
    fn golden_unit_struct_json() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                #[json = { kind: "widget", version: 1 }]
                struct WidgetMarker;
            }
        });
        expectorate::assert_contents("tests/output/builder_unit_struct_json.rs", &out);
    }

    #[test]
    fn golden_nested_containers() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Item {
                    value: u32,
                }

                struct Bag {
                    items: Vec<Nullable<Item>>,
                    by_name: Map<String, Item>,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_nested_containers.rs", &out);
    }

    #[test]
    fn golden_type_alias() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Item {
                    value: u32,
                }

                type ItemList = Vec<Item>;
            }
        });
        expectorate::assert_contents("tests/output/builder_type_alias.rs", &out);
    }

    #[test]
    fn golden_variant_shapes() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Point {
                    x: u32,
                    y: u32,
                }

                enum Shape {
                    Empty,
                    Circle(f64),
                    Line(Point, Point),
                    Rect { top_left: Point, bottom_right: Point },
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_variant_shapes.rs", &out);
    }

    #[test]
    fn test_native_no_traits() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                native chrono::NaiveDate;

                struct Event {
                    when: chrono::NaiveDate,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_no_traits.rs", &out);
    }

    #[test]
    fn test_native_declared_traits() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                native ::std::path::PathBuf: Clone + Debug + Display + FromStr;

                struct Config {
                    path: ::std::path::PathBuf,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_declared_traits.rs", &out);
    }

    #[test]
    fn test_native_unknown_traits() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                native ::chrono::naive::NaiveDate: Clone + Debug + ?Ord + ?Hash;
                native ::foo::Opaque: Clone + .. + !Default;

                struct Event {
                    when: ::chrono::naive::NaiveDate,
                    what: ::foo::Opaque,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_unknown_traits.rs", &out);
    }

    #[test]
    fn test_native_generic_parameters() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Inner {
                    count: u32,
                }

                native ::foo::Wrapper<Inner>: Clone + Debug;

                type Wrapped = ::foo::Wrapper<Inner>;
            }
        });
        expectorate::assert_contents("tests/output/test_native_generic_parameters.rs", &out);
    }

    #[test]
    fn test_native_leading_colon_distinct() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                native chrono::NaiveDate: Clone;
                native ::chrono::NaiveDate: Clone;

                struct Span {
                    start: chrono::NaiveDate,
                    end: ::chrono::NaiveDate,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_leading_colon_distinct.rs", &out);
    }

    #[test]
    fn test_native_in_containers() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                native ::uuid::Uuid: Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Hash;

                struct Batch {
                    ids: Vec<::uuid::Uuid>,
                    primary: Optional<::uuid::Uuid>,
                    by_id: Map<::uuid::Uuid, String>,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_in_containers.rs", &out);
    }

    /// A field's serde name treatment: renamed, flattened, and a rename
    /// alongside `#[default]`.
    #[test]
    fn test_native_field_serde_names() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Inner {
                    value: u32,
                }

                struct Outer {
                    #[rename = "my-field"]
                    my_field: String,
                    #[flatten]
                    inner: Inner,
                    #[default = 7, rename = "count-x"]
                    count: u32,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_native_field_serde_names.rs", &out);
    }

    /// `#[rename]` and `#[flatten]` reach a struct-shaped variant's
    /// fields, and compose with a bare `#[default]`.
    #[test]
    fn test_native_variant_field_serde_names() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Inner {
                    value: u32,
                }

                enum Shape {
                    Rect {
                        #[rename = "top-left"]
                        #[default]
                        top_left: u32,
                        #[flatten]
                        inner: Inner,
                    },
                }
            }
        });
        expectorate::assert_contents(
            "tests/output/test_native_variant_field_serde_names.rs",
            &out,
        );
    }

    /// `#[flatten]` on a tuple struct's last field: the positional
    /// fields stay in `.fields(...)`, and the last becomes
    /// `.rest(...)`.
    #[test]
    fn test_tuple_struct_flatten() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Widget(u32, #[flatten] String);
            }
        });
        expectorate::assert_contents("tests/output/test_tuple_struct_flatten.rs", &out);
    }

    /// `#[deny_unknown_fields]` on a struct and an enum: lowers to a
    /// `.deny_unknown_fields()` call in the builder chain. Rendering
    /// does not read the flag yet, so this only proves the lowering
    /// compiles and the type builds, not any effect on output.
    #[test]
    fn test_deny_unknown_fields() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                #[deny_unknown_fields]
                struct Widget {
                    name: String,
                }

                #[deny_unknown_fields]
                #[untagged]
                enum Shape {
                    Text(String),
                }
            }
        });
        expectorate::assert_contents("tests/output/test_deny_unknown_fields.rs", &out);
    }

    // Targeted behavioral checks (fast, string-match versions of a few
    // representative error paths); the full error-path oracle--every
    // message and span--is the trybuild suite in `tests/ui/`.

    #[test]
    fn bare_option_is_a_compile_error() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Widget {
                    name: Option<String>,
                }
            }
        });
        assert!(
            out.contains("Option` is ambiguous here"),
            "expected the bare-Option guidance in: {out}"
        );
    }

    #[test]
    fn optional_in_nested_position_is_a_compile_error() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Widget {
                    items: Vec<Optional<String>>,
                }
            }
        });
        assert!(
            out.contains("property of a field, not a type"),
            "expected the nested-Optional guidance in: {out}"
        );
    }

    #[test]
    fn unit_struct_without_json_is_a_compile_error() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Marker;
            }
        });
        assert!(
            out.contains("requires #[json"),
            "expected the missing-#[json] guidance in: {out}"
        );
    }

    /// `!` is usable inside the containers, tuples, and arrays whose
    /// ids route through `anon_id`, not only as a whole field type.
    #[test]
    fn test_never_in_containers() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Test {
                    vec: Vec<!>,
                    map: Map<String, !>,
                    boxed: Box<!>,
                    tuple: (u32, !),
                    array: [!; 3],
                    nested: Optional<Vec<!>>,
                }
            }
        });
        expectorate::assert_contents("tests/output/test_never_in_containers.rs", &out);
    }

    #[test]
    fn struct_with_never_field() {
        let out = expand_pretty(quote! {
            Settings::typical(), {
                struct Test {
                    foo: String,
                    bar: !,
                }
            }
        });
        expectorate::assert_contents("tests/output/builder_struct_with_never.rs", &out);
    }
}

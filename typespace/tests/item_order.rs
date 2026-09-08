// Copyright 2026 Oxide Computer Company

//! Order guard over every generated file in `tests/output`.
//!
//! Each file in that directory is complete output written by the
//! `check_and_include` macro, so together they are the corpus this test
//! reads. It parses all of them, groups the top level items by the type
//! that owns them, and checks each group against a canonical order.
//!
//! Only the items a type actually has are checked: an absent slot is
//! fine, so a group passes when its observed sequence is a subsequence
//! of the canonical order. Every `impl` must land in exactly one slot,
//! and one the active order ranks; an `impl` the ranking does not know
//! fails the test by name rather than passing unnoticed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

/// A position an item can occupy in its type's run of items.
///
/// One variant per distinct rendered item, so the order tables below are
/// the only place a position is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// `pub struct T`, `pub enum T`, `pub type T`.
    Declaration,
    /// `impl Deref for T`.
    Deref,
    /// `impl From<T> for Inner`: a newtype's out-of-Self conversion.
    FromSelfIntoOther,
    /// `impl From<Inner> for T`, `T` a struct: the into-Self conversion.
    FromOtherIntoSelf,
    /// `impl From<Payload> for T`, `T` an enum: a variant conversion.
    VariantFrom,
    /// `impl Display for T`.
    Display,
    /// `impl FromStr for T`.
    FromStr,
    /// `impl TryFrom<&str> for T`.
    TryFromStrRef,
    /// `impl TryFrom<String> for T`.
    TryFromString,
    /// `impl TryFrom<Inner> for T`: the constrained-newtype constructor.
    TryFromInner,
    /// `impl TryFrom<T> for Other`: a builder's fallible finish.
    TryFromSelfIntoOther,
    /// `impl Default for T`.
    Default,
    /// `impl T { .. }`.
    Inherent,
    /// `impl Serialize for T`.
    Serialize,
    /// `impl Deserialize for T`.
    Deserialize,
    /// `impl JsonSchema for T`.
    JsonSchema,
}

impl Slot {
    /// A name for this position, as it reads in a failure message.
    fn label(self) -> &'static str {
        match self {
            Slot::Declaration => "declaration",
            Slot::Deref => "Deref",
            Slot::FromSelfIntoOther => "From<Self> for Other",
            Slot::FromOtherIntoSelf => "From<Other> for Self",
            Slot::VariantFrom => "From<Payload> for Self (enum variant)",
            Slot::Display => "Display",
            Slot::FromStr => "FromStr",
            Slot::TryFromStrRef => "TryFrom<&str>",
            Slot::TryFromString => "TryFrom<String>",
            Slot::TryFromInner => "TryFrom<Inner> for Self",
            Slot::TryFromSelfIntoOther => "TryFrom<Self> for Other",
            Slot::Default => "Default",
            Slot::Inherent => "inherent impl",
            Slot::Serialize => "Serialize",
            Slot::Deserialize => "Deserialize",
            Slot::JsonSchema => "JsonSchema",
        }
    }
}

// ---------------------------------------------------------------------
// The order. Switching orders is an edit to these tables and nothing
// else, and the failures then enumerate every site that has to move.
// ---------------------------------------------------------------------

/// The order typify 1 emits, which typespace adopts for the integration.
///
/// `Serialize` sits immediately ahead of `Deserialize`: the two are
/// rendered together for unit and tuple structs, always in that order.
const ORDER: &[Slot] = &[
    // The declaration itself.
    Slot::Declaration,
    // Newtype conversions, out of Self and then into Self.
    Slot::Deref,
    Slot::FromSelfIntoOther,
    Slot::FromOtherIntoSelf,
    // The string surface.
    Slot::Display,
    Slot::FromStr,
    Slot::TryFromStrRef,
    Slot::TryFromString,
    // The constrained-newtype constructor.
    Slot::TryFromInner,
    // Enum per-variant payload conversions.
    Slot::VariantFrom,
    Slot::Default,
    // The `pub fn builder()` accessor.
    Slot::Inherent,
    Slot::Serialize,
    Slot::Deserialize,
    Slot::JsonSchema,
];

/// The order inside a `builder` module, which is its own arrangement.
///
/// typify emits the builder struct, its `Default`, its setters, and then
/// the two conversions between builder and built type.
const BUILDER_ORDER: &[Slot] = &[
    Slot::Declaration,
    Slot::Default,
    Slot::Inherent,
    Slot::TryFromSelfIntoOther,
    Slot::FromOtherIntoSelf,
];

/// The order to adopt once the typify integration lands.
///
/// It reads as four groups: the definition, then the type-specific
/// conversions (a struct's builder accessor, a newtype's `Deref` and its
/// out-of-Self then into-Self conversions, an enum's variant payload
/// conversions), then the string constructors, then the hand-written
/// impls.
///
/// `TryFrom<&str>` and `TryFrom<String>` assert what the type is, a
/// string that is not any old string, so they belong near the
/// declaration; `FromStr` is interpretation, closer to `Deserialize`,
/// so it belongs among the hand-written impls.
///
/// The definition group also fixes an attribute order ahead of the
/// declaration: doc comment, custom attributes, `derive`, `serde`. That
/// is attribute order rather than item order, so no slot ranks it.
#[allow(dead_code)]
const INTENDED_ORDER: &[Slot] = &[
    // A: the definition.
    Slot::Declaration,
    // B: type-specific conversions.
    Slot::Inherent,
    Slot::Deref,
    Slot::FromSelfIntoOther,
    Slot::FromOtherIntoSelf,
    Slot::TryFromInner,
    Slot::VariantFrom,
    // C: string constructors.
    Slot::TryFromStrRef,
    Slot::TryFromString,
    // D: hand-written impls.
    Slot::Default,
    Slot::Display,
    Slot::FromStr,
    Slot::Serialize,
    Slot::Deserialize,
    Slot::JsonSchema,
];

/// Modules whose contents are a hand-authored literal, order and all.
const LITERAL_MODULES: &[&str] = &["error"];

/// The lowest corpus totals that mean the walk really read the corpus.
///
/// A checker that silently parses nothing passes every assertion, so the
/// totals are asserted too. These sit well under the corpus, which holds
/// 70 files, 190 type groups, and 390 items.
const MIN_FILES: usize = 50;
const MIN_GROUPS: usize = 100;
const MIN_ITEMS: usize = 250;

// ---------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------

/// What a declaration declares, which decides how to read `From`.
#[derive(Clone, Copy)]
enum Kind {
    Struct,
    Enum,
    Alias,
}

/// One item, with the position it was classified into.
struct Entry {
    slot: Slot,
    /// The item as it reads in a failure message.
    rendered: String,
}

/// The items one type owns, in the order they appear.
struct Group {
    file: PathBuf,
    module: Vec<String>,
    type_name: String,
    entries: Vec<Entry>,
}

impl Group {
    /// The order that governs this group.
    fn order(&self) -> &'static [Slot] {
        match self.module.last().map(String::as_str) {
            Some("builder") => BUILDER_ORDER,
            _ => ORDER,
        }
    }
}

/// Everything one pass over the corpus produces.
struct Scan {
    files: usize,
    groups: Vec<Group>,
    /// Impls that could not be classified or ranked.
    unclassified: Vec<String>,
    /// `derive` lists that are not in string order.
    derive_problems: Vec<String>,
}

impl Scan {
    fn item_count(&self) -> usize {
        self.groups.iter().map(|group| group.entries.len()).sum()
    }
}

/// Read, parse, and group every generated file in `tests/output`.
fn scan() -> Scan {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/output");
    let paths = rust_files(&root);
    assert!(
        !paths.is_empty(),
        "no generated files found under {}",
        root.display()
    );

    let mut scan = Scan {
        files: paths.len(),
        groups: Vec::new(),
        unclassified: Vec::new(),
        derive_problems: Vec::new(),
    };

    for path in &paths {
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()));
        let file = syn::parse_file(&contents)
            .unwrap_or_else(|e| panic!("{} did not parse: {e}", path.display()));
        scan_scope(&file.items, path, &[], &mut scan);
    }

    scan
}

/// Every `*.rs` file under `root`, in a stable order.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} could not be listed: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("directory entry readable").path();
            match path.is_dir() {
                true => pending.push(path),
                false if path.extension().is_some_and(|ext| ext == "rs") => found.push(path),
                false => {}
            }
        }
    }
    found.sort();
    found
}

/// Group one module body's items, then recurse into nested modules.
fn scan_scope(items: &[syn::Item], file: &Path, module: &[String], scan: &mut Scan) {
    let declared = declared_types(items);

    // First seen order, so a group reads in the order it was rendered.
    let mut names: Vec<String> = Vec::new();
    let mut owned: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    let mut push = |name: String, entry: Entry| {
        if !owned.contains_key(&name) {
            names.push(name.clone());
        }
        owned.entry(name).or_default().push(entry);
    };

    for item in items {
        check_derives(item, file, module, scan);

        match item {
            syn::Item::Struct(item) => push(item.ident.to_string(), declaration(&item.ident)),
            syn::Item::Enum(item) => push(item.ident.to_string(), declaration(&item.ident)),
            syn::Item::Type(item) => push(item.ident.to_string(), declaration(&item.ident)),
            syn::Item::Union(item) => push(item.ident.to_string(), declaration(&item.ident)),
            syn::Item::Impl(item) => match classify(item, &declared) {
                Ok((name, slot)) => push(
                    name,
                    Entry {
                        slot,
                        rendered: render_impl(item),
                    },
                ),
                Err(reason) => scan.unclassified.push(format!(
                    "{}: {}: {reason}",
                    file.display(),
                    module_label(module)
                )),
            },
            syn::Item::Mod(item) => {
                let name = item.ident.to_string();
                if LITERAL_MODULES.contains(&name.as_str()) {
                    continue;
                }
                let Some((_brace, inner)) = &item.content else {
                    continue;
                };
                let nested = [module, &[name]].concat();
                scan_scope(inner, file, &nested, scan);
            }
            // Items no type owns and that carry no position of their
            // own: the `defaults` module's functions, `use`, consts.
            _ => {}
        }
    }

    scan.groups.extend(names.into_iter().map(|type_name| Group {
        file: file.to_path_buf(),
        module: module.to_vec(),
        entries: owned.remove(&type_name).expect("name was pushed"),
        type_name,
    }));
}

fn declaration(ident: &syn::Ident) -> Entry {
    Entry {
        slot: Slot::Declaration,
        rendered: format!("{ident}"),
    }
}

/// The types this module body declares, and what each one is.
fn declared_types(items: &[syn::Item]) -> BTreeMap<String, Kind> {
    items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some((item.ident.to_string(), Kind::Struct)),
            syn::Item::Union(item) => Some((item.ident.to_string(), Kind::Struct)),
            syn::Item::Enum(item) => Some((item.ident.to_string(), Kind::Enum)),
            syn::Item::Type(item) => Some((item.ident.to_string(), Kind::Alias)),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------

/// The type an `impl` belongs to, and the position it occupies.
///
/// Ownership follows the locally declared side. The `for` side wins when
/// it is a type this module declares, which covers inherent impls, trait
/// impls, and both `From<Inner> for Self` and `From<Payload> for Self`.
/// A newtype's out-of-Self conversion, `impl From<Ty> for String`, has a
/// `for` side that is not local, so it belongs to `Ty` on the argument
/// side; attributing it to `String` would drop it from the check.
fn classify(
    item: &syn::ItemImpl,
    declared: &BTreeMap<String, Kind>,
) -> Result<(String, Slot), String> {
    let self_local = local_name(&item.self_ty, declared);

    let Some((trait_path, _for)) = &item.trait_ else {
        let name = self_local.ok_or_else(|| {
            format!(
                "`{}` is an inherent impl on a type this module does not declare",
                render_impl(item)
            )
        })?;
        return Ok((name, Slot::Inherent));
    };

    let trait_name = trait_path
        .segments
        .last()
        .expect("a trait path has a segment")
        .ident
        .to_string();
    let argument = first_type_argument(trait_path);
    let argument_local = argument.and_then(|ty| local_name(ty, declared));

    let (owner, self_owns) = match (self_local, argument_local) {
        (Some(name), _) => (name, true),
        (None, Some(name)) => (name, false),
        (None, None) => {
            return Err(format!(
                "`{}` names no locally declared type on either side, \
                 so no type owns it",
                render_impl(item)
            ));
        }
    };

    // `From` and `TryFrom` are the only conversions out of Self, so they
    // are the only ones an argument-side owner can hold.
    let slot = match (trait_name.as_str(), self_owns) {
        ("From", false) => Slot::FromSelfIntoOther,
        ("TryFrom", false) => Slot::TryFromSelfIntoOther,
        (other, false) => {
            return Err(format!(
                "`{}` is owned by `{owner}` on the argument side, but \
                 `{other}` is not a conversion out of Self",
                render_impl(item)
            ));
        }
        ("Deref", _) => Slot::Deref,
        ("Display", _) => Slot::Display,
        ("FromStr", _) => Slot::FromStr,
        ("Default", _) => Slot::Default,
        ("Serialize", _) => Slot::Serialize,
        ("Deserialize", _) => Slot::Deserialize,
        ("JsonSchema", _) => Slot::JsonSchema,
        ("From", _) => match declared.get(&owner) {
            Some(Kind::Enum) => Slot::VariantFrom,
            _ => Slot::FromOtherIntoSelf,
        },
        ("TryFrom", _) => match argument {
            Some(ty) if is_str_reference(ty) => Slot::TryFromStrRef,
            Some(ty) if is_string(ty) => Slot::TryFromString,
            _ => Slot::TryFromInner,
        },
        (other, _) => {
            return Err(format!(
                "`{}` implements `{other}`, which no slot covers; \
                 add a `Slot` for it and place it in the order tables",
                render_impl(item)
            ));
        }
    };

    Ok((owner, slot))
}

/// The name of a type this module declares, if the type names one.
///
/// A single unqualified segment only: `super::MyStruct` and
/// `::std::string::String` name types from elsewhere even when a local
/// type happens to share the last segment, as a builder module's struct
/// shares its name with the type it builds.
fn local_name(ty: &syn::Type, declared: &BTreeMap<String, Kind>) -> Option<String> {
    let syn::Type::Path(ty) = ty else {
        return None;
    };
    if ty.qself.is_some() || ty.path.leading_colon.is_some() || ty.path.segments.len() != 1 {
        return None;
    }
    let name = ty.path.segments[0].ident.to_string();
    declared.contains_key(&name).then_some(name)
}

/// The first type argument of a path's last segment.
fn first_type_argument(path: &syn::Path) -> Option<&syn::Type> {
    let segment = path.segments.last()?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

fn is_str_reference(ty: &syn::Type) -> bool {
    let syn::Type::Reference(ty) = ty else {
        return false;
    };
    last_segment_is(&ty.elem, "str")
}

fn is_string(ty: &syn::Type) -> bool {
    last_segment_is(ty, "String")
}

fn last_segment_is(ty: &syn::Type, name: &str) -> bool {
    let syn::Type::Path(ty) = ty else {
        return false;
    };
    ty.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

// ---------------------------------------------------------------------
// Derive lists
// ---------------------------------------------------------------------

/// Record any `derive` on this item that is not in string order.
///
/// typify collects derives into a `BTreeSet<&str>`, so its lists come
/// out sorted by the rendered path.
fn check_derives(item: &syn::Item, file: &Path, module: &[String], scan: &mut Scan) {
    let (name, attrs) = match item {
        syn::Item::Struct(item) => (item.ident.to_string(), &item.attrs),
        syn::Item::Enum(item) => (item.ident.to_string(), &item.attrs),
        syn::Item::Type(item) => (item.ident.to_string(), &item.attrs),
        syn::Item::Union(item) => (item.ident.to_string(), &item.attrs),
        _ => return,
    };

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("derive")) {
        let paths = attr
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "{}: {}: `{name}` has a derive that did not parse: {e}",
                    file.display(),
                    module_label(module)
                )
            });
        let derived = paths.iter().map(render_tokens).collect::<Vec<_>>();
        let sorted = derived.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let sorted = sorted.into_iter().collect::<Vec<_>>();

        if derived != sorted {
            let inverted = derived
                .windows(2)
                .find(|pair| pair[0] > pair[1])
                .map(|pair| format!("`{}` precedes `{}`", pair[0], pair[1]))
                .unwrap_or_else(|| String::from("a repeated derive is listed twice"));
            scan.derive_problems.push(format!(
                "{}: {}: type `{name}`\n  \
                 derived:  {}\n  \
                 in order: {}\n  \
                 first inversion: {inverted}",
                file.display(),
                module_label(module),
                derived.join(", "),
                sorted.join(", "),
            ));
        }
    }
}

// ---------------------------------------------------------------------
// Rendering for failure messages
// ---------------------------------------------------------------------

fn module_label(module: &[String]) -> String {
    match module.is_empty() {
        true => String::from("file root"),
        false => format!("mod {}", module.join("::")),
    }
}

fn render_impl(item: &syn::ItemImpl) -> String {
    let self_ty = render_tokens(&item.self_ty);
    match &item.trait_ {
        Some((path, _for)) => format!("impl {} for {self_ty}", render_tokens(path)),
        None => format!("impl {self_ty}"),
    }
}

/// Tokens as a message reads them, with the spacing `quote` adds undone.
///
/// A path comes back as written, so `::std::convert::From<T>` reads that
/// way rather than with a space around every separator.
fn render_tokens<T: quote::ToTokens>(tokens: &T) -> String {
    let rendered = quote::quote!(#tokens).to_string();
    [" :: ", ":: ", " ::"]
        .iter()
        .fold(rendered, |acc, pattern| acc.replace(pattern, "::"))
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace("& ", "&")
        .replace(" ,", ",")
        .replace(" '", "'")
}

/// A group's items, numbered, one per line.
fn render_sequence(group: &Group) -> String {
    group
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            format!(
                "    {}. {} [{}]\n",
                i + 1,
                entry.rendered,
                entry.slot.label()
            )
        })
        .collect()
}

// ---------------------------------------------------------------------
// The assertions
// ---------------------------------------------------------------------

/// Every type's items appear in the canonical order.
#[test]
fn item_order_is_canonical() {
    let scan = scan();
    let mut problems = scan.unclassified.clone();

    for group in &scan.groups {
        let order = group.order();
        let mut ranked: Vec<(usize, &Entry)> = Vec::new();
        let mut unranked: Vec<String> = Vec::new();

        for entry in &group.entries {
            match order.iter().position(|slot| *slot == entry.slot) {
                Some(rank) => ranked.push((rank, entry)),
                None => unranked.push(format!(
                    "    {} [{}] has no place in this order",
                    entry.rendered,
                    entry.slot.label()
                )),
            }
        }

        let inversions = ranked
            .windows(2)
            .filter(|pair| pair[0].0 > pair[1].0)
            .map(|pair| {
                let ((left_rank, left), (right_rank, right)) = (pair[0], pair[1]);
                format!(
                    "    `{}` [{}] precedes `{}` [{}], but the order puts \
                     {} (position {}) ahead of {} (position {})\n",
                    left.rendered,
                    left.slot.label(),
                    right.rendered,
                    right.slot.label(),
                    right.slot.label(),
                    right_rank + 1,
                    left.slot.label(),
                    left_rank + 1,
                )
            })
            .collect::<String>();

        if inversions.is_empty() && unranked.is_empty() {
            continue;
        }

        problems.push(format!(
            "{}: {}: type `{}`\n  rendered:\n{}  out of order:\n{}{}",
            group.file.display(),
            module_label(&group.module),
            group.type_name,
            render_sequence(group),
            inversions,
            match unranked.is_empty() {
                true => String::new(),
                false => format!("  unranked:\n{}\n", unranked.join("\n")),
            },
        ));
    }

    assert!(
        problems.is_empty(),
        "{} of {} type groups are not in the canonical order \
         (see the order tables in tests/item_order.rs):\n\n{}",
        problems.len(),
        scan.groups.len(),
        problems.join("\n"),
    );
}

/// Every `derive` list is in string order.
#[test]
fn derive_lists_are_sorted() {
    let scan = scan();
    assert!(
        scan.derive_problems.is_empty(),
        "{} derive lists are not in string order:\n\n{}",
        scan.derive_problems.len(),
        scan.derive_problems.join("\n\n"),
    );
}

/// The walk reads the whole corpus rather than quietly skipping it.
#[test]
fn corpus_is_read() {
    let scan = scan();
    println!(
        "read {} files, {} type groups, {} items",
        scan.files,
        scan.groups.len(),
        scan.item_count()
    );

    assert!(
        scan.files >= MIN_FILES,
        "only {} generated files were read, fewer than the {MIN_FILES} \
         the corpus holds",
        scan.files
    );
    assert!(
        scan.groups.len() >= MIN_GROUPS,
        "only {} type groups were found, fewer than the {MIN_GROUPS} \
         the corpus holds",
        scan.groups.len()
    );
    assert!(
        scan.item_count() >= MIN_ITEMS,
        "only {} items were classified, fewer than the {MIN_ITEMS} \
         the corpus holds",
        scan.item_count()
    );
}

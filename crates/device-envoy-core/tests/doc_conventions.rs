#![allow(missing_docs)]
//! Host tests of documentation conventions across Core, RP, and ESP.
//!
//! These read the crates' sources as text, so they run on the host without compiling the RP
//! or ESP HALs. Like `macro_expansion.rs`, this file is excluded from the published package
//! because it reads the RP and ESP sources.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use quote::ToTokens;
use syn::{Attribute, Item, Visibility, parse::Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Where RP and ESP macro summaries intentionally differ, as (macro, chip, the start of the
/// paragraph only that chip has). A paragraph is compared after chip names and line
/// wrapping are normalized.
const SUMMARY_DIFFERENCES: [(&str, &str, &str); 30] = [
    // RP's button example uses `PIN_13` and RP imports; ESP's uses `GPIO13`.
    (
        "button_watch",
        "RP",
        "Use `button_watch!` instead of [`ButtonRp`]",
    ),
    ("button_watch", "RP", "```rust,no_run"),
    ("button_watch", "RP", "async fn example("),
    (
        "button_watch",
        "RP",
        "button_watch! { ButtonWatch13 { pin: PIN_13",
    ),
    (
        "button_watch",
        "ESP",
        "Use `button_watch!` instead of [`ButtonEsp`]",
    ),
    ("button_watch", "ESP", "```rust,no_run"),
    ("button_watch", "ESP", "async fn example("),
    (
        "button_watch",
        "ESP",
        "button_watch! { ButtonWatch13 { pin: GPIO13",
    ),
    // RP IR receivers share a PIO resource; each ESP receiver has its own RMT channel.
    (
        "ir",
        "RP",
        "- [`irs!`](crate::irs) — Share one PIO resource",
    ),
    (
        "ir",
        "ESP",
        "- [`irs!`](crate::irs) — Generate multiple IR receivers",
    ),
    (
        "ir_kepler",
        "RP",
        "- [`ir_keplers!`](crate::ir_keplers) — Share one PIO resource",
    ),
    (
        "ir_kepler",
        "ESP",
        "- [`ir_keplers!`](crate::ir_keplers) — Build multiple",
    ),
    (
        "ir_keplers",
        "RP",
        "Macro to generate multiple Kepler IR struct types that share",
    ),
    (
        "ir_keplers",
        "RP",
        "- [`ir_kepler!`](crate::ir_kepler) — Generate a single Kepler",
    ),
    (
        "ir_keplers",
        "ESP",
        "Macro to generate multiple Kepler IR struct types.",
    ),
    (
        "ir_keplers",
        "ESP",
        "- [`ir_kepler!`](crate::ir_kepler) — Generate a single Kepler",
    ),
    (
        "ir_mapping",
        "RP",
        "- [`ir_mappings!`](crate::ir_mappings) — Share one PIO resource",
    ),
    (
        "ir_mapping",
        "ESP",
        "- [`ir_mappings!`](crate::ir_mappings) — Generate multiple",
    ),
    (
        "ir_mappings",
        "RP",
        "Macro to generate multiple IR mapping struct types that share",
    ),
    (
        "ir_mappings",
        "RP",
        "- [`ir_mapping!`](crate::ir_mapping) — Generate a single IR",
    ),
    (
        "ir_mappings",
        "ESP",
        "Macro to generate multiple IR mapping struct types.",
    ),
    (
        "ir_mappings",
        "ESP",
        "- [`ir_mapping!`](crate::ir_mapping) — Generate a single IR",
    ),
    (
        "irs",
        "RP",
        "Macro to generate multiple IR receiver struct types that share",
    ),
    (
        "irs",
        "RP",
        "- [`ir!`](crate::ir!) — Generate a single IR receiver type",
    ),
    (
        "irs",
        "ESP",
        "Macro to generate multiple IR receiver struct types.",
    ),
    (
        "irs",
        "ESP",
        "- [`ir!`](crate::ir!) — Generate a single IR receiver type",
    ),
    // `led_strips!` exists only on RP.
    ("led2d", "RP", "- [`led_strips!`](crate::led_strips)"),
    ("led2d", "ESP", "- [`led_strip!`](macro@crate::led_strip)"),
    ("led_strip", "RP", "- [`led_strips!`](crate::led_strips)"),
    ("led_strip", "ESP", "- [`led2d!`](macro@crate::led2d)"),
];

/// ESP `servo_player!` also explains LEDC claims, which RP servos do not have.
const SUMMARY_ESP_ONLY_PARAGRAPHS: [(&str, &str); 2] = [
    (
        "servo_player",
        "The macro claims one whole [LEDC](crate#glossary)",
    ),
    (
        "servo_player",
        "[LEDC](crate#glossary) timer/channel availability",
    ),
];

#[test]
fn rp_and_esp_macro_summaries_match() -> TestResult {
    // The Syntax and Fields parity test in `macro_expansion.rs` covers generated docs; this
    // covers the hand-written summary above them, where ESP `led_strip!` once lost its
    // summary and examples link without any test noticing.
    let rp = macro_summaries("device-envoy-rp")?;
    let esp = macro_summaries("device-envoy-esp")?;
    let mut problems = Vec::new();
    for (name, rp_paragraphs) in &rp {
        let Some(esp_paragraphs) = esp.get(name) else {
            continue;
        };
        for (chip, paragraph) in rp_paragraphs
            .difference(esp_paragraphs)
            .map(|paragraph| ("RP", paragraph))
            .chain(
                esp_paragraphs
                    .difference(rp_paragraphs)
                    .map(|paragraph| ("ESP", paragraph)),
            )
        {
            let allowed = SUMMARY_DIFFERENCES
                .iter()
                .any(|(macro_name, allowed_chip, start)| {
                    macro_name == name && *allowed_chip == chip && paragraph.starts_with(start)
                })
                || (chip == "ESP"
                    && SUMMARY_ESP_ONLY_PARAGRAPHS
                        .iter()
                        .any(|(macro_name, start)| {
                            macro_name == name && paragraph.starts_with(start)
                        }));
            if !allowed {
                problems.push(format!("`{name}!` {chip} only: {paragraph}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "RP and ESP macro summaries differ; fix the drift or add the paragraph to \
         `SUMMARY_DIFFERENCES`:\n{}",
        problems.join("\n")
    );
    Ok(())
}

/// Core items whose docs may point at "the platform crate" in plain text, because no platform
/// page shows them.
const CORE_ONLY_POINTERS: [&str; 1] = [
    // ESP's `time_sync` module is hidden and RP does not re-export it, so `TimeSync`'s page
    // exists only on Core's site, where the pointer is the right advice.
    "TimeSync",
];

#[test]
fn core_item_docs_link_instead_of_naming_the_platform_crate() -> TestResult {
    // Core cannot link to RP or ESP, and RP and ESP show Core's text unchanged on their own
    // pages for re-exported items. So a Core item's docs must not send readers to "the
    // platform crate" in plain text; the platform crates add a linked pointer on their
    // re-export instead (rustdoc shows a named re-export's doc comment before the item's own).
    let mut problems = Vec::new();
    for path in rust_files(&crate_src("device-envoy-core"))? {
        let file = syn::parse_file(&fs::read_to_string(&path)?)?;
        let mut items = Vec::new();
        public_documented_items(&file.items, &mut items);
        for (name, docs) in items {
            if CORE_ONLY_POINTERS.contains(&name.as_str()) {
                continue;
            }
            let lowered = docs.to_lowercase();
            if [
                "the platform crate's",
                "your platform crate",
                "the platform crate (",
                "the platform crate `",
            ]
            .iter()
            .any(|pointer| lowered.contains(pointer))
            {
                problems.push(format!("{}: `{name}`", path.display()));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "these Core items point at the platform crate in plain text; remove the pointer and \
         add a linked doc comment on the RP and ESP re-exports instead:\n{}",
        problems.join("\n")
    );
    Ok(())
}

#[test]
fn rp_and_esp_led_strip_reexport_core_api_and_macro_helpers() -> TestResult {
    // The platform `led_strip` modules list Core's items by name (a glob would drop their
    // re-export doc comments), so a new Core API item or macro helper must be added there too.
    let mut core_names = public_item_names(&syn::parse_file(&fs::read_to_string(
        crate_src("device-envoy-core").join("led_strip.rs"),
    )?)?);
    // Correction is shared across crates but called only by their private device loops.
    assert!(core_names.remove("apply_correction"));
    let mut problems = Vec::new();
    for crate_dir in ["device-envoy-rp", "device-envoy-esp"] {
        let source = fs::read_to_string(crate_src(crate_dir).join("led_strip.rs"))?;
        let reexported = reexported_names(&syn::parse_file(&source)?, "led_strip");
        assert!(
            !reexported.contains("apply_correction"),
            "{crate_dir} must keep frame correction private"
        );
        for name in core_names.difference(&reexported) {
            problems.push(format!("{crate_dir} does not re-export `{name}`"));
        }
    }
    assert!(
        problems.is_empty(),
        "add these to the named `pub use device_envoy_core::led_strip::{{...}}` list:\n{}",
        problems.join("\n")
    );
    Ok(())
}

#[test]
fn doc_links_follow_the_conventions() -> TestResult {
    // Conventions from `AGENTS.md` that a reader notices but rustdoc does not check.
    let mut problems = Vec::new();
    for crate_dir in ["device-envoy-core", "device-envoy-rp", "device-envoy-esp"] {
        for path in doc_files(&crate_src(crate_dir))? {
            for (line_index, line) in fs::read_to_string(&path)?.lines().enumerate() {
                let is_markdown = path.extension().is_some_and(|extension| extension == "md");
                let trimmed = line.trim_start();
                if !is_markdown && !(trimmed.starts_with("///") || trimmed.starts_with("//!")) {
                    continue;
                }
                let place = format!("{}:{}", path.display(), line_index + 1);
                // A link whose text names a macro must go to the macro, not its module.
                if line.contains("!`](mod@") {
                    problems.push(format!("{place}: a `name!` link targets a module"));
                }
                // Module links read "<name> module documentation".
                for (link_start, _) in line.match_indices("](mod@") {
                    let text_start = line[..link_start].rfind('[').unwrap_or(0);
                    let link_text = &line[text_start..link_start];
                    if link_text.contains("module") && !link_text.contains("module documentation") {
                        problems.push(format!(
                            "{place}: module link text should say \"module documentation\""
                        ));
                    }
                }
                let lowered = line.to_lowercase();
                for phrase in ["module-level example", "struct-level example"] {
                    if lowered.contains(phrase) {
                        problems.push(format!(
                            "{place}: \"{phrase}\"; name the example instead (\"cyd module example\")"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "documentation does not follow the link conventions:\n{}",
        problems.join("\n")
    );
    Ok(())
}

/// Each public `define!` macro's hand-written summary under `crate_dir`, by macro name, as
/// a set of normalized paragraphs.
fn macro_summaries(
    crate_dir: &str,
) -> Result<BTreeMap<String, BTreeSet<String>>, Box<dyn std::error::Error>> {
    let crate_name = crate_dir.replace('-', "_");
    let mut summaries = BTreeMap::new();
    for path in rust_files(&crate_src(crate_dir))? {
        let file = syn::parse_file(&fs::read_to_string(&path)?)?;
        let mut definitions = Vec::new();
        collect_definitions(&file.items, &mut definitions);
        for tokens in definitions {
            let (attrs, name) = (|input: syn::parse::ParseStream| {
                let attrs = input.call(Attribute::parse_outer)?;
                input.parse::<Visibility>()?;
                let name: syn::Ident = input.parse()?;
                input.parse::<proc_macro2::TokenStream>()?;
                Ok((attrs, name.to_string()))
            })
            .parse2(tokens)?;
            if name.starts_with("__") {
                continue;
            }
            let docs = doc_text(&attrs).replace(&crate_name, "device_envoy_CHIP");
            let paragraphs = docs
                .split("\n\n")
                .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|paragraph| !paragraph.is_empty())
                .collect();
            summaries.insert(name, paragraphs);
        }
    }
    Ok(summaries)
}

/// The doc text of `attrs`: literal `#[doc]` strings joined with newlines, and each
/// `include_str!` written as its call so both chips compare equal when they include the
/// same file.
fn doc_text(attrs: &[Attribute]) -> String {
    let mut docs = String::new();
    for attr in attrs {
        let syn::Meta::NameValue(name_value) = &attr.meta else {
            continue;
        };
        if !name_value.path.is_ident("doc") {
            continue;
        }
        match &name_value.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) => docs.push_str(&text.value()),
            other => docs.push_str(&format!("\n\n{}\n\n", other.to_token_stream())),
        }
        docs.push('\n');
    }
    docs
}

fn is_doc_hidden(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("doc") && attr.to_token_stream().to_string().contains("hidden")
    })
}

/// Every public, not hidden item in `items` that has docs, with its name, including items
/// in inline modules, impl blocks, and traits.
fn public_documented_items(items: &[Item], found: &mut Vec<(String, String)>) {
    for item in items {
        let (attrs, vis, name): (&[Attribute], Option<&Visibility>, String) = match item {
            Item::Struct(item) => (&item.attrs, Some(&item.vis), item.ident.to_string()),
            Item::Enum(item) => (&item.attrs, Some(&item.vis), item.ident.to_string()),
            Item::Trait(item) => {
                if matches!(item.vis, Visibility::Public(_)) && !is_doc_hidden(&item.attrs) {
                    for trait_item in &item.items {
                        if let syn::TraitItem::Fn(function) = trait_item {
                            found.push((
                                format!("{}::{}", item.ident, function.sig.ident),
                                doc_text(&function.attrs),
                            ));
                        }
                    }
                }
                (&item.attrs, Some(&item.vis), item.ident.to_string())
            }
            Item::Fn(item) => (&item.attrs, Some(&item.vis), item.sig.ident.to_string()),
            Item::Const(item) => (&item.attrs, Some(&item.vis), item.ident.to_string()),
            Item::Static(item) => (&item.attrs, Some(&item.vis), item.ident.to_string()),
            Item::Type(item) => (&item.attrs, Some(&item.vis), item.ident.to_string()),
            Item::Impl(item_impl) => {
                if !is_doc_hidden(&item_impl.attrs) {
                    let self_type = item_impl.self_ty.to_token_stream().to_string();
                    for impl_item in &item_impl.items {
                        if let syn::ImplItem::Fn(function) = impl_item
                            && matches!(function.vis, Visibility::Public(_))
                            && !is_doc_hidden(&function.attrs)
                        {
                            found.push((
                                format!("{self_type}::{}", function.sig.ident),
                                doc_text(&function.attrs),
                            ));
                        }
                    }
                }
                continue;
            }
            Item::Mod(item_mod) => {
                if let Some((_, mod_items)) = &item_mod.content
                    && !is_doc_hidden(&item_mod.attrs)
                {
                    public_documented_items(mod_items, found);
                }
                continue;
            }
            _ => continue,
        };
        if vis.is_some_and(|vis| matches!(vis, Visibility::Public(_))) && !is_doc_hidden(attrs) {
            found.push((name, doc_text(attrs)));
        }
    }
}

/// The names of `file`'s public top-level items, including names it re-exports.
fn public_item_names(file: &syn::File) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in &file.items {
        let (vis, name) = match item {
            Item::Struct(item) => (&item.vis, item.ident.to_string()),
            Item::Enum(item) => (&item.vis, item.ident.to_string()),
            Item::Trait(item) => (&item.vis, item.ident.to_string()),
            Item::Fn(item) => (&item.vis, item.sig.ident.to_string()),
            Item::Const(item) => (&item.vis, item.ident.to_string()),
            Item::Static(item) => (&item.vis, item.ident.to_string()),
            Item::Type(item) => (&item.vis, item.ident.to_string()),
            Item::Mod(item) => (&item.vis, item.ident.to_string()),
            Item::Use(item_use) => {
                if matches!(item_use.vis, Visibility::Public(_)) {
                    use_tree_names(&item_use.tree, &mut names);
                }
                continue;
            }
            _ => continue,
        };
        if matches!(vis, Visibility::Public(_)) {
            names.insert(name);
        }
    }
    names
}

/// The names `file` publicly re-exports from `device_envoy_core::<module>`.
fn reexported_names(file: &syn::File, module: &str) -> BTreeSet<String> {
    let prefix = format!("device_envoy_core :: {module} ::");
    let mut names = BTreeSet::new();
    for item in &file.items {
        if let Item::Use(item_use) = item
            && matches!(item_use.vis, Visibility::Public(_))
            && item_use
                .tree
                .to_token_stream()
                .to_string()
                .starts_with(&prefix)
        {
            use_tree_names(&item_use.tree, &mut names);
        }
    }
    names
}

/// The final names a `use` tree brings in (renames count by their new name).
fn use_tree_names(tree: &syn::UseTree, names: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => use_tree_names(&path.tree, names),
        syn::UseTree::Name(name) => {
            names.insert(name.ident.to_string());
        }
        syn::UseTree::Rename(rename) => {
            names.insert(rename.rename.to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                use_tree_names(item, names);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

/// Every `macro_schema::define!` input in `items`, including inside inline modules.
fn collect_definitions(items: &[Item], definitions: &mut Vec<proc_macro2::TokenStream>) {
    for item in items {
        match item {
            Item::Macro(item_macro)
                if item_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "define") =>
            {
                definitions.push(item_macro.mac.tokens.clone());
            }
            Item::Mod(item_mod) => {
                if let Some((_, mod_items)) = &item_mod.content {
                    collect_definitions(mod_items, definitions);
                }
            }
            _ => {}
        }
    }
}

fn crate_src(crate_dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_dir)
        .join("src")
}

/// The `.rs` files under `dir`, recursively.
fn rust_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    Ok(doc_files(dir)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect())
}

/// The `.rs` and `.md` files under `dir`, recursively.
fn doc_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(doc_files(&path)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "md")
        {
            files.push(path);
        }
    }
    Ok(files)
}

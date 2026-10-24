#![allow(missing_docs)]
//! Host tests of what the declaration macros expand to.
//!
//! Each macro is a `macro_schema::define!` block in the module whose API it
//! generates. These tests read that module's source as text, find the block by
//! macro name, and expand calls with `macro_schema_core::expand_definition`, so
//! they run on the host without compiling the RP or ESP HALs. In an expansion,
//! `crate` is the crate that defines the macro.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use macro_schema_core::{Error, Result};
use pretty_assertions::assert_eq;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::{Attribute, Ident, Item, Visibility, parse::Parser};

const CORE_AUDIO_PLAYER: &str = include_str!("../src/audio_player.rs");
const RP_BUTTON: &str = include_str!("../../device-envoy-rp/src/button.rs");
const RP_IR: &str = include_str!("../../device-envoy-rp/src/ir.rs");
const RP_LED_STRIP: &str = include_str!("../../device-envoy-rp/src/led_strip.rs");
const ESP_LED2D: &str = include_str!("../../device-envoy-esp/src/led2d.rs");

#[test]
fn button_watch_names_its_struct_and_task_after_the_declaration() -> Result<()> {
    let expanded = expand(
        RP_BUTTON,
        "button_watch",
        quote! { ButtonWatch13 { pin: PIN_13 } },
    )?;
    assert_contains(&expanded, "struct ButtonWatch13 {");
    assert_contains(&expanded, "async fn button_watch13_task(");
    assert_contains(&expanded, "::embassy_rp::peripherals::PIN_13>");
    Ok(())
}

#[test]
fn led_strips_defaults_pio_and_assigns_dma_by_member_index() -> Result<()> {
    let expanded = expand(
        RP_LED_STRIP,
        "led_strips",
        quote! {
            pub LedStrips {
                S0 { pin: PIN_2, len: 8, max_current: Current::Milliamps(100) },
                S1 { pin: PIN_3, len: 4, max_current: Current::Milliamps(100) },
            }
        },
    )?;
    assert_contains(&expanded, "::embassy_rp::peripherals::PIO0>");
    assert_contains(
        &expanded,
        "s0_dma: impl Into<\n            ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::DMA_CH0>,",
    );
    assert_contains(
        &expanded,
        "s1_dma: impl Into<\n            ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::DMA_CH1>,",
    );
    Ok(())
}

#[test]
fn led_strips_members_inherit_the_group_visibility() -> Result<()> {
    let expanded = expand(
        RP_LED_STRIP,
        "led_strips",
        quote! {
            pub LedStrips {
                S0 { pin: PIN_2, len: 8, max_current: Current::Milliamps(100) },
            }
        },
    )?;
    assert_contains(&expanded, "crate::led_strip::__led_strip_type! {");
    assert_contains(&expanded, "pub S0 { pio : PIO0, sm : 0,");
    assert_contains(&expanded, "pub struct LedStrips;");
    Ok(())
}

#[test]
fn led_strip_type_names_its_struct_and_task_after_the_declaration() -> Result<()> {
    let expanded = expand(
        RP_LED_STRIP,
        "__led_strip_type",
        quote! {
            pub S0 {
                pio: PIO0,
                sm: 0,
                pin: PIN_2,
                len: 8,
                max_current: Current::Milliamps(100),
                dma: DMA_CH0,
                gamma: Gamma::Srgb,
                max_frames: 16,
            }
        },
    )?;
    assert_contains(&expanded, "pub struct S0 {");
    assert_contains(&expanded, "async fn s0_device_task(");
    assert_contains(
        &expanded,
        "::embassy_rp::peripherals::PIO0,
        0,",
    );
    Ok(())
}

#[test]
fn led_strips_rejects_the_0_1_member_colon() -> Result<()> {
    assert_eq!(
        expand_error(
            RP_LED_STRIP,
            "led_strips",
            quote! {
                LedStrips {
                    S0: { pin: PIN_2, len: 8, max_current: Current::Milliamps(100) },
                }
            },
        )?
        .as_deref(),
        Some("remove the `:` after `S0`; declarations are written `S0 { ... }`")
    );
    Ok(())
}

#[test]
fn led_strips_rejects_a_member_visibility() -> Result<()> {
    assert_eq!(
        expand_error(
            RP_LED_STRIP,
            "led_strips",
            quote! {
                LedStrips {
                    pub S0 { pin: PIN_2, len: 8, max_current: Current::Milliamps(100) },
                }
            },
        )?
        .as_deref(),
        Some("members take their group's visibility; remove this visibility")
    );
    Ok(())
}

#[test]
fn rp_ir_defaults_pio_to_pio0() -> Result<()> {
    let expanded = expand(RP_IR, "ir", quote! { Ir5 { pin: PIN_5 } })?;
    assert_contains(
        &expanded,
        "pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::PIO0>>,",
    );
    Ok(())
}

#[test]
fn esp_led2d_takes_its_length_from_led_layout() -> Result<()> {
    assert_eq!(
        expand_error(
            ESP_LED2D,
            "led2d",
            quote! {
                Panel { pin: GPIO4, len: 64, led_layout: LAYOUT, font: Led2dFont::Font3x4Trim }
            },
        )?
        .as_deref(),
        Some(
            "unknown field `len`; expected one of `pin`, `led_layout`, `font`, `max_current`, `engine`, `gamma`, `max_frames`"
        )
    );
    Ok(())
}

#[test]
fn pcm_clip_accepts_only_source_sample_rate_hz() -> Result<()> {
    assert_eq!(
        expand_error(
            CORE_AUDIO_PLAYER,
            "pcm_clip",
            quote! { Nasa { file: "nasa.s16", sample_rate_hz: 22_050 } },
        )?
        .as_deref(),
        Some(
            "unknown field `sample_rate_hz`; expected one of `file`, `source_sample_rate_hz`, `target_sample_rate_hz`"
        )
    );
    Ok(())
}

/// Macros that expand only inside Device Envoy's own crates, where `crate::`
/// in a doc link is correct.
const INTERNAL_MACROS: [&str; 1] = ["__impl_wifi_auto_fields"];

#[test]
fn generated_doc_links_name_the_crate() -> core::result::Result<(), Box<dyn std::error::Error>> {
    // Generated docs land in the user's crate, where `crate::` is the user's
    // crate. Rustdoc renders such a link as a dead `href` without a warning.
    let mut offenders = Vec::new();
    for crate_dir in ["device-envoy-core", "device-envoy-rp", "device-envoy-esp"] {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(crate_dir)
            .join("src");
        for path in rust_files(&src)? {
            let file = syn::parse_file(&fs::read_to_string(&path)?)?;
            for (name, tokens) in user_expanded_macros(&file.items) {
                let text = tokens.to_string();
                if text.contains("](crate::") || text.contains("](mod@crate::") {
                    offenders.push(format!("{} (`{name}!`)", path.display()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "generated doc links use `crate::`; name the crate, as in `device_envoy_rp::`:\n{}",
        offenders.join("\n")
    );
    Ok(())
}

/// Crates that generated code reaches through `$crate` re-exports, so the user's
/// crate need not depend on them.
const HIDDEN_CRATES: [&str; 4] = ["static_cell", "heapless", "embassy_sync", "paste"];

#[test]
fn generated_code_reaches_helper_crates_through_dollar_crate()
-> core::result::Result<(), Box<dyn std::error::Error>> {
    let mut offenders = Vec::new();
    for crate_dir in ["device-envoy-core", "device-envoy-rp", "device-envoy-esp"] {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(crate_dir)
            .join("src");
        for path in rust_files(&src)? {
            let file = syn::parse_file(&fs::read_to_string(&path)?)?;
            for (name, tokens) in user_expanded_macros(&file.items) {
                for hidden in names_bare_crate(tokens, &HIDDEN_CRATES) {
                    offenders.push(format!("{} (`{name}!` names `{hidden}`)", path.display()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "generated code names a helper crate directly; use `$crate::__<crate>`:\n{}",
        offenders.join("\n")
    );
    Ok(())
}

/// The names in `crates` that `tokens` uses as the first segment of a path, as in
/// `heapless::Vec` or `::heapless::Vec`, rather than after another segment such as
/// `$crate::__heapless`.
fn names_bare_crate(tokens: TokenStream, crates: &[&str]) -> Vec<String> {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let is_colon = |token: Option<&TokenTree>| matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == ':');
    let mut found = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => found.extend(names_bare_crate(group.stream(), crates)),
            TokenTree::Ident(ident) if crates.contains(&ident.to_string().as_str()) => {
                let starts_path =
                    is_colon(tokens.get(index + 1)) && is_colon(tokens.get(index + 2));
                let after_segment = index >= 3
                    && is_colon(tokens.get(index - 1))
                    && is_colon(tokens.get(index - 2))
                    && matches!(tokens.get(index - 3), Some(TokenTree::Ident(_)));
                if starts_path && !after_segment {
                    found.push(ident.to_string());
                }
            }
            _ => {}
        }
    }
    found
}

/// Where RP and ESP intentionally document a macro differently, by macro: the fields
/// (or `members`, for the group's member note) whose Syntax and Fields lines may differ
/// because the chips use different hardware or a different API.
const CHIP_DIFFERENCES: [(&str, &[&str]); 12] = [
    ("audio_player", &["pio", "dma", "i2s"]),
    ("button_watch", &["pin"]),
    ("ir", &["pio"]),
    ("ir_kepler", &["pio"]),
    ("ir_keplers", &["pio", "members"]),
    ("ir_mapping", &["pio"]),
    ("ir_mappings", &["pio", "members"]),
    ("irs", &["pio", "members"]),
    ("led2d", &["pio", "dma", "engine", "pin"]),
    ("led_strip", &["pio", "dma", "engine", "reset_us", "pin"]),
    ("servo", &["pin", "timer", "channel"]),
    ("servo_player", &["pin", "timer", "channel"]),
];

/// Macros that only one chip has.
const ONE_CHIP_MACROS: [&str; 1] = ["led_strips"];

#[test]
fn rp_and_esp_macros_document_the_same_contract()
-> core::result::Result<(), Box<dyn std::error::Error>> {
    // Each chip has its own `define!`; this keeps their user-facing Syntax and Fields
    // documentation from drifting apart except where `CHIP_DIFFERENCES` says they differ.
    let rp = public_macro_docs("device-envoy-rp")?;
    let esp = public_macro_docs("device-envoy-esp")?;
    let mut problems = Vec::new();
    for name in rp.keys().chain(esp.keys()).collect::<BTreeSet<_>>() {
        let (Some(rp_doc), Some(esp_doc)) = (rp.get(name), esp.get(name)) else {
            if !ONE_CHIP_MACROS.contains(&name.as_str()) {
                problems.push(format!("`{name}!` is defined on only one chip"));
            }
            continue;
        };
        let allowed = CHIP_DIFFERENCES
            .iter()
            .find(|(macro_name, _)| *macro_name == name)
            .map_or(&[][..], |(_, fields)| *fields);
        let rp_lines: BTreeSet<&str> = rp_doc.lines().collect();
        let esp_lines: BTreeSet<&str> = esp_doc.lines().collect();
        for (chip, line) in rp_lines
            .difference(&esp_lines)
            .map(|line| ("RP", line))
            .chain(esp_lines.difference(&rp_lines).map(|line| ("ESP", line)))
        {
            if !allowed.contains(&doc_line_key(line)) {
                problems.push(format!("`{name}!` {chip} only: {line}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "RP and ESP macro documentation differ; fix the drift or add the field to \
         `CHIP_DIFFERENCES`:\n{}",
        problems.join("\n")
    );
    Ok(())
}

/// The field a Syntax or Fields line describes (`pin` for `pin: <ident>,` or
/// ``| `pin` | ...``), or `members` for a member-group note.
fn doc_line_key(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("| `") {
        return rest.split('`').next().unwrap_or(rest);
    }
    if trimmed.starts_with("Each member is") {
        return "members";
    }
    trimmed.split(':').next().unwrap_or(trimmed)
}

/// The generated Syntax and Fields documentation of every public `define!` macro under
/// `crate_dir`'s `src`, by macro name.
fn public_macro_docs(
    crate_dir: &str,
) -> core::result::Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_dir)
        .join("src");
    let mut docs = BTreeMap::new();
    for path in rust_files(&src)? {
        let mut definitions = Vec::new();
        collect_definitions(
            &syn::parse_file(&fs::read_to_string(&path)?)?.items,
            &mut definitions,
        );
        for tokens in definitions {
            let name = defined_name(tokens.clone())?.to_string();
            if name.starts_with("__") {
                continue;
            }
            let emitted: syn::File = syn::parse2(macro_schema_core::define(tokens)?)?;
            for item in emitted.items {
                let Item::Macro(item_macro) = item else {
                    continue;
                };
                for attr in item_macro.attrs {
                    if let syn::Meta::NameValue(name_value) = &attr.meta
                        && name_value.path.is_ident("doc")
                        && let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(text),
                            ..
                        }) = &name_value.value
                        && text.value().contains("**Syntax:**")
                    {
                        docs.insert(name.clone(), text.value());
                    }
                }
            }
        }
    }
    Ok(docs)
}

/// Every `macro_schema::define!` input in `items`, including inside inline modules.
fn collect_definitions(items: &[Item], definitions: &mut Vec<TokenStream>) {
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
                if let Some((_, items)) = &item_mod.content {
                    collect_definitions(items, definitions);
                }
            }
            _ => {}
        }
    }
}

/// The `.rs` files under `dir`, recursively.
fn rust_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

/// The macro code in `items` that expands in a user's crate, by macro name:
/// each `define!` block's `generate` template and each `#[macro_export]`
/// `macro_rules!` body, except [`INTERNAL_MACROS`].
fn user_expanded_macros(items: &[Item]) -> Vec<(String, TokenStream)> {
    let mut found = Vec::new();
    for item in items {
        match item {
            Item::Macro(item_macro) => {
                let Some(last) = item_macro.mac.path.segments.last() else {
                    continue;
                };
                if last.ident == "define" {
                    if let Ok(name) = defined_name(item_macro.mac.tokens.clone())
                        && let Some(template) = generate_template(item_macro.mac.tokens.clone())
                    {
                        found.push((name.to_string(), template));
                    }
                } else if last.ident == "macro_rules"
                    && let Some(name) = &item_macro.ident
                    && item_macro
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("macro_export"))
                    && !INTERNAL_MACROS.contains(&name.to_string().as_str())
                {
                    found.push((name.to_string(), item_macro.mac.tokens.clone()));
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    found.extend(user_expanded_macros(items));
                }
            }
            _ => {}
        }
    }
    found
}

/// The body of the `generate { ... }` block in a `define!` input, if any.
fn generate_template(tokens: TokenStream) -> Option<TokenStream> {
    let mut tokens = tokens.into_iter();
    while let Some(token) = tokens.next() {
        if let TokenTree::Ident(ident) = &token
            && ident == "generate"
            && let Some(TokenTree::Group(group)) = tokens.next()
        {
            return Some(group.stream());
        }
    }
    None
}

fn assert_contains(expanded: &str, expected: &str) {
    assert!(
        expanded.contains(expected),
        "expected to find\n{expected}\nin the expansion\n{expanded}"
    );
}

/// Expands `call` with the `define!` block named `macro_name` in `source`,
/// pretty-printed.
fn expand(source: &str, macro_name: &str, call: TokenStream) -> Result<String> {
    let expanded = macro_schema_core::expand_definition(definition(source, macro_name)?, call)?;
    Ok(prettyplease::unparse(&syn::parse2(expanded)?))
}

/// The messages of the errors `call` produces with the `define!` block named
/// `macro_name` in `source`, one per line, or `None` if it expands.
fn expand_error(source: &str, macro_name: &str, call: TokenStream) -> Result<Option<String>> {
    Ok(
        macro_schema_core::expand_definition(definition(source, macro_name)?, call)
            .err()
            .map(|error| {
                error
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
    )
}

/// The input of the `macro_schema::define!` block named `macro_name` in `source`.
fn definition(source: &str, macro_name: &str) -> Result<TokenStream> {
    find_definition(&syn::parse_file(source)?.items, macro_name)?.ok_or_else(|| {
        Error::new(
            Span::call_site(),
            format!("no `macro_schema::define!` block named `{macro_name}`"),
        )
    })
}

fn find_definition(items: &[Item], macro_name: &str) -> Result<Option<TokenStream>> {
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
                if defined_name(item_macro.mac.tokens.clone())? == macro_name {
                    return Ok(Some(item_macro.mac.tokens.clone()));
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content
                    && let Some(tokens) = find_definition(items, macro_name)?
                {
                    return Ok(Some(tokens));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

/// The macro name a `define!` input declares: `{ATTR} VIS NAME ...`.
fn defined_name(tokens: TokenStream) -> Result<Ident> {
    (|input: syn::parse::ParseStream| {
        input.call(Attribute::parse_outer)?;
        input.parse::<Visibility>()?;
        let name: Ident = input.parse()?;
        input.parse::<TokenStream>()?;
        Ok(name)
    })
    .parse2(tokens)
}

#![allow(missing_docs)]
//! Host tests of what the declaration macros expand to.
//!
//! Each macro is a `macro_schema::define!` block in the module whose API it
//! generates. These tests read that module's source as text, find the block by
//! macro name, and expand calls with `macro_schema_core::expand_definition`, so
//! they run on the host without compiling the RP or ESP HALs. In an expansion,
//! `crate` is the crate that defines the macro.

use macro_schema_core::{Error, Result};
use pretty_assertions::assert_eq;
use proc_macro2::{Span, TokenStream};
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
    assert_contains(&expanded, "pub struct S0 {");
    assert_contains(&expanded, "pub struct LedStrips;");
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

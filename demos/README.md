# Demos

## A1 – 8-LED strip, blue/white alternating

Verifies wiring, PIO, DMA, and the LED strip device abstraction.
Demonstrates a frame as pure data using a palette-based pattern.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a1_strip_8_blue_gray --target thumbv6m-none-eabi --features pico1,arm
```

## A2 – 8-LED strip, blue/white blink

Alternates blue/white and white/blue frames without using `animate`.
Shows frame updates in a tight loop inside the demo.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a2_strip_8_blue_white_blink --target thumbv6m-none-eabi --features pico1,arm
```

## A3 – 8-LED strip, blue/white blink (animate)

Uses `animate` to alternate blue/white and white/blue frames.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a3_strip_8_blue_white_blink_animate --target thumbv6m-none-eabi --features pico1,arm
```

## A4 – 96-LED strip, white dot on blue background

Moves a single white dot along a 96-LED strip on PIN_4.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a4_strip_96_blue_white_dot --target thumbv6m-none-eabi --features pico1,arm
```

## B1 – 12×8 panel, "Rust" with cursor

Renders "Rust" on the first line with alternating blue and light gray letters and a 3-pixel cursor under the `R`.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_b1_panel_12x8_rust_cursor --target thumbv6m-none-eabi --features pico1,arm
```

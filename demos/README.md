# Demos

## A1 – 8-LED strip, blue/white alternating

Verifies wiring, PIO, DMA, and the LED strip device abstraction.
Demonstrates a frame as pure data using a palette-based pattern.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a1_strip_8_blue_gray --target thumbv6m-none-eabi --features pico1,arm
```

## A2 – 8-LED strip, white dot on blue background

Animates a single white dot over a blue background without using `animate`.
Shows frame updates in a tight loop inside the demo.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a2_strip_8_blue_white_dot --target thumbv6m-none-eabi --features pico1,arm
```

## A3 – 8-LED strip, blue/white blink with animate

Uses `animate` to blink between blue/white and white/blue frames.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a3_strip_8_blue_white_blink --target thumbv6m-none-eabi --features pico1,arm
```

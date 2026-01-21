# Demos

## A01 – 8-LED strip, blue/gray alternating

Verifies wiring, PIO, DMA, and the LED strip device abstraction.
Demonstrates a frame as pure data using a palette-based pattern.

Run/flash (Pico 1 ARM):

```bash
cargo run --release --bin demo_a01_strip_8_blue_gray --target thumbv6m-none-eabi --features pico1,arm
```

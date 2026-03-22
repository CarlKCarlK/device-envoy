# Demos

## A1 - 8-LED strip, blue/gray alternating

Verifies wiring and the LED strip device abstraction on ESP32.
This demo writes one static frame with alternating blue/gray pixels.

Wiring (ESP32-C6 and ESP32-S3):

- `GPIO10` -> NeoPixel-style (WS2812) strip `DIN`
- Board `GND` -> strip `GND`
- Strip `5V` -> external 5V supply
- External supply `GND` -> board `GND`

Run/flash (ESP32-C6):

```bash
cargo run --package device-envoy-esp-demos --bin demo_a1_strip_8_blue_gray --release --target riscv32imac-unknown-none-elf --no-default-features
```

Run/flash (ESP32-S3):

```bash
cargo +esp run --package device-envoy-esp-demos --bin demo_a1_strip_8_blue_gray --release --target xtensa-esp32s3-none-elf --no-default-features -Zbuild-std=core,alloc
```

## A3 - 8-LED strip, blue/gray blink (animate)

Creates two offset frames and animates between them continuously.

Run/flash (ESP32-C6):

```bash
cargo run --package device-envoy-esp-demos --bin demo_a3_strip_8_blue_white_blink_animate --release --target riscv32imac-unknown-none-elf --no-default-features
```

Run/flash (ESP32-S3):

```bash
cargo +esp run --package device-envoy-esp-demos --bin demo_a3_strip_8_blue_white_blink_animate --release --target xtensa-esp32s3-none-elf --no-default-features -Zbuild-std=core,alloc
```

## A4 - 96-LED strip, white dot on blue background

Moves a single white dot along a 96-LED strip on GPIO18.

Run/flash (ESP32-C6):

```bash
cargo run --package device-envoy-esp-demos --bin demo_a4_strip_96_blue_white_dot --release --target riscv32imac-unknown-none-elf --no-default-features
```

Run/flash (ESP32-S3):

```bash
cargo +esp run --package device-envoy-esp-demos --bin demo_a4_strip_96_blue_white_dot --release --target xtensa-esp32s3-none-elf --no-default-features -Zbuild-std=core,alloc
```

## B1 - 12x8 panel, "Go/Go" text colors

Writes `Go\nGo` on a rotated 12x8 panel, with per-character colors:
light gray, light gray, orange, hot pink.

Wiring (ESP32-C6 and ESP32-S3):

- `GPIO18` -> NeoPixel-style (WS2812) panel `DIN`
- Board `GND` -> panel `GND`
- Panel `5V` -> external 5V supply
- External supply `GND` -> board `GND`

Run/flash (ESP32-C6):

```bash
cargo run --package device-envoy-esp-demos --bin demo_b1_panel_12x8_rust_cursor --release --target riscv32imac-unknown-none-elf --no-default-features
```

Run/flash (ESP32-S3):

```bash
cargo +esp run --package device-envoy-esp-demos --bin demo_b1_panel_12x8_rust_cursor --release --target xtensa-esp32s3-none-elf --no-default-features -Zbuild-std=core,alloc
```

## B2 - 12x8 panel, text to frame and graphics

Writes colored `Go` to a 2D frame, edits pixels directly, then draws crossing
lines with embedded-graphics before writing the frame to the panel.

Run/flash (ESP32-C6):

```bash
cargo run --package device-envoy-esp-demos --bin demo_b2_panel_12x8_text_graphics --release --target riscv32imac-unknown-none-elf --no-default-features
```

Run/flash (ESP32-S3):

```bash
cargo +esp run --package device-envoy-esp-demos --bin demo_b2_panel_12x8_text_graphics --release --target xtensa-esp32s3-none-elf --no-default-features -Zbuild-std=core,alloc
```

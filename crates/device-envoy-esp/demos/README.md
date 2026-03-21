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

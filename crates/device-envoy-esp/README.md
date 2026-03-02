# device-envoy-esp

Device abstractions and examples for ESP32-C6 and ESP32-S3.

## Default Pin Assignments

This section is the single source of truth for default pin assignments used by examples in this repo.

### Core defaults (shared where practical)

- `GPIO6` - Button input (wired to GND in examples using `PressedTo::Ground`)
- `GPIO7` - IR receiver data input
- `GPIO11` - I2S bit clock (`BCLK`)
- `GPIO12` - I2S word select (`WS` / `LRCLK`)
- `GPIO21` - I2S serial data output (`DIN`)

GPIO11 and GPIO12 replaced the previous GPIO22/GPIO23 defaults because GPIO22–GPIO25 are not
exposed as peripherals in esp-hal on ESP32-S3.

### LED defaults used in current examples

- Built-in NeoPixel-style (WS2812) RGB LED (board-specific):
  - `GPIO8` on ESP32-C6-DevKitC-1
  - `GPIO48` on ESP32-S3-DevKitC-1
- This built-in LED mapping is board-dependent. On ESP32-C6, `GPIO8` is also a strapping pin, so it is not a general-purpose default for new external wiring.
- `GPIO10` - External 8-pixel NeoPixel-style (WS2812) strip
- `GPIO18` - 12x8 panel examples
- `GPIO2` - 16x16 panel examples

### Peripheral resource defaults used in current examples

- `I2S0 + DMA_CH0` - Audio examples
- `SPI2` - SPI-driven LED panel/strip examples
- RMT channels for IR receiver:
  - `channel2` on ESP32-C6 (channels 0–3 all support RX)
  - `channel4` on ESP32-S3 (channels 0–3 are TX-only; RX requires channel 4+)

### Portability guardrails for new defaults

- Avoid C6 USB pins: `GPIO12`, `GPIO13`
- Avoid S3 USB pins: `GPIO19`, `GPIO20`
- Avoid C6 strapping pins for defaults: `GPIO4`, `GPIO5`, `GPIO8`, `GPIO9`, `GPIO15`
- Avoid S3 strapping pins for defaults: `GPIO0`, `GPIO3`, `GPIO45`, `GPIO46`
- Avoid S3 unavailable pins: `GPIO22`–`GPIO25` are not exposed in esp-hal for ESP32-S3
- Avoid flash/SPI0/1-connected pins as defaults on board variants where they are not GPIO-safe

Note: `GPIO8` appears both as a built-in LED pin and in the C6 strapping-pin avoid list. That is intentional: keep it for the board's built-in LED only, and avoid using it as a default for new external signals.

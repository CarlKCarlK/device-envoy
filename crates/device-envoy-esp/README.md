# device-envoy-esp32

Device abstractions and examples for ESP32-C6.

## Default Pin Assignments

This section is the single source of truth for default pin assignments used by examples in this repo.

### Core defaults (shared where practical)

- `GPIO21` - I2S serial data output (`DOUT`)
- `GPIO22` - I2S bit clock (`BCLK`)
- `GPIO23` - I2S word select (`WS` / `LRCLK`)
- `GPIO6` - Button input (wired to GND in examples using `PressedTo::Ground`)
- `GPIO7` - IR receiver data input

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
- `RMT channel0/channel1/channel2` - LED + IR examples, depending on demo. Channel availability differs between ESP32-C6 and ESP32-S3; examples use channels present on both.

### Portability guardrails for new defaults

- Avoid C6 USB pins: `GPIO12`, `GPIO13`
- Avoid S3 USB pins: `GPIO19`, `GPIO20`
- Avoid C6 strapping pins for defaults: `GPIO4`, `GPIO5`, `GPIO8`, `GPIO9`, `GPIO15`
- Avoid S3 strapping pins for defaults: `GPIO0`, `GPIO3`, `GPIO45`, `GPIO46`
- Avoid flash/SPI0/1-connected pins as defaults on board variants where they are not GPIO-safe

Note: `GPIO8` appears both as a built-in LED pin and in the C6 strapping-pin avoid list. That is intentional: keep it for the board's built-in LED only, and avoid using it as a default for new external signals.

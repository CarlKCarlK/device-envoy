# Current Limiting

The `max_current` field automatically scales brightness to stay within your electrical current budget.

Each WS2812 LED is assumed to draw 60 mA at full brightness. For example:

- 16 LEDs × 60 mA = 960 mA at full brightness
- With `max_current: Current::Milliamps(1000)`, all LEDs fit at 100% brightness
- With the default electrical current limit (250 mA), the generated `MAX_BRIGHTNESS` limits LEDs to ~26% brightness

The electrical current limit is compiled into a lookup table at device initialization, so it has no per-frame runtime cost.

**Powering LEDs from a board 5 V pin:** On Pico, this is pin 40 (`VBUS`). On ESP32-C6 D6 and ESP32-S3 boards, this is usually a header pin labeled `5V` or `VBUS`. These rails are typically USB pass-through and have practical current limits, so avoid heavy loads through the board itself. Small LED panels (a few hundred mA) are usually fine with a solid USB supply; for larger loads (around 1 A+), use a separate 5 V supply and share ground with the board. If your board labels the pin `5Vin`, treat it as a power-input rail (not a general-purpose 5 V output for peripherals).

# Color Correction (Gamma)

The `gamma` field applies a color response curve to make colors look more natural:

- [`Gamma::Linear`](crate::led_strip::Gamma::Linear) — No correction (raw values)
- [`Gamma::Srgb`](crate::led_strip::Gamma::Srgb) — Perceptual sRGB semantics (default; preserves named color constants)
- [`Gamma::SmartLeds`](crate::led_strip::Gamma::SmartLeds) — `smart_leds::gamma()` compatibility (2.8)

The gamma curve is compiled into a lookup table at device initialization, so it has no per-frame runtime cost.

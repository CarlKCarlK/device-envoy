# Current Limiting

The `max_current` field automatically scales brightness to stay within your electrical current budget.

Each WS2812 LED is assumed to draw 60 mA at full brightness. For example:

- 16 LEDs × 60 mA = 960 mA at full brightness
- With `max_current: Current::Milliamps(1000)`, all LEDs fit at 100% brightness
- With the default electrical current limit (250 mA), the generated `MAX_BRIGHTNESS` limits LEDs to ~26% brightness

The electrical current limit is compiled into a lookup table at device initialization, so it has no per-frame runtime cost.

**Powering LEDs from the Pico's pin 40 (VBUS):** Pin 40 is the USB 5 V rail pass-through, but the Pico itself has practical electrical current limits — the USB connector, cable, and internal circuitry aren't designed for heavy loads. Small LED panels (a few hundred mA) can usually power from pin 40 with a decent USB supply; for larger loads (1 A+), use a separate 5 V supply and share ground with the Pico.

# Color Correction (Gamma)

The `gamma` field applies a color response curve to make colors look more natural:

- [`Gamma::Linear`](crate::led_strip::Gamma::Linear) — No correction (raw values)
- [`Gamma::Srgb`](crate::led_strip::Gamma::Srgb) — Perceptual sRGB semantics (default; preserves named color constants)
- [`Gamma::SmartLeds`](crate::led_strip::Gamma::SmartLeds) — `smart_leds::gamma()` compatibility (≈ 2.8)
- [`Gamma::Power`](crate::led_strip::Gamma::Power) — Power-law tuning (`out = in^(1/gamma)`)

The gamma curve is compiled into a lookup table at device initialization, so it has no per-frame runtime cost.

# Device Envoy Completeness

| Submodule | Rp | Esp | Trait | TDTest | EspPar | PoolInd | Eff | Own |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| led_strip | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| led_strip_spi | *na* | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| led2d | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| audio_player | [x] | [x] | [x] | [x] | ? | [x] | [90%](#note-audio-player) | [x] |
| ir | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| ir/mapping | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| ir/kepler | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| flash_block | [x] | [x] | [x] | [x] | ? | *na* | [x] | [x] |
| button | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| servo | [x] | [x] | [x] | [x] | ? | [x] | [75%](#note-servo-density) | [x] |
| servo_player | [x] | [x] | [x] | [x] | [  ] | [x] | [75%](#note-servo-density) | [x] |
| wifi_auto | [x] | [x] | [x] | [x] | ? | *na* | [x](#note-wifi-auto-eff) | ? |
| led4 | [x] | [x] | [x] | [x] | ? | [ ] | [xTest] | ? |
| lcd_text | [x] | [x] | [x] | [x] | ? | [x] | [xAsk] | [x] |
| clock_sync | [x] | [x] | [x] | [x] | [  ] | [x] | [xTest] | [ ] ([note](#note-clock-sync-own)) |
| led | [x] | ? | [  ] | [ ] | [  ] | [ ] | ? | ? |
| rfid | [x] | ? | [  ] | [ ] | [  ] | [ ] ([note](#note-rfid)) | [x](#note-rfid) | [x](#note-rfid) |

## Key

- `Submodule`: Device abstraction module.

- `Rp`: Implemented in `device-envoy-rp`.

- `Esp`: Implemented in `device-envoy-esp`.

- `Trait`: Core trait surface exists in `device-envoy-core`.

- `TDTest`: Trait documentation includes a doc test.

- `EspPar`: ESP docs are on par with RP docs.

- `PoolInd`: Any number can be created independent of any `pool_size`.

- `Eff`: Efficiency of resource usage.

- `Own`: Ownership safety against hardware contention.

## Notes

### Note: audio-player

`audio_player`: Effectively 100% efficient on ESP for its chosen resource model (`I2S0` + DMA). On RP it is not 100% because the current implementation uses one PIO state machine (`sm0`) per instance rather than spreading across all four state machines. In principle a PIO block can run up to four independent programs, but practical multi-I2S constraints (timing/clocking, DMA feed pressure, pin routing, jitter risk) make full four-stream utilization non-trivial for reliable audio.

### Note: servo-player-own

`servo_player`: Marked `Own [ ]` because the generated static/task pattern can still be contended at runtime (for example repeated `new` on one generated type) rather than always being rejected by the compiler.

### Note: wifi-auto-eff

`wifi_auto`: Marked `Eff [x]` under the current RP design assumption because it takes a whole PIO block and the remaining state machines are not treated as practically shareable for this abstraction.

### Note: clock-sync-own

`clock_sync`: Marked `Own [ ]` because constructor safety relies on one-time static initialization patterns (`StaticCell::init`) that fail at runtime if reused, rather than producing a compile-time ownership error.

### Note: servo-density

`servo` / `servo_player`: Marked `75%` because ESP currently enforces unique LEDC timer claims per generated type. This is safe, but not timer-dense. In principle, multiple channels could share one timer when PWM base settings match, improving density.

### Note: rfid

`rfid`: Marked `Eff [x]` and `Own [x]` for current RP implementation because hardware resources are passed as owned tokens (`SPI0`, GPIO, DMA), preventing unsafe sharing at compile time. `PoolInd` remains `[ ]` due current `SPI0`-only API and single task-entry scaling limits.

# Device Envoy Completeness

| Submodule | Rp | Esp | Trait | TDTest | EspPar | PoolInd | Eff | Own |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| led_strip | [x] | [x] | [x] | [x] | ? | [x] | [xTest] | ? |
| led_strip_spi | *na* | [x] | [x] | [x] | ? | [x] | [xTest] | ? |
| led2d | [x] | [x] | [x] | [x] | ? | [x] | [xTest] | ? |
| audio_player | [x] | [x] | [x] | [x] | [x] | [x] | [90%](#note-audio-player) | ? |
| ir | [x] | [x] | [x] | [x] | ? | [x] | [x] | [ ] ([note](#note-ir)) |
| ir/mapping | [x] | [x] | [x] | [x] | ? | [x] | [x] | [ ] ([note](#note-ir)) |
| ir/kepler | [x] | [x] | [x] | [x] | ? | [x] | [x] | [ ] ([note](#note-ir)) |
| flash_block | [x] | [x] | [x] | [x] | ? | *na* | [x] | ? |
| button | [x] | [x] | [x] | [x] | ? | [x] | [xTest] | ? |
| servo | [x] | [x] | [x] | [x] | ? | [x] | [75%](#note-servo-density) | ? |
| servo_player | [x] | [x] | [x] | [x] | [  ] | [x](#note-servo-player-poolind) | [75%](#note-servo-density) | [ ] ([note](#note-servo-player-own)) |
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

### Note: ir

`ir` / `ir/mapping` / `ir/kepler`: Marked `PoolInd [x]` because ESP and RP now use per-type task entries (no shared task-pool bottleneck), so instance scaling is no longer constrained by Embassy task `pool_size`. `EspPar` remains `?` until the remaining RP/ESP doc and behavior gaps are closed. Marked `Own [ ]` because generated types still rely on one-time static initialization (`StaticCell::init`) and can fail at runtime if `new` is called again for the same generated type, rather than always producing a compile-time ownership error.

### Note: servo-player-poolind

`servo_player`: Marked `PoolInd [x]` because multiple independent instances are achievable by generating additional typed players via the macro.

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

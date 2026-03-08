# Device Envoy Completeness

| Submodule | Rp | Esp | Trait | TDTest | EspPar | PoolInd | Eff | Own |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| led_strip | [x] | [x] | [x] | [x] | ? | [x] | [x] | ? |
| led_strip_spi | *na* | [x] | [x] | [x] | ? | [x] | [x] | ? |
| led2d | [x] | [x] | [x] | [x] | ? | [x] | [x] | ? |
| audio_player | [x] | [x] | [x] | [x] | [x] | [x] | [90%](#note-audio-player) | ? |
| ir | [x] | [x] | [x] | [x] | ? | [ ] | [50%](#note-ir) | ? |
| ir/mapping | [x] | [x] | [x] | [x] | ? | [ ] | [50%](#note-ir) | ? |
| ir/kepler | [x] | [x] | [x] | [x] | ? | [ ] | [50%](#note-ir) | ? |
| flash_block | [x] | [x] | [x] | [x] | ? | *na* | [x] | ? |
| button | [x] | [x] | [x] | [x] | ? | [x] | [x] | ? |
| servo | [x] | [x] | [x] | [x] | ? | [x] | [75%](#note-servo-density) | ? |
| servo_player | [x] | [x] | [x] | [x] | [  ] | [x](#note-servo-player-poolind) | [75%](#note-servo-density) | [ ](#note-servo-player-own) |
| wifi_auto | [x] | [x] | [x] | [x] | ? | *na* | [x](#note-wifi-auto-eff) | ? |
| led4 | [x] | [x] | [x] | [x] | ? | [ ] | [x] | ? |
| lcd_text | [x] | [x] | [x] | [x] | ? | [x] | [x] | [x] |
| clock_sync | [x] | [x] | [x] | [x] | [  ] | [x] | [x] | [ ](#note-clock-sync-own) |
| led | [x] | ? | [  ] | [ ] | [  ] | [ ] | ? | ? |
| rfid | [x] | ? | [  ] | [ ] | [  ] | [ ](#note-rfid) | [x](#note-rfid) | [x](#note-rfid) |

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

- <a id="note-audio-player"></a>`audio_player`: Effectively 100% efficient on ESP for its chosen resource model (`I2S0` + DMA). On RP it is not 100% because the current implementation uses one PIO state machine (`sm0`) per instance rather than spreading across all four state machines. In principle a PIO block can run up to four independent programs, but practical multi-I2S constraints (timing/clocking, DMA feed pressure, pin routing, jitter risk) make full four-stream utilization non-trivial for reliable audio.

- <a id="note-ir"></a>`ir` / `ir/mapping` / `ir/kepler`: Marked `50%`. ESP can use different RMT channels across different devices in principle, but current task-pool limits make the practical ceiling one active IR receiver task per task function without redesign (`pool_size`/task architecture changes). RP also has an additional limitation of a fixed `sm0` path in the current implementation.

- <a id="note-servo-player-poolind"></a>`servo_player`: Marked `PoolInd [x]` because multiple independent instances are achievable by generating additional typed players via the macro.

- <a id="note-servo-player-own"></a>`servo_player`: Marked `Own [ ]` because the generated static/task pattern can still be contended at runtime (for example repeated `new` on one generated type) rather than always being rejected by the compiler.

- <a id="note-wifi-auto-eff"></a>`wifi_auto`: Marked `Eff [x]` under the current RP design assumption because it takes a whole PIO block and the remaining state machines are not treated as practically shareable for this abstraction.

- <a id="note-clock-sync-own"></a>`clock_sync`: Marked `Own [ ]` because constructor safety relies on one-time static initialization patterns (`StaticCell::init`) that fail at runtime if reused, rather than producing a compile-time ownership error.

- <a id="note-servo-density"></a>`servo` / `servo_player`: Marked `75%` because ESP currently enforces unique LEDC timer claims per generated type. This is safe, but not timer-dense. In principle, multiple channels could share one timer when PWM base settings match, improving density.

- <a id="note-rfid"></a>`rfid`: Marked `Eff [x]` and `Own [x]` for current RP implementation because hardware resources are passed as owned tokens (`SPI0`, GPIO, DMA), preventing unsafe sharing at compile time. `PoolInd` remains `[ ]` due current `SPI0`-only API and single task-entry scaling limits.

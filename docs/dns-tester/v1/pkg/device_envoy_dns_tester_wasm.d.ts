/* tslint:disable */
/* eslint-disable */

/**
 * Browser input and lifecycle control shared by an application launcher.
 */
export class CydSimulatorControlWasm {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Forward a physical BOOT-button press.
     */
    boot_down(): void;
    /**
     * Forward a physical BOOT-button release.
     */
    boot_up(): void;
    /**
     * Return whether the simulated display is presented upside down.
     */
    orientation_is_inverted(): boolean;
    /**
     * Clear transient browser input after a simulated reset.
     */
    reset_transient_state(): void;
    /**
     * Forward a browser pointer-down position in logical canvas coordinates.
     */
    touch_down(x: number, y: number): void;
    /**
     * Forward a browser pointer-move position in logical canvas coordinates.
     */
    touch_move(x: number, y: number): void;
    /**
     * Forward a browser pointer-up or pointer-cancel event.
     */
    touch_up(): void;
}

/**
 * Stable browser control handle returned by [`start_cyd_web_app`].
 */
export class CydWebAppHandle {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Press the simulated BOOT button.
     */
    boot_down(): void;
    /**
     * Release the simulated BOOT button.
     */
    boot_up(): void;
    /**
     * Clear framework storage and restart the application.
     */
    clear_storage_and_restart(): void;
    /**
     * Return whether the application has requested the clock control.
     */
    clock_control_is_visible(): boolean;
    /**
     * Return whether the current orientation is inverted.
     */
    orientation_is_inverted(): boolean;
    /**
     * Return the configured interaction instructions.
     */
    page_controls(): string;
    /**
     * Return the configured platform-neutral source URL.
     */
    page_core_code_url(): string;
    /**
     * Return the configured page description.
     */
    page_description(): string;
    /**
     * Return the configured preview text.
     */
    page_preview(): string;
    /**
     * Return the configured page title.
     */
    page_title(): string;
    /**
     * Request an application restart.
     */
    request_restart(): void;
    /**
     * Set the simulated local time, in seconds after midnight.
     */
    set_clock_time_of_day(seconds_of_day: number): void;
    /**
     * Remove and return the oldest pending framework notice.
     */
    take_notice(): CydWebNotice | undefined;
    /**
     * Press the simulated touch panel at canvas coordinates.
     */
    touch_down(position_x: number, position_y: number): void;
    /**
     * Move the simulated touch point.
     */
    touch_move(position_x: number, position_y: number): void;
    /**
     * Release the simulated touch panel.
     */
    touch_up(): void;
    /**
     * Restore the browser's live local clock.
     */
    use_live_clock(): void;
}

/**
 * Typed notice emitted by the framework for the shared browser shell.
 */
export class CydWebNotice {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return optional diagnostic detail.
     */
    detail(): string | undefined;
    /**
     * Return the stable notice identifier.
     */
    id(): string;
    /**
     * Return the notice severity.
     */
    severity(): CydWebNoticeSeverity;
}

export enum CydWebNoticeSeverity {
    /**
     * Informational notice.
     */
    Info = 0,
    /**
     * Recoverable warning.
     */
    Warning = 1,
    /**
     * Terminal runtime failure.
     */
    Fatal = 2,
}

export function start(canvas_id: string): CydWebAppHandle;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly _embassy_time_now: () => bigint;
    readonly start: (a: number, b: number) => [number, number, number];
    readonly __wbg_cydsimulatorcontrolwasm_free: (a: number, b: number) => void;
    readonly __wbg_cydwebapphandle_free: (a: number, b: number) => void;
    readonly __wbg_cydwebnotice_free: (a: number, b: number) => void;
    readonly cydsimulatorcontrolwasm_boot_down: (a: number) => void;
    readonly cydsimulatorcontrolwasm_boot_up: (a: number) => void;
    readonly cydsimulatorcontrolwasm_orientation_is_inverted: (a: number) => number;
    readonly cydsimulatorcontrolwasm_reset_transient_state: (a: number) => void;
    readonly cydsimulatorcontrolwasm_touch_down: (a: number, b: number, c: number) => void;
    readonly cydsimulatorcontrolwasm_touch_move: (a: number, b: number, c: number) => void;
    readonly cydsimulatorcontrolwasm_touch_up: (a: number) => void;
    readonly cydwebapphandle_boot_down: (a: number) => void;
    readonly cydwebapphandle_boot_up: (a: number) => void;
    readonly cydwebapphandle_clear_storage_and_restart: (a: number) => void;
    readonly cydwebapphandle_clock_control_is_visible: (a: number) => number;
    readonly cydwebapphandle_orientation_is_inverted: (a: number) => number;
    readonly cydwebapphandle_page_controls: (a: number) => [number, number];
    readonly cydwebapphandle_page_core_code_url: (a: number) => [number, number];
    readonly cydwebapphandle_page_description: (a: number) => [number, number];
    readonly cydwebapphandle_page_preview: (a: number) => [number, number];
    readonly cydwebapphandle_page_title: (a: number) => [number, number];
    readonly cydwebapphandle_request_restart: (a: number) => void;
    readonly cydwebapphandle_set_clock_time_of_day: (a: number, b: number) => [number, number];
    readonly cydwebapphandle_take_notice: (a: number) => number;
    readonly cydwebapphandle_touch_down: (a: number, b: number, c: number) => void;
    readonly cydwebapphandle_touch_move: (a: number, b: number, c: number) => void;
    readonly cydwebapphandle_touch_up: (a: number) => void;
    readonly cydwebapphandle_use_live_clock: (a: number) => void;
    readonly cydwebnotice_detail: (a: number) => [number, number];
    readonly cydwebnotice_id: (a: number) => [number, number];
    readonly cydwebnotice_severity: (a: number) => number;
    readonly _embassy_time_schedule_wake: (a: bigint, b: number) => void;
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke___f64______true_: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke___wasm_bindgen_c4636c65afc58f47___JsValue__core_7d5f0a2ba6a62c33___result__Result_____wasm_bindgen_c4636c65afc58f47___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

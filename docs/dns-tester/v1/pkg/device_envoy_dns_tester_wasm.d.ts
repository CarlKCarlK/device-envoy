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
 * Stable browser handle shared by every CYD web application.
 */
export class CydWebAppHandle {
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
     * Clear framework storage and request an orderly supervisor restart.
     */
    clear_storage_and_restart(): void;
    /**
     * Return whether the current presentation is inverted.
     */
    orientation_is_inverted(): boolean;
    /**
     * Request an orderly supervisor restart.
     */
    request_restart(): void;
    /**
     * Take the oldest pending typed notice.
     */
    take_notice(): CydWebNotice | undefined;
    /**
     * Forward a pointer-down position in logical canvas coordinates.
     */
    touch_down(position_x: number, position_y: number): void;
    /**
     * Forward a pointer-move position in logical canvas coordinates.
     */
    touch_move(position_x: number, position_y: number): void;
    /**
     * Forward a pointer-up or pointer-cancel event.
     */
    touch_up(): void;
}

/**
 * A typed browser notice with a stable, localizable identifier.
 */
export class CydWebNotice {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Return the formatted diagnostic, when this is a fatal runtime notice.
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

/**
 * Severity for a notice consumed by the shared browser shell.
 */
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
     * Fatal runtime failure.
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
    readonly cydwebapphandle_orientation_is_inverted: (a: number) => number;
    readonly cydwebapphandle_request_restart: (a: number) => void;
    readonly cydwebapphandle_take_notice: (a: number) => number;
    readonly cydwebapphandle_touch_down: (a: number, b: number, c: number) => void;
    readonly cydwebapphandle_touch_move: (a: number, b: number, c: number) => void;
    readonly cydwebapphandle_touch_up: (a: number) => void;
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

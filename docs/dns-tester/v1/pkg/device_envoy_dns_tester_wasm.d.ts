/* tslint:disable */
/* eslint-disable */

export class DnsTesterWeb {
    free(): void;
    [Symbol.dispose](): void;
    boot_down(): void;
    boot_up(): void;
    clear_storage(): Promise<void>;
    constructor(canvas: HTMLCanvasElement);
    /**
     * Whether the current simulated display orientation is upside down.
     */
    orientation_is_inverted(): boolean;
    /**
     * Present the simulated CYD in landscape while touch calibration runs.
     */
    prepare_calibration_landscape(): void;
    present(): Promise<void>;
    reboot(): Promise<void>;
    start(): Promise<void>;
    tick(): string;
    touch_down(x: number, y: number): void;
    touch_move(x: number, y: number): void;
    touch_up(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_dnstesterweb_free: (a: number, b: number) => void;
    readonly dnstesterweb_boot_down: (a: number) => void;
    readonly dnstesterweb_boot_up: (a: number) => void;
    readonly dnstesterweb_clear_storage: (a: number) => any;
    readonly dnstesterweb_new: (a: any) => [number, number, number];
    readonly dnstesterweb_orientation_is_inverted: (a: number) => number;
    readonly dnstesterweb_prepare_calibration_landscape: (a: number) => void;
    readonly dnstesterweb_present: (a: number) => any;
    readonly dnstesterweb_reboot: (a: number) => any;
    readonly dnstesterweb_start: (a: number) => any;
    readonly dnstesterweb_tick: (a: number) => [number, number];
    readonly dnstesterweb_touch_down: (a: number, b: number, c: number) => void;
    readonly dnstesterweb_touch_move: (a: number, b: number, c: number) => void;
    readonly dnstesterweb_touch_up: (a: number) => void;
    readonly _embassy_time_now: () => bigint;
    readonly _embassy_time_schedule_wake: (a: bigint, b: number) => void;
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke___f64______true_: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke___wasm_bindgen_c4636c65afc58f47___JsValue__core_7d5f0a2ba6a62c33___result__Result_____wasm_bindgen_c4636c65afc58f47___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke___js_sys_649ec69cc13967a8___Function_fn_wasm_bindgen_c4636c65afc58f47___JsValue_____wasm_bindgen_c4636c65afc58f47___sys__Undefined___js_sys_649ec69cc13967a8___Function_fn_wasm_bindgen_c4636c65afc58f47___JsValue_____wasm_bindgen_c4636c65afc58f47___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_c4636c65afc58f47___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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

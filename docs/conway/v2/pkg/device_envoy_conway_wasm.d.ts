/* tslint:disable */
/* eslint-disable */

/**
 * Browser adapter for the shared Conway application.
 */
export class ConwayWeb {
    free(): void;
    [Symbol.dispose](): void;
    constructor();
    /**
     * Forward one browser control key to the shared application.
     */
    press_key(key: string): string;
    /**
     * Render the shared application as a PNG.
     */
    render_png(): Uint8Array;
    /**
     * Render the shared application as a PNG with a maximum dimension.
     */
    render_png_with_max_dimension(max_dimension: number): Uint8Array;
    /**
     * Advance the shared application by one browser tick.
     */
    tick(): string;
    /**
     * Return the shared simulation's current animation interval.
     */
    tick_interval_ms(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_conwayweb_free: (a: number, b: number) => void;
    readonly conwayweb_new: () => number;
    readonly conwayweb_press_key: (a: number, b: number, c: number) => [number, number];
    readonly conwayweb_render_png: (a: number) => [number, number, number, number];
    readonly conwayweb_render_png_with_max_dimension: (a: number, b: number) => [number, number, number, number];
    readonly conwayweb_tick: (a: number) => [number, number];
    readonly conwayweb_tick_interval_ms: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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

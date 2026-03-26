/* tslint:disable */
/* eslint-disable */

/**
 * Decode a UTM tape back into guest TM state.
 *
 * - `spec_json`: JSON string of the guest machine spec
 * - `utm_tape_str`: the UTM tape as a string of UTM symbol chars
 *
 * Returns a JSON string: `{ "state": "...", "tape": "...", "pos": N }`
 */
export function decode(spec_json: string, utm_tape_str: string): string;

/**
 * Encode a guest TM into a UTM tape.
 *
 * - `spec_json`: JSON string of the machine spec (same format as machine-specs.json entries)
 * - `tape_str`: the tape contents as display characters (e.g. "01011")
 *
 * Returns the encoded UTM tape as a string of UTM symbol display characters.
 */
export function encode(spec_json: string, tape_str: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly decode: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly encode: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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

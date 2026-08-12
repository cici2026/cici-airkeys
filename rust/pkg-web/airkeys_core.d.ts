/* tslint:disable */
/* eslint-disable */

/**
 * The translation engine, owned by the JS side.
 *
 * Constructed empty rather than from an embedded catalog: this module ships
 * **zero** locales (see the crate's Cargo.toml), so the first thing the web app
 * does is fetch `/i18n/en.json` and call `load_catalog_json`. That is one
 * network round trip the native shells do not need — and it is what keeps the
 * wasm module from carrying every language for every user.
 */
export class I18n {
    free(): void;
    [Symbol.dispose](): void;
    change_language(requested: string): any;
    load_catalog_json(language: string, json: string): void;
    /**
     * An engine with no strings yet. `t()` errors until a catalog is loaded,
     * which the shell renders as the key — never as an empty string, because
     * an empty string looks like a design decision.
     */
    constructor(language: string, catalog_json: string);
    /**
     * Resolve a key.
     *
     * `options_json` is the i18next object literal verbatim: `count` selects
     * the plural form, every other key is an interpolation variable. Taking it
     * as JSON rather than a typed struct is what lets a TypeScript caller pass
     * the object it already has.
     */
    t(key: string, options_json: string): string;
    readonly language: string;
}

/**
 * r" The unlocked vault's item list: load → search → favorite → copy.
 */
export class ItemListCore {
    free(): void;
    [Symbol.dispose](): void;
    dispatch(event_json: string): string;
    constructor();
    resolve_effect(effect_id: bigint, result_json: string): string;
    view(): string;
}

/**
 * r" Opening a locked vault: support → authenticate → open → enter.
 */
export class UnlockCore {
    free(): void;
    [Symbol.dispose](): void;
    dispatch(event_json: string): string;
    constructor();
    resolve_effect(effect_id: bigint, result_json: string): string;
    view(): string;
}

export function from_base64url(text: string): Uint8Array;

export function from_hex(text: string): Uint8Array;

export function item_fingerprint(kind: string, title: string, account: string): string;

/**
 * Which shipped language a tag resolves to, and the chain it walked. Web calls
 * this **before** fetching anything — it is what decides which catalog file to
 * request.
 */
export function resolve_language(requested: string): any;

/**
 * `StrengthReport` as a plain JS object.
 */
export function secret_strength(secret: string): any;

export function sha256(bytes: Uint8Array): Uint8Array;

export function supported_languages(): string[];

export function to_base64url(bytes: Uint8Array): string;

export function to_hex(bytes: Uint8Array, prefix: boolean): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_i18n_free: (a: number, b: number) => void;
    readonly __wbg_itemlistcore_free: (a: number, b: number) => void;
    readonly __wbg_unlockcore_free: (a: number, b: number) => void;
    readonly from_base64url: (a: number, b: number) => [number, number, number, number];
    readonly from_hex: (a: number, b: number) => [number, number, number, number];
    readonly i18n_change_language: (a: number, b: number, c: number) => [number, number, number];
    readonly i18n_language: (a: number) => [number, number];
    readonly i18n_load_catalog_json: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly i18n_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly i18n_t: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly item_fingerprint: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly itemlistcore_dispatch: (a: number, b: number, c: number) => [number, number, number, number];
    readonly itemlistcore_new: () => number;
    readonly itemlistcore_resolve_effect: (a: number, b: bigint, c: number, d: number) => [number, number, number, number];
    readonly itemlistcore_view: (a: number) => [number, number, number, number];
    readonly resolve_language: (a: number, b: number) => [number, number, number];
    readonly secret_strength: (a: number, b: number) => [number, number, number];
    readonly sha256: (a: number, b: number) => [number, number];
    readonly supported_languages: () => [number, number];
    readonly to_base64url: (a: number, b: number) => [number, number];
    readonly to_hex: (a: number, b: number, c: number) => [number, number];
    readonly unlockcore_dispatch: (a: number, b: number, c: number) => [number, number, number, number];
    readonly unlockcore_new: () => number;
    readonly unlockcore_resolve_effect: (a: number, b: bigint, c: number, d: number) => [number, number, number, number];
    readonly unlockcore_view: (a: number) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
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

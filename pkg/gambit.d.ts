/* tslint:disable */
/* eslint-disable */

export class Engine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Top-N candidate moves. Returns "uci:cp" entries joined by "," —
     * cp is from White's perspective. Sorted best-for-side-to-move first.
     */
    analyse_multi(fen: string, depth: number, n: number): string;
    /**
     * Best move at fixed depth.
     */
    best_move(fen: string, depth: number): string;
    /**
     * Best move with a time budget in milliseconds (iterative deepening).
     */
    best_move_time(fen: string, ms: number): string;
    clear_info_callback(): void;
    /**
     * Position eval in centipawns from White's perspective.
     */
    eval_position(fen: string, depth: number): number;
    /**
     * All legal moves for given FEN as comma-separated UCI strings.
     */
    legal_moves_for(fen: string): string;
    constructor();
    /**
     * Provide the game's prior positions as a "|"-delimited FEN list.
     * Drives threefold-repetition detection in search.
     */
    set_history(fens_pipe: string): void;
    /**
     * Register a JS callback: `fn(depth, score_cp, pv_uci, nodes, elapsed_ms)`.
     */
    set_info_callback(cb: Function): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_engine_free: (a: number, b: number) => void;
    readonly engine_analyse_multi: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly engine_best_move: (a: number, b: number, c: number, d: number) => [number, number];
    readonly engine_best_move_time: (a: number, b: number, c: number, d: number) => [number, number];
    readonly engine_clear_info_callback: (a: number) => void;
    readonly engine_eval_position: (a: number, b: number, c: number, d: number) => number;
    readonly engine_legal_moves_for: (a: number, b: number, c: number) => [number, number];
    readonly engine_new: () => number;
    readonly engine_set_history: (a: number, b: number, c: number) => void;
    readonly engine_set_info_callback: (a: number, b: any) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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

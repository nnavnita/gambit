# Gambit

A chess engine written in Rust, compiled to WebAssembly, playable in the browser.

**Play it:** open `index.html` in any modern browser (no server needed) — the engine runs entirely client-side. Analyse a position in `analysis.html`, solve tactics in `puzzles.html`.

## What's inside

- **Bitboard board representation** with legal move generation, castling, en passant, promotion (`src/board.rs`, `src/movegen.rs`).
- **Alpha-beta search** with iterative deepening, quiescence, transposition table, and killer moves (`src/search.rs`, `src/tt.rs`).
- **Handcrafted evaluation** — material, piece-square tables, pawn structure, king safety (`src/eval.rs`).
- **Opening book** for the first few plies (`src/book.rs`).
- **UCI protocol** support so it can be plugged into any standard chess GUI (`src/uci.rs`).
- **Browser UI** with theme picker, move history, analysis mode, and tactics puzzles — all vanilla HTML/CSS/JS talking to the WASM engine over web workers.

## Build

Requires Rust + [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/).

```bash
# Build the WASM package into ./pkg
wasm-pack build --target web --release

# Then open index.html directly, or serve locally:
python3 -m http.server 8000
```

Native binary (for UCI / benchmarking) builds too:

```bash
cargo build --release
./target/release/gambit   # UCI mode on stdin/stdout
```

## Layout

```
src/
  board.rs      Bitboard state + make/unmake
  movegen.rs    Legal move generation
  search.rs     Alpha-beta + quiescence
  eval.rs       Static evaluation
  tt.rs         Transposition table
  book.rs       Opening book
  uci.rs        UCI protocol driver
  types.rs      Shared types
  lib.rs        WASM entry points
  main.rs       Native UCI entry
index.html      Play mode
analysis.html   Analysis mode
puzzles.html    Tactics trainer
worker.js       Search worker (WASM)
eval_worker.js  Analysis-mode eval worker
puzzle_worker.js Puzzle-mode worker
```

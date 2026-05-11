mod types;
mod board;
mod movegen;
mod eval;
mod tt;
mod search;
mod book;

use wasm_bindgen::prelude::*;
use board::Board;
use movegen::{init_attacks, legal_moves};
use search::{Searcher, SearchInfo};
use types::Move;

#[wasm_bindgen]
pub struct Engine {
    searcher: Searcher,
    book_salt: u64,
    /// Zobrist hashes of every prior game position (excluding current).
    game_history: Vec<u64>,
    /// Optional JS callback fired per iterative-deepening completion.
    info_cb: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        console_error_panic_hook::set_once();
        init_attacks();
        Engine {
            searcher: Searcher::new(),
            book_salt: 0x9E3779B97F4A7C15,
            game_history: Vec::new(),
            info_cb: None,
        }
    }

    /// Register a JS callback: `fn(depth, score_cp, pv_uci, nodes, elapsed_ms)`.
    pub fn set_info_callback(&mut self, cb: js_sys::Function) {
        self.info_cb = Some(cb);
    }

    pub fn clear_info_callback(&mut self) {
        self.info_cb = None;
    }

    /// Provide the game's prior positions as a "|"-delimited FEN list.
    /// Drives threefold-repetition detection in search.
    pub fn set_history(&mut self, fens_pipe: &str) {
        self.game_history.clear();
        if fens_pipe.is_empty() { return; }
        for f in fens_pipe.split('|') {
            if f.trim().is_empty() { continue; }
            let b = Board::from_fen(f);
            self.game_history.push(b.zobrist);
        }
    }

    /// Best move at fixed depth.
    pub fn best_move(&mut self, fen: &str, depth: u32) -> String {
        let board = Board::from_fen(fen);
        if let Some(m) = self.book_move(&board, fen) { return m; }
        let mut info = SearchInfo::new_depth(depth);
        info.history = self.game_history.clone();
        #[cfg(target_arch = "wasm32")]
        if let Some(cb) = &self.info_cb { info.info_cb = Some(cb.clone()); }
        self.searcher.clear_for_search();
        let (mv, _) = self.searcher.search(&board, &mut info);
        finalize_move(&board, mv)
    }

    /// Best move with a time budget in milliseconds (iterative deepening).
    pub fn best_move_time(&mut self, fen: &str, ms: u32) -> String {
        let board = Board::from_fen(fen);
        if let Some(m) = self.book_move(&board, fen) { return m; }
        let mut info = SearchInfo::new_time_ms(ms as f64);
        info.history = self.game_history.clone();
        #[cfg(target_arch = "wasm32")]
        if let Some(cb) = &self.info_cb { info.info_cb = Some(cb.clone()); }
        self.searcher.clear_for_search();
        let (mv, _) = self.searcher.search(&board, &mut info);
        finalize_move(&board, mv)
    }

    /// Position eval in centipawns from White's perspective.
    pub fn eval_position(&mut self, fen: &str, depth: u32) -> i32 {
        let board = Board::from_fen(fen);
        self.searcher.clear_for_search();
        let mut info = SearchInfo::new_depth(depth);
        info.history = self.game_history.clone();
        let (_, score) = self.searcher.search(&board, &mut info);
        if board.side == crate::types::Color::White { score } else { -score }
    }

    /// Top-N candidate moves. Returns "uci:cp" entries joined by "," —
    /// cp is from White's perspective. Sorted best-for-side-to-move first.
    pub fn analyse_multi(&mut self, fen: &str, depth: u32, n: u32) -> String {
        let board = Board::from_fen(fen);
        let moves = legal_moves(&board);
        let inner_depth = depth.saturating_sub(1).max(1);
        let stm_white = board.side == crate::types::Color::White;
        let mut results: Vec<(String, i32)> = Vec::with_capacity(moves.len());
        for &mv in &moves {
            let child = board.make_move(mv);
            self.searcher.clear_for_search();
            let mut info = SearchInfo::new_depth(inner_depth);
            info.history = self.game_history.clone();
            info.history.push(board.zobrist);
            let (_, raw) = self.searcher.search(&child, &mut info);
            // raw is from child's STM (= opponent) perspective.
            let from_us = -raw;
            let white_cp = if stm_white { from_us } else { -from_us };
            results.push((mv.to_uci(), white_cp));
        }
        results.sort_by(|a, b| if stm_white { b.1.cmp(&a.1) } else { a.1.cmp(&b.1) });
        results.truncate(n as usize);
        results.iter().map(|(u, s)| format!("{}:{}", u, s)).collect::<Vec<_>>().join(",")
    }

    /// All legal moves for given FEN as comma-separated UCI strings.
    pub fn legal_moves_for(&self, fen: &str) -> String {
        let board = Board::from_fen(fen);
        legal_moves(&board)
            .iter()
            .map(|m| m.to_uci())
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Engine {
    fn book_move(&mut self, board: &Board, fen: &str) -> Option<String> {
        self.book_salt = self.book_salt
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let uci = book::probe(fen, self.book_salt)?;
        for mv in &legal_moves(board) {
            if mv.to_uci() == uci { return Some(uci); }
        }
        None
    }
}

fn finalize_move(board: &Board, mv: Move) -> String {
    let legal = legal_moves(board);
    if legal.iter().any(|&m| m == mv) { return mv.to_uci(); }
    if let Some(&fb) = legal.first() { return fb.to_uci(); }
    String::new()
}

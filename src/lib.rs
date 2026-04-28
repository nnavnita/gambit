mod types;
mod board;
mod movegen;
mod eval;
mod tt;
mod search;

use wasm_bindgen::prelude::*;
use board::Board;
use movegen::{init_attacks, legal_moves};
use search::{Searcher, SearchInfo};
use types::Move;

#[wasm_bindgen]
pub struct Engine {
    searcher: Searcher,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        console_error_panic_hook::set_once();
        init_attacks();
        Engine { searcher: Searcher::new() }
    }

    /// Best move for given FEN at given depth. Returns UCI string e.g. "e2e4".
    pub fn best_move(&mut self, fen: &str, depth: u32) -> String {
        let board = Board::from_fen(fen);
        self.searcher.clear_for_search();
        let mut info = SearchInfo::new_depth(depth);
        let (mv, _) = self.searcher.search(&board, &mut info);
        // Fallback: if search returned null move (TT miss / no improvement),
        // return the first legal move so the game is never stuck.
        if mv == Move::default() {
            let moves = legal_moves(&board);
            if let Some(&fallback) = moves.first() {
                return fallback.to_uci();
            }
            return String::new();
        }
        mv.to_uci()
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

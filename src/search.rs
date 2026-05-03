#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use crate::types::*;
use crate::board::Board;
use crate::movegen::{legal_moves, in_check};
use crate::eval::{evaluate, is_endgame, mvv_lva, INF, MATE_SCORE, PIECE_VALUE};
use crate::tt::{TranspositionTable, TTFlag};

pub struct SearchInfo {
    pub nodes: u64,
    pub stop: bool,
    pub depth: u32,
    /// 0 = unlimited (use time limit); >0 = stop after this depth
    pub max_depth: u32,
    #[cfg(not(target_arch = "wasm32"))]
    pub start: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    pub time_limit: Duration,
}

impl SearchInfo {
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)]
    pub fn new(time_limit: Duration) -> Self {
        SearchInfo {
            nodes: 0,
            stop: false,
            depth: 0,
            max_depth: 64,
            start: Instant::now(),
            time_limit,
        }
    }

    #[allow(dead_code)]
    pub fn new_depth(max_depth: u32) -> Self {
        SearchInfo {
            nodes: 0,
            stop: false,
            depth: 0,
            max_depth,
            #[cfg(not(target_arch = "wasm32"))]
            start: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            time_limit: Duration::from_secs(3600),
        }
    }

    fn check_time(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.nodes & 4095 == 0 {
            if self.start.elapsed() >= self.time_limit {
                self.stop = true;
            }
        }
    }
}

pub struct Searcher {
    pub tt: TranspositionTable,
    pub killer: [[Move; 2]; 128],
    pub history: [[i32; 64]; 64],
}

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            tt: TranspositionTable::new(64),
            killer: [[Move::default(); 2]; 128],
            history: [[0; 64]; 64],
        }
    }

    pub fn clear_for_search(&mut self) {
        self.killer = [[Move::default(); 2]; 128];
        self.history = [[0; 64]; 64];
    }

    /// Iterative deepening — returns (best_move, score)
    pub fn search(&mut self, board: &Board, info: &mut SearchInfo) -> (Move, i32) {
        let mut best_move = Move::default();
        let mut best_score = -INF;

        for depth in 1..=info.max_depth {
            info.depth = depth;

            // ── Aspiration windows with gradual widening ──────────────────────
            let score = if depth >= 4 && best_score.abs() < MATE_SCORE / 2 {
                let mut window = 25i32;
                let mut lo = best_score - window;
                let mut hi = best_score + window;
                'asp: loop {
                    let s = self.alpha_beta(board, lo, hi, depth as i32, 0, true, info);
                    if info.stop { break 'asp s; }
                    if s <= lo {
                        window = (window * 2).min(INF);
                        lo = s - window;
                        hi = (hi + 10).min(INF);
                    } else if s >= hi {
                        window = (window * 2).min(INF);
                        hi = s + window;
                    } else {
                        break 'asp s;
                    }
                    if window > 600 {
                        break 'asp self.alpha_beta(board, -INF, INF, depth as i32, 0, true, info);
                    }
                }
            } else {
                self.alpha_beta(board, -INF, INF, depth as i32, 0, true, info)
            };

            if info.stop { break; }
            best_score = score;

            // Retrieve best move from TT
            if let Some(entry) = self.tt.probe(board.zobrist) {
                if entry.best_move != Move::default() {
                    best_move = entry.best_move;
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let elapsed = info.start.elapsed();
                let ms = elapsed.as_millis().max(1);
                let nps = info.nodes * 1000 / ms as u64;
                println!(
                    "info depth {} score cp {} nodes {} nps {} time {}",
                    depth, score, info.nodes, nps, ms
                );
            }
        }
        (best_move, best_score)
    }

    fn alpha_beta(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        ply: usize,
        null_allowed: bool,
        info: &mut SearchInfo,
    ) -> i32 {
        info.check_time();
        if info.stop { return 0; }
        info.nodes += 1;

        // TT probe
        let tt_move = if let Some(entry) = self.tt.probe(board.zobrist) {
            if entry.depth >= depth {
                let score = entry.score;
                match entry.flag {
                    TTFlag::Exact => return score,
                    TTFlag::Lower => { if score >= beta { return score; } }
                    TTFlag::Upper => { if score <= alpha { return alpha; } }
                }
            }
            entry.best_move
        } else {
            Move::default()
        };

        // Quiescence at leaf
        if depth <= 0 {
            return self.quiescence(board, alpha, beta, 0, info);
        }

        let in_chk = in_check(board);

        // ── Reverse futility pruning ──────────────────────────────────────────
        // If static eval is far above beta, we're very unlikely to fall below it.
        if !in_chk && depth <= 7 {
            let static_eval = evaluate(board);
            let margin = 120 * depth;
            if static_eval - margin >= beta {
                return static_eval - margin;
            }
        }

        // ── Null move pruning ─────────────────────────────────────────────────
        if null_allowed && depth >= 3 && !in_chk && !is_endgame(board) {
            let null_board = board.make_null_move();
            let null_score = -self.alpha_beta(&null_board, -beta, -beta + 1, depth - 3, ply + 1, false, info);
            if !info.stop && null_score >= beta {
                return beta;
            }
        }

        // ── Futility pruning ──────────────────────────────────────────────────
        if !in_chk && depth <= 2 {
            let static_eval = evaluate(board);
            let margin = if depth == 1 { 200 } else { 400 };
            if static_eval + margin <= alpha {
                return self.quiescence(board, alpha, beta, 0, info);
            }
        }

        let mut moves = legal_moves(board);
        if moves.is_empty() {
            return if in_chk {
                -MATE_SCORE + ply as i32
            } else {
                0
            };
        }

        self.order_moves(board, &mut moves, tt_move, ply);

        let mut best_move = Move::default();
        let mut tt_flag = TTFlag::Upper;

        for (i, &mv) in moves.iter().enumerate() {
            let child = board.make_move(mv);

            // ── Check extension ───────────────────────────────────────────────
            // Extend by 1 ply when the move gives check to the opponent.
            let gives_check = depth >= 2 && in_check(&child);
            let extension = if gives_check { 1 } else { 0 };

            let score = if i == 0 {
                -self.alpha_beta(&child, -beta, -alpha, depth - 1 + extension, ply + 1, true, info)
            } else {
                // ── Scaled LMR ────────────────────────────────────────────────
                // Reduce late, quiet, non-check moves by log(depth)*log(i)/2.
                let reduction = if depth >= 3
                    && i >= 3
                    && !in_chk
                    && !gives_check
                    && child.occ[board.side as usize] == board.occ[board.side as usize] // quiet move
                {
                    let r = (0.75 + (depth as f64).ln() * (i as f64).ln() / 2.5) as i32;
                    r.max(1).min(depth - 1)
                } else {
                    0
                };

                let reduced_depth = depth - 1 + extension - reduction;
                let s = -self.alpha_beta(&child, -alpha - 1, -alpha, reduced_depth, ply + 1, true, info);

                // Re-search at full depth if LMR failed high or window missed
                if s > alpha && (reduction > 0 || s < beta) {
                    -self.alpha_beta(&child, -beta, -alpha, depth - 1 + extension, ply + 1, true, info)
                } else {
                    s
                }
            };

            if info.stop { return 0; }

            if score > alpha {
                alpha = score;
                best_move = mv;
                tt_flag = TTFlag::Exact;

                self.history[mv.from()][mv.to()] += depth * depth;

                if score >= beta {
                    if self.killer[ply][0] != mv {
                        self.killer[ply][1] = self.killer[ply][0];
                        self.killer[ply][0] = mv;
                    }
                    self.tt.store(board.zobrist, depth, TTFlag::Lower, score, mv);
                    return score;
                }
            }
        }

        self.tt.store(board.zobrist, depth, tt_flag, alpha, best_move);
        alpha
    }

    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32, qply: usize, info: &mut SearchInfo) -> i32 {
        info.nodes += 1;
        let stand_pat = evaluate(board);
        if stand_pat >= beta { return beta; }

        // ── Delta pruning ─────────────────────────────────────────────────────
        // If even capturing the most valuable piece on the board can't raise alpha,
        // there's no point searching any captures.
        const DELTA_MARGIN: i32 = 975; // ~queen value
        if stand_pat + DELTA_MARGIN < alpha {
            return alpha;
        }

        if stand_pat > alpha { alpha = stand_pat; }

        if qply >= 8 { return alpha; }

        let mut moves = legal_moves(board);
        moves.retain(|&mv| {
            board.piece_on(mv.to()).is_some() || mv.flags() == FLAG_EP_CAPTURE
        });
        self.order_moves(board, &mut moves, Move::default(), 0);

        for &mv in &moves {
            // Per-capture delta pruning: skip if winning this piece can't raise alpha
            if let Some((_, captured)) = board.piece_on(mv.to()) {
                if stand_pat + PIECE_VALUE[captured as usize] + 200 < alpha {
                    continue;
                }
            }

            info.check_time();
            if info.stop { return 0; }
            let child = board.make_move(mv);
            let score = -self.quiescence(&child, -beta, -alpha, qply + 1, info);
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
        alpha
    }

    fn order_moves(&self, board: &Board, moves: &mut Vec<Move>, tt_move: Move, ply: usize) {
        moves.sort_unstable_by_key(|&mv| {
            let mut score = 0i32;
            if mv == tt_move { return i32::MIN; }
            let is_capture = board.piece_on(mv.to()).is_some();
            if is_capture {
                score -= mvv_lva(board, mv) + 10_000;
            } else {
                if ply < 128 && (self.killer[ply][0] == mv || self.killer[ply][1] == mv) {
                    score -= 9_000;
                } else {
                    score -= self.history[mv.from()][mv.to()];
                }
            }
            score
        });
    }
}

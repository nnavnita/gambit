#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use crate::types::*;
use crate::board::Board;
use crate::movegen::{legal_moves, in_check};
use crate::eval::{evaluate, is_endgame, mvv_lva, see, insufficient_material, INF, MATE_SCORE, PIECE_VALUE};
use crate::tt::{TranspositionTable, TTFlag};

pub struct SearchInfo {
    pub nodes: u64,
    pub stop: bool,
    pub depth: u32,
    /// 0 = unlimited (use time limit); >0 = stop after this depth
    pub max_depth: u32,
    /// Repetition history (zobrist keys of all prior positions in the game).
    /// Searcher extends this stack as moves are made.
    pub history: Vec<u64>,
    /// Move to exclude at exactly `excluded_ply` — used by singular-extension probing.
    pub excluded_move: Move,
    pub excluded_ply: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pub start: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    pub time_limit: Duration,
    #[cfg(target_arch = "wasm32")]
    pub start_ms: f64,
    /// 0.0 = no time limit (depth-only)
    #[cfg(target_arch = "wasm32")]
    pub time_limit_ms: f64,
    #[cfg(target_arch = "wasm32")]
    pub info_cb: Option<js_sys::Function>,
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
            history: Vec::new(),
            excluded_move: Move::default(),
            excluded_ply: usize::MAX,
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
            history: Vec::new(),
            excluded_move: Move::default(),
            excluded_ply: usize::MAX,
            #[cfg(not(target_arch = "wasm32"))]
            start: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            time_limit: Duration::from_secs(3600),
            #[cfg(target_arch = "wasm32")]
            start_ms: 0.0,
            #[cfg(target_arch = "wasm32")]
            time_limit_ms: 0.0,
            #[cfg(target_arch = "wasm32")]
            info_cb: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_time_ms(ms: f64) -> Self {
        SearchInfo {
            nodes: 0,
            stop: false,
            depth: 0,
            max_depth: 64,
            history: Vec::new(),
            excluded_move: Move::default(),
            excluded_ply: usize::MAX,
            start_ms: js_sys::Date::now(),
            time_limit_ms: ms,
            info_cb: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_time_ms(ms: f64) -> Self {
        Self::new(Duration::from_millis(ms as u64))
    }

    fn check_time(&mut self) {
        if self.nodes & 4095 != 0 { return; }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.start.elapsed() >= self.time_limit {
                self.stop = true;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if self.time_limit_ms > 0.0
                && js_sys::Date::now() - self.start_ms >= self.time_limit_ms
            {
                self.stop = true;
            }
        }
    }
}

pub struct Searcher {
    pub tt: TranspositionTable,
    pub killer: [[Move; 2]; 128],
    pub history: [[i32; 64]; 64],
    /// Small zobrist-keyed eval cache (always-replace).
    pub eval_cache: Vec<(u64, i32)>,
}

const EVAL_CACHE_SIZE: usize = 1 << 16; // 64K entries

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            tt: TranspositionTable::new(64),
            killer: [[Move::default(); 2]; 128],
            history: [[0; 64]; 64],
            eval_cache: vec![(0u64, 0i32); EVAL_CACHE_SIZE],
        }
    }

    pub fn clear_for_search(&mut self) {
        self.killer = [[Move::default(); 2]; 128];
        self.history = [[0; 64]; 64];
        // Keep eval_cache + tt across moves; only kill killers/history.
    }

    fn cached_eval(&mut self, board: &Board) -> i32 {
        let idx = (board.zobrist as usize) & (EVAL_CACHE_SIZE - 1);
        let entry = self.eval_cache[idx];
        if entry.0 == board.zobrist { return entry.1; }
        let s = evaluate(board);
        self.eval_cache[idx] = (board.zobrist, s);
        s
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
                let pv = self.extract_pv(board, depth as usize);
                let pv_str: String = pv.iter().map(|m| m.to_uci()).collect::<Vec<_>>().join(" ");
                println!(
                    "info depth {} score cp {} nodes {} nps {} time {} pv {}",
                    depth, score, info.nodes, nps, ms, pv_str
                );
            }

            // ── PV streaming callback (wasm only) ─────────────────────────────
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(cb) = info.info_cb.clone() {
                    let pv = self.extract_pv(board, depth as usize);
                    let pv_str: String = pv.iter().map(|m| m.to_uci()).collect::<Vec<_>>().join(" ");
                    let elapsed = js_sys::Date::now() - info.start_ms;
                    let arr = js_sys::Array::new();
                    arr.push(&wasm_bindgen::JsValue::from(depth));
                    arr.push(&wasm_bindgen::JsValue::from(score));
                    arr.push(&wasm_bindgen::JsValue::from_str(&pv_str));
                    arr.push(&wasm_bindgen::JsValue::from_f64(info.nodes as f64));
                    arr.push(&wasm_bindgen::JsValue::from_f64(elapsed));
                    let _ = cb.call1(&wasm_bindgen::JsValue::NULL, &arr);
                }
            }
        }
        (best_move, best_score)
    }

    /// Walk TT from `board` to extract the principal variation.  Stops on
    /// missing entry, illegal move, or repeat zobrist (cycle guard).
    pub fn extract_pv(&self, board: &Board, max_len: usize) -> Vec<Move> {
        let mut pv = Vec::with_capacity(max_len);
        let mut b = board.clone();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..max_len {
            if !seen.insert(b.zobrist) { break; }
            let entry = match self.tt.probe(b.zobrist) { Some(e) => e, None => break };
            let mv = entry.best_move;
            if mv == Move::default() { break; }
            let legal = legal_moves(&b);
            if !legal.iter().any(|&m| m == mv) { break; }
            pv.push(mv);
            b = b.make_move(mv);
        }
        pv
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

        // ── Forced draws (50-move, insufficient material, repetition) ─────────
        // Only return draw if not at the root — root must always pick a real move.
        if ply > 0 {
            if board.halfmove >= 100 || insufficient_material(board) { return 0; }
            // Two-fold repetition inside search counts as a draw — opponent will
            // claim the third by repeating again.
            if info.history.iter().filter(|&&z| z == board.zobrist).count() >= 1 {
                return 0;
            }
        }

        // TT probe — keep entry for singular probing later in this node.
        let in_excluded_search = info.excluded_move != Move::default() && info.excluded_ply == ply;
        let (tt_move, tt_depth, tt_score, tt_flag_opt) = if let Some(entry) = self.tt.probe(board.zobrist) {
            // Skip TT cutoffs when we're verifying a singular candidate at this ply.
            if !in_excluded_search && entry.depth >= depth {
                let score = entry.score;
                match entry.flag {
                    TTFlag::Exact => return score,
                    TTFlag::Lower => { if score >= beta { return score; } }
                    TTFlag::Upper => { if score <= alpha { return alpha; } }
                }
            }
            (entry.best_move, entry.depth, entry.score, Some(entry.flag))
        } else {
            (Move::default(), 0, 0, None)
        };

        // Quiescence at leaf
        if depth <= 0 {
            return self.quiescence(board, alpha, beta, 0, info);
        }

        let in_chk = in_check(board);

        // ── Reverse futility pruning ──────────────────────────────────────────
        // If static eval is far above beta, we're very unlikely to fall below it.
        if !in_chk && depth <= 7 {
            let static_eval = self.cached_eval(board);
            let margin = 120 * depth;
            if static_eval - margin >= beta {
                return static_eval - margin;
            }
        }

        // ── Razoring ──────────────────────────────────────────────────────────
        // At low depth, if static eval + a wide margin is still well below alpha,
        // drop directly into quiescence: this position is unlikely to recover.
        if !in_chk && depth <= 3 {
            let static_eval = self.cached_eval(board);
            let razor_margin = 250 + 200 * depth;
            if static_eval + razor_margin <= alpha {
                let q = self.quiescence(board, alpha - 1, alpha, 0, info);
                if q <= alpha { return q; }
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
            let static_eval = self.cached_eval(board);
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

        // ── Singular extension probe ──────────────────────────────────────────
        // If the TT move is uniquely best (all alternatives fail low against a
        // reduced-depth search at a margin below the TT score), extend it.
        let mut singular_tt_move = Move::default();
        if !in_excluded_search
            && depth >= 8
            && tt_move != Move::default()
            && tt_depth >= depth - 3
            && tt_score.abs() < MATE_SCORE / 2
        {
            if let Some(flag) = tt_flag_opt {
                if matches!(flag, TTFlag::Lower | TTFlag::Exact) {
                    let s_beta = tt_score - depth * 2;
                    let s_depth = depth / 2;
                    let saved_excl = info.excluded_move;
                    let saved_ply  = info.excluded_ply;
                    info.excluded_move = tt_move;
                    info.excluded_ply = ply;
                    let s = self.alpha_beta(board, s_beta - 1, s_beta, s_depth, ply, false, info);
                    info.excluded_move = saved_excl;
                    info.excluded_ply = saved_ply;
                    if !info.stop && s < s_beta { singular_tt_move = tt_move; }
                }
            }
        }

        // ── Late Move Pruning threshold ──────────────────────────────────────
        // At low depth, only the first LMP_LIMIT quiet moves are searched.
        let lmp_limit: usize = if depth <= 5 { 4 + (depth as usize) * 2 } else { usize::MAX };

        for (i, &mv) in moves.iter().enumerate() {
            // Skip the move flagged out by a singular probe at this ply.
            if in_excluded_search && mv == info.excluded_move { continue; }

            let is_capture = board.piece_on(mv.to()).is_some();
            let is_promo   = mv.flags() == FLAG_PROMO;

            // LMP: skip late quiet non-checking moves when not in check.
            if !in_chk
                && depth <= 5
                && i >= lmp_limit
                && !is_capture
                && !is_promo
                && best_move != Move::default()
            {
                // Confirm it doesn't give check before pruning.
                let trial = board.make_move(mv);
                if !in_check(&trial) { continue; }
            }

            let child = board.make_move(mv);

            // ── Check / singular extension ───────────────────────────────────
            let gives_check = depth >= 2 && in_check(&child);
            let mut extension = if gives_check { 1 } else { 0 };
            if mv == singular_tt_move { extension = extension.max(1); }

            // Push current position onto repetition history for the child's view.
            info.history.push(board.zobrist);

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

            info.history.pop();

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
        let stand_pat = self.cached_eval(board);
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

            // SEE pruning: skip captures that lose material at this square.
            // Skip only for non-EP captures (SEE doesn't model EP victim correctly).
            if mv.flags() != FLAG_EP_CAPTURE && see(board, mv) < 0 {
                continue;
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
                // Use SEE to demote captures that lose material; otherwise MVV-LVA.
                let see_val = if mv.flags() == FLAG_EP_CAPTURE { 100 } else { see(board, mv) };
                if see_val >= 0 {
                    score -= mvv_lva(board, mv) + 10_000;
                } else {
                    // Losing capture: rank below killers and history.
                    score -= see_val; // negative, pushes after quiet moves
                }
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

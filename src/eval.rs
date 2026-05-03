use crate::types::*;
use crate::board::Board;
use crate::movegen::{bishop_attacks, rook_attacks, knight_attacks};

const FILE_A: u64 = 0x0101010101010101;

pub const INF: i32 = 1_000_000;
pub const MATE_SCORE: i32 = 900_000;
const TEMPO: i32 = 10;

pub const PIECE_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 20000];

// Phase weights: max phase = 24
// pawn=0, knight=1, bishop=1, rook=2, queen=4, king=0
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

// ── Midgame PSTs (white perspective, a1=index 0) ──────────────────────────────

#[rustfmt::skip]
const PST_PAWN_MG: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

// Endgame: rank advancement is primary, central files slightly preferred
#[rustfmt::skip]
const PST_PAWN_EG: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    80, 80, 80, 80, 80, 80, 80, 80,
    50, 52, 54, 54, 54, 54, 52, 50,
    30, 32, 34, 34, 34, 34, 32, 30,
    20, 22, 24, 24, 24, 24, 22, 20,
    10, 11, 12, 12, 12, 12, 11, 10,
     5,  5,  5,  5,  5,  5,  5,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const PST_KNIGHT_MG: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

// Knights slightly worse in endgame, still want centralization
#[rustfmt::skip]
const PST_KNIGHT_EG: [i32; 64] = [
    -60,-50,-40,-40,-40,-40,-50,-60,
    -50,-30, -5, -5, -5, -5,-30,-50,
    -40, -5, 10, 15, 15, 10, -5,-40,
    -40,  0, 15, 20, 20, 15,  0,-40,
    -40, -5, 15, 20, 20, 15, -5,-40,
    -40,  0, 10, 15, 15, 10,  0,-40,
    -50,-30, -5,  0,  0, -5,-30,-50,
    -60,-50,-40,-40,-40,-40,-50,-60,
];

#[rustfmt::skip]
const PST_BISHOP_MG: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

// Bishops excellent in open endgames
#[rustfmt::skip]
const PST_BISHOP_EG: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  5,  5,  5,  5,  5,-10,
    -10,  5, 12, 12, 12, 12,  5,-10,
    -10,  5, 12, 16, 16, 12,  5,-10,
    -10,  5, 12, 16, 16, 12,  5,-10,
    -10,  5, 12, 12, 12, 12,  5,-10,
    -10,  5,  5,  5,  5,  5,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const PST_ROOK_MG: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];

#[rustfmt::skip]
const PST_ROOK_EG: [i32; 64] = [
     5,  5,  5,  5,  5,  5,  5,  5,
    10, 12, 12, 12, 12, 12, 12, 10,
     0,  2,  2,  2,  2,  2,  2,  0,
     0,  2,  2,  2,  2,  2,  2,  0,
     0,  2,  2,  2,  2,  2,  2,  0,
     0,  2,  2,  2,  2,  2,  2,  0,
     0,  2,  2,  2,  2,  2,  2,  0,
     5,  5,  5,  8,  8,  5,  5,  5,
];

#[rustfmt::skip]
const PST_QUEEN_MG: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

#[rustfmt::skip]
const PST_QUEEN_EG: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -10,  5,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  5,  5,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

// Midgame: king hides in corner behind pawns
#[rustfmt::skip]
const PST_KING_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];

// Endgame: king should centralize and become active
#[rustfmt::skip]
const PST_KING_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
];

const PST_MG: [[i32; 64]; 6] = [
    PST_PAWN_MG, PST_KNIGHT_MG, PST_BISHOP_MG,
    PST_ROOK_MG, PST_QUEEN_MG,  PST_KING_MG,
];

const PST_EG: [[i32; 64]; 6] = [
    PST_PAWN_EG, PST_KNIGHT_EG, PST_BISHOP_EG,
    PST_ROOK_EG, PST_QUEEN_EG,  PST_KING_EG,
];

// ── Game phase ────────────────────────────────────────────────────────────────

fn game_phase(board: &Board) -> i32 {
    let mut phase = 0i32;
    for c in 0..2 {
        for p in 0..6 {
            phase += board.pieces[c][p].count_ones() as i32 * PHASE_WEIGHTS[p];
        }
    }
    phase.min(24)
}

// ── Main evaluation ───────────────────────────────────────────────────────────

/// Static evaluation, positive = good for side to move
pub fn evaluate(board: &Board) -> i32 {
    let phase = game_phase(board);
    let mut mg = 0i32;
    let mut eg = 0i32;

    for c in 0..2usize {
        let sign = if c == 0 { 1 } else { -1 };

        // Material + PST (tapered per piece)
        for p in 0..6usize {
            let mut bb = board.pieces[c][p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let pst_sq = if c == 0 { sq } else { sq ^ 56 };
                mg += sign * (PIECE_VALUE[p] + PST_MG[p][pst_sq]);
                eg += sign * (PIECE_VALUE[p] + PST_EG[p][pst_sq]);
                bb &= bb - 1;
            }
        }

        // Structural features (both phases)
        let structural = pawn_structure(board, c)
            + mobility_score(board, c)
            + bishop_pair(board, c)
            + rook_open_files(board, c);
        mg += sign * structural;
        eg += sign * structural;

        // King safety: midgame only
        mg += sign * king_safety(board, c);
    }

    // Tapered interpolation
    let score = (mg * phase + eg * (24 - phase)) / 24;
    let score = if board.side == Color::Black { -score } else { score };
    score + TEMPO
}

/// True if endgame (no queens or rooks for side to move) — used by null move pruning.
pub fn is_endgame(board: &Board) -> bool {
    let us = board.side as usize;
    board.pieces[us][Piece::Queen as usize] == 0
        && board.pieces[us][Piece::Rook as usize] == 0
}

// ── Pawn structure ────────────────────────────────────────────────────────────

const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 35, 60, 100, 0];
const DOUBLED_PAWN_PENALTY: i32 = 20;
const ISOLATED_PAWN_PENALTY: i32 = 15;

fn pawn_structure(board: &Board, us: usize) -> i32 {
    let them = 1 - us;
    let our_pawns   = board.pieces[us][Piece::Pawn as usize];
    let their_pawns = board.pieces[them][Piece::Pawn as usize];
    let mut score = 0i32;

    // Doubled pawns
    for file in 0..8usize {
        let file_mask = FILE_A << file;
        let count = (our_pawns & file_mask).count_ones();
        if count > 1 {
            score -= DOUBLED_PAWN_PENALTY * (count - 1) as i32;
        }
    }

    let mut p = our_pawns;
    while p != 0 {
        let sq = p.trailing_zeros() as usize;
        let file = sq % 8;

        // Isolated pawn
        let adj_files = (if file > 0 { FILE_A << (file - 1) } else { 0 })
                      | (if file < 7 { FILE_A << (file + 1) } else { 0 });
        if our_pawns & adj_files == 0 {
            score -= ISOLATED_PAWN_PENALTY;
        }

        // Passed pawn
        if their_pawns & passed_pawn_mask(us, sq) == 0 {
            let rank   = sq / 8;
            let advance = if us == 0 { rank } else { 7 - rank };
            score += PASSED_PAWN_BONUS[advance.min(7)];
        }

        p &= p - 1;
    }
    score
}

fn passed_pawn_mask(us: usize, sq: usize) -> u64 {
    let file = sq % 8;
    let rank = sq / 8;
    let adj = (FILE_A << file)
        | (if file > 0 { FILE_A << (file - 1) } else { 0 })
        | (if file < 7 { FILE_A << (file + 1) } else { 0 });
    if us == 0 {
        if rank >= 7 { 0 } else { adj & (u64::MAX << ((rank + 1) * 8)) }
    } else {
        if rank == 0 { 0 } else { adj & ((1u64 << (rank * 8)) - 1) }
    }
}

// ── Mobility ──────────────────────────────────────────────────────────────────

fn mobility_score(board: &Board, us: usize) -> i32 {
    let our_occ = board.occ[us];
    let occ = board.all;
    let mut score = 0i32;

    // Knights
    let mut knights = board.pieces[us][Piece::Knight as usize];
    while knights != 0 {
        let sq = knights.trailing_zeros() as usize;
        score += (knight_attacks(sq) & !our_occ).count_ones() as i32 * 4;
        knights &= knights - 1;
    }

    // Bishops
    let mut bishops = board.pieces[us][Piece::Bishop as usize];
    while bishops != 0 {
        let sq = bishops.trailing_zeros() as usize;
        score += (bishop_attacks(sq, occ) & !our_occ).count_ones() as i32 * 3;
        bishops &= bishops - 1;
    }

    // Rooks
    let mut rooks = board.pieces[us][Piece::Rook as usize];
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as usize;
        score += (rook_attacks(sq, occ) & !our_occ).count_ones() as i32 * 2;
        rooks &= rooks - 1;
    }

    score
}

// ── Bishop pair ───────────────────────────────────────────────────────────────

fn bishop_pair(board: &Board, us: usize) -> i32 {
    if board.pieces[us][Piece::Bishop as usize].count_ones() >= 2 {
        30
    } else {
        0
    }
}

// ── Rook on open / semi-open files ────────────────────────────────────────────

const ROOK_OPEN_FILE_BONUS: i32 = 25;
const ROOK_SEMI_OPEN_BONUS: i32 = 12;

fn rook_open_files(board: &Board, us: usize) -> i32 {
    let them = 1 - us;
    let our_pawns   = board.pieces[us][Piece::Pawn as usize];
    let their_pawns = board.pieces[them][Piece::Pawn as usize];
    let mut score = 0i32;

    let mut rooks = board.pieces[us][Piece::Rook as usize];
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as usize;
        let file_mask = FILE_A << (sq % 8);

        if our_pawns & file_mask == 0 {
            if their_pawns & file_mask == 0 {
                score += ROOK_OPEN_FILE_BONUS;   // fully open
            } else {
                score += ROOK_SEMI_OPEN_BONUS;   // semi-open
            }
        }
        rooks &= rooks - 1;
    }
    score
}

// ── King safety ───────────────────────────────────────────────────────────────

fn king_safety(board: &Board, us: usize) -> i32 {
    let them = 1 - us;
    if board.pieces[them][Piece::Queen as usize] == 0
        && board.pieces[them][Piece::Rook as usize] == 0 {
        return 0;
    }

    let king_bb = board.pieces[us][Piece::King as usize];
    if king_bb == 0 { return 0; }
    let king_sq   = king_bb.trailing_zeros() as usize;
    let king_file = king_sq % 8;
    let king_rank = king_sq / 8;
    let our_pawns = board.pieces[us][Piece::Pawn as usize];
    let mut score = 0i32;

    // Pawn shield bonus
    let shield_rank = if us == 0 { king_rank + 1 } else { king_rank.wrapping_sub(1) };
    if shield_rank < 8 {
        let shield_files = (FILE_A << king_file)
            | (if king_file > 0 { FILE_A << (king_file - 1) } else { 0 })
            | (if king_file < 7 { FILE_A << (king_file + 1) } else { 0 });
        let rank_mask = 0xFFu64 << (shield_rank * 8);
        score += (our_pawns & shield_files & rank_mask).count_ones() as i32 * 10;
    }

    // Open file toward king
    if our_pawns & (FILE_A << king_file) == 0 {
        score -= 20;
    }

    score
}

// ── MVV-LVA for move ordering ─────────────────────────────────────────────────

pub fn mvv_lva(board: &Board, mv: Move) -> i32 {
    let to = mv.to();
    if let Some((_, victim)) = board.piece_on(to) {
        let from = mv.from();
        let attacker = board.piece_on(from).map(|(_, p)| p).unwrap_or(Piece::Pawn);
        return PIECE_VALUE[victim as usize] * 10 - PIECE_VALUE[attacker as usize];
    }
    0
}

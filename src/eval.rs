use crate::types::*;
use crate::board::Board;
use crate::movegen::{bishop_attacks, rook_attacks};

const FILE_A: u64 = 0x0101010101010101;

pub const INF: i32 = 1_000_000;
pub const MATE_SCORE: i32 = 900_000;

const PIECE_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 20000];

// Piece-square tables (white's perspective, a1=index 0)
// Stored rank 0..7 (a1 bottom-left)
#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const PST_ROOK: [i32; 64] = [
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
const PST_QUEEN: [i32; 64] = [
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

const PST: [[i32; 64]; 6] = [
    PST_PAWN,
    PST_KNIGHT,
    PST_BISHOP,
    PST_ROOK,
    PST_QUEEN,
    PST_KING_MG,
];

/// Static evaluation, positive = good for side to move
pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0i32;
    for c in 0..2usize {
        let sign = if c == 0 { 1 } else { -1 };
        for p in 0..6usize {
            let mut bb = board.pieces[c][p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let pst_sq = if c == 0 { sq } else { sq ^ 56 };
                score += sign * (PIECE_VALUE[p] + PST[p][pst_sq]);
                bb &= bb - 1;
            }
        }
        score += sign * pawn_structure(board, c);
        score += sign * mobility_score(board, c);
        score += sign * king_safety(board, c);
    }
    if board.side == Color::Black { score = -score; }
    score
}

/// True if side to move has no major pieces — null move pruning should be skipped (zugzwang risk).
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
    let our_pawns  = board.pieces[us][Piece::Pawn as usize];
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

        // Isolated: no friendly pawns on adjacent files
        let adj_files = (if file > 0 { FILE_A << (file - 1) } else { 0 })
                      | (if file < 7 { FILE_A << (file + 1) } else { 0 });
        if our_pawns & adj_files == 0 {
            score -= ISOLATED_PAWN_PENALTY;
        }

        // Passed pawn: no enemy pawns ahead on same/adjacent files
        if their_pawns & passed_pawn_mask(us, sq) == 0 {
            let rank = sq / 8;
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

    let mut bishops = board.pieces[us][Piece::Bishop as usize];
    while bishops != 0 {
        let sq = bishops.trailing_zeros() as usize;
        score += (bishop_attacks(sq, occ) & !our_occ).count_ones() as i32 * 3;
        bishops &= bishops - 1;
    }

    let mut rooks = board.pieces[us][Piece::Rook as usize];
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as usize;
        score += (rook_attacks(sq, occ) & !our_occ).count_ones() as i32 * 2;
        rooks &= rooks - 1;
    }

    score
}

// ── King safety ───────────────────────────────────────────────────────────────

fn king_safety(board: &Board, us: usize) -> i32 {
    let them = 1 - us;
    // Only relevant when opponent has major pieces
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

/// MVV-LVA capture score for move ordering
pub fn mvv_lva(board: &Board, mv: Move) -> i32 {
    let to = mv.to();
    if let Some((_, victim)) = board.piece_on(to) {
        let us = board.side as usize;
        // find attacker
        let from = mv.from();
        let attacker = board.piece_on(from).map(|(_, p)| p).unwrap_or(Piece::Pawn);
        return PIECE_VALUE[victim as usize] * 10 - PIECE_VALUE[attacker as usize];
    }
    0
}

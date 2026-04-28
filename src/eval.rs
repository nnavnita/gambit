use crate::types::*;
use crate::board::Board;

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
                // Mirror sq for black (so PST is always from white's view)
                let pst_sq = if c == 0 { sq } else { sq ^ 56 };
                score += sign * (PIECE_VALUE[p] + PST[p][pst_sq]);
                bb &= bb - 1;
            }
        }
    }
    if board.side == Color::Black { score = -score; }
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

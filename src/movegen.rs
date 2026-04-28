use crate::types::*;
use crate::board::*;

// ── Precomputed attack tables ─────────────────────────────────────────────────

static KNIGHT_ATTACKS: std::sync::OnceLock<[Bitboard; 64]> = std::sync::OnceLock::new();
static KING_ATTACKS:   std::sync::OnceLock<[Bitboard; 64]> = std::sync::OnceLock::new();
static PAWN_ATTACKS:   std::sync::OnceLock<[[Bitboard; 64]; 2]> = std::sync::OnceLock::new();

pub fn init_attacks() {
    KNIGHT_ATTACKS.get_or_init(|| {
        let mut t = [0u64; 64];
        for sq in 0..64usize {
            let bb = 1u64 << sq;
            t[sq] =
                ((bb << 17) & !FILE_A) |
                ((bb << 15) & !FILE_H) |
                ((bb << 10) & !(FILE_A | FILE_B)) |
                ((bb <<  6) & !(FILE_G | FILE_H)) |
                ((bb >> 17) & !FILE_H) |
                ((bb >> 15) & !FILE_A) |
                ((bb >> 10) & !(FILE_G | FILE_H)) |
                ((bb >>  6) & !(FILE_A | FILE_B));
        }
        t
    });
    KING_ATTACKS.get_or_init(|| {
        let mut t = [0u64; 64];
        for sq in 0..64usize {
            let bb = 1u64 << sq;
            t[sq] =
                ((bb << 1) & !FILE_A) |
                ((bb >> 1) & !FILE_H) |
                (bb << 8) | (bb >> 8) |
                ((bb << 9) & !FILE_A) |
                ((bb << 7) & !FILE_H) |
                ((bb >> 9) & !FILE_H) |
                ((bb >> 7) & !FILE_A);
        }
        t
    });
    PAWN_ATTACKS.get_or_init(|| {
        let mut t = [[0u64; 64]; 2];
        for sq in 0..64usize {
            let bb = 1u64 << sq;
            t[0][sq] = ((bb << 9) & !FILE_A) | ((bb << 7) & !FILE_H); // white
            t[1][sq] = ((bb >> 9) & !FILE_H) | ((bb >> 7) & !FILE_A); // black
        }
        t
    });
}

const FILE_A: Bitboard = 0x0101010101010101;
const FILE_B: Bitboard = FILE_A << 1;
const FILE_G: Bitboard = FILE_A << 6;
const FILE_H: Bitboard = FILE_A << 7;

// ── Slider attacks (classical fill) ──────────────────────────────────────────

fn ray(sq: usize, dir: i32, occ: Bitboard, blocker_mask: Bitboard) -> Bitboard {
    let mut result = 0u64;
    let mut s = sq as i32 + dir;
    let mut prev_file = (sq % 8) as i32;
    while s >= 0 && s < 64 {
        let f = s % 8;
        // Stop wrapping around the board
        if (dir == 1 || dir == -1) && (f - prev_file).abs() != 1 { break; }
        if (dir == 9 || dir == -7) && f == 0 { result |= 1u64 << s; break; }
        if (dir == 7 || dir == -9) && f == 7 { result |= 1u64 << s; break; }
        result |= 1u64 << s;
        if occ & (1u64 << s) != 0 { break; }
        prev_file = f;
        s += dir;
    }
    result & blocker_mask
}

pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    sliding_attacks(sq, occ, &[8, -8, 1, -1])
}

pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    sliding_attacks(sq, occ, &[9, 7, -9, -7])
}

fn sliding_attacks(sq: usize, occ: Bitboard, dirs: &[i32]) -> Bitboard {
    let mut result = 0u64;
    for &dir in dirs {
        result |= slide_ray(sq, dir, occ);
    }
    result
}

fn slide_ray(sq: usize, dir: i32, occ: Bitboard) -> Bitboard {
    let mut result = 0u64;
    let mut s = sq as i32;
    loop {
        let prev_file = (s % 8) as i32;
        s += dir;
        if s < 0 || s >= 64 { break; }
        let f = (s % 8) as i32;
        // Prevent file wrapping for horizontal/diagonal moves
        match dir {
             1 | 9 | -7 => { if f == 0 { break; } }
            -1 | -9 | 7 => { if f == 7 { break; } }
            _ => {}
        }
        result |= 1u64 << s;
        if occ & (1u64 << s) != 0 { break; }
    }
    result
}

// ── Attack detection ──────────────────────────────────────────────────────────

pub fn square_attacked(board: &Board, sq: usize, by: Color) -> bool {
    let c = by as usize;
    let occ = board.all;
    let ka = KING_ATTACKS.get().unwrap();
    let na = KNIGHT_ATTACKS.get().unwrap();
    let pa = PAWN_ATTACKS.get().unwrap();

    if pa[1 - c][sq] & board.pieces[c][Piece::Pawn as usize] != 0 { return true; }
    if na[sq] & board.pieces[c][Piece::Knight as usize] != 0 { return true; }
    if ka[sq] & board.pieces[c][Piece::King as usize] != 0 { return true; }
    if bishop_attacks(sq, occ) & (board.pieces[c][Piece::Bishop as usize] | board.pieces[c][Piece::Queen as usize]) != 0 { return true; }
    if rook_attacks(sq, occ) & (board.pieces[c][Piece::Rook as usize] | board.pieces[c][Piece::Queen as usize]) != 0 { return true; }
    false
}

pub fn in_check(board: &Board) -> bool {
    let us = board.side as usize;
    let king_bb = board.pieces[us][Piece::King as usize];
    if king_bb == 0 { return false; }
    let king_sq = king_bb.trailing_zeros() as usize;
    square_attacked(board, king_sq, board.side.flip())
}

// ── Move generation ───────────────────────────────────────────────────────────

pub fn generate_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::with_capacity(64);
    let us = board.side as usize;
    let them = 1 - us;
    let occ = board.all;
    let our_occ = board.occ[us];
    let their_occ = board.occ[them];

    let ka = KING_ATTACKS.get().unwrap();
    let na = KNIGHT_ATTACKS.get().unwrap();

    // Pawns
    gen_pawn_moves(board, &mut moves);

    // Knights
    let mut knights = board.pieces[us][Piece::Knight as usize];
    while knights != 0 {
        let from = knights.trailing_zeros() as usize;
        let mut attacks = na[from] & !our_occ;
        while attacks != 0 {
            let to = attacks.trailing_zeros() as usize;
            moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
            attacks &= attacks - 1;
        }
        knights &= knights - 1;
    }

    // Bishops
    let mut bishops = board.pieces[us][Piece::Bishop as usize];
    while bishops != 0 {
        let from = bishops.trailing_zeros() as usize;
        let mut attacks = bishop_attacks(from, occ) & !our_occ;
        while attacks != 0 {
            let to = attacks.trailing_zeros() as usize;
            moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
            attacks &= attacks - 1;
        }
        bishops &= bishops - 1;
    }

    // Rooks
    let mut rooks = board.pieces[us][Piece::Rook as usize];
    while rooks != 0 {
        let from = rooks.trailing_zeros() as usize;
        let mut attacks = rook_attacks(from, occ) & !our_occ;
        while attacks != 0 {
            let to = attacks.trailing_zeros() as usize;
            moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
            attacks &= attacks - 1;
        }
        rooks &= rooks - 1;
    }

    // Queens
    let mut queens = board.pieces[us][Piece::Queen as usize];
    while queens != 0 {
        let from = queens.trailing_zeros() as usize;
        let mut attacks = (bishop_attacks(from, occ) | rook_attacks(from, occ)) & !our_occ;
        while attacks != 0 {
            let to = attacks.trailing_zeros() as usize;
            moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
            attacks &= attacks - 1;
        }
        queens &= queens - 1;
    }

    // King
    let king_sq = board.pieces[us][Piece::King as usize].trailing_zeros() as usize;
    let mut attacks = ka[king_sq] & !our_occ;
    while attacks != 0 {
        let to = attacks.trailing_zeros() as usize;
        moves.push(Move::new(king_sq as u32, to as u32, FLAG_NONE, 0));
        attacks &= attacks - 1;
    }

    // Castling
    gen_castling(board, &mut moves);

    moves
}

fn gen_pawn_moves(board: &Board, moves: &mut Vec<Move>) {
    let us = board.side as usize;
    let them = 1 - us;
    let occ = board.all;
    let their_occ = board.occ[them];
    let pawns = board.pieces[us][Piece::Pawn as usize];
    let pa = PAWN_ATTACKS.get().unwrap();

    let (push1, push2_rank, promo_rank, start_rank): (fn(u64) -> u64, u64, u64, u64) = if us == 0 {
        (|bb| bb << 8, 0x00000000FF000000, 0xFF00000000000000, 0x000000000000FF00)
    } else {
        (|bb| bb >> 8, 0x000000FF00000000, 0x00000000000000FF, 0x00FF000000000000)
    };

    // Single push
    let single = push1(pawns) & !occ;
    // Double push
    let double = push1(single & push1(start_rank & pawns) & !occ) & !occ;
    // Actually simpler:
    let single2 = push1(pawns) & !occ;
    let double2 = push1(single2 & if us == 0 { 0x0000000000FF0000u64 } else { 0x0000FF0000000000u64 }) & !occ;

    let mut s = single2;
    while s != 0 {
        let to = s.trailing_zeros() as usize;
        let from = if us == 0 { to - 8 } else { to + 8 };
        if (1u64 << to) & promo_rank != 0 {
            for p in 0..4u32 { moves.push(Move::new(from as u32, to as u32, FLAG_PROMO, p)); }
        } else {
            moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
        }
        s &= s - 1;
    }

    let mut d = double2;
    while d != 0 {
        let to = d.trailing_zeros() as usize;
        let from = if us == 0 { to - 16 } else { to + 16 };
        moves.push(Move::new(from as u32, to as u32, FLAG_DOUBLE_PUSH, 0));
        d &= d - 1;
    }

    // Captures
    let mut p = pawns;
    while p != 0 {
        let from = p.trailing_zeros() as usize;
        let mut caps = pa[us][from] & their_occ;
        while caps != 0 {
            let to = caps.trailing_zeros() as usize;
            if (1u64 << to) & promo_rank != 0 {
                for pr in 0..4u32 { moves.push(Move::new(from as u32, to as u32, FLAG_PROMO, pr)); }
            } else {
                moves.push(Move::new(from as u32, to as u32, FLAG_NONE, 0));
            }
            caps &= caps - 1;
        }
        // En passant
        if board.ep_sq != 255 {
            let ep = board.ep_sq as usize;
            if pa[us][from] & (1u64 << ep) != 0 {
                moves.push(Move::new(from as u32, ep as u32, FLAG_EP_CAPTURE, 0));
            }
        }
        p &= p - 1;
    }
}

fn gen_castling(board: &Board, moves: &mut Vec<Move>) {
    let us = board.side as usize;
    let occ = board.all;
    let them = board.side.flip();

    if us == 0 {
        // White kingside
        if board.castling & CASTLE_WK != 0
            && occ & 0x60 == 0
            && !square_attacked(board, 4, them)
            && !square_attacked(board, 5, them)
            && !square_attacked(board, 6, them)
        {
            moves.push(Move::new(4, 6, FLAG_KINGSIDE_CASTLE, 0));
        }
        // White queenside
        if board.castling & CASTLE_WQ != 0
            && occ & 0xE == 0
            && !square_attacked(board, 4, them)
            && !square_attacked(board, 3, them)
            && !square_attacked(board, 2, them)
        {
            moves.push(Move::new(4, 2, FLAG_QUEENSIDE_CASTLE, 0));
        }
    } else {
        // Black kingside
        if board.castling & CASTLE_BK != 0
            && occ & 0x6000000000000000 == 0
            && !square_attacked(board, 60, them)
            && !square_attacked(board, 61, them)
            && !square_attacked(board, 62, them)
        {
            moves.push(Move::new(60, 62, FLAG_KINGSIDE_CASTLE, 0));
        }
        // Black queenside
        if board.castling & CASTLE_BQ != 0
            && occ & 0x0E00000000000000 == 0
            && !square_attacked(board, 60, them)
            && !square_attacked(board, 59, them)
            && !square_attacked(board, 58, them)
        {
            moves.push(Move::new(60, 58, FLAG_QUEENSIDE_CASTLE, 0));
        }
    }
}

/// Filter pseudolegal moves to only legal ones
pub fn legal_moves(board: &Board) -> Vec<Move> {
    generate_moves(board)
        .into_iter()
        .filter(|&mv| {
            let after = board.make_move(mv);
            let us = board.side as usize;
            let king_bb = after.pieces[us][Piece::King as usize];
            if king_bb == 0 { return false; }
            let king_sq = king_bb.trailing_zeros() as usize;
            !square_attacked(&after, king_sq, board.side.flip())
        })
        .collect()
}

/// Parse UCI move string (e.g. "e2e4", "e7e8q")
pub fn parse_uci_move(board: &Board, s: &str) -> Option<Move> {
    if s.len() < 4 { return None; }
    let b = s.as_bytes();
    let from = ((b[1] - b'1') as usize) * 8 + (b[0] - b'a') as usize;
    let to   = ((b[3] - b'1') as usize) * 8 + (b[2] - b'a') as usize;
    let promo_char = if s.len() > 4 { Some(b[4]) } else { None };

    for mv in legal_moves(board) {
        if mv.from() != from || mv.to() != to { continue; }
        if mv.flags() == FLAG_PROMO {
            let pc = match promo_char {
                Some(b'n') => 0,
                Some(b'b') => 1,
                Some(b'r') => 2,
                _          => 3, // queen default
            };
            if mv.promo() as u32 == pc { return Some(mv); }
        } else {
            return Some(mv);
        }
    }
    None
}

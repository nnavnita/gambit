use crate::types::*;

/// Castling rights bitmask
pub const CASTLE_WK: u8 = 1;
pub const CASTLE_WQ: u8 = 2;
pub const CASTLE_BK: u8 = 4;
pub const CASTLE_BQ: u8 = 8;

#[derive(Clone)]
pub struct Board {
    /// pieces[color][piece]
    pub pieces: [[Bitboard; 6]; 2],
    /// combined occupancy per color
    pub occ: [Bitboard; 2],
    /// all occupied
    pub all: Bitboard,
    pub side: Color,
    pub castling: u8,
    /// en passant target square (255 = none)
    pub ep_sq: u8,
    pub halfmove: u32,
    pub fullmove: u32,
    pub zobrist: u64,
}

// Zobrist tables (initialized once via lazy_static-style const arrays via build-time is complex;
// we'll init at runtime via thread_local or just store in a struct)
// For simplicity, generate at startup and store in a global.
use std::sync::OnceLock;

static ZOBRIST: OnceLock<ZobristKeys> = OnceLock::new();

pub struct ZobristKeys {
    pub pieces: [[[u64; 64]; 6]; 2], // [color][piece][sq]
    pub side: u64,
    pub castling: [u64; 16],
    pub ep: [u64; 8], // file
}

pub fn zobrist() -> &'static ZobristKeys {
    ZOBRIST.get_or_init(|| {
        let mut seed: u64 = 0x123456789ABCDEF0;
        let mut next = || -> u64 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut pieces = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pieces[c][p][s] = next(); }}}
        let side = next();
        let mut castling = [0u64; 16];
        for i in 0..16 { castling[i] = next(); }
        let mut ep = [0u64; 8];
        for i in 0..8 { ep[i] = next(); }
        ZobristKeys { pieces, side, castling, ep }
    })
}

impl Board {
    #[allow(dead_code)]
    pub fn startpos() -> Board {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn from_fen(fen: &str) -> Board {
        let mut b = Board {
            pieces: [[0; 6]; 2],
            occ: [0; 2],
            all: 0,
            side: Color::White,
            castling: 0,
            ep_sq: 255,
            halfmove: 0,
            fullmove: 1,
            zobrist: 0,
        };
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.is_empty() { return b; }

        // Piece placement
        let mut rank = 7i32;
        let mut file = 0i32;
        for ch in parts[0].chars() {
            match ch {
                '/' => { rank -= 1; file = 0; }
                '1'..='8' => { file += (ch as i32) - ('0' as i32); }
                _ => {
                    let (color, piece) = char_to_piece(ch);
                    let sq = (rank * 8 + file) as usize;
                    b.pieces[color as usize][piece as usize] |= 1u64 << sq;
                    file += 1;
                }
            }
        }

        // Side to move
        if parts.len() > 1 { b.side = if parts[1] == "b" { Color::Black } else { Color::White }; }

        // Castling
        if parts.len() > 2 {
            for ch in parts[2].chars() {
                match ch {
                    'K' => b.castling |= CASTLE_WK,
                    'Q' => b.castling |= CASTLE_WQ,
                    'k' => b.castling |= CASTLE_BK,
                    'q' => b.castling |= CASTLE_BQ,
                    _ => {}
                }
            }
        }

        // En passant
        if parts.len() > 3 && parts[3] != "-" {
            let ep = parts[3].as_bytes();
            if ep.len() >= 2 {
                let f = (ep[0] - b'a') as u8;
                let r = (ep[1] - b'1') as u8;
                b.ep_sq = r * 8 + f;
            }
        }

        if parts.len() > 4 { b.halfmove = parts[4].parse().unwrap_or(0); }
        if parts.len() > 5 { b.fullmove = parts[5].parse().unwrap_or(1); }

        b.update_occ();
        b.compute_zobrist();
        b
    }

    fn update_occ(&mut self) {
        for c in 0..2 {
            self.occ[c] = self.pieces[c].iter().fold(0, |a, &x| a | x);
        }
        self.all = self.occ[0] | self.occ[1];
    }

    fn compute_zobrist(&mut self) {
        let z = zobrist();
        let mut h = 0u64;
        for c in 0..2 {
            for p in 0..6 {
                let mut bb = self.pieces[c][p];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    h ^= z.pieces[c][p][sq];
                    bb &= bb - 1;
                }
            }
        }
        if self.side == Color::Black { h ^= z.side; }
        h ^= z.castling[self.castling as usize];
        if self.ep_sq != 255 { h ^= z.ep[(self.ep_sq % 8) as usize]; }
        self.zobrist = h;
    }

    /// Returns piece (color, type) on a square, or None
    pub fn piece_on(&self, sq: usize) -> Option<(Color, Piece)> {
        let mask = 1u64 << sq;
        for c in 0..2 {
            for p in 0..6 {
                if self.pieces[c][p] & mask != 0 {
                    return Some((
                        if c == 0 { Color::White } else { Color::Black },
                        index_to_piece(p),
                    ));
                }
            }
        }
        None
    }

    /// Pass move (null move) — flips side, clears ep. Used by null-move pruning.
    pub fn make_null_move(&self) -> Board {
        let mut b = self.clone();
        let z = zobrist();
        if b.ep_sq != 255 {
            b.zobrist ^= z.ep[(b.ep_sq % 8) as usize];
            b.ep_sq = 255;
        }
        b.halfmove += 1;
        b.side = b.side.flip();
        b.zobrist ^= z.side;
        b
    }

    pub fn make_move(&self, mv: Move) -> Board {
        let mut b = self.clone();
        let z = zobrist();
        let us = b.side as usize;
        let them = 1 - us;
        let from = mv.from();
        let to = mv.to();
        let flags = mv.flags();

        // Un-hash ep/castling before change
        if b.ep_sq != 255 { b.zobrist ^= z.ep[(b.ep_sq % 8) as usize]; }
        b.zobrist ^= z.castling[b.castling as usize];

        // Find moving piece
        let mut moving_piece = Piece::Pawn;
        for p in 0..6 {
            if b.pieces[us][p] & (1u64 << from) != 0 {
                moving_piece = index_to_piece(p);
                break;
            }
        }

        let from_mask = 1u64 << from;
        let to_mask = 1u64 << to;

        // Remove moving piece from source
        b.pieces[us][moving_piece as usize] &= !from_mask;
        b.zobrist ^= z.pieces[us][moving_piece as usize][from];

        // Remove captured piece (normal capture)
        if flags != FLAG_EP_CAPTURE {
            for p in 0..6 {
                if b.pieces[them][p] & to_mask != 0 {
                    b.pieces[them][p] &= !to_mask;
                    b.zobrist ^= z.pieces[them][p][to];
                    break;
                }
            }
        }

        // Place piece at destination (or promo piece)
        let land_piece = if flags == FLAG_PROMO {
            mv.promo()
        } else {
            moving_piece
        };
        b.pieces[us][land_piece as usize] |= to_mask;
        b.zobrist ^= z.pieces[us][land_piece as usize][to];

        // En passant capture
        if flags == FLAG_EP_CAPTURE {
            let ep_cap_sq = if us == 0 { to - 8 } else { to + 8 };
            b.pieces[them][Piece::Pawn as usize] &= !(1u64 << ep_cap_sq);
            b.zobrist ^= z.pieces[them][Piece::Pawn as usize][ep_cap_sq];
        }

        // Castling: move rook
        if flags == FLAG_KINGSIDE_CASTLE {
            let (rook_from, rook_to) = if us == 0 { (7, 5) } else { (63, 61) };
            b.pieces[us][Piece::Rook as usize] &= !(1u64 << rook_from);
            b.pieces[us][Piece::Rook as usize] |= 1u64 << rook_to;
            b.zobrist ^= z.pieces[us][Piece::Rook as usize][rook_from];
            b.zobrist ^= z.pieces[us][Piece::Rook as usize][rook_to];
        }
        if flags == FLAG_QUEENSIDE_CASTLE {
            let (rook_from, rook_to) = if us == 0 { (0, 3) } else { (56, 59) };
            b.pieces[us][Piece::Rook as usize] &= !(1u64 << rook_from);
            b.pieces[us][Piece::Rook as usize] |= 1u64 << rook_to;
            b.zobrist ^= z.pieces[us][Piece::Rook as usize][rook_from];
            b.zobrist ^= z.pieces[us][Piece::Rook as usize][rook_to];
        }

        // Update castling rights
        b.castling &= CASTLING_RIGHTS_MASK[from] & CASTLING_RIGHTS_MASK[to];

        // Update en passant square
        b.ep_sq = if flags == FLAG_DOUBLE_PUSH {
            if us == 0 { (to - 8) as u8 } else { (to + 8) as u8 }
        } else {
            255
        };

        // Half/full move counters
        if moving_piece == Piece::Pawn || flags != FLAG_NONE {
            b.halfmove = 0;
        } else {
            b.halfmove += 1;
        }
        if us == 1 { b.fullmove += 1; }

        b.side = b.side.flip();
        b.zobrist ^= z.side;
        if b.ep_sq != 255 { b.zobrist ^= z.ep[(b.ep_sq % 8) as usize]; }
        b.zobrist ^= z.castling[b.castling as usize];

        b.update_occ();
        b
    }
}

/// Castling rights update mask per square
const CASTLING_RIGHTS_MASK: [u8; 64] = {
    let mut m = [0xFFu8; 64];
    m[0]  &= !CASTLE_WQ;
    m[4]  &= !(CASTLE_WK | CASTLE_WQ);
    m[7]  &= !CASTLE_WK;
    m[56] &= !CASTLE_BQ;
    m[60] &= !(CASTLE_BK | CASTLE_BQ);
    m[63] &= !CASTLE_BK;
    m
};

fn char_to_piece(ch: char) -> (Color, Piece) {
    let color = if ch.is_uppercase() { Color::White } else { Color::Black };
    let piece = match ch.to_ascii_lowercase() {
        'p' => Piece::Pawn,
        'n' => Piece::Knight,
        'b' => Piece::Bishop,
        'r' => Piece::Rook,
        'q' => Piece::Queen,
        'k' => Piece::King,
        _   => Piece::Pawn,
    };
    (color, piece)
}

fn index_to_piece(i: usize) -> Piece {
    match i {
        0 => Piece::Pawn,
        1 => Piece::Knight,
        2 => Piece::Bishop,
        3 => Piece::Rook,
        4 => Piece::Queen,
        5 => Piece::King,
        _ => Piece::Pawn,
    }
}

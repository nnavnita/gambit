/// Bitboard: u64 where bit i = square i (a1=0, h8=63)
pub type Bitboard = u64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

/// Compact move: from(6) | to(6) | promo(3) | flags(3)
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Move(pub u32);

pub const FLAG_NONE: u32 = 0;
pub const FLAG_DOUBLE_PUSH: u32 = 1;
pub const FLAG_KINGSIDE_CASTLE: u32 = 2;
pub const FLAG_QUEENSIDE_CASTLE: u32 = 3;
pub const FLAG_EP_CAPTURE: u32 = 4;
pub const FLAG_PROMO: u32 = 5; // promo piece in bits 12-14

impl Move {
    pub fn new(from: u32, to: u32, flags: u32, promo: u32) -> Move {
        Move(from | (to << 6) | (flags << 12) | (promo << 15))
    }

    pub fn from(self) -> usize { (self.0 & 0x3F) as usize }
    pub fn to(self) -> usize   { ((self.0 >> 6) & 0x3F) as usize }
    pub fn flags(self) -> u32  { (self.0 >> 12) & 0x7 }
    pub fn promo(self) -> Piece {
        match (self.0 >> 15) & 0x7 {
            0 => Piece::Knight,
            1 => Piece::Bishop,
            2 => Piece::Rook,
            3 => Piece::Queen,
            _ => Piece::Queen,
        }
    }

    /// UCI string e.g. "e2e4", "e7e8q"
    pub fn to_uci(self) -> String {
        let from = self.from();
        let to = self.to();
        let fc = (b'a' + (from % 8) as u8) as char;
        let fr = (b'1' + (from / 8) as u8) as char;
        let tc = (b'a' + (to % 8) as u8) as char;
        let tr = (b'1' + (to / 8) as u8) as char;
        let mut s = format!("{}{}{}{}", fc, fr, tc, tr);
        if self.flags() == FLAG_PROMO {
            s.push(match self.promo() {
                Piece::Knight => 'n',
                Piece::Bishop => 'b',
                Piece::Rook   => 'r',
                Piece::Queen  => 'q',
                _             => 'q',
            });
        }
        s
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

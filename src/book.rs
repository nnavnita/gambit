/// Tiny hand-curated opening book.
/// Keys are FEN prefixes (board + side + castling + ep — first four FEN tokens).
/// Values: candidate UCI moves; one is picked pseudo-randomly per call.

const BOOK: &[(&str, &[&str])] = &[
    // ── White first moves ──
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -",
        &["e2e4", "d2d4", "g1f3", "c2c4"]),

    // ── Black replies to 1.e4 ──
    ("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3",
        &["e7e5", "c7c5", "e7e6", "c7c6", "g8f6"]),
    // 1.e4 e5
    ("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &["g1f3", "f1c4", "b1c3"]),
    // 1.e4 e5 2.Nf3
    ("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -",
        &["b8c6", "g8f6", "d7d6"]),
    // Italian: 1.e4 e5 2.Nf3 Nc6 3.Bc4
    ("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq -",
        &["g8f6", "f8c5", "f8e7"]),
    // 1.e4 c5 (Sicilian)
    ("rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6",
        &["g1f3", "b1c3", "c2c3"]),
    // 1.e4 e6 (French)
    ("rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &["d2d4", "g1f3"]),
    // 1.e4 c6 (Caro-Kann)
    ("rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -",
        &["d2d4", "b1c3"]),

    // ── Black replies to 1.d4 ──
    ("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3",
        &["g8f6", "d7d5", "e7e6"]),
    // 1.d4 d5
    ("rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &["c2c4", "g1f3"]),
    // 1.d4 Nf6
    ("rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq -",
        &["c2c4", "g1f3"]),

    // ── Black replies to 1.Nf3 ──
    ("rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq -",
        &["g8f6", "d7d5", "c7c5"]),

    // ── Black replies to 1.c4 (English) ──
    ("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3",
        &["e7e5", "g8f6", "c7c5"]),
];

/// Look up a book move. Returns UCI string if found.
pub fn probe(fen: &str, salt: u64) -> Option<String> {
    let key = fen_key(fen);
    for (book_fen, moves) in BOOK {
        if *book_fen == key {
            if moves.is_empty() { return None; }
            let idx = (salt as usize) % moves.len();
            return Some(moves[idx].to_string());
        }
    }
    None
}

/// Strip halfmove + fullmove counters from a FEN so book lookups ignore them.
fn fen_key(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().take(4).collect();
    parts.join(" ")
}

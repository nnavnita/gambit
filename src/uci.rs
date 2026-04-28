use std::io::{self, BufRead};
use std::time::Duration;
use crate::board::Board;
use crate::movegen::{init_attacks, parse_uci_move};
use crate::search::{Searcher, SearchInfo};

pub fn run() {
    init_attacks();
    let stdin = io::stdin();
    let mut board = Board::startpos();
    let mut searcher = Searcher::new();

    println!("Gambit chess engine");

    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let line = line.trim();
        if line.is_empty() { continue; }

        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("uci") => {
                println!("id name Gambit");
                println!("id author Navnita");
                println!("uciok");
            }
            Some("isready") => println!("readyok"),
            Some("ucinewgame") => {
                board = Board::startpos();
                searcher.clear_for_search();
                searcher.tt = crate::tt::TranspositionTable::new(64);
            }
            Some("position") => {
                board = parse_position(tokens.collect::<Vec<_>>().join(" ").as_str());
            }
            Some("go") => {
                let args: Vec<&str> = tokens.collect();
                let time_ms = parse_time(&args, board.side == crate::types::Color::White);
                searcher.clear_for_search();
                let mut info = SearchInfo::new(Duration::from_millis(time_ms));
                let (best_move, _) = searcher.search(&board, &mut info);
                println!("bestmove {}", best_move.to_uci());
            }
            Some("stop") => {} // handled by time limit
            Some("quit") | None => break,
            Some(cmd) => eprintln!("Unknown command: {}", cmd),
        }
    }
}

fn parse_position(args: &str) -> Board {
    let mut tokens = args.split_whitespace().peekable();
    let mut board = match tokens.next() {
        Some("startpos") => Board::startpos(),
        Some("fen") => {
            // collect up to "moves" keyword
            let mut fen_parts = Vec::new();
            while let Some(&t) = tokens.peek() {
                if t == "moves" { break; }
                fen_parts.push(t);
                tokens.next();
            }
            Board::from_fen(&fen_parts.join(" "))
        }
        _ => Board::startpos(),
    };

    if tokens.next() == Some("moves") {
        for mv_str in tokens {
            if let Some(mv) = parse_uci_move(&board, mv_str) {
                board = board.make_move(mv);
            }
        }
    }
    board
}

fn parse_time(args: &[&str], white: bool) -> u64 {
    let mut wtime = 0u64;
    let mut btime = 0u64;
    let mut winc  = 0u64;
    let mut binc  = 0u64;
    let mut movetime = 0u64;
    let mut movestogo = 30u64;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "wtime"     => { i += 1; wtime     = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            "btime"     => { i += 1; btime     = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            "winc"      => { i += 1; winc      = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            "binc"      => { i += 1; binc      = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            "movetime"  => { i += 1; movetime  = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            "movestogo" => { i += 1; movestogo = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(30); }
            _ => {}
        }
        i += 1;
    }

    if movetime > 0 { return movetime; }

    let (time, inc) = if white { (wtime, winc) } else { (btime, binc) };
    if time == 0 { return 1000; } // fallback 1s

    // Allocate: time/movestogo + increment with overhead buffer
    let alloc = time / movestogo + inc;
    alloc.min(time / 2).max(50) // never use more than half remaining time
}

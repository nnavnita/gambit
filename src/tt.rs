use crate::types::Move;

#[derive(Clone, Copy)]
pub enum TTFlag { Exact, Lower, Upper }

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key:       u64,
    pub depth:     i32,
    pub flag:      TTFlag,
    pub score:     i32,
    pub best_move: Move,
}

pub struct TranspositionTable {
    table: Vec<Option<TTEntry>>,
    size:  usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let size = (mb * 1024 * 1024) / std::mem::size_of::<Option<TTEntry>>();
        TranspositionTable {
            table: vec![None; size],
            size,
        }
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let idx = (key as usize) % self.size;
        if let Some(e) = self.table[idx] {
            if e.key == key { return Some(e); }
        }
        None
    }

    pub fn store(&mut self, key: u64, depth: i32, flag: TTFlag, score: i32, best_move: Move) {
        let idx = (key as usize) % self.size;
        // Always-replace strategy
        self.table[idx] = Some(TTEntry { key, depth, flag, score, best_move });
    }
}

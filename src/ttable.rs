use eval::*;
use std::mem;
use std::ops;

use atomic_option::AtomicOption;

use std::sync::atomic::Ordering;

use chess::*;

#[derive(Debug, Clone, Copy)]
pub enum ValueInfo {
    Exact(Score),
    LowerBound(Score),
    UpperBound(Score),
}

use ValueInfo::*;

impl ValueInfo {
    pub fn as_approximation(self) -> Score {
        match self {
            Exact(s) => s,
            LowerBound(s) => s,
            UpperBound(s) => s,
        }
    }
}

impl ops::Neg for ValueInfo {
    type Output = ValueInfo;

    fn neg(self) -> ValueInfo {
        match self {
            Exact(s) => Exact(-s),
            LowerBound(s) => UpperBound(-s),
            UpperBound(s) => LowerBound(-s),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TEntry {
    pub hash: u64,
    pub depth: i32,
    // pub age: i32,
    pub value: ValueInfo,
    pub best_move: ChessMove,
}

pub struct TTable {
    table: Vec<AtomicOption<TEntry>>,
}

impl TTable {
    pub fn new(capacity: usize) -> TTable {
        let entry_size = mem::size_of::<TEntry>();
        let num = capacity / entry_size;
        let mut table = Vec::with_capacity(num);
        for _ in 0..num {
            table.push(AtomicOption::empty());
        }
        TTable { table }
    }

    pub fn put(&self, entry: TEntry) {
        let index = self.hash_index(entry.hash);
        let bucket = &self.table[index];
        let boxed_entry = match bucket.take(Ordering::Relaxed) {
            None => Box::new(entry),
            Some(mut cur_entry) => {
                if cur_entry.hash != entry.hash || entry.depth >= cur_entry.depth {
                    *cur_entry = entry;
                }
                cur_entry
            }
        };
        self.table[index].swap(boxed_entry, Ordering::Relaxed);
    }

    pub fn fetch(&self, hash: u64) -> Option<TEntry> {
        let index = self.hash_index(hash);
        let bucket = &self.table[index];
        match bucket.take(Ordering::Relaxed) {
            None => None,
            Some(entry) => {
                let result = if entry.hash == hash {
                    Some(*entry)
                } else {
                    None
                };
                bucket.try_store(entry, Ordering::Relaxed);
                result
            }
        }
    }

    fn hash_index(&self, hash: u64) -> usize {
        (hash % self.table.len() as u64) as usize
    }
}

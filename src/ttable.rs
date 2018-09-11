
use eval::*;
use std::mem;

use atomic_option::AtomicOption;

use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Debug, Clone, Copy)]
pub struct TEntry {
    pub hash: u64,
    pub depth: i32,
    // pub age: i32,
    pub value: Score,
    pub value_type: ValueType,
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
        TTable {table}
    }

    pub fn put(&self, entry: Box<TEntry>) -> Option<Box<TEntry>> {
        let index = self.hash_index(entry.hash);
        self.table[index].swap(entry, Ordering::Relaxed)
    }

    pub fn fetch(&self, hash: u64) -> Option<TEntry> {
        let index = self.hash_index(hash);
        let bucket = &self.table[index];
        match bucket.take(Ordering::Relaxed) {
            None => None,
            Some(entry) => {
                let result = if entry.hash == hash { Some(*entry) } else { None };
                bucket.try_store(entry, Ordering::Relaxed);
                result
            }
        }
    }

    fn hash_index(&self, hash: u64) -> usize {
        (hash % self.table.len() as u64) as usize
    }
}
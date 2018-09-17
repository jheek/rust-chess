


use std::mem;
use std::cmp::{min, max};
use std::time::Instant;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender, channel};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use std::marker::Send;

use std::thread::{JoinHandle, spawn};

use chess::*;
use eval::*;
use ttable::*;


pub struct AlphaBetaResult {
    pub line: Vec<ChessMove>,
    pub score: Score,
}

struct ABResult {
    best_move: Option<ChessMove>,
    best_value: Score,
}

impl ABResult {
    fn from_tentry(entry: TEntry, min_depth: i32) -> Option<ABResult> {
        match entry {
            TEntry { value: ValueInfo::Exact(best_value), depth, ..} if depth >= min_depth => Some(ABResult {best_value, best_move: Some(entry.best_move) }),
            _ => None,
        }
    }
}

pub struct ISUpdate {
    pub line: Vec<ChessMove>,
    pub score: Score,
    pub depth: i32,
}

pub struct InfiniteSearch {
    kill_switch: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl InfiniteSearch {
    pub fn start<F>(ttable: &'static TTable, board: Board, max_depth: i32, mut callback: F) -> InfiniteSearch
    where F: FnMut(ISUpdate) -> () + Send + 'static {
        let (sender, receiver) = channel();
        let mut is = InfiniteSearch { kill_switch: Arc::new(AtomicBool::new(false)), workers: Vec::new() };
        let kill_switch = is.kill_switch.clone();
        let worker = spawn(move || infinite_search(ttable, &board, max_depth, &kill_switch, sender));
        spawn(move || {
            for msg in receiver.iter() {
                callback(msg);
            }
            println!("Handler is dying!");
        });
        is.workers.push(worker);
        is
    }

    pub fn join(mut self) {
        self.join_internal();
    }

    fn join_internal(&mut self) {
        if self.workers.len() > 0 {
            self.kill_switch.store(true, Ordering::Relaxed);
            self.workers.drain(..).for_each(|worker| { worker.join().unwrap(); } );
            println!("join complete!");
        }
    }
}

impl Drop for InfiniteSearch {
    fn drop(&mut self) {
        self.join_internal();
    }
}

pub fn infinite_search(ttable: &TTable, board: &Board, max_depth: i32, kill_switch: &AtomicBool, sender: Sender<ISUpdate>) {
    let mut ticks = 0;
    let mut killed = false; 
    let mut guess = [0, 0];
    for depth in 1..=max_depth {
        {
            let mut callback = || {
                ticks += 1;
                if ticks % 10000 == 0 && kill_switch.load(Ordering::Relaxed) {
                    killed = true;
                    true
                } else {
                    false
                }
            };
            let gi = (depth as usize) % 2;
            for result in mtdf(&mut callback, ttable, board, depth, guess[gi]) {
                guess[gi] = result.best_value;
                match alpha_beta_line(&mut || false, ttable, board, depth, result) {
                    Some(AlphaBetaResult {line, score, ..}) => {
                        if sender.send(ISUpdate { line, score, depth }).is_err() {
                            return;
                        }
                    },
                    None => {
                        return;
                    }
                }
            };
        }
        if killed {
            return;
        }
    }
}

fn mtdf<'r, 'a: 'r, 'b: 'r, 'c: 'r, F>(callback: &'a mut F, ttable: &'b TTable, board: &'c Board, depth: i32, guess: Score) -> impl Iterator<Item=ABResult> + 'r
    where F: FnMut() -> bool {
    (0..).scan((MIN_SCORE, MAX_SCORE, guess), move |state, _x| {
        let (lower_bound, upper_bound, guess) = state;
        if lower_bound >= upper_bound {
            return None;
        }
        let beta = max(*guess, *lower_bound + 1);
        let result = alpha_beta(callback, ttable, board, depth, beta - 1, beta)?;
        *guess = result.best_value;
        if *guess < beta {
            *upper_bound = *guess;
        } else {
            *lower_bound = *guess;
        }
        Some(result)
    })
}

// pub fn find_best_move(ttable: &TTable, board: &Board, depth: i32) -> AlphaBetaResult {
//     let best_move = alpha_beta(&mut || false, ttable, board, depth, MIN_SCORE, MAX_SCORE).unwrap();
//     let result = alpha_beta_line(&mut || false, ttable, board, depth, best_move).unwrap();
//     result
// }

fn alpha_beta_line<F>(callback: &mut F, ttable: &TTable, board: &Board, depth: i32, result: ABResult) -> Option<AlphaBetaResult>
    where F: FnMut() -> bool  {
    let mut line = Vec::with_capacity(depth as usize);
    let mut sub_board = *board;
    let mut sub_move = result.best_move;
    while let Some(cmove) = sub_move {
        line.push(cmove);
        let d = line.len() as i32;
        sub_move = if d < depth {
            sub_board = sub_board.make_move(cmove);
            let window = if sub_board.side_to_move() == board.side_to_move() { result.best_value } else { -result.best_value };
            let sub_result = alpha_beta(callback, ttable, &sub_board, depth - d, window - 1, window)?;
            sub_result.best_move
        } else {
            None
        }
    }
    Some(AlphaBetaResult { line, score: result.best_value })
}

fn alpha_beta<F>(callback: &mut F, ttable: &TTable, board: &Board, depth: i32, alpha: Score, beta: Score) -> Option<ABResult>
    where F: FnMut() -> bool {
    let entry = ttable.fetch(board.get_hash());
    if let Some(result) = entry.and_then(|entry| ABResult::from_tentry(entry, depth)) {
        Some(result)
    } else {
        let mut moves_ar: [ChessMove; 256] = unsafe { mem::uninitialized() };
        let num_moves = board.enumerate_moves(&mut moves_ar);
        let moves = &moves_ar[..num_moves];
        alpha_beta_raw(callback, ttable, board, &entry, moves, depth, alpha, beta)
    }
}

fn alpha_beta_raw<F>(callback: &mut F, ttable: &TTable, board: &Board, entry_op: &Option<TEntry>, moves: &[ChessMove], depth: i32, mut alpha: Score, mut beta: Score) -> Option<ABResult>
    where F: FnMut() -> bool {
    if callback() {
        return None;
    }
    let alpha_orig = alpha;
    let score_mul = if board.side_to_move() == Color::White { 1 } else { -1 };
    if moves.len() == 0 {
        let value = score_mul * board_score(&board, moves, depth);
        return Some(ABResult { best_move: None, best_value: value });
    }
    let mut prev_best_move = None;
    if let Some(entry) = entry_op {
        prev_best_move = Some(entry.best_move);
        if entry.depth >= depth {
            match entry.value {
                ValueInfo::Exact(best_value) => {
                    return Some(ABResult { best_value, best_move: Some(entry.best_move) });
                },
                ValueInfo::LowerBound(value) => {
                    alpha = max(alpha, value);
                },
                ValueInfo::UpperBound(value) => {
                    beta = min(beta, value);
                },
            }
            if alpha >= beta {
                return Some(ABResult { best_value: entry.value.as_approximation(), best_move: Some(entry.best_move) });
            }
        } 
    }

    let mut moves_data: [(Board, Option<TEntry>); 256] = unsafe { mem::uninitialized() };
    let mut ordered_moves: [(u8, Score); 256] = unsafe { mem::uninitialized() };
    let mut best_value = MIN_SCORE;
    let mut best_move = moves[0];
    for (i, &cmove) in moves.iter().enumerate() {
        let move_board = board.make_move(cmove);
        let move_entry = ttable.fetch(move_board.get_hash());
        let mut fast_score = move_entry
            .map(|x| x.value.as_approximation())
            .unwrap_or_else(|| score_mul * fast_board_score(&move_board, depth as i32));
        if Some(cmove) == prev_best_move {
            fast_score += 1000;
        }
        moves_data[i] = (move_board, move_entry);
        ordered_moves[i] = (i as u8, fast_score);
    }
    ordered_moves[..moves.len()].sort_unstable_by(|(_, a), (_, b)| b.cmp(&a) );

    for (ind, _) in ordered_moves[..moves.len()].iter() {
        let i = *ind as usize;
        let (move_board, move_entry) = &moves_data[i];
        let cmove = moves[i];
        let value;
        if let Some(sub_result) = move_entry.and_then(|move_entry| ABResult::from_tentry(move_entry, depth - 1)) {
            value = -sub_result.best_value;
        } else {
            let mut sub_moves_ar: [ChessMove; 256] = unsafe { mem::uninitialized() };
            let sub_num_moves = move_board.enumerate_moves(&mut sub_moves_ar);
            let sub_moves = &sub_moves_ar[..sub_num_moves];
            if depth > 1 {
                let sub_result = alpha_beta_raw(callback, ttable, move_board, move_entry, sub_moves, depth - 1, -beta, -alpha)?;
                value = -sub_result.best_value;
            } else {
                value = score_mul * board_score(&move_board, sub_moves, depth);
            }
        }
        
        if value > best_value {
            best_value = value;
            best_move = cmove;
            alpha = max(alpha, value);
            if alpha >= beta {
                break
            }
        }
    }
    let value = if best_value <= alpha_orig {
        ValueInfo::UpperBound(best_value)
    } else if best_value >= beta {
        ValueInfo::LowerBound(best_value)
    } else {
        ValueInfo::Exact(best_value)
    };
    let entry = TEntry {
        hash: board.get_hash(),
        depth,
        value,
        best_move,
    };
    ttable.put(entry);
    Some(ABResult { best_move: Some(best_move), best_value })
}

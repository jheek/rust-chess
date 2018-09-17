


use std::mem;
use std::cmp::{min, max};
use std::time::Instant;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;

use chess::*;
use eval::*;
use ttable::*;

pub struct AlphaBetaResult {
    pub line: Vec<ChessMove>,
    pub score: Score,
    pub n: u64,
    pub m: u64,
}

struct ABResult {
    best_move: Option<ChessMove>,
    best_value: Score,
    n: u64,
    m: u64,
}

impl ABResult {
    fn from_tentry(entry: TEntry, min_depth: i32) -> Option<ABResult> {
        match entry {
            TEntry { value: ValueInfo::Exact(best_value), depth, ..} if depth >= min_depth => Some(ABResult {best_value, best_move: Some(entry.best_move), n: 0, m: 1 }),
            _ => None,
        }
    }
}

pub struct ISUpdate {
    pub line: Vec<ChessMove>,
    pub score: Score,
    pub depth: i32,
}

pub fn infinite_search(ttable: &TTable, board: &Board, max_depth: i32, kill_switch: AtomicBool, sender: Sender<ISUpdate>) {
    let mut ticks = 0;
    let mut callback = || {
        ticks += 1;
        return ticks % 10 == 0 && kill_switch.load(Ordering::Relaxed);
    };
    for depth in 1..=max_depth {
        match alpha_beta_line(&mut callback, ttable, board, depth, MIN_SCORE, MAX_SCORE) {
            Some(AlphaBetaResult {line, score, ..}) => {
                if sender.send(ISUpdate { line, score, depth }).is_err() {
                    break;
                }
            },
            None => {
                break;
            }
        }
    }
}

pub fn find_best_move(ttable: &TTable, board: &Board, depth: i32) -> AlphaBetaResult {
    let start = Instant::now();
    let result = alpha_beta_line(&mut || false, ttable, board, depth, MIN_SCORE, MAX_SCORE).unwrap();
    let elapsed = start.elapsed();
    println!("Elapsed: {}s {}m, N: {}, M: {}", elapsed.as_secs(), elapsed.subsec_millis(), result.n, result.m);
    result
}


fn alpha_beta_line<F>(callback: &mut F, ttable: &TTable, board: &Board, depth: i32, alpha: Score, beta: Score) -> Option<AlphaBetaResult>
    where F: FnMut() -> bool  {
    let result = alpha_beta(callback, ttable, board, depth, MIN_SCORE, MAX_SCORE)?;
    let mut line = Vec::with_capacity(depth as usize);
    let mut sub_board = *board;
    let mut sub_move = result.best_move;
    let mut n = 0;
    let mut m = 0;
    while let Some(cmove) = sub_move {
        line.push(cmove);
        let d = line.len() as i32;
        sub_move = if d < depth {
            sub_board = sub_board.make_move(cmove);
            let window = if sub_board.side_to_move() == board.side_to_move() { result.best_value } else { -result.best_value };
            let sub_result = alpha_beta(callback, ttable, &sub_board, depth - d, window - 1, window)?;
            n += sub_result.n;
            m += sub_result.m;
            sub_result.best_move
        } else {
            None
        }
    }
    Some(AlphaBetaResult { line, score: result.best_value, n, m })
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
        return Some(ABResult { best_move: None, best_value: value, n: 1, m: 0 });
    }
    let mut prev_best_move = None;
    if let Some(entry) = entry_op {
        prev_best_move = Some(entry.best_move);
        if entry.depth >= depth {
            match entry.value {
                ValueInfo::Exact(best_value) => {
                    return Some(ABResult { best_value, best_move: Some(entry.best_move), n: 0, m: 1 });
                },
                ValueInfo::LowerBound(value) => {
                    alpha = max(alpha, value);
                },
                ValueInfo::UpperBound(value) => {
                    beta = min(beta, value);
                },
            }
            if alpha >= beta {
                return Some(ABResult { best_value: entry.value.as_approximation(), best_move: Some(entry.best_move), n: 0, m: 1 });
            }
        } 
    }

    let mut moves_data: [(Board, Option<TEntry>); 256] = unsafe { mem::uninitialized() };
    let mut ordered_moves: [(u8, Score); 256] = unsafe { mem::uninitialized() };
    let mut best_value = MIN_SCORE;
    let mut best_move = moves[0];
    let mut n = 0;
    let mut m = 0;
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
            n += sub_result.n;
            m += sub_result.m;
            value = -sub_result.best_value;
        } else {
            let mut sub_moves_ar: [ChessMove; 256] = unsafe { mem::uninitialized() };
            let sub_num_moves = move_board.enumerate_moves(&mut sub_moves_ar);
            let sub_moves = &sub_moves_ar[..sub_num_moves];
            if depth > 1 {
                let sub_result = alpha_beta_raw(callback, ttable, move_board, move_entry, sub_moves, depth - 1, -beta, -alpha)?;

                n += sub_result.n;
                m += sub_result.m;
                value = -sub_result.best_value;
            } else {
                value = score_mul * board_score(&move_board, sub_moves, depth);
                n += 1;
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
    Some(ABResult { best_move: Some(best_move), best_value, n, m })
}

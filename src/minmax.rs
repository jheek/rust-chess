


use std::mem;
use std::cmp;

use chess::*;
use eval::*;
use ttable::*;

#[derive(Copy, Clone)]
pub struct SearchInfo<'a> {
    ttable: &'a TTable,
}

pub struct AlphaBetaResult {
    pub cmove: Option<ChessMove>,
    pub score: Score,
}

#[derive(Default, Copy, Clone)]
struct MoveAndScore {
    cmove: ChessMove,
    score: Score,
}

fn ordered_moves<'a>(info: SearchInfo<'a>, board: &Board, target: &mut [MoveAndScore; 256]) -> usize {
    let is_maximizing = board.side_to_move() == Color::White;
    let mut moves:[ChessMove; 256] = unsafe { mem::uninitialized() };
    let num_moves = board.enumerate_moves(&mut moves);
    for (i, &cmove) in moves[..num_moves].iter().enumerate() {
        let move_board = board.make_move(cmove);
        let fast_score = info.ttable.fetch(move_board.get_hash())
            .map(|x| x.value)
            .unwrap_or_else(|| board_score(&move_board, 0));

        target[i] = MoveAndScore { cmove, score: if is_maximizing {-fast_score} else {fast_score} };
    }
    target[..num_moves].sort_unstable_by(|a, b| b.score.cmp(&a.score) );
    
    num_moves
}


pub fn find_line<'a>(mut board: Board, mut max_depth: i32, ttable_op: Option<&'a TTable>) -> String {
    let mut result = String::new();
    let mut first = true;
    while max_depth > 0 {
        let best_move = find_best_move(board, max_depth, ttable_op, false);
        match best_move.cmove {
            Some(cmove) => {
                board = board.make_move(cmove);
                if !first {
                    result += " ";
                }
                result += &cmove.get_source().to_string();
                result += "-";
                result += &cmove.get_dest().to_string();
                max_depth -= 1;
                first = false;
            },
            None => {
                break;
            }
        }
    }
    result
}

pub fn find_best_move<'a>(board: Board, max_depth: i32, ttable_op: Option<&'a TTable>, verbose: bool) -> AlphaBetaResult {
    let local_ttable;
    let ttable = match ttable_op {
        Some(t) => t,
        None => {
            local_ttable = TTable::new(100 * 1024 * 1024);
            &local_ttable
        }
    };

    let info = SearchInfo { ttable };
    let mut moves_with_score: [MoveAndScore; 256] = unsafe { mem::uninitialized() };
    let num_moves = ordered_moves(info, &board, &mut moves_with_score);
    if num_moves == 0 {
        return AlphaBetaResult { cmove: None, score: MIN_SCORE };
    }

    let mut alpha_orig = MIN_SCORE;
    let mut beta_orig = MAX_SCORE;

    for depth in max_depth..=max_depth {
        for r in 1.. {
            if r > 1 {
                println!("repetition: {} depth: {}", r, depth);
            }
            let mut alpha = alpha_orig;
            let beta = beta_orig;
            for MoveAndScore {cmove, score} in moves_with_score[..num_moves].iter_mut() {
                let move_board = board.make_move(*cmove);
                let sub_result = alpha_beta(info, move_board, depth - 1, alpha, beta);
                *score = -sub_result;
                alpha = cmp::max(alpha, sub_result);
            }
            if is_maximizing {
                moves_with_score[..num_moves].sort_unstable_by(|a, b| b.score.cmp(&a.score) );
            } else {
                moves_with_score[..num_moves].sort_unstable_by(|a, b| a.score.cmp(&b.score) );
            }
            let best_value = -moves_with_score[0].score;
            if best_value <= alpha_orig {
                alpha_orig = cmp::min(alpha_orig - 25 * r, best_value);
            } else if best_value >= beta_orig {
                beta_orig = cmp::max(beta_orig + 25 * r, best_value);
            } else {
                alpha_orig = best_value - 25;
                beta_orig = best_value + 25;
                break;
            }
        }
    }
    if verbose {
        for MoveAndScore {cmove, score} in moves_with_score[..num_moves].iter_mut() {
            println!("{} {} {}", cmove.get_source(), cmove.get_dest(), -*score);
            // println!("{} {} {} ({})", cmove.get_source(), cmove.get_dest(), -*score, find_line(board.make_move(*cmove), max_depth - 1, None));
        }
    }
    
    let best_move = &moves_with_score[0];
    AlphaBetaResult { cmove: Some(best_move.cmove), score: -best_move.score }
}

pub fn alpha_beta<'a>(info: SearchInfo<'a>, board: Board, depth: i32, mut alpha: Score, mut beta: Score) -> Score {
    let is_maximizing = board.side_to_move() == Color::White;
    let orig_alpha = alpha;
    let orig_beta = beta;
    let mut result = if is_maximizing {MIN_SCORE} else {MAX_SCORE};
    if let Some(entry) = info.ttable.fetch(board.get_hash()) {
        if entry.depth >= depth {
            match entry.value_type {
                ValueType::Exact => {return entry.value;},
                ValueType::LowerBound => { alpha = cmp::max(alpha, entry.value); },
                ValueType::UpperBound => { beta = cmp::min(beta, entry.value); },
            }
        }
    }
    if depth <= 0 || board.status() != BoardStatus::Ongoing {
        result = board_score(&board, depth);
    } else {
        let mut moves_with_score: [MoveAndScore; 256] = unsafe { mem::uninitialized() };
        let num_moves = ordered_moves(info, &board, &mut moves_with_score);
        
        for &MoveAndScore {cmove, ..} in moves_with_score[..num_moves].iter() {
            let move_board = board.make_move(cmove);
            let sub_result = alpha_beta(info, move_board, depth - 1, alpha, beta);
            if is_maximizing {
                if sub_result > result {
                    alpha = cmp::max(alpha, sub_result);
                    result = sub_result;
                }
            } else {
                if sub_result < result {
                    beta = cmp::min(beta, sub_result);
                    result = sub_result;
                }
            }
            if alpha >= beta {
                break;
            }
        }
    }
    let value_type = if result <= orig_alpha {
        ValueType::UpperBound
    } else if result >= orig_beta {
        ValueType::LowerBound
    } else {
        ValueType::Exact
    };
    let entry = TEntry { hash: board.get_hash(), depth, value: result, value_type };
    info.ttable.put(Box::new(entry));
    result
}
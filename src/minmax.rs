


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
    pub line: Vec<ChessMove>,
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

pub const MAX_DEPTH: usize = 100;

struct ABStack {
    board: Board,
    alpha: Score,
    beta: Score,
    orig_alpha: Score,
    moves_with_score: [MoveAndScore; 256],
    num_moves: usize,
    c_move: usize,
    best_index: usize,
    best_value: Score,
    best_line: [ChessMove; MAX_DEPTH],
    best_length: usize
}


pub fn find_best_move<'a>(board: Board, max_depth: usize, ttable: &'a TTable) -> AlphaBetaResult {
    let global_score_mul = if board.side_to_move() == Color::White { 1 } else { -1 };
    let mut score_mul = global_score_mul;
    let mut stack: [ABStack; MAX_DEPTH] = unsafe { mem::uninitialized() };
    let mut n: u64 = 0;
    stack[0] = ABStack {
        board,
        alpha: MIN_SCORE, beta: MAX_SCORE, orig_alpha: MIN_SCORE,
        moves_with_score: unsafe { mem::uninitialized() },
        num_moves: 0,
        c_move: 0,
        best_index: 0,
        best_value: MIN_SCORE,
        best_line: unsafe { mem::uninitialized() },
        best_length: 0
    };
    let mut d = 0;
    loop {
        let depth_to_go = max_depth - d - 1;
        if stack[d].c_move == 0 {
            if let Some(entry) = ttable.fetch(stack[d].board.get_hash()) {
                if entry.depth >= depth_to_go as i32 {
                    let value_type = if score_mul == 1 { entry.value_type } else { !entry.value_type };
                    let value = score_mul * entry.value;
                    match value_type {
                        ValueType::Exact => {
                            stack[d].alpha = cmp::max(stack[d].alpha, value);
                            stack[d].beta = cmp::min(stack[d].beta, value + 1);
                        },
                        ValueType::LowerBound => {
                            stack[d].alpha = cmp::max(stack[d].alpha, value);
                        },
                        ValueType::UpperBound => {
                            stack[d].beta = cmp::min(stack[d].beta, value);
                        }
                    }
                }
            }
            // init moves
            let mut moves:[ChessMove; 256] = unsafe { mem::uninitialized() };
            stack[d].num_moves = stack[d].board.enumerate_moves(&mut moves);
            for (i, &cmove) in moves[..stack[d].num_moves].iter().enumerate() {
                let move_board = stack[d].board.make_move(cmove);
                let fast_score = ttable.fetch(move_board.get_hash())
                    .map(|x| x.value)
                    .unwrap_or_else(|| board_score(&move_board, 0));

                stack[d].moves_with_score[i] = MoveAndScore { cmove, score: score_mul * fast_score };
            }
            stack[d].moves_with_score[..stack[d].num_moves].sort_unstable_by(|a, b| b.score.cmp(&a.score) );
        }
        // exit conditions
        if stack[d].c_move == stack[d].num_moves || stack[d].alpha >= stack[d].beta {
            // make ttable entry
            let value = stack[d].best_value;
            let mut value_type = if value <= stack[d].orig_alpha {
                ValueType::LowerBound
            } else if value >= stack[d].beta {
                ValueType::UpperBound
            } else {
                ValueType::Exact
            };
            if score_mul == -1 {
                value_type = !value_type;
            }
            let entry = TEntry {
                hash: stack[d].board.get_hash(),
                value: value * score_mul,
                depth: depth_to_go as i32,
                value_type,
            };
            ttable.put(entry);
            // pop stack
            if d == 0 {
                break;
            }
            let value = -value;
            d -= 1;
            score_mul *= -1;
            stack[d].moves_with_score[stack[d].c_move].score = value;
            if value > stack[d].best_value {
                stack[d].alpha = cmp::max(stack[d].alpha, value);
                stack[d].best_index = stack[d].c_move;
                stack[d].best_value = value;
                let length = stack[d + 1].best_length;
                stack[d].best_length = length + 1;
                stack[d].best_line[0] = stack[d].moves_with_score[stack[d].c_move].cmove;
                if let Some((head, tail)) = stack[d..].split_first_mut() {
                    head.best_line[1..=length].copy_from_slice(&tail[0].best_line[0..length]);
                } else {
                    panic!("Can't split stack!");
                }
            }
            stack[d].c_move += 1;

            continue;
        }
        let cmove = stack[d].moves_with_score[stack[d].c_move].cmove;
        let next_board = stack[d].board.make_move(cmove);
        let terminal = depth_to_go == 0 || next_board.status() != BoardStatus::Ongoing;

        if terminal {
            n += 1;
            let score = board_score(&next_board, (max_depth - d) as i32);
            let value = score_mul * score;
            if value > stack[d].best_value {
                // local improvement
                stack[d].alpha = cmp::max(stack[d].alpha, value);
                stack[d].best_index = stack[d].c_move;
                stack[d].best_value = value;
                stack[d].best_length = 1;
                stack[d].best_line[0] = cmove;
            }
            stack[d].c_move += 1;
        } else {
            // push stack
            stack[d + 1] = ABStack {
                board: next_board,
                alpha: -stack[d].beta, beta: -stack[d].alpha, orig_alpha: -stack[d].beta,
                moves_with_score: unsafe { mem::uninitialized() },
                num_moves: 0,
                c_move: 0,
                best_index: 0,
                best_value: MIN_SCORE,
                best_line: unsafe { mem::uninitialized() },
                best_length: 0
            };
            d += 1;
            score_mul *= -1;
        }
    }
    stack[d].moves_with_score[..stack[d].num_moves].sort_unstable_by(|a, b| b.score.cmp(&a.score) );
    println!("N: {}", n);
    for MoveAndScore {cmove, score} in stack[0].moves_with_score[..stack[0].num_moves].iter() {
        println!("{} {} {}", cmove.get_source().to_string(), cmove.get_dest().to_string(), score);
    }
    let head = &stack[0];
    let line = head.best_line[0..head.best_length].iter()
        .cloned()
        .collect();
    AlphaBetaResult { score: head.best_value, line }
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
    info.ttable.put(entry);
    result
}
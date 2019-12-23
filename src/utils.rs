use std::mem;

use chess::*;

pub fn generate_moves(board: &Board) -> Vec<ChessMove> {
    let mut moves = unsafe { mem::uninitialized() };
    let num_moves = board.enumerate_moves(&mut moves);
    moves[..num_moves].iter().cloned().collect()
}

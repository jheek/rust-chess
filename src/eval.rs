

use chess::*;

pub type Score = i32;

pub const WIN_SCORE: Score = 100000;
pub const DRAW_SCORE: Score = 0;

pub const MIN_SCORE: Score = -1000000;
pub const MAX_SCORE: Score = 1000000;

const PT_PAWNS: [i32; 64] = [
0,  0,  0,  0,  0,  0,  0,  0,
50, 50, 50, 50, 50, 50, 50, 50,
10, 10, 20, 30, 30, 20, 10, 10,
 5,  5, 10, 25, 25, 10,  5,  5,
 0,  0,  0, 20, 20,  0,  0,  0,
 5, -5,-10,  0,  0,-10, -5,  5,
 5, 10, 10,-20,-20, 10, 10,  5,
 0,  0,  0,  0,  0,  0,  0,  0];

const PT_KNIGHT: [i32; 64] = [
-50,-40,-30,-30,-30,-30,-40,-50,
-40,-20,  0,  0,  0,  0,-20,-40,
-30,  0, 10, 15, 15, 10,  0,-30,
-30,  5, 15, 20, 20, 15,  5,-30,
-30,  0, 15, 20, 20, 15,  0,-30,
-30,  5, 10, 15, 15, 10,  5,-30,
-40,-20,  0,  5,  5,  0,-20,-40,
-50,-40,-30,-30,-30,-30,-40,-50,
];

const PT_BISHOP: [i32; 64] = [
-20,-10,-10,-10,-10,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5, 10, 10,  5,  0,-10,
-10,  5,  5, 10, 10,  5,  5,-10,
-10,  0, 10, 10, 10, 10,  0,-10,
-10, 10, 10, 10, 10, 10, 10,-10,
-10,  5,  0,  0,  0,  0,  5,-10,
-20,-10,-10,-10,-10,-10,-10,-20,
];

const PT_ROOK: [i32; 64] = [
  0,  0,  0,  0,  0,  0,  0,  0,
  5, 10, 10, 10, 10, 10, 10,  5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
 -5,  0,  0,  0,  0,  0,  0, -5,
  0,  0,  0,  5,  5,  0,  0,  0
];

const PT_QUEEN: [i32; 64] = [
-20,-10,-10, -5, -5,-10,-10,-20,
-10,  0,  0,  0,  0,  0,  0,-10,
-10,  0,  5,  5,  5,  5,  0,-10,
 -5,  0,  5,  5,  5,  5,  0, -5,
  0,  0,  5,  5,  5,  5,  0, -5,
-10,  5,  5,  5,  5,  5,  0,-10,
-10,  0,  5,  0,  0,  0,  0,-10,
-20,-10,-10, -5, -5,-10,-10,-20
];

const PT_KING: [i32; 64] = [
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-30,-40,-40,-50,-50,-40,-40,-30,
-20,-30,-30,-40,-40,-30,-30,-20,
-10,-20,-20,-20,-20,-20,-20,-10,
 20, 20,  0,  0,  0,  0, 20, 20,
 20, 30, 10,  0,  0, 10, 30, 20
];

fn piece_score(piece: Piece) -> Score {
    match piece {
        Piece::Bishop => 330,
        Piece::Rook   => 500,
        Piece::Knight => 320,
        Piece::Pawn   => 100,
        Piece::Queen  => 900,
        Piece::King   => 0,
    }
}

fn piece_position_table(piece: Piece) -> &'static[i32; 64] {
    match piece {
        Piece::Bishop => &PT_BISHOP,
        Piece::Rook   => &PT_ROOK,
        Piece::Knight => &PT_KNIGHT,
        Piece::Pawn   => &PT_PAWNS,
        Piece::Queen  => &PT_QUEEN,
        Piece::King   => &PT_KING,
    }
}

fn material_score(board: &Board, color: Color) -> Score {
    let color_bb = board.color_combined(color);
    ALL_PIECES.iter()
        .map(|&p| ((board.pieces(p) & color_bb).popcnt() as i32) * piece_score(p) )
        .sum()
}

fn position_score(board: &Board, color: Color) -> Score {
    let mut score = 0;
    let color_bb = board.color_combined(color);
    for &piece in ALL_PIECES.iter() {
        let pt = piece_position_table(piece);
        for sq in board.pieces(piece) & color_bb {
            let mut i = sq.to_index();
            if color == Color::White {
                i = 8 * (7 - i / 8) + (i % 8); // mirror
            }
            score += pt[i];
        }
    }
    score
}

pub fn board_score(board: &Board, depth: i32) -> Score {
    let active_color = board.side_to_move();
    match board.status() {
        BoardStatus::Checkmate if active_color == Color::White => -(WIN_SCORE + depth),
        BoardStatus::Checkmate => WIN_SCORE + depth,
        BoardStatus::Stalemate => DRAW_SCORE,
        BoardStatus::Ongoing => {
            let material = material_score(&board, Color::White) - material_score(&board, Color::Black);
            let position = position_score(&board, Color::White) - position_score(&board, Color::Black);
            let score = material + position;
            score
        }
    }
}
type Lineup = Vec<String>;

use chess::*;
use eval::Score;
use minmax::ISUpdate;

#[derive(Serialize, Deserialize, Debug)]
pub struct WSMove {
    pub from: String,
    pub to: String,
}

impl From<ChessMove> for WSMove {
    fn from(other: ChessMove) -> WSMove {
        let from = other.get_source().to_string();
        let to = other.get_dest().to_string();
        WSMove { from, to }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WSState {
    pub legal_moves: Vec<WSMove>,
    pub lineup: Lineup,
    pub best_line: Vec<WSMove>,
    pub best_value: Score,
    pub side_to_move: &'static str,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WSRMessage {
    Move { from: String, to: String },
    Reset,
}

pub fn lineup(board: &Board) -> Lineup {
    let mut lineup = vec![];
    for color in [Color::White, Color::Black].iter() {
        for sq in board.color_combined(*color) {
            if let Some(piece) = board.piece_on(sq) {
                let mut c = String::from(match piece {
                    Piece::Bishop => "b",
                    Piece::Rook => "r",
                    Piece::Knight => "n",
                    Piece::Pawn => "p",
                    Piece::King => "k",
                    Piece::Queen => "q",
                });
                if *color == Color::White {
                    c = c.to_uppercase();
                }
                c += "@";
                c += &sq.to_string();
                lineup.push(c)
            }
        }
    }
    lineup
}

pub fn compute_ws_state(board: Board, result: Option<ISUpdate>) -> WSState {
    let iterable = MoveGen::new(board, true);
    let legal_moves = iterable.map(WSMove::from).collect();
    let side_to_move = match board.side_to_move() {
        Color::White => "white",
        Color::Black => "black",
    };
    let (best_line, best_value) = match result {
        None => (Vec::new(), 0),
        Some(update) => {
            let best_line = update.line.iter().map(|m| WSMove::from(*m)).collect();
            (best_line, update.score)
        }
    };
    WSState {
        legal_moves,
        lineup: lineup(&board),
        best_line,
        best_value,
        side_to_move,
    }
}

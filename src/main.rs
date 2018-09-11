
extern crate ws;
extern crate chess;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate crossbeam;
extern crate atomic_option;

use ws::*;
use chess::*;

use std::cell::{Cell, RefCell};

mod eval;
mod minmax;
mod ttable;

use minmax::*;
use eval::*;

#[derive(Serialize, Deserialize, Debug)]
struct WSMove {
    from: String,
    to: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct WSState {
    legal_moves: Vec<WSMove>,
    lineup: Lineup,
}

#[derive(Serialize, Deserialize, Debug)]
enum WSRMessage {
    Move {
        from: String,
        to: String,
    },
    Reset,
}

type Lineup = Vec<String>;

fn main() {
    listen("127.0.0.1:3012", handle_connection)
        .unwrap_or_else(|err| panic!("Cannot listen to port 3012: {}", err));
}

fn lineup(board: &Board) -> Lineup {
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

fn compute_ws_state(board: Board) -> WSState {
    let iterable = MoveGen::new(board, true);
    let legal_moves = iterable.map(|m| {
        let from = m.get_source().to_string();
        let to = m.get_dest().to_string();
        WSMove {from, to}
    }).collect();
    WSState {legal_moves, lineup: lineup(&board)}
}

fn handle_connection(out: Sender) -> impl Handler {
    let board_cell = Cell::new(Board::default());
    let moves_cell = RefCell::new(([ChessMove::default(); 256], 0));
    move |raw_msg| {
        match raw_msg {
            Message::Binary(_) => out.close(CloseCode::Error),
            Message::Text(text) => {
                match serde_json::from_str(&text) {
                    Ok(msg) => {
                        let mut board = board_cell.get();
                        let (ref mut moves, ref mut num_moves) = *moves_cell.borrow_mut();
                        match msg {
                            WSRMessage::Reset => {
                                board_cell.set(Board::default());
                            },
                            WSRMessage::Move {from, to} => {
                                for i in 0..*num_moves {
                                    let m = moves[i];
                                    match m.get_promotion() {
                                        None | Some(Piece::Queen) if (m.get_source().to_string() == from && m.get_dest().to_string() == to) => {
                                            board = board.make_move(m);
                                        },
                                        _ => (),
                                    };
                                }
                            }
                        };

                        println!("board score (before): {}", board_score(&board, 0));
                        for sq in board.checkers() {
                            println!("checkers: {}", sq.to_string());
                        }
                        for sq in board.pinned() {
                            println!("pinned: {}", sq.to_string());
                        }

                        if board.side_to_move() == Color::Black {
                            if let AlphaBetaResult {cmove: Some(best_move), ..} = find_best_move(board, 6, None, true) {
                                board = board.make_move(best_move);
                            }
                        }

                        println!("board score: {}", board_score(&board, 0));
                        for sq in board.checkers() {
                            println!("checkers: {}", sq.to_string());
                        }
                        for sq in board.pinned() {
                            println!("pinned: {}", sq.to_string());
                        }

                        *num_moves = board.enumerate_moves(moves);
                        let state = compute_ws_state(board);
                        let msg = serde_json::to_string(&state).unwrap();
                        board_cell.set(board);
                        out.send(Message::Text(msg))
                    },
                    Err(_) => {
                        out.close(CloseCode::Error)
                    },
                }
            }
        }
    }
}

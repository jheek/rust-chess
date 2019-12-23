extern crate chess;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate atomic_option;

#[macro_use]
extern crate lazy_static;

extern crate futures;
extern crate tokio;
extern crate tokio_tungstenite;
extern crate tungstenite;

mod client;
mod eval;
mod minmax;
mod ttable;
mod utils;

use std::io::{Error, ErrorKind};

use futures::future::{err, loop_fn, ok, Either, Loop};
use futures::stream::Stream;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tungstenite::protocol::Message;

use tokio_tungstenite::accept_async;

use chess::*;

use client::*;
use eval::*;
use minmax::*;
use ttable::*;
use utils::*;

struct GameState<St, Si> {
    board: Board,
    incoming: St,
    outgoing: Si,
    search: Option<InfiniteSearch>,
}

lazy_static! {
    static ref TTABLE: TTable = { TTable::new(100 * 1024 * 1024) };
}

fn step<St, Si>(
    GameState {
        mut board,
        incoming,
        outgoing,
        search,
    }: GameState<St, Si>,
) -> impl Future<Item = Loop<(), GameState<St, Si>>, Error = String>
where
    St: Stream<Item = Message, Error = String>,
    Si: Sink<SinkItem = Message, SinkError = String>,
{
    let moves = generate_moves(&board);
    let state = compute_ws_state(board, None);
    let msg = serde_json::to_string(&state).unwrap();
    outgoing.send(Message::Text(msg)).and_then(move |outgoing| {
        incoming
            .into_future()
            .map_err(move |(err, _incoming)| err)
            .and_then(move |(ws_msg, incoming)| match ws_msg {
                Some(Message::Text(text_msg)) => match serde_json::from_str(&text_msg) {
                    Ok(msg) => {
                        println!("Received message {:?}", msg);
                        match msg {
                            WSRMessage::Reset => {
                                board = Board::default();
                            }
                            WSRMessage::Move { from, to } => {
                                let option_cmove = moves.iter().cloned().find(|m| {
                                    let from2 = m.get_source().to_string();
                                    let to2 = m.get_dest().to_string();
                                    match m.get_promotion() {
                                        Some(Piece::Queen) | None if from == from2 && to == to2 => {
                                            true
                                        }
                                        _ => false,
                                    }
                                });
                                match option_cmove {
                                    None => return Err("Invalid move".to_string()),
                                    Some(cmove) => {
                                        board = board.make_move(cmove);
                                    }
                                }
                            }
                        }
                        Ok(Loop::Continue(GameState {
                            board,
                            incoming,
                            outgoing,
                            search,
                        }))
                    }
                    Err(_serde_err) => Err("Invalid message".to_string()),
                },
                None => Ok(Loop::Break(())),
                _ => Err("Invalid message".to_string()),
            })
    })
}

fn main() {
    // let start = Instant::now();
    // let board = Board::default();
    // let n = board.perft(6);
    // let elapsed = start.elapsed();
    // println!("Elapsed: {}s {}m, N: {}", elapsed.as_secs(), elapsed.subsec_millis(), n);

    let addr = "127.0.0.1:3012";
    let addr = addr.parse().unwrap();

    // Create the event loop and TCP listener we'll accept connections on.
    let socket = TcpListener::bind(&addr).unwrap();
    println!("Listening on: {}", addr);

    let srv = socket.incoming().for_each(move |socket| {
        let addr = socket
            .peer_addr()
            .expect("connected streams should have a peer address");

        accept_async(socket)
            .and_then(move |ws_stream| {
                let (sink, stream) = ws_stream.split();
                let stream = stream.map_err(|err| err.to_string());
                let sink = sink.sink_map_err(|err| err.to_string());

                let state = GameState {
                    board: Board::default(),
                    incoming: stream,
                    outgoing: sink,
                    search: None,
                };

                let connection_handler = loop_fn(state, step);

                tokio::spawn(connection_handler.then(move |_| {
                    println!("Connection {} closed.", addr);
                    Ok(())
                }));

                Ok(())
            }).map_err(|e| {
                println!("Error during the websocket handshake occurred: {}", e);
                Error::new(ErrorKind::Other, e)
            })
    });

    tokio::runtime::run(srv.map_err(|_e| ()));

    // listen("127.0.0.1:3012", handle_connection)
    //     .unwrap_or_else(|err| panic!("Cannot listen to port 3012: {}", err));
}

// fn handle_connection(out: Sender) -> impl Handler {
//     let board_cell = Cell::new(Board::default());
//     let moves_cell = RefCell::new(([ChessMove::default(); 256], 0));
//     let is_cell: Cell<Option<InfiniteSearch>> = Cell::new(None);
//     move |raw_msg| {
//         match raw_msg {
//             Message::Binary(_) => out.close(CloseCode::Error),
//             Message::Text(text) => {
//                 match serde_json::from_str(&text) {
//                     Ok(msg) => {
//                         if let Some(is) = is_cell.take() {
//                             is.join();
//                         }
//                         let mut board = board_cell.get();
//                         let (ref mut moves, ref mut num_moves) = *moves_cell.borrow_mut();
//                         match msg {
//                             WSRMessage::Reset => {
//                                 board_cell.set(Board::default());
//                             },
//                             WSRMessage::Move {from, to} => {
//                                 for i in 0..*num_moves {
//                                     let m = moves[i];
//                                     match m.get_promotion() {
//                                         None | Some(Piece::Queen) if (m.get_source().to_string() == from && m.get_dest().to_string() == to) => {
//                                             board = board.make_move(m);
//                                         },
//                                         _ => (),
//                                     };
//                                 }
//                             }
//                         };
//                         let sender = out.clone();
//                         let start = Instant::now();
//                         let is = InfiniteSearch::start(&TTABLE, board, 99, move |update| {
//                             let elapsed = start.elapsed();
//                             println!("Received depth: {}, score: {} in {}s {}ms", update.depth, update.score, elapsed.as_secs(), elapsed.subsec_millis());
//                             let state = compute_ws_state(board, Some(update));
//                             let msg = serde_json::to_string(&state).unwrap();
//                             if sender.send(Message::Text(msg)).is_err() {
//                                 sender.close(CloseCode::Error).unwrap();
//                             }
//                         });
//                         is_cell.set(Some(is));
//                         *num_moves = board.enumerate_moves(moves);
//                         board_cell.set(board);
//                         Ok(())
//                     },
//                     Err(_) => {
//                         out.close(CloseCode::Error)
//                     },
//                 }
//             }
//         }
//     }
// }

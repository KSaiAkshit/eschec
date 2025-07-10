#![allow(unused)]
use std::{
    io::BufRead,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, spawn},
};

use tracing::*;

use crate::{
    Board, Side, Square,
    comms::uci_parser::{GoParams, UciCommand, parse_line},
    evaluation::{CompositeEvaluator, Evaluator},
    moves::move_info::{Move, MoveInfo},
    search::{Search, SearchResult},
};

#[derive(Debug)]
pub struct UciState {
    board: Board,
    search_depth: u8,
    evaluator: Arc<dyn Evaluator>,
    search_running: Arc<AtomicBool>,
    best_move: Arc<Mutex<Option<(Square, Square)>>>,
    search_thread: Option<thread::JoinHandle<SearchResult>>,
    move_history: Vec<MoveInfo>,
}

impl Default for UciState {
    fn default() -> Self {
        Self {
            board: Board::default(),
            search_depth: u8::default(),
            evaluator: Arc::new(CompositeEvaluator::default()),
            search_running: Arc::default(),
            best_move: Arc::default(),
            search_thread: None,
            move_history: Vec::default(),
        }
    }
}

impl Drop for UciState {
    fn drop(&mut self) {
        self.search_running.store(false, Ordering::Relaxed);
        if let Some(jh) = self.search_thread.take() {
            jh.join().unwrap();
        }
    }
}

impl UciState {
    pub fn new(depth: Option<u8>) -> Self {
        let depth = depth.unwrap_or(7);
        Self {
            board: Board::new(),
            search_depth: depth,
            evaluator: Arc::new(CompositeEvaluator::balanced()),
            search_running: Arc::new(AtomicBool::new(false)),
            best_move: Arc::new(Mutex::new(None)),
            search_thread: None,
            move_history: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.board = Board::new();
        *self.best_move.lock().unwrap() = None;
    }
}

pub fn play() -> miette::Result<()> {
    let mut state = UciState::new(None);

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    while let Some(Ok(line)) = lines.next() {
        match parse_line(&line) {
            UciCommand::Uci => cmd_uci(),
            UciCommand::IsReady => cmd_isready(),
            UciCommand::UciNewGame => {
                cmd_stop(&mut state);
                state.reset();
            }
            UciCommand::Position {
                startpos,
                fen,
                moves,
            } => {
                cmd_stop(&mut state);
                if let Err(e) = cmd_position(&mut state, startpos, fen, moves) {
                    warn!("Error processing position command: {:?}", e);
                }
            }
            UciCommand::Go(go_params) => {
                cmd_stop(&mut state);
                cmd_go(&mut state, go_params);
            }
            UciCommand::Stop => cmd_stop(&mut state),
            UciCommand::Quit => break,
            UciCommand::Unknown(cmd) => {
                if !cmd.is_empty() {
                    info!("Received unknown command: {cmd}");
                }
            }
        }
    }

    cmd_stop(&mut state);

    Ok(())
}

fn cmd_position(
    state: &mut UciState,
    startpos: bool,
    fen: Option<String>,
    moves: Vec<String>,
) -> miette::Result<()> {
    if startpos {
        state.reset();
    } else if let Some(fen_str) = fen {
        state.board = Board::from_fen(&fen_str);
        state.move_history.clear();
    }

    for move_uci in moves {
        let mov = Move::from_uci(&state.board, &move_uci)?;
        let move_info = state.board.make_move(mov)?;
        state.move_history.push(move_info);
    }

    Ok(())
}

#[instrument(skip(state))]
fn cmd_go(state: &mut UciState, params: GoParams) {
    state.search_running.store(true, Ordering::Relaxed);

    let board = state.board;
    let evaluator = state.evaluator.clone();
    let search_running = state.search_running.clone();
    let default_depth = state.search_depth;

    // --- Time Management Logic ---
    let mut max_time_ms: Option<u64> = None;
    let (time_remaining, increment) = if board.stm == Side::White {
        (params.wtime, params.winc.unwrap_or(0))
    } else {
        (params.btime, params.binc.unwrap_or(0))
    };

    if let Some(time) = time_remaining {
        let allocation;
        if let Some(moves) = params.moves_to_go {
            // Time control with move count: use a fraction of the remaining time.
            allocation = (time / moves).saturating_sub(50); // Subtract 50ms for overhead
        } else {
            // Sudden death: use a smaller fraction of remaining time plus the increment.
            allocation = (time / 30) + increment;
        }
        max_time_ms = Some(allocation.max(50));
    }

    state.search_thread = Some(thread::spawn(move || {
        let mut search = if let Some(time) = max_time_ms {
            Search::with_time_control(default_depth, time)
        } else {
            Search::new(params.depth.unwrap_or(default_depth))
        };

        let result = search.find_best_move(&board, &*evaluator, Some(search_running));

        if let Some(best_move) = result.best_move {
            println!("bestmove {}", best_move.uci());
        } else {
            println!("bestmove 0000");
        }
        result
    }));
}

fn cmd_stop(state: &mut UciState) {
    state.search_running.store(false, Ordering::Relaxed);
    if let Some(handle) = state.search_thread.take() {
        let _ = handle.join();
    }
}

fn cmd_isready() {
    println!("readyok");
}

fn cmd_uci() {
    println!("id name {}", env!("CARGO_PKG_NAME"));
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!();
    println!("option name Debug Log File type string default");
    println!("uciok");
}

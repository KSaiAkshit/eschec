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
    Board, Square,
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
        let depth = depth.unwrap_or(5);
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
        let cmd = line.trim();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "uci" => cmd_uci(),
            "isready" => cmd_isready(),
            "ucinewgame" => {
                cmd_stop(&mut state);
                state.reset();
            }
            "position" => {
                cmd_stop(&mut state);
                if let Err(e) = cmd_position(&mut state, &parts[1..]) {
                    warn!("Error processing position command : {e:?}");
                }
            }
            "go" => {
                cmd_stop(&mut state);
                cmd_go(&mut state, &parts[1..]);
            }
            "stop" => cmd_stop(&mut state),
            "quit" => break,
            _ => {}
        }
    }

    cmd_stop(&mut state);

    Ok(())
}

fn cmd_position(state: &mut UciState, parts: &[&str]) -> miette::Result<()> {
    let mut moves_start_idx: Option<usize> = None;

    if parts.is_empty() {
        miette::bail!("'position' command is missing arguments")
    }

    if parts[0] == "startpos" {
        state.reset();
        moves_start_idx = parts.iter().position(|&s| s == "moves");
    } else if parts[0] == "fen" {
        moves_start_idx = parts.iter().position(|&s| s == "moves");
        let fen_parts = if let Some(idx) = moves_start_idx {
            &parts[1..idx]
        } else {
            &parts[1..]
        };
        let fen_str = fen_parts.join(" ");
        state.board = Board::from_fen(&fen_str);
        state.move_history.clear();
    }

    if let Some(idx) = moves_start_idx {
        let moves = &parts[idx + 1..];
        for move_uci in moves {
            let mov = Move::from_uci(&state.board, move_uci)?;
            let move_info = state.board.make_move(mov)?;
            state.move_history.push(move_info);
        }
    }

    Ok(())
}

fn cmd_go(state: &mut UciState, parts: &[&str]) {
    state.search_running.store(true, Ordering::Relaxed);

    let board = state.board;
    let evaluator = state.evaluator.clone();
    let search_running = state.search_running.clone();
    let depth = state.search_depth;

    state.search_thread = Some(thread::spawn(move || {
        let mut search = Search::new(depth);
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
    println!("uciok");
}

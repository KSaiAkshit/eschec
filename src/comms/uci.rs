#![allow(unused)]
use std::{
    io::BufRead,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, spawn},
};

use crate::{
    Board, Square,
    evaluation::{CompositeEvaluator, Evaluator},
    search::Search,
};

#[derive(Debug)]
pub struct UciState {
    board: Board,
    search_depth: u8,
    evaluator: Arc<dyn Evaluator>,
    search_running: Arc<AtomicBool>,
    best_move: Arc<Mutex<Option<(Square, Square)>>>,
    search_thread: Option<thread::JoinHandle<Search>>,
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
            search_thread: Some(thread::spawn(move || Search::new(depth))),
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
                cmd_position(&mut state, &parts[1..]);
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

fn cmd_position(state: &mut UciState, parts: &[&str]) {
    todo!()
}

fn cmd_go(state: &mut UciState, parts: &[&str]) {
    todo!()
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

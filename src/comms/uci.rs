use std::{
    io::BufRead,
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread,
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
    search_thread: Option<thread::JoinHandle<()>>,
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

impl UciState {
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            search_depth: 5,
            evaluator: Arc::new(CompositeEvaluator::balanced()),
            search_running: Arc::new(AtomicBool::new(false)),
            best_move: Arc::new(Mutex::new(None)),
            search_thread: None,
        }
    }

    fn reset(&mut self) {
        self.board = Board::new();
        *self.best_move.lock().unwrap() = None;
    }
}

pub fn play() -> miette::Result<()> {
    crate::init();

    let mut state = UciState::new();

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
    todo!()
}

fn cmd_isready() {
    todo!()
}

fn cmd_uci() {
    println!("id name Eschec");
    println!("id author Akira");
}

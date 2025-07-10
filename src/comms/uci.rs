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

#[instrument(skip(state))]
fn cmd_go(state: &mut UciState, parts: &[&str]) {
    state.search_running.store(true, Ordering::Relaxed);

    let board = state.board;
    let evaluator = state.evaluator.clone();
    let search_running = state.search_running.clone();

    let mut wtime_ms: Option<u64> = None;
    let mut btime_ms: Option<u64> = None;
    let mut moves_to_go: Option<u64> = None;
    let mut depth = state.search_depth;

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "wtime" => {
                if i + 1 < parts.len() {
                    wtime_ms = parts[i + 1].parse().ok();
                    i += 1;
                }
            }
            "btime" => {
                if i + 1 < parts.len() {
                    btime_ms = parts[i + 1].parse().ok();
                    i += 1;
                }
            }
            "movestogo" => {
                if i + 1 < parts.len() {
                    moves_to_go = parts[i + 1].parse().ok();
                    i += 1;
                }
            }
            "depth" => {
                if i + 1 < parts.len() {
                    depth = parts[i + 1].parse().unwrap_or(state.search_depth);
                    i += 1;
                }
            }
            "infinite" => {
                // For infinite search, there is no time limit.
                // The other time variables will remain None.
            }
            _ => {}
        }
        i += 1;
    }

    // --- NEW: Time Management Logic ---
    let mut max_time_ms: Option<u64> = None;
    let time_remaining = if board.stm == Side::White {
        wtime_ms
    } else {
        btime_ms
    };

    if let Some(time) = time_remaining {
        let allocation;
        if let Some(moves) = moves_to_go {
            // We have a specific number of moves until the next time control.
            // Use a fraction of the remaining time. A safety margin is good.
            // Let's use 95% of the average time per move.
            allocation = (time as f64 * 0.95 / moves as f64) as u64;
        } else {
            // No 'movestogo', so we are in a "sudden death" time control.
            // Use a fixed fraction of the remaining time. 1/30 is a reasonable default.
            allocation = time / 30;
        }
        // A small buffer to ensure we always have some time to think.
        max_time_ms = Some(allocation.max(50)); // e.g., minimum 50ms
    }

    state.search_thread = Some(thread::spawn(move || {
        let mut search = if let Some(time) = max_time_ms {
            // When time is a factor, we often want to search as deep as possible
            // within that time, so we use a high max_depth.
            Search::with_time_control(64, time)
        } else {
            Search::new(depth)
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

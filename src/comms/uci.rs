use std::{
    io::BufRead,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self},
};

use crate::{
    comms::uci_parser::{GoParams, UciCommand, parse_line},
    prelude::*,
    search::{SearchStats, common::SearchConfig},
};

#[derive(Debug)]
pub struct UciState {
    board: Board,
    search_depth: u8,
    evaluator: Arc<dyn Evaluator>,
    search: Arc<Mutex<AlphaBetaSearch>>,
    search_running: Arc<AtomicBool>,
    search_thread: Option<thread::JoinHandle<SearchResult>>,
    move_history: Vec<MoveInfo>,
    search_stats: Arc<Mutex<SearchStats>>,
}

impl Default for UciState {
    fn default() -> Self {
        Self {
            board: Board::default(),
            search_depth: u8::default(),
            search: Arc::default(),
            evaluator: Arc::new(CompositeEvaluator::default()),
            search_running: Arc::default(),
            search_thread: None,
            move_history: Vec::default(),
            search_stats: Arc::new(Mutex::new(SearchStats::default())),
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
    pub fn new(depth: Option<u8>) -> miette::Result<Self> {
        let depth = depth.unwrap_or(20);
        let search_running = Arc::new(AtomicBool::new(false));
        let conf = SearchConfig {
            hash_size_mb: 256,
            ..Default::default()
        };
        let mut s = AlphaBetaSearch::new(Box::new(CompositeEvaluator::balanced()))
            .with_config(conf)?
            .init(search_running.clone());
        s.set_depth(depth);
        let search = Arc::new(Mutex::new(s));
        Ok(Self {
            board: Board::new(),
            search_depth: depth,
            search,
            evaluator: Arc::new(CompositeEvaluator::balanced()),
            search_running,
            search_thread: None,
            move_history: Vec::new(),
            search_stats: Arc::new(Mutex::new(SearchStats::default())),
        })
    }

    fn reset(&mut self) {
        trace!("Resetting UciState");
        self.board = Board::new();
        self.move_history.clear();
        self.search.lock().unwrap().clear();
    }
}

pub fn play() -> miette::Result<()> {
    let mut state = UciState::new(None)?;

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
            UciCommand::SetOption { name, value } => {
                if let Err(e) = cmd_setoption(&name, &value) {
                    warn!("Error setting option: {e:?}");
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
        state.board = Board::new();
        state.move_history.clear();
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

// #[instrument(skip_all)]
fn cmd_go(state: &mut UciState, params: GoParams) {
    let board = state.board;
    let evaluator = state.evaluator.clone();
    let search_running = state.search_running.clone();
    let default_depth = state.search_depth;
    let search = state.search.clone();
    let search_stats = state.search_stats.clone();

    // Time Management Logic
    let mut max_time_ms: Option<u64> = None;
    let (time_remaining, increment) = if board.stm == Side::White {
        (params.wtime, params.winc.unwrap_or(0))
    } else {
        (params.btime, params.binc.unwrap_or(0))
    };

    if let Some(time) = time_remaining {
        // Safety buffer
        let overhead = 50;
        let allocation;

        if let Some(moves_to_go) = params.moves_to_go {
            let divisor = (moves_to_go + 2).clamp(1, 50);
            allocation = (time / divisor).saturating_sub(overhead);
        } else {
            // Sudden death
            let divisor = 20;
            let target = (time / divisor) + (increment / 2);
            let safety_max = (time * 8) / 10;

            allocation = target.min(safety_max).saturating_sub(overhead);
        }
        max_time_ms = Some(allocation.max(10));

        info!(
            "Time Management: Remaning={:?}ms, Inc={:?}ms, Allocating={:?}ms",
            time,
            increment,
            max_time_ms.expect("Just assigned above")
        );
    }

    info!("Spawning thread");
    state.search_thread = Some(thread::spawn(move || {
        let result: SearchResult;
        let stat: SearchStats;
        {
            let mut search = search.lock().unwrap();
            let e = evaluator.clone_box();
            search
                .set_evaluator(e)
                .expect("Should be able to set evaluator");
            if let Some(time) = max_time_ms {
                info!("changing time {:?}", max_time_ms);
                search.set_time(time);
            } else {
                info!("changing depth {:?}", params.depth.unwrap_or(default_depth));
                search.set_depth(params.depth.unwrap_or(default_depth));
            }

            search_running.store(true, Ordering::Relaxed);
            result = search.find_best_move(&board);
            stat = search.get_stats();
            search_running.store(false, Ordering::Relaxed);
        }

        {
            let mut current_stats = search_stats.lock().unwrap();
            *current_stats = *current_stats + stat;
            current_stats.log_summary();
        }

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

fn cmd_setoption(name: &str, value: &str) -> miette::Result<()> {
    match name {
        "LogFile" => {
            let enable = value.to_lowercase() == "true";
            toggle_file_logging(enable)?;
            info!("Set file logging to {enable}");
        }
        _ => {
            info!("Unknown option: {name} = {value}");
        }
    }
    Ok(())
}

fn cmd_isready() {
    println!("readyok");
}

fn cmd_uci() {
    println!("id name {}", env!("CARGO_PKG_NAME"));
    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
    println!();
    println!("option name LogFile type string default");
    println!("uciok");
}

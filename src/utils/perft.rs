use crate::moves::move_info::Move;
use crate::*;
use miette::Context;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Run Stockfish perft divide and return a Vec<(uci_move, count)>
pub fn stockfish_perft_divide(fen: &str, depth: u8) -> miette::Result<Vec<(String, u64)>> {
    let mut child = Command::new("stockfish")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .into_diagnostic()
        .context("Failed to start Stockfish")?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    writeln!(stdin, "uci").unwrap();
    writeln!(stdin, "isready").unwrap();
    writeln!(stdin, "position fen {fen}").unwrap();
    println!("position fen {fen}");
    writeln!(stdin, "go perft {depth}").unwrap();
    println!("go perft {depth}");

    let mut results = Vec::new();
    let mut buf = String::new();
    while reader.read_line(&mut buf).unwrap() > 0 {
        let line = buf.trim();
        if let Some((mv, count)) = line.split_once(':') {
            let mv = mv.trim().to_uppercase().to_string();
            let count = count.trim().parse::<u64>().unwrap_or(0);
            results.push((mv, count));
        }
        if line.starts_with("Nodes searched") || line.starts_with("bestmove") {
            break;
        }
        buf.clear();
    }
    // dbg!(&results);

    Ok(results)
}

#[derive(Debug)]
pub struct PerftResult {
    /// Total nodes counted
    pub nodes: u64,
    /// Time taken
    pub duration: Duration,
    /// Nodes per second
    pub nps: u64,
    /// Move breakdown showing count for each move
    pub move_counts: Option<Vec<(Move, u64)>>,
}

impl PerftResult {
    /// Creates a new PerftResult with the given data
    pub fn new(nodes: u64, duration: Duration, move_counts: Option<Vec<(Move, u64)>>) -> Self {
        let nanos = duration.as_nanos();
        let nps = if nanos > 0 {
            nodes * 1_000_000_000 / nanos as u64
        } else {
            0
        };

        Self {
            nodes,
            duration,
            nps,
            move_counts,
        }
    }
}

pub fn perft_divide_uci(board: &mut Board, depth: u8) -> miette::Result<Vec<(String, u64)>> {
    let result = perft(board, depth, true);
    let r = result
        .move_counts
        .unwrap_or_default()
        .into_iter()
        .map(|(mov, count)| {
            // Convert to UCI string, e.g. "e2e4"
            (mov.uci(), count)
        })
        .collect();
    Ok(r)
}

pub fn debug_perft_vs_stockfish(
    board: &mut Board,
    depth: u8,
    path: Vec<String>,
) -> miette::Result<()> {
    let fen = board.to_fen()?;
    println!("Comparing at depth {depth} path: {path:?}");

    let mut our_moves = perft_divide_uci(board, depth)?;
    let stock_moves = stockfish_perft_divide(&fen, depth)?;
    our_moves.sort_by(|a, b| a.0.cmp(&b.0));
    let mut stockfish_moves = stock_moves.clone();
    stockfish_moves.sort_by(|a, b| a.0.cmp(&b.0));

    let mut found_mismatch = false;

    for ((our_mv, our_count), (sf_mv, sf_count)) in our_moves.iter().zip(stockfish_moves.iter()) {
        if our_mv != sf_mv {
            println!("Move mismatch: our {our_mv} vs sf {sf_mv}");
            found_mismatch = true;
            break;
        }

        if our_count != sf_count {
            println!("Count mismatch on move {our_mv}: our {our_count} vs sf {sf_count}");
            // Decend into this move
            let mov = {
                let from = Square::from_str(&our_mv[0..2])?;
                let to = Square::from_str(&our_mv[2..4])?;
                Move::new(from.index() as u8, to.index() as u8, Move::QUIET)
            };
            let move_data = board.make_move(mov)?;
            let mut new_path = path.clone();
            new_path.push(our_mv.clone());
            debug_perft_vs_stockfish(board, depth - 1, new_path)?;
            board.unmake_move(&move_data)?;
            found_mismatch = true;
            break;
        }
    }

    if !found_mismatch {
        println!("No misatch at this node (path: {path:?})");
    }
    Ok(())
}

pub fn perft(board: &mut Board, depth: u8, divide: bool) -> PerftResult {
    let start_time = Instant::now();

    if depth == 0 {
        return PerftResult::new(1, Duration::from_nanos(1), None);
    }

    let legal_moves = board.generate_legal_moves();

    if depth == 1 {
        return PerftResult::new(
            legal_moves.len() as u64,
            start_time.elapsed(),
            if divide {
                Some(legal_moves.into_iter().map(|m| (m, 1)).collect())
            } else {
                None
            },
        );
    }

    let mut total_nodes = 0;
    let mut move_counts = if divide {
        Some(Vec::with_capacity(legal_moves.len()))
    } else {
        None
    };

    for m in legal_moves {
        let move_data = match board.make_move(m) {
            Ok(data) => data,
            Err(_) => continue,
        };

        let sub_nodes = if depth == 1 {
            1
        } else {
            let mut board_copy = *board;
            perft(&mut board_copy, depth - 1, false).nodes
        };

        board
            .unmake_move(&move_data)
            .wrap_err_with(|| {
                format!(
                    "fucked up at {} to {} at {depth} depth. {move_data:?}",
                    m.from_sq(),
                    m.to_sq()
                )
            })
            .expect("Should be able to unmake move");

        total_nodes += sub_nodes;

        if let Some(ref mut counts) = move_counts {
            counts.push((m, sub_nodes));
        }
    }

    PerftResult::new(total_nodes, start_time.elapsed(), move_counts)
}

/// Performs a Perft test and prints a detailed breakdown
pub fn perft_divide(board: &mut Board, depth: u8) -> PerftResult {
    println!("Starting perft...");
    let result = perft(board, depth, true);

    if let Some(ref move_counts) = result.move_counts {
        println!("Perft results at depth {depth}");
        println!("----------------------------");

        for (mov, count) in move_counts {
            println!("{mov}: {count}");
        }

        println!("----------------------------");
        println!("Total nodes: {}", result.nodes);
        println!("Time: {} ms", result.duration.as_millis());
        println!("Nodes per second: {}", result.nps);
    }

    result
}

/// Runs a suite of perft tests for depths 1 through max_depth
pub fn run_perft_suite(board: &mut Board, max_depth: u8) {
    println!("Running Perft suite up to depth {max_depth}");
    println!("----------------------------");

    for depth in 1..=max_depth {
        let start = Instant::now();
        let nodes = perft(board, depth, false).nodes;
        let duration = start.elapsed();

        let nanos = duration.as_nanos();
        let nps = if nanos > 0 {
            nodes * 1_000_000_000 / nanos as u64
        } else {
            0
        };

        println!(
            "Depth {}: {} nodes in {} ms ({} nps)",
            depth,
            nodes,
            duration.as_millis(),
            nps
        );
    }

    println!("----------------------------");
}

#[cfg(test)]
mod perft_tests {
    use super::*;
    use crate::init;

    /// Known Perft values for the starting position
    const STARTING_PERFT: &[(u8, u64)] = &[
        (1, 20),      // depth 1: 20 nodes
        (2, 400),     // depth 2: 400 nodes
        (3, 8902),    // depth 3: 8,902 nodes
        (4, 197281),  // depth 4: 197,281 nodes
        (5, 4865609), // depth 5: 4,865,609 nodes
    ];

    /// Known Perft values for position 2
    const KIWIPETE_PERFT: &[(u8, u64)] = &[
        (1, 48),      // depth 1: 48 nodes
        (2, 2039),    // depth 2: 2,039 nodes
        (3, 97862),   // depth 3: 97,862 nodes
        (4, 4085603), // depth 4: 4,085,603 nodes
    ];

    #[test]
    fn test_perft_starting_position() {
        init();
        let mut board = Board::new();

        for &(depth, expected) in STARTING_PERFT.iter().take(4) {
            // limit to depth 4 for time
            let result = perft(&mut board, depth, false);
            assert_eq!(
                result.nodes, expected,
                "Perft failed at depth {}: got {} expected {}",
                depth, result.nodes, expected
            );
        }
    }

    #[test]
    fn test_perft_kiwipete() {
        init();
        // This is the "Kiwipete" position, a common test position
        let mut board = Board::from_fen(KIWIPETE);
        println!("{board}");

        for &(depth, expected) in KIWIPETE_PERFT.iter().take(4) {
            let result = perft(&mut board, depth, false);
            assert_eq!(
                result.nodes, expected,
                "Perft failed at depth {}: got {} expected {}",
                depth, result.nodes, expected
            );
        }
    }

    #[test]
    fn test_perft_position3() {
        init();
        // Position 3 from CPW
        let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");

        let expected_results = [(1, 14), (2, 191), (3, 2812), (4, 43238)];

        for &(depth, expected) in &expected_results {
            let result = perft(&mut board, depth, false);
            assert_eq!(
                result.nodes, expected,
                "Perft failed at depth {}: got {} expected {}",
                depth, result.nodes, expected
            );
        }
    }

    #[test]
    fn test_perft_position4() {
        init();
        // Position 4 from CPW (en passant capture test)
        let mut board =
            Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");

        let expected_results = [(1, 6), (2, 264), (3, 9467)];

        for &(depth, expected) in &expected_results {
            let result = perft(&mut board, depth, false);
            assert_eq!(
                result.nodes, expected,
                "Perft failed at depth {}: got {} expected {}",
                depth, result.nodes, expected
            );
        }
    }

    #[test]
    fn test_make_unmake_consistency() {
        init();
        let mut board = Board::new();
        let original_board = board;

        let legal_moves = board.generate_legal_moves();
        if !legal_moves.is_empty() {
            for mov in legal_moves {
                // Make the move
                let move_data = board.make_move(mov).unwrap();

                // The board should now be different
                assert_ne!(
                    board, original_board,
                    "Board should change after making a move"
                );

                // Unmake the move
                board.unmake_move(&move_data).unwrap();

                // The board should now be identical to the original
                assert_eq!(
                    board, original_board,
                    "Board should be the same after unmaking a move"
                );
            }
        }
    }
}

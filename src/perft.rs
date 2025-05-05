use crate::*;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct PerftResult {
    /// Total nodes counted
    pub nodes: u64,
    /// Time taken
    pub duration: Duration,
    /// Nodes per second
    pub nps: u64,
    /// Move breakdown showing count for each move
    pub move_counts: Option<Vec<(Square, Square, u64)>>,
}

impl PerftResult {
    /// Creates a new PerftResult with the given data
    pub fn new(
        nodes: u64,
        duration: Duration,
        move_counts: Option<Vec<(Square, Square, u64)>>,
    ) -> Self {
        let nps = if duration.as_secs() > 0 {
            nodes / duration.as_secs()
        } else {
            nodes * 1_000_000 / duration.as_micros() as u64
        };

        Self {
            nodes,
            duration,
            nps,
            move_counts,
        }
    }
}

pub fn perft(board: &mut Board, depth: u8, divide: bool) -> PerftResult {
    let start_time = Instant::now();

    if depth == 0 {
        return PerftResult::new(1, Duration::from_nanos(1), None);
    }

    let legal_moves = match board.generate_legal_moves() {
        Ok(moves) => moves,
        Err(_) => return PerftResult::new(0, start_time.elapsed(), None),
    };

    if depth == 1 {
        return PerftResult::new(
            legal_moves.len() as u64,
            start_time.elapsed(),
            if divide {
                Some(
                    legal_moves
                        .into_iter()
                        .map(|(from, to)| (from, to, 1))
                        .collect(),
                )
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

    for (from, to) in legal_moves {
        let move_data = match board.try_move_with_info(from, to) {
            Ok(data) => data,
            Err(_) => continue,
        };

        if from.index() == 8 && to.index() == 24 {
            println!("move_made successfully");
            println!("{board}");
        }

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
                    "fucked up at {from} to {to} at {depth} depth. {:?}",
                    move_data
                )
            })
            .expect("Should be able to unmake move");

        total_nodes += sub_nodes;

        if let Some(ref mut counts) = move_counts {
            counts.push((from, to, sub_nodes));
        }
    }

    PerftResult::new(total_nodes, start_time.elapsed(), move_counts)
}

/// Performs a Perft test and prints a detailed breakdown
pub fn perft_divide(board: &mut Board, depth: u8) -> PerftResult {
    let result = perft(board, depth, true);

    if let Some(ref move_counts) = result.move_counts {
        println!("Perft results at depth {}", depth);
        println!("----------------------------");

        for (from, to, count) in move_counts {
            println!("{}{}: {}", from, to, count);
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
    println!("Running Perft suite up to depth {}", max_depth);
    println!("----------------------------");

    for depth in 1..=max_depth {
        let start = Instant::now();
        let nodes = perft(board, depth, false).nodes;
        let duration = start.elapsed();

        let nps = if duration.as_secs() > 0 {
            nodes / duration.as_secs()
        } else if duration.as_millis() > 0 {
            nodes * 1000 / duration.as_millis() as u64
        } else {
            nodes * 1_000_000 / duration.as_micros() as u64
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
        let mut board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        println!("{board}");

        for &(depth, expected) in KIWIPETE_PERFT.iter().take(3) {
            // limit to depth 3 for time
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

        // Generate moves and test make/unmake for each one
        if let Ok(legal_moves) = board.generate_legal_moves() {
            for (from, to) in legal_moves {
                // Make the move
                let move_data = board.try_move_with_info(from, to).unwrap();

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

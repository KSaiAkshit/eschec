use clap::Parser;
use eschec::prelude::*; // Your engine's prelude
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;

/// A scored EPD test runner for the Eschec engine using STS conventions.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to the EPD file or a directory containing EPD files.
    #[arg(required = true)]
    path: PathBuf,

    /// Search depth for the engine.
    #[arg(short, long, default_value_t = 5)]
    depth: u8,
    // TODO: Implement TimeControl
    // Placeholder for future time control
    // #[arg(long, default_value_t = 5000)]
    // time_ms: u64,
}

/// Represents a single parsed EPD test case with scoring.
struct EPDTest {
    fen: String,
    id: String,
    move_scores: HashMap<String, i32>,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = Cli::parse();

    let epd_files = find_epd_files(&cli.path)?;

    let mut grand_total_score = 0;
    let mut grand_max_score = 0;

    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::new(Box::new(evaluator), cli.depth);

    for file_path in epd_files {
        println!("Running test suite: {}", file_path.display());
        println!("{:-<50}", "");

        let tests = parse_epd_file(&file_path)?;
        let mut suite_score = 0;
        let mut suite_max_score = 0;
        let start_time = Instant::now();

        for test in &tests {
            let board = Board::from_fen(&test.fen);

            let result = search.find_best_move(&board);
            let engine_move = result.best_move.map(|m| m.uci()).unwrap_or_default();

            let score = *test.move_scores.get(&engine_move).unwrap_or(&0);
            suite_score += score;

            let max_score_for_pos = *test.move_scores.values().max().unwrap_or(&0);
            suite_max_score += max_score_for_pos;

            if score < max_score_for_pos {
                println!(
                    "[FAIL] ID: {:<15} | Score: {}/{} | Got: {:<6} | FEN: {}",
                    test.id, score, max_score_for_pos, engine_move, test.fen
                );
            }
        }

        let duration = start_time.elapsed();
        let percentage = if suite_max_score > 0 {
            (suite_score as f64 / suite_max_score as f64) * 100.0
        } else {
            0.0
        };

        println!("{:-<50}", "");
        println!(
            "Suite Summary: Score {}/{} ({:.2}%)",
            suite_score, suite_max_score, percentage
        );
        println!("Time taken: {:.2?}", duration);
        println!();

        grand_total_score += suite_score;
        grand_max_score += suite_max_score;
    }

    if grand_max_score > 0 {
        let grand_percentage = (grand_total_score as f64 / grand_max_score as f64) * 100.0;
        println!("==================================================");
        println!(
            "Overall Result: {}/{} ({:.2}%)",
            grand_total_score, grand_max_score, grand_percentage
        );
        println!("==================================================");
    }

    Ok(())
}

/// Finds all .epd files in a given path (file or directory).
fn find_epd_files(path: &PathBuf) -> miette::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path).into_diagnostic()?.flatten() {
            if entry.path().extension().is_some_and(|e| e == "epd") {
                files.push(entry.path());
            }
        }
    } else {
        files.push(path.clone());
    }
    Ok(files)
}

/// Parses an EPD file into a vector of test cases, including 'c0' scores.
/// Very ugly function
fn parse_epd_file(path: &PathBuf) -> miette::Result<Vec<EPDTest>> {
    let file = File::open(path).into_diagnostic()?;
    let reader = BufReader::new(file);
    let mut tests = Vec::new();

    for line in reader.lines() {
        let line = line.into_diagnostic()?;
        if line.trim().is_empty() {
            continue;
        }

        let mut fen_parts: Vec<&str> = line.split_whitespace().take(4).collect();
        fen_parts.push("0");
        fen_parts.push("1");
        let fen = fen_parts.join(" ");

        // Create a temporary board to resolve SAN moves to UCI
        let board = Board::from_fen(&fen);

        let mut move_scores = HashMap::new();
        let mut id = String::from("unknown");

        if let Some(opcode_str) = line.splitn(5, ' ').nth(4) {
            if let Some(id_part) = opcode_str.split("id \"").nth(1) {
                id = id_part.split('"').next().unwrap_or("").to_string();
            }
            if let Some(c0_part) = opcode_str.split("c0 \"").nth(1) {
                let scores_str = c0_part.split('"').next().unwrap_or("");
                for score_pair in scores_str.split(',') {
                    let pair: Vec<&str> = score_pair.trim().split('=').collect();
                    if pair.len() == 2 {
                        let san_move = pair[0];
                        if let Ok(score) = pair[1].parse::<i32>() {
                            match Move::from_san(&board, san_move) {
                                Ok(mov) => {
                                    move_scores.insert(mov.uci(), score);
                                }
                                Err(e) => {
                                    warn!(
                                        "Could not parse SAN move '{}' for FEN '{}': {:?}",
                                        san_move, fen, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        tests.push(EPDTest {
            fen,
            id,
            move_scores,
        });
    }

    Ok(tests)
}

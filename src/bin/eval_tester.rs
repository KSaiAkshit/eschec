use clap::Parser;
use eschec::prelude::*;
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::{BufRead, BufReader},
    path::PathBuf,
    time::Instant,
};

/// A scored EPD test runner for Eschec engine using STS
#[derive(Parser, Debug)]
#[command(version, about)]
struct EvalCli {
    /// Path to EPD file or a directory containing EPD files
    #[arg(required = true)]
    path: PathBuf,

    /// Search depth for the engine
    #[arg(short, long, default_value_t = 5)]
    depth: u8,
}

#[derive(Debug)]
struct EPDTest {
    fen: String,
    id: String,
    /// The single "best move" (bm) in UCI format
    best_move: Option<String>,
    /// A map of moves (UCI format) to their point values from the 'c0' opcode
    move_scores: HashMap<String, i32>,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = EvalCli::parse();

    let epd_files = find_epd_files(&cli.path)?;
    if epd_files.is_empty() {
        warn!("No .epd files found in path: {}", cli.path.display());
        return Ok(());
    }

    let mut grand_total_score = 0;
    let mut grand_max_score = 0;
    let mut grand_total_tests = 0;
    let mut grand_bm_correct = 0;

    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::new(Box::new(evaluator), cli.depth);
    search.set_emit_info(false); // Don't need UCI info during tests

    for file_path in epd_files {
        println!("Running test suite: {}", file_path.display());
        println!("{:-<60}", "");

        let tests = parse_epd_files(&file_path)?;
        let mut suite_score = 0;
        let mut suite_max_score = 0;
        let mut suite_bm_correct = 0;
        let start_time = Instant::now();

        for (i, test) in tests.iter().enumerate() {
            let _ = i;
            let board = Board::from_fen(&test.fen);
            let result = search.find_best_move(&board);
            let engine_move_uci = result.best_move.map(|m| m.uci()).unwrap_or_default();

            // Scoring based on 'c0' opcode
            let score = *test.move_scores.get(&engine_move_uci).unwrap_or(&0);
            suite_score += score;

            let max_score_for_pos = *test.move_scores.values().max().unwrap_or(&1); // Avoid div by zero
            suite_max_score += max_score_for_pos;

            // Check against 'bm' opcode
            let bm_correct = test
                .best_move
                .as_ref()
                .is_some_and(|bm| *bm == engine_move_uci);
            if bm_correct {
                suite_bm_correct += 1;
            }

            // Print failures
            if score < max_score_for_pos {
                println!(
                    "[FAIL] ID: {:<15} | Score: {}/{} | BM Correct: {} | Got: {:<6} | FEN: {}",
                    test.id, score, max_score_for_pos, bm_correct, engine_move_uci, test.fen
                );
            }
        }

        let duration = start_time.elapsed();
        let percentage = if suite_max_score > 0 {
            (suite_score as f64 / suite_max_score as f64) * 100.0
        } else {
            0.0
        };
        let bm_percentage = if !tests.is_empty() {
            (suite_bm_correct as f64 / tests.len() as f64) * 100.0
        } else {
            0.0
        };

        println!("{:-<60}", "");
        println!("Suite Summary for: {}", file_path.display());
        println!(
            "STS Score: {}/{} ({:.2}%)",
            suite_score, suite_max_score, percentage
        );
        println!(
            "Best Move (bm) Accuracy: {}/{} ({:.2}%)",
            suite_bm_correct,
            tests.len(),
            bm_percentage
        );
        println!("Time taken: {:.2?}", duration);
        println!();

        grand_total_score += suite_score;
        grand_max_score += suite_max_score;
        grand_total_tests += tests.len();
        grand_bm_correct += suite_bm_correct;
    }

    if grand_max_score > 0 {
        let grand_percentage = (grand_total_score as f64 / grand_max_score as f64) * 100.0;
        let grand_bm_percentage = (grand_bm_correct as f64 / grand_total_tests as f64) * 100.0;
        println!("==============================================================");
        println!("Overall Results");
        println!(
            "Total STS Score: {}/{} ({:.2}%)",
            grand_total_score, grand_max_score, grand_percentage
        );
        println!(
            "Total Best Move (bm) Accuracy: {}/{} ({:.2}%)",
            grand_bm_correct, grand_total_tests, grand_bm_percentage
        );
        println!("==============================================================");
    }

    Ok(())
}

/// Find all .epd files in a given path (file or dir)
fn find_epd_files(path: &PathBuf) -> miette::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_dir() {
        for entry in read_dir(path).into_diagnostic()?.flatten() {
            let entry_path = entry.path();
            if entry_path.extension().is_some_and(|e| e == "epd") {
                files.push(entry_path);
            }
        }
    } else if path.is_file() {
        files.push(path.clone());
    }

    Ok(files)
}

fn parse_epd_files(path: &PathBuf) -> miette::Result<Vec<EPDTest>> {
    let file = File::open(path).into_diagnostic()?;
    let reader = BufReader::new(file);
    let mut tests = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.into_diagnostic()?;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_epd_line(&line) {
            Ok(test) => tests.push(test),
            Err(e) => warn!(
                "Skipping Invalid EPD line {} in {}: {:?}",
                line_num + 1,
                path.display(),
                e
            ),
        }
    }

    Ok(tests)
}

/// Parses a single line of an EPD file.
fn parse_epd_line(line: &str) -> miette::Result<EPDTest> {
    // Find where the FEN part ends and opcodes begin.
    let first_opcode_idx = line.find("id ").or(line.find("bm ")).or(line.find("c0 "));
    let (fen_part, opcode_part) = if let Some(idx) = first_opcode_idx {
        line.split_at(idx)
    } else {
        (line, "") // Line might just be a FEN string
    };

    // EPD FENs are often incomplete, so we add the missing parts.
    let fen = fen_part.trim().to_string();
    println!("FEN: {}", fen);
    let board = Board::from_fen(&fen);

    let mut id = String::from("unknown");
    let mut best_move = None;
    let mut move_scores = HashMap::new();

    // Process each opcode, separated by semicolons.
    for part in opcode_part.split(';').map(str::trim) {
        if part.is_empty() {
            continue;
        }

        let (opcode, value) = part.split_at(part.find(' ').unwrap_or(part.len()));
        let value = value.trim();

        match opcode {
            "id" => id = parse_quoted_string(value).unwrap_or_default(),
            "bm" => {
                if let Ok(mov) = Move::from_san(&board, value) {
                    best_move = Some(mov.uci());
                } else {
                    warn!("Failed to parse 'bm' move '{}' for FEN: {}", value, fen);
                }
            }
            "c0" => {
                if let Some(scores_str) = parse_quoted_string(value) {
                    for score_pair in scores_str.split(',') {
                        let pair: Vec<&str> = score_pair.trim().split('=').collect();
                        if pair.len() == 2 {
                            let san_move = pair[0];
                            if let (Ok(mov), Ok(score)) =
                                (Move::from_san(&board, san_move), pair[1].parse::<i32>())
                            {
                                move_scores.insert(mov.uci(), score);
                            } else {
                                warn!(
                                    "Failed to parse 'c0' pair '{}' for FEN: {}",
                                    score_pair, fen
                                );
                            }
                        }
                    }
                }
            }
            _ => {} // Ignore other opcodes like 'am', 'dm', etc.
        }
    }

    Ok(EPDTest {
        fen,
        id,
        best_move,
        move_scores,
    })
}

/// Helper to extract the content from a quoted string like `"this content"`.
fn parse_quoted_string(s: &str) -> Option<String> {
    s.strip_prefix('"')?.strip_suffix('"').map(str::to_owned)
}

use clap::{Parser, ValueEnum};
use eschec::prelude::*;
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

// Use conditional compilation (cfg) to only include these when the 'parallel' feature is enabled
#[cfg(feature = "parallel")]
use indicatif::{ProgressBar, ProgressStyle};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

const EXPECTED_MOVES_WIDTH: usize = 24;

/// A scored EPD test runner for Eschec engine using STS
#[derive(Parser, Debug)]
#[command(version, about)]
struct EvalCli {
    /// Path to EPD file or a directory containing EPD files
    #[arg(required = true)]
    path: PathBuf,

    /// Set the time control for the test
    #[arg(short, long, value_enum, default_value_t = TimeControl::Short)]
    time_control: TimeControl,

    /// Optional fixed search depth (overrides time control if set).
    #[arg(short, long)]
    depth: Option<u8>,

    // This argument will only be available when the "parallel" feature is active
    #[cfg(feature = "parallel")]
    #[arg(
        long,
        default_value_t = 1,
        help = "Number of threads to use (0 for auto) [requires 'parallel' feature]"
    )]
    threads: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TimeControl {
    /// Short Time Control (10 seconds per move)
    #[clap(name = "stc")]
    Short,
    /// Intermediate Time Control (1 minute per move)
    #[clap(name = "itc")]
    Intermediate,
    /// Long Time Control (7 minutes per move)
    #[clap(name = "ltc")]
    Long,
}

impl Display for TimeControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time_str = match self {
            TimeControl::Short => "10 sec",
            TimeControl::Intermediate => "1 min",
            TimeControl::Long => "7 min",
        };
        write!(f, "{}", time_str)
    }
}

impl TimeControl {
    fn to_ms(self) -> u64 {
        match self {
            TimeControl::Short => 10 * 1000,
            TimeControl::Intermediate => 60 * 1000,
            TimeControl::Long => 7 * 60 * 1000,
        }
    }
}

#[derive(Debug)]
struct EPDTest {
    fen: String,
    theme: String,
    id: String,
    best_move: Option<String>,
    move_scores: HashMap<String, i32>,
}

#[derive(Default, Clone)]
struct SuiteResultSummary {
    name: String,
    score: i32,
    max_score: i32,
    bm_correct: u64,
    num_tests: usize,
}

#[derive(Default, Clone)]
struct TestResult {
    score: i32,
    max_score: i32,
    bm_correct: u32,
    theme: String,
    log_message: Option<String>,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = EvalCli::parse();

    // The core logic is now split into two functions.
    // We call the appropriate one based on whether the "parallel" feature is enabled.
    #[cfg(feature = "parallel")]
    {
        // Configure Rayon Thread Pool
        let num_threads: usize = if cli.threads == 0 {
            std::thread::available_parallelism()
                .into_diagnostic()?
                .into()
        } else {
            cli.threads
        };
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .into_diagnostic()?;
        info!("Running in parallel with {} threads", num_threads);
        run_tests_parallel(cli)
    }
    #[cfg(not(feature = "parallel"))]
    {
        info!(
            "Running in single-threaded mode. Compile with '--features parallel' for multithreading."
        );
        run_tests_sequential(cli)
    }
}

// #[cfg(not(feature = "parallel"))]
fn run_tests_sequential(cli: EvalCli) -> miette::Result<()> {
    let epd_files = find_epd_files(&cli.path)?;
    miette::ensure!(
        !epd_files.is_empty(),
        "No .epd files found in path: {}",
        cli.path.display()
    );

    let mut all_suite_results: HashMap<String, SuiteResultSummary> = HashMap::new();

    for file_path in epd_files {
        println!("Running test suite: {}", file_path.display());
        println!("{:-<160}", "");
        let tests = parse_epd_files(&file_path)?;
        let mut suite_score = 0;
        let mut suite_max_score = 0;
        let mut suite_bm_correct = 0;
        let start_time = Instant::now();

        println!(
            "{:<6} | {:<18} | {:<8} | {:<width$} | {:<8}",
            "Status",
            "ID",
            "Score",
            "Expected Moves",
            "Got",
            width = EXPECTED_MOVES_WIDTH
        );
        println!("{:-<160}", "");

        let mut results_by_theme: HashMap<String, Vec<TestResult>> = HashMap::new();

        for test in &tests {
            let result = run_single_test(test, &cli);
            suite_score += result.score;
            suite_max_score += result.max_score;
            suite_bm_correct += result.bm_correct as u64;

            results_by_theme
                .entry(result.theme.clone())
                .or_default()
                .push(result.clone());

            if let Some(log_msg) = result.log_message {
                println!("{}", log_msg);
            }
        }

        print_suite_summary(
            &file_path,
            suite_score,
            suite_max_score.max(1),
            suite_bm_correct,
            tests.len(),
            &cli,
            start_time.elapsed(),
        );

        for (theme, theme_results) in results_by_theme {
            let suite_summary = all_suite_results.entry(theme).or_default();
            suite_summary.score += theme_results.iter().map(|r| r.score).sum::<i32>();
            suite_summary.max_score += theme_results.iter().map(|r| r.max_score).sum::<i32>();
            suite_summary.bm_correct += theme_results
                .iter()
                .map(|r| r.bm_correct as u64)
                .sum::<u64>();
            suite_summary.num_tests += theme_results.len();
        }
    }

    let mut final_summaries: Vec<SuiteResultSummary> = all_suite_results
        .into_iter()
        .map(|(theme, mut summary)| {
            summary.name = theme;
            summary
        })
        .collect();
    final_summaries.sort_by(|a, b| a.name.cmp(&b.name));

    print_thematic_summary(&final_summaries);
    Ok(())
}

#[cfg(feature = "parallel")]
fn run_tests_parallel(cli: EvalCli) -> miette::Result<()> {
    let epd_files = find_epd_files(&cli.path)?;
    miette::ensure!(
        !epd_files.is_empty(),
        "No .epd files found in path: {}",
        cli.path.display()
    );

    let mut all_suite_results: HashMap<String, SuiteResultSummary> = HashMap::new();

    for file_path in epd_files {
        println!("Running test suite: {}", file_path.display());
        let tests = parse_epd_files(&file_path)?;
        let num_tests = tests.len();

        let pb = ProgressBar::new(num_tests as u64);
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .into_diagnostic()?
                .progress_chars("#>-"),
        );

        let test_results: Vec<TestResult> = tests
            .par_iter()
            .map(|test| {
                let result = run_single_test(test, &cli);
                pb.inc(1);
                result
            })
            .collect();

        pb.finish_with_message("Done");

        let mut passes = Vec::new();
        let mut failures = Vec::new();
        let mut results_by_theme: HashMap<String, Vec<TestResult>> = HashMap::new();

        for result in test_results {
            results_by_theme
                .entry(result.theme.clone())
                .or_default()
                .push(result.clone());

            if let Some(log) = result.log_message {
                if log.contains("FAIL") {
                    failures.push(log);
                } else {
                    passes.push(log);
                }
            }
        }

        println!("{:-<160}", "");
        passes.sort();
        for pass in &passes {
            println!("{}", pass);
        }

        if !passes.is_empty() {
            println!();
        }

        if failures.is_empty() {
            println!("All tests passed!");
        } else {
            println!("Failures:");
            failures.sort();
            for fail in failures {
                println!("{}", fail);
            }
        }

        for (theme, theme_results) in results_by_theme {
            let suite_summary = all_suite_results.entry(theme).or_default();
            suite_summary.score += theme_results.iter().map(|r| r.score).sum::<i32>();
            suite_summary.max_score += theme_results.iter().map(|r| r.max_score).sum::<i32>();
            suite_summary.bm_correct += theme_results
                .iter()
                .map(|r| r.bm_correct as u64)
                .sum::<u64>();
            suite_summary.num_tests += theme_results.len();
        }
    }

    // After processing all files, create the final list of summaries
    let mut final_summaries: Vec<SuiteResultSummary> = all_suite_results
        .into_iter()
        .map(|(theme, mut summary)| {
            summary.name = theme;
            summary
        })
        .collect();

    final_summaries.sort_by(|a, b| a.name.cmp(&b.name));

    print_thematic_summary(&final_summaries);
    Ok(())
}

/// Helper function to run a single EPD test, callable from both modes.
fn run_single_test(test: &EPDTest, cli: &EvalCli) -> TestResult {
    let evaluator = CompositeEvaluator::balanced();
    let mut search = if let Some(d) = cli.depth {
        Search::new(Box::new(evaluator), d)
    } else {
        Search::with_time_control(Box::new(evaluator), MAX_PLY as u8, cli.time_control.to_ms())
    };
    search.set_emit_info(false);

    let board = Board::from_fen(&test.fen);
    let result = search.find_best_move(&board);
    let engine_move_uci = result.best_move.map(|m| m.uci()).unwrap_or_default();

    let score = *test.move_scores.get(&engine_move_uci).unwrap_or(&0);
    let max_score_for_pos = *test.move_scores.values().max().unwrap_or(&0);
    let bm_correct = test
        .best_move
        .as_ref()
        .is_some_and(|bm| *bm == engine_move_uci);

    let expected_moves_str =
        parse_expected_moves(&test.move_scores).unwrap_or_else(|| "...".to_string());

    let log_message = if score < max_score_for_pos {
        Some(format!(
            "[{}{:<4}{}] ID: {:<15} | S: {:>2}/{} | Ex: {:<width$} | Got: {:<6} | FEN: {}",
            RED,
            "FAIL",
            RESET,
            test.id,
            score,
            max_score_for_pos,
            expected_moves_str,
            engine_move_uci,
            test.fen,
            width = EXPECTED_MOVES_WIDTH
        ))
    } else {
        Some(format!(
            "[{}{:<4}{}] ID: {:<15} | S: {:>2}/{} | Ex: {:<width$} | Got: {:<6}",
            GREEN,
            "PASS",
            RESET,
            test.id,
            score,
            max_score_for_pos,
            expected_moves_str,
            engine_move_uci,
            width = EXPECTED_MOVES_WIDTH
        ))
    };

    TestResult {
        theme: test.theme.clone(),
        score,
        max_score: max_score_for_pos,
        bm_correct: bm_correct as u32,
        log_message,
    }
}

fn print_suite_summary(
    file_path: &Path,
    suite_score: i32,
    suite_max_score: i32,
    suite_bm_correct: u64,
    num_tests: usize,
    cli: &EvalCli,
    duration: Duration,
) {
    let percentage = (suite_score as f64 / suite_max_score as f64) * 100.0;
    let bm_percentage = (suite_bm_correct as f64 / num_tests as f64) * 100.0;

    println!("{:-<60}", "");
    println!("Suite Summary for: {}", file_path.display());
    println!(
        "STS Score: {}/{} ({:.2}%)",
        suite_score, suite_max_score, percentage
    );
    println!(
        "Best Move (bm) Accuracy: {}/{} ({:.2}%)",
        suite_bm_correct, num_tests, bm_percentage
    );
    if let Some(d) = cli.depth {
        println!("Search depth: {d}");
    } else {
        println!("Time control: {}", cli.time_control);
    }
    println!("Time taken: {:.2?}", duration);
    println!();
}

fn print_thematic_summary(results: &[SuiteResultSummary]) {
    if results.is_empty() {
        return;
    }

    let grand_total_score: i32 = results.iter().map(|r| r.score).sum();
    let grand_max_score: i32 = results.iter().map(|r| r.max_score).sum();
    let grand_bm_correct: u64 = results.iter().map(|r| r.bm_correct).sum();
    let grand_total_tests: usize = results.iter().map(|r| r.num_tests).sum();

    println!("===================== Thematic Summary =====================");
    println!(
        "| {:<20} | {:<15} | {:<18} |",
        "Theme", "STS Score", "Best Move Accuracy"
    );
    println!("|----------------------|-----------------|--------------------|");

    for result in results {
        let percentage = (result.score as f64 / result.max_score as f64) * 100.0;
        let bm_percentage = (result.bm_correct as f64 / result.num_tests as f64) * 100.0;
        println!(
            "| {:<20} | {:>7}/{:<7} | {:>7}/{:<10} |",
            result.name,
            format!("{} ({:.1}%)", result.score, percentage),
            result.max_score,
            format!("{} ({:.1}%)", result.bm_correct, bm_percentage),
            result.num_tests
        );
    }
    println!("|----------------------|-----------------|--------------------|");

    print_grand_total_summary(
        grand_total_score,
        grand_max_score,
        grand_bm_correct,
        grand_total_tests,
    );
}

fn print_grand_total_summary(
    grand_total_score: i32,
    grand_max_score: i32,
    grand_bm_correct: u64,
    grand_total_tests: usize,
) {
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
}

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

fn parse_epd_line(line: &str) -> miette::Result<EPDTest> {
    const OPCODES: [&str; 4] = [" bm ", " am ", " dm ", " c0 "];
    let first_opcode_idx = OPCODES.iter().filter_map(|op| line.find(op)).min();
    let (fen_part, opcode_part) = if let Some(idx) = first_opcode_idx {
        line.split_at(idx)
    } else {
        (line, "")
    };

    let fen_part_trimmed = fen_part.trim();
    let mut fen_fields: Vec<&str> = fen_part_trimmed.split_whitespace().collect();
    if fen_fields.len() < 6 {
        if fen_fields.len() < 2 {
            fen_fields.push("w");
        }
        if fen_fields.len() < 3 {
            fen_fields.push("-");
        }
        if fen_fields.len() < 4 {
            fen_fields.push("-");
        }
        if fen_fields.len() < 5 {
            fen_fields.push("0");
        }
        if fen_fields.len() < 6 {
            fen_fields.push("1");
        }
    }
    let fen = fen_fields.join(" ");
    let board = Board::from_fen(&fen);

    let mut id = String::from("unknown");
    let mut best_move = None;
    let mut move_scores = HashMap::new();

    for part in opcode_part.split(';').map(str::trim) {
        if part.is_empty() {
            continue;
        }
        if let Some((opcode, value)) = part.split_once(' ') {
            let value = value.trim();
            match opcode {
                "id" => id = parse_quoted_string(value).unwrap_or_default(),
                "bm" => {
                    for san_move in value.split_whitespace() {
                        if let Ok(mov) = Move::from_san(&board, san_move) {
                            best_move = Some(mov.uci());
                            break; // EPD spec says only first bm is official
                        } else {
                            warn!("Failed to parse 'bm' move '{}' for FEN: {}", value, fen);
                        }
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
                _ => {} // Ignore other opcodes
            }
        }
    }

    Ok(EPDTest {
        fen,
        id,
        best_move,
        move_scores,
    })
}

fn parse_expected_moves(s: &HashMap<String, i32>) -> Option<String> {
    let mut expected_moves: Vec<_> = s.keys().map(|m| m.as_str()).collect();
    expected_moves.sort(); // Sort for consistent output
    let mut expected_moves_str = expected_moves.join(" ");

    if expected_moves_str.len() > EXPECTED_MOVES_WIDTH {
        expected_moves_str.truncate(EXPECTED_MOVES_WIDTH - 3);
        expected_moves_str.push_str("...");
    }
    Some(expected_moves_str)
}

fn parse_quoted_string(s: &str) -> Option<String> {
    s.strip_prefix('"')?.strip_suffix('"').map(str::to_owned)
}

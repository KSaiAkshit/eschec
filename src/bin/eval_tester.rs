use clap::{Parser, ValueEnum};
use eschec::{
    prelude::*,
    tuning::params::TunableParams,
    utils::sts_runner::{self, SuiteSummary, TestResult},
};
use indicatif::{ProgressBar, ProgressStyle};
use std::{collections::HashMap, fmt::Display, path::PathBuf, time::Duration};

const EXPECTED_MOVES_WIDTH: usize = 24;
const MAX_ID_LEN: usize = 24;

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
    depth: Option<u16>,

    // This argument will only be available when the "parallel" feature is active
    #[cfg(feature = "parallel")]
    #[arg(
        long,
        default_value_t = 0, // Default to auto-detecting cores
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

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = EvalCli::parse();

    // Default to seq.
    let num_threads: usize;

    #[cfg(feature = "parallel")]
    {
        num_threads = if cli.threads == 0 {
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
        info!(
            "Running in parallel with {} threads with time-control: {}",
            num_threads, cli.time_control
        );
    }

    #[cfg(not(feature = "parallel"))]
    {
        num_threads = 1;
    }

    let all_tests = sts_runner::load_epd_files_from_path(&cli.path)?;
    let eta = Duration::from_millis(
        all_tests.len() as u64 * cli.time_control.to_ms() / num_threads as u64,
    );
    println!(
        "Running test suite on {} positions with time-control: {} on {num_threads} threads...ETA: {:?}",
        all_tests.len(),
        cli.time_control,
        eta
    );
    println!("{:-<80}", "");

    let params = TunableParams::default(); // Using default, non-tuned params
    let evaluator = CompositeEvaluator::with_params(&params);

    let pb = ProgressBar::new(all_tests.len() as u64);
    let pb_style = ProgressStyle::default_bar()
        .template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
        )
        .into_diagnostic()?
        .progress_chars("#>-");
    pb.set_style(pb_style);
    pb.enable_steady_tick(Duration::from_millis(100));

    let results: Vec<TestResult> = sts_runner::run_suite(
        &all_tests,
        Box::new(evaluator),
        cli.time_control.to_ms(),
        Some(&pb),
    );
    pb.finish_with_message("Done!");

    let mut thematic_results: HashMap<String, SuiteSummary> = HashMap::new();
    let mut passes = Vec::new();
    let mut failures = Vec::new();

    for result in &results {
        let theme_summary = thematic_results.entry(result.theme.clone()).or_default();
        theme_summary.name = result.theme.clone();
        theme_summary.num_tests += 1;
        theme_summary.score += result.score;
        theme_summary.max_score += result.max_score;
        if result.bm_correct {
            theme_summary.bm_correct += 1;
        }

        let log_message = format_log_message(result);
        if result.score < result.max_score {
            failures.push(log_message);
        } else {
            passes.push(log_message);
        }
    }

    passes.sort();
    failures.sort();
    for pass in &passes {
        println!("{}", pass);
    }

    if !failures.is_empty() {
        println!("\nFailures:");
        for fail in &failures {
            println!("{}", fail);
        }
    }

    // 7. Convert map to a sorted list for the final summary table
    let mut final_summaries: Vec<SuiteSummary> = thematic_results.into_values().collect();
    final_summaries.sort_by(|a, b| a.name.cmp(&b.name));

    // 8. Print the final summary table
    print_thematic_summary(&final_summaries);

    Ok(())
}

// --- All presentation and formatting logic lives in the binary ---

fn format_log_message(result: &TestResult) -> String {
    let status_str = if result.score < result.max_score {
        format!("[{}{:<4}{}]", RED, "FAIL", RESET)
    } else {
        format!("[{}{:<4}{}]", GREEN, "PASS", RESET)
    };

    let truncated_id =
        truncate_with_elipses(&format!("{}.{}", result.theme, result.id), MAX_ID_LEN);
    let expected_moves_str = parse_expected_moves(&result.move_scores);

    let base_log = format!(
        "{} ID: {:<width1$} | S: {:>2}/{} | Ex: {:<width2$} | Got: {:<6}",
        status_str,
        truncated_id,
        result.score,
        result.max_score,
        expected_moves_str,
        result.engine_move_uci,
        width1 = MAX_ID_LEN,
        width2 = EXPECTED_MOVES_WIDTH
    );

    if result.score < result.max_score {
        format!("{} | FEN: {}", base_log, result.fen)
    } else {
        base_log
    }
}

fn print_thematic_summary(results: &[SuiteSummary]) {
    if results.is_empty() {
        return;
    }

    let mut grand_total = SuiteSummary::default();
    for summary in results {
        grand_total.score += summary.score;
        grand_total.max_score += summary.max_score;
        grand_total.bm_correct += summary.bm_correct;
        grand_total.num_tests += summary.num_tests;
    }

    println!("\n{}", "=".repeat(75));
    println!("THEMATIC SUMMARY");
    println!("{}", "=".repeat(75));
    println!(
        "{:<25} {:>12} {:>15} {:>18}",
        "Theme", "STS Score", "STS %", "Best Move %"
    );
    println!("{}", "-".repeat(75));

    for summary in results {
        let percentage = if summary.max_score > 0 {
            (summary.score as f64 / summary.max_score as f64) * 100.0
        } else {
            0.0
        };
        let bm_percentage = if summary.num_tests > 0 {
            (summary.bm_correct as f64 / summary.num_tests as f64) * 100.0
        } else {
            0.0
        };
        let truncated_theme = truncate_with_elipses(&summary.name, 25);

        println!(
            "{:<25} {:>6}/{:<5} {:>14.1}% {:>12.1}%",
            truncated_theme, summary.score, summary.max_score, percentage, bm_percentage
        );
    }

    println!("{}", "-".repeat(75));

    let grand_percentage = if grand_total.max_score > 0 {
        (grand_total.score as f64 / grand_total.max_score as f64) * 100.0
    } else {
        0.0
    };
    let grand_bm_percentage = if grand_total.num_tests > 0 {
        (grand_total.bm_correct as f64 / grand_total.num_tests as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "{:<25} {:>6}/{:<5} {:>14.1}% {:>12.1}%",
        "OVERALL", grand_total.score, grand_total.max_score, grand_percentage, grand_bm_percentage
    );
    println!("{}", "=".repeat(75));
}

fn parse_expected_moves(s: &HashMap<String, i32>) -> String {
    let mut expected_moves: Vec<_> = s.keys().map(|m| m.as_str()).collect();
    expected_moves.sort();
    let mut expected_moves_str = expected_moves.join(" ");

    if expected_moves_str.len() > EXPECTED_MOVES_WIDTH {
        expected_moves_str.truncate(EXPECTED_MOVES_WIDTH - 3);
        expected_moves_str.push_str("...");
    }
    expected_moves_str
}

fn truncate_with_elipses(s: &str, max_len: usize) -> String {
    if s.len() < max_len {
        return s.to_string();
    }

    if let Some(last_dot_pose) = s.rfind('.') {
        let ending_part = &s[last_dot_pose..];
        if ending_part.len() > 1 && ending_part[1..].chars().all(|c| c.is_ascii_digit()) {
            let ellipsis = "...";
            let available_for_start = max_len.saturating_sub(ending_part.len() + ellipsis.len());

            if available_for_start > 0 {
                let start_part = &s[..available_for_start.min(s.len() - ending_part.len())];
                return format!("{start_part}{ellipsis}{ending_part}");
            }
        }
    }
    format!("{}...", &s[..max_len.saturating_sub(3)])
}

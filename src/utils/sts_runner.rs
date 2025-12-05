use crate::{
    prelude::*,
    search::common::{SearchConfig, SearchLimits},
    tuning::params::TunableParams,
};
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[cfg(feature = "parallel")]
use {indicatif::ProgressBar, rayon::prelude::*};

/// Represents a single parsed EPD test case.
#[derive(Debug, Clone)]
pub struct EpdTest {
    pub fen: String,
    pub theme: String,
    pub id: String,
    pub best_move: Option<String>,
    pub move_scores: HashMap<String, i32>,
    // Optional raw data for debugging the EPD files themselves
    pub c7_san_moves: Option<Vec<String>>,
    pub c8_scores: Option<Vec<i32>>,
    pub c9_uci_moves: Option<Vec<String>>,
}

/// A summary of the results from running a suite of tests.
#[derive(Default, Debug, Clone)]
pub struct SuiteSummary {
    pub name: String,
    pub score: i32,
    pub max_score: i32,
    pub num_tests: usize,
    pub bm_correct: u64,
}

#[derive(Default, Debug, Clone)]
pub struct TestResult {
    pub theme: String,
    pub score: i32,
    pub max_score: i32,
    pub bm_correct: bool,
    pub id: String,
    pub fen: String,
    pub engine_move_uci: String,
    pub move_scores: HashMap<String, i32>,
}

/// Runs a suite of EPD tests against a given evaluator configuration.
///
/// This is the core function of the module. It orchestrates the search for each
/// position, scores the results, and returns a summary. It will use Rayon
/// for parallel execution if the "parallel" feature is enabled.
///
/// # Arguments
/// * `tests` - A slice of `EpdTest`s to run.
/// * `evaluator` - A boxed `Evaluator` trait object representing the engine's configuration.
/// * `time_ms_per_move` - The time limit in milliseconds for each search.
pub fn run_suite(
    tests: &[EpdTest],
    params: &TunableParams,
    time_ms_per_move: u64,
    #[cfg(feature = "parallel")] progress_bar: Option<&ProgressBar>,
    #[cfg(not(feature = "parallel"))] _progress_bar: Option<()>,
) -> Vec<TestResult> {
    let run_single_test = |test: &EpdTest| -> TestResult {
        let mut lim = SearchLimits::time(time_ms_per_move);
        lim.max_depth = Some(MAX_PLY as u16); // Ensure there's a depth limit

        // Each thread needs its own independent search engine instance.
        let mut search = AlphaBetaSearch::with_eval(params.clone()).with_limits(lim);

        let conf = SearchConfig {
            emit_info: false,
            ..Default::default()
        };
        search = search
            .with_config(conf)
            .expect("Failed to set search config");

        let board = Board::from_fen(&test.fen);
        let result = search.find_best_move(&board);
        let engine_move_uci = result.best_move.map(|m| m.uci()).unwrap_or_default();

        let score = *test.move_scores.get(&engine_move_uci).unwrap_or(&0);
        let max_score_for_pos = *test.move_scores.values().max().unwrap_or(&0);
        let engine_move_uci = result.best_move.map(|m| m.uci()).unwrap_or_default();
        let bm_correct = test
            .best_move
            .as_ref()
            .is_some_and(|bm| *bm == engine_move_uci);

        #[cfg(feature = "parallel")]
        if let Some(pb) = progress_bar {
            pb.inc(1);
        }
        TestResult {
            theme: test.theme.clone(),
            score,
            max_score: max_score_for_pos,
            bm_correct,
            id: test.id.clone(),
            fen: test.fen.clone(),
            engine_move_uci,
            move_scores: test.move_scores.clone(),
        }
    };

    // TOOD: Add support for setting number of threads
    #[cfg(feature = "parallel")]
    let results: Vec<TestResult> = tests.par_iter().map(run_single_test).collect();

    #[cfg(not(feature = "parallel"))]
    let results: Vec<TestResult> = tests.iter().map(run_single_test).collect();

    results
}

/// Loads and parses all .epd files from a given path.
/// If the path is a directory, it finds and parses all .epd files within it.
/// If the path is a single file, it parses that file.
pub fn load_epd_files_from_path(path: &PathBuf) -> miette::Result<Vec<EpdTest>> {
    let mut all_tests = Vec::new();
    let files_to_parse = find_epd_files(path)?;

    miette::ensure!(
        !files_to_parse.is_empty(),
        "No .epd files found in the specified path: {}",
        path.display()
    );

    for file_path in files_to_parse {
        let file = File::open(&file_path).into_diagnostic()?;
        let reader = BufReader::new(file);

        for line_result in reader.lines() {
            let line = line_result.into_diagnostic()?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            match parse_epd_line(&line) {
                Ok(test) => all_tests.push(test),
                Err(e) => warn!(
                    "Skipping invalid EPD line in {}: {:?}",
                    file_path.display(),
                    e
                ),
            }
        }
    }
    Ok(all_tests)
}

/// Finds all files with the .epd extension in a given path.
fn find_epd_files(path: &PathBuf) -> miette::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_dir() {
        for entry in read_dir(path).into_diagnostic()? {
            let entry_path = entry.into_diagnostic()?.path();
            if entry_path.is_file() && entry_path.extension().is_some_and(|e| e == "epd") {
                files.push(entry_path);
            }
        }
        files.sort(); // Sort for consistent order
    } else if path.is_file() {
        files.push(path.clone());
    }
    Ok(files)
}

fn parse_quoted_string(s: &str) -> Option<String> {
    s.strip_prefix('"')?.strip_suffix('"').map(str::to_owned)
}

pub fn parse_epd_line(line: &str) -> miette::Result<EpdTest> {
    const OPCODES: [&str; 8] = [" bm", " am", " dm", " id", " c0", " c7", " c8", " c9"];

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

    let mut id_str = String::from("unknown");
    let mut best_move = None;
    let mut move_scores = HashMap::new();

    let mut c7_data = None;
    let mut c8_data: Option<Vec<i32>> = None;
    let mut c9_data: Option<Vec<String>> = None;

    for part in opcode_part.split(';').map(str::trim) {
        if part.is_empty() {
            continue;
        }
        if let Some((opcode, value)) = part.split_once(' ') {
            let value = value.trim();
            match opcode {
                "id" => id_str = parse_quoted_string(value).unwrap_or_else(|| value.to_owned()),
                "bm" => {
                    if let Some(san_move) = value.split_whitespace().next() {
                        if let Ok(mov) = Move::from_san(&board, san_move) {
                            best_move = Some(mov.uci());
                        } else {
                            warn!("Failed to parse 'bm' move '{}' for ID: '{}'", value, id_str);
                        }
                    }
                }
                "c0" => {
                    if let Some(scores_str) = parse_quoted_string(value) {
                        for score_pair in scores_str.split(',') {
                            // Correctly split by comma
                            let pair: Vec<&str> = score_pair.trim().split('=').collect();
                            if pair.len() == 2 {
                                let san_move = pair[0];
                                if let (Ok(mov), Ok(score)) =
                                    (Move::from_san(&board, san_move), pair[1].parse::<i32>())
                                {
                                    move_scores.insert(mov.uci(), score);
                                }
                            }
                        }
                    }
                }
                // ... (c7, c8, c9 parsing is unchanged) ...
                "c7" => {
                    if let Some(s) = parse_quoted_string(value) {
                        c7_data = Some(s.split_whitespace().map(str::to_owned).collect());
                    }
                }
                "c8" => {
                    if let Some(s) = parse_quoted_string(value) {
                        c8_data = Some(
                            s.split_whitespace()
                                .filter_map(|n| n.parse().ok())
                                .collect(),
                        );
                    }
                }
                "c9" => {
                    if let Some(s) = parse_quoted_string(value) {
                        c9_data = Some(s.split_whitespace().map(str::to_owned).collect());
                    }
                }
                _ => {}
            }
        }
    }

    if move_scores.is_empty() { /* ... */ }
    if let Some(bm) = &best_move {
        move_scores.entry(bm.clone()).or_insert(100);
    }

    let (theme, id) = match id_str.rsplit_once('.') {
        Some((t, i)) => (t.to_string(), i.trim_end_matches(';').to_string()),
        None => ("unknown".to_string(), id_str),
    };

    Ok(EpdTest {
        fen,
        id,
        theme,
        move_scores, // Renamed from move_score
        best_move,
        c7_san_moves: c7_data,
        c8_scores: c8_data,
        c9_uci_moves: c9_data,
    })
}

#[cfg(test)]
mod epd_tests {
    use crate::utils::sts_runner::parse_epd_line;

    #[test]
    fn test_epd_parse() {
        let epd_line = r#"1kr5/3n4/q3p2p/p2n2p1/PppB1P2/5BP1/1P2Q2P/3R2K1 w - - bm f5; id "Undermine.001"; c0 "f5=100, Bf2=68, fxg5=46, b3=39, Bg7=32, Bg4=22, Kh1=11, Be3=8, Bxd5=6, h3=5"; c7 "f5 Bf2 fxg5 b3 Bg7 Bg4 Kh1 Be3 Bxd5 h3"; c8 "100 68 46 39 32 22 11 8 6 5"; c9 "f4f5 d4f2 f4g5 b2b3 d4g7 f3g4 g1h1 d4e3 f3d5 h2h3""#;

        let res = parse_epd_line(epd_line);

        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result.theme, "Undermine".to_owned());
        assert_eq!(result.best_move, Some("f4f5".to_owned()));
    }
}

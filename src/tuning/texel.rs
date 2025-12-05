use crate::{
    evaluation::{self},
    prelude::*,
    tuning::trace::EvalTrace,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

/// A single position in the tuning dataset
pub struct TexelEntry {
    pub trace: EvalTrace,
    /// Score from non-tunable sources
    pub fixed_score: Score,
    /// 0.0 (Black Win), 0.5 (Draw), 1.0 (White Win)
    pub result: f64,
    /// Normalized Phase: 0.0 (MG) -> 1.0 (EG)
    pub phase: f64,
}

impl TexelEntry {
    /// Calculates the static evaluation fo the position using provided weights
    pub fn evaluate(&self, weights: &[f64], feature_map: &[usize], mobility_map: &[usize]) -> f64 {
        let mut mg = self.fixed_score.mg as f64;
        let mut eg = self.fixed_score.eg as f64;

        // Sum Standard Features & PSTs
        for (trace_idx, &count) in self.trace.features.iter().enumerate() {
            if count != 0 {
                let spsa_idx = feature_map[trace_idx];
                mg += weights[spsa_idx] * count as f64;
                eg += weights[spsa_idx + 1] * count as f64;
            }
        }

        // Sum mobility
        for (mob_idx, &count) in self.trace.mobility.iter().enumerate() {
            if count != 0 {
                let spsa_idx = mobility_map[mob_idx];
                mg += weights[spsa_idx] * count as f64;
                eg += weights[spsa_idx + 1] * count as f64;
            }
        }

        mg * (1.0 - self.phase) + eg * self.phase
    }
}
/// Loads the dataset from a .book  file
pub fn load_texel_dataset<P: AsRef<Path>>(path: P) -> miette::Result<Vec<TexelEntry>> {
    let file = File::open(path).expect("Failed to open book file");
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    #[cfg(feature = "parallel")]
    let iter = lines.par_iter();
    #[cfg(not(feature = "parallel"))]
    let iter = lines.iter();

    let entries: Vec<TexelEntry> = iter.filter_map(|line| parse_book_line(line)).collect();

    Ok(entries)
}

fn parse_book_line(line: &str) -> Option<TexelEntry> {
    // Format: "1r2qrk1/2b2pp1/3pb2p/3Bp3/1pP5/3QP3/1PN2PPP/1R1R2K1 b - - 1 26 [0.5]"
    let parts: Vec<&str> = line.split('[').collect();
    if parts.len() < 2 {
        return None;
    }

    let fen = parts[0].trim();
    let result_str = parts[1].trim_end_matches(']');

    let result = match result_str {
        "1.0" => 1.0,
        "0.5" => 0.5,
        "0.0" => 0.0,
        _ => return None,
    };

    let board = match Board::try_from_fen(fen) {
        Ok(b) => b,
        Err(_) => return None,
    };

    // Normalize result to Side-to-Move perspective
    // If it's White to move and result is 1.0 (White Win), score is 1.0
    // If it's Black to move and result is 1.0 (White Win), score is 0.0 (Loss for Black)
    let stm_result = if board.stm == Side::White {
        result
    } else {
        1.0 - result
    };

    let mut trace = EvalTrace::default();

    // Run static trace gen
    let mut fixed_score = evaluation::trace(&board, &mut trace);

    if board.stm == Side::Black {
        for f in trace.features.iter_mut() {
            *f = -*f;
        }
        for m in trace.mobility.iter_mut() {
            *m = -*m;
        }
        fixed_score = -fixed_score;
    }

    let phase_val = board.game_phase();

    // Normalize 0..256 to 0.0..1.0
    // 0 = Midgame, 256 = Endgame
    let phase_normalized = phase_val.0 as f64 / ENDGAME_PHASE as f64;

    Some(TexelEntry {
        trace,
        fixed_score,
        result: stm_result,
        phase: phase_normalized,
    })
}

// Calculate Mean Square Error for the dataset
pub fn calculate_mse(
    entries: &[TexelEntry],
    weights: &[f64],
    feature_map: &[usize],
    mobility_map: &[usize],
    k: f64,
) -> f64 {
    #[cfg(feature = "parallel")]
    let iter = entries.par_iter();
    #[cfg(not(feature = "parallel"))]
    let iter = entries.iter();

    let total_error: f64 = iter
        .map(|entry| {
            let eval = entry.evaluate(weights, feature_map, mobility_map);
            // Sigmoid: 1 / (1 + 10^(-K * eval / 400))
            // Note: Using base 10 or base e depends on preference.
            // Standard Texel usually uses base 10, but base e is fine if K is adjusted.
            // S = 1 / (1 + e^(-k * eval / 400))
            let sigmoid = 1.0 / (1.0 + (-k * eval / 400.0).exp());
            (entry.result - sigmoid).powi(2)
        })
        .sum();

    total_error / entries.len() as f64
}

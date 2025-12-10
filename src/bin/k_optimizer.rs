use clap::Parser;
use eschec::{
    prelude::*,
    tuning::{
        params::{NUM_TRACE_FEATURES, TunableParams},
        texel::{self},
        trace::EvalTrace,
    },
};
use rayon::prelude::*;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
struct KOptCli {
    /// Path to the .book or .epd file
    #[arg(required = true)]
    dataset: PathBuf,

    /// Path to params file (toml)
    #[arg(required = true)]
    params: PathBuf,

    /// Number of threads
    #[arg(long, default_value_t = 0)]
    threads: usize,
}

fn main() -> miette::Result<()> {
    let cli = KOptCli::parse();

    #[cfg(feature = "parallel")]
    {
        let num_threads = if cli.threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            cli.threads
        };
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .into_diagnostic()?;
    }

    println!("Loading parameters from: {}", cli.params.display());
    let current_params = TunableParams::load_from_file(&cli.params)?;
    let weights = current_params.to_vector();

    println!("Loading dataset from: {}", cli.dataset.display());
    let entries = texel::load_texel_dataset(&cli.dataset)?;
    println!("Loaded {} positions", entries.len());

    let feature_map: Vec<usize> = (0..NUM_TRACE_FEATURES)
        .map(EvalTrace::map_feature_to_spsa_index)
        .collect();

    println!("Pre-calc static evaluations for all positions");

    #[cfg(feature = "parallel")]
    let iter = entries.par_iter();
    #[cfg(not(feature = "parallel"))]
    let iter = entries.iter();

    let data_points: Vec<(f64, f64)> = iter
        .map(|entry| {
            let eval = entry.evaluate(&weights, &feature_map);
            (eval, entry.result)
        })
        .collect();

    // Calculate MSE with current K (1.13) for comparison
    let current_k_mse = calculate_mse_for_k(&data_points, 1.13);
    println!("Current K (1.13) MSE: {:.6}", current_k_mse);

    let mut best_k = 0.0;
    let mut best_mse = f64::MAX;

    let start_k = 0.5;
    let end_k = 2.5;
    let step = 0.001;
    let steps = ((end_k - start_k) / step) as usize;

    for i in 0..steps {
        let k = start_k + (i as f64 * step);
        let mse = calculate_mse_for_k(&data_points, k);

        if mse < best_mse {
            best_mse = mse;
            best_k = k;
        }

        if i % 10 == 0 {
            println!("Current ( step: {i}/{steps} ) best MSE: {best_mse}, K: {best_k}");
        }
    }

    println!("\nResults:");
    println!("--------------------------------");
    println!("Best K-Factor: {:.4}", best_k);
    println!("Best MSE:      {:.6}", best_mse);
    println!("--------------------------------");

    if best_mse < current_k_mse {
        println!("Potential Improvement: {:.8}", current_k_mse - best_mse);
        println!("\nUpdate your tuner command to use --k {:.2}", best_k);
        println!("Then run the tuner again for ~50-100 epochs to realign weights.");
    } else {
        println!("Current K is already optimal.");
    }

    Ok(())
}

fn calculate_mse_for_k(data: &[(f64, f64)], k: f64) -> f64 {
    let total_error: f64 = data
        .iter()
        .map(|(eval, result)| {
            let sigmoid = 1.0 / (1.0 + (-k * eval / 400.0).exp());
            (result - sigmoid).powi(2)
        })
        .sum();

    total_error / data.len() as f64
}

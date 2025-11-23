use std::path::PathBuf;

use clap::Parser;
use eschec::{
    prelude::*,
    tuning::{params::TunableParams, spsa_tuner},
    utils::sts_runner,
};

/// An SPSA based tuner for Eschec's evaluation parameters
#[derive(Parser, Debug)]
#[command(version, about)]
struct TunerCli {
    /// Path to the directory containing the STS EPD files for tuning
    #[arg(required = true)]
    path: PathBuf,

    /// Time in milliseconds for each search during a test run
    #[arg(long, default_value_t = 1000)]
    time_ms: u64,

    /// Number of tuning iterations to run
    #[arg(long, default_value_t = 200)]
    iterations: usize,

    /// SPSA learning rate. Controls how big the steps are
    #[arg(long, default_value_t = 1.0)]
    alpha: f64,

    /// SPSA pertubation size. Controls how far to 'look' in rand direction
    #[arg(long, default_value_t = 0.5)]
    gamma: f64,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = TunerCli::parse();

    println!("Loading test suite from: {}", cli.path.display());
    let all_tests = sts_runner::load_epd_files_from_path(&cli.path)?;
    println!("Loaded {} test positions.", all_tests.len());

    println!("\n==> Starting SPSA tuning");
    println!(
        "Iterations: {}, Time/Move: {}ms",
        cli.iterations, cli.time_ms
    );
    println!("Alpha: {}, Gamma: {}", cli.alpha, cli.gamma);
    println!("{:-<20}\n", "");

    let fitness_function = |params_vec: &[f64]| -> f64 {
        let params = TunableParams::from_vector(params_vec);
        let evaluator = CompositeEvaluator::with_params(&params);

        let results = sts_runner::run_suite(&all_tests, Box::new(evaluator), cli.time_ms, None);

        let mut total_score = 0;
        let mut total_max_score = 0;
        for result in results {
            total_score += result.score;
            total_max_score += result.max_score;
        }

        if total_max_score > 0 {
            (total_score as f64 / total_max_score as f64) * 100.0
        } else {
            0.0
        }
    };

    let initial_params = TunableParams::default().to_vector();

    let final_params_vec = spsa_tuner::run_spsa_tuning_session(
        initial_params,
        fitness_function,
        cli.iterations,
        cli.alpha,
        cli.gamma,
    );

    println!("\n==> Tuning complete");
    std::fs::write(
        "tuned_params.txt",
        final_params_vec
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .into_diagnostic()?;

    println!("Final params have been written to 'tuned_params.txt'");
    Ok(())
}

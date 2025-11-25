use clap::Parser;
use eschec::{
    prelude::*,
    tuning::{params::TunableParams, spsa_tuner},
    utils::sts_runner,
};
use std::{path::PathBuf, time::Duration};

const NUM_RUNS: u64 = 3;

/// An SPSA based tuner for Eschec's evaluation parameters
#[derive(Parser, Debug)]
#[command(version, about)]
struct TunerCli {
    /// Path to the directory containing the STS EPD files for tuning
    #[arg(required = true)]
    path: PathBuf,

    /// Number of threads to use (0 = auto-detect/all-cores)
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Time in milliseconds for each search during a test run
    #[arg(long, default_value_t = 1000)]
    time_ms: u64,

    /// Number of tuning iterations to run
    #[arg(long, default_value_t = 200)]
    iterations: usize,

    /// SPSA learning rate. Controls how big the steps are
    #[arg(long, default_value_t = 10.0)]
    alpha: f64,

    /// SPSA pertubation size. Controls how far to 'look' in rand direction
    #[arg(long, default_value_t = 2.0)]
    gamma: f64,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = TunerCli::parse();

    let num_threads;
    #[cfg(feature = "parallel")]
    {
        num_threads = if cli.threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            cli.threads
        };

        println!("Running tuner using {num_threads} threads!");

        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .into_diagnostic()?;
    }
    #[cfg(not(feature = "parallel"))]
    {
        num_threads = 1;
    }

    let start_time = std::time::Instant::now();
    let all_tests = sts_runner::load_epd_files_from_path(&cli.path)?;
    let eta = Duration::from_millis(
        NUM_RUNS * cli.iterations as u64 * cli.time_ms * all_tests.len() as u64
            / num_threads as u64,
    );
    println!(
        "Loading test suite from: {}...ETA: {:?}",
        cli.path.display(),
        eta
    );
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

        let results = sts_runner::run_suite(&all_tests, &params, cli.time_ms, None);

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

    println!("\n==> Tuning complete, took: {:?}", start_time.elapsed());
    let final_params = TunableParams::from_vector(&final_params_vec);
    final_params.save_to_file("tuned_params.toml")?;

    println!("{:?}", final_params);

    println!("Final params have been written to 'tuned_params.toml'");
    Ok(())
}

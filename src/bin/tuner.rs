use clap::Parser;
use eschec::{
    prelude::*,
    tuning::{
        gd_tuner::{self, GdParams},
        params::{NUM_TRACE_FEATURES, TunableParams},
        texel::{self},
        trace::EvalTrace,
    },
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
struct TunerCli {
    /// Path to the .book or .epd file
    #[arg(required = true)]
    dataset: PathBuf,

    /// Number of threads to use (0 = auto-detect/all-cores)
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Number of epochs (passes through the dataset)
    #[arg(long, default_value_t = 200)]
    epochs: usize,

    /// Learning Rate (try 10.0 or 100.0 for AdaGrad on CP scores)
    #[arg(long, default_value_t = 10.0)]
    lr: f64,

    /// Batch size (higher = faster but less frequent updates)
    #[arg(long, default_value_t = 16384)]
    batch_size: usize,

    /// Sigmoid K factor
    #[arg(long, default_value_t = 1.13)]
    k: f64,
}

fn main() -> miette::Result<()> {
    eschec::utils::log::init();
    let cli = TunerCli::parse();

    // Setup ThreadPool if parallel feature is enabled
    #[cfg(feature = "parallel")]
    {
        let num_threads = if cli.threads == 0 {
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

    println!("Loading dataset from: {}", cli.dataset.display());
    let entries = texel::load_texel_dataset(&cli.dataset)?;
    println!("Loaded {} positions.", entries.len());

    // Precompute maps
    let feature_map: Vec<usize> = (0..NUM_TRACE_FEATURES)
        .map(|i| EvalTrace::map_feature_to_spsa_index(i))
        .collect();

    let mobility_map: Vec<usize> = (0..5)
        .map(|i| EvalTrace::map_mobility_to_spsa_index(i))
        .collect();

    // Initial Error
    let initial_params = TunableParams::default();
    let initial_vec = initial_params.to_vector();

    let initial_error =
        texel::calculate_mse(&entries, &initial_vec, &feature_map, &mobility_map, cli.k);
    println!("Initial MSE: {:.6}", initial_error);

    println!("\n==> Starting Gradient Descent (AdaGrad)...");

    let params = GdParams {
        learning_rate: cli.lr,
        k: cli.k,
        epochs: cli.epochs,
        batch_size: cli.batch_size,
    };

    let final_vec =
        gd_tuner::run_gd_tuning(&entries, initial_vec, &feature_map, &mobility_map, params);

    let final_error =
        texel::calculate_mse(&entries, &final_vec, &feature_map, &mobility_map, cli.k);
    println!("\nFinal MSE: {:.6}", final_error);
    println!("Improvement: {:.6}", initial_error - final_error);

    let final_params = TunableParams::from_vector(&final_vec);
    final_params.save_to_file("tuned_params.toml")?;
    println!("Saved raw params to tuned_params.toml");

    let pawn_mg = final_params.material[0].mg as f64;
    let target_pawn_mg = 100.0;

    let scale = if pawn_mg.abs() > 1e-4 {
        target_pawn_mg / pawn_mg
    } else {
        1.0
    };

    println!(
        "Normalizing parameters... (Pawn MG: {:.2} -> 100.0, Scale: {:.4}",
        pawn_mg, scale
    );

    let normalized_vec: Vec<f64> = final_vec.iter().map(|&x| x * scale).collect();
    let normalized_params = TunableParams::from_vector(&normalized_vec);

    normalized_params.save_to_file("normalized_tuned_params.toml")?;
    println!("Saved normalized params to 'normalized_tuned_params.toml'");

    Ok(())
}

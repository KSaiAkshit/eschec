use crate::utils::prng::Prng;

/// Runs a tuning session using SPSA.
///
/// Args:
/// * `initial_params` - The starting vector of parameters to be tuned.
/// * `fitness_function` - A closure that takes a param vector and returns a fitness
/// score to be maximized.
/// * `iterations` - The number of tuning iterations to run.
/// * `alpha` - The SPSA 'a' param (learning rate)
/// * `gamma` - The SPSA 'c' param (pertubation rate))
///
/// Returns
/// Final optimized parameter vector

pub fn run_spsa_tuning_session(
    initial_params: Vec<f64>,
    fitness_function: impl Fn(&[f64]) -> f64,
    iterations: usize,
    alpha: f64,
    gamma: f64,
) -> Vec<f64> {
    let mut params_vec = initial_params;
    let num_params = params_vec.len();
    let mut rng = Prng::init(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );

    for k in 0..iterations {
        println!("[Iteration {}/{}]", k + 1, iterations);

        let delta: Vec<f64> = (0..num_params).map(|_| rng.pm_one()).collect();

        let mut params_plus = params_vec.clone();
        let mut params_minus = params_vec.clone();
        for i in 0..num_params {
            params_plus[i] += gamma * delta[i];
            params_minus[i] -= gamma * delta[i];
        }

        println!("  Evaluating pertubations...");
        let fitness_plus = fitness_function(&params_plus);
        let fitness_minus = fitness_function(&params_minus);
        println!(
            "  -> Fitness+: {:.4}%, Fitness-: {:.4}%",
            fitness_plus, fitness_minus
        );

        let mut grad_estimate = vec![0.0; num_params];
        let diff = fitness_plus - fitness_minus;

        if diff.abs() > 1e-9 {
            for i in 0..num_params {
                grad_estimate[i] = diff / (2.0 * gamma * delta[i]);
            }
        }

        for i in 0..num_params {
            params_vec[i] += alpha * grad_estimate[i];
        }

        let current_fitness = fitness_function(&params_vec);
        println!(
            "  -> Iteration {} complete. Current Fitness: {:.4}%",
            k + 1,
            current_fitness
        );
        println!("----------------------------"); // Separator for clarity
    }

    params_vec
}

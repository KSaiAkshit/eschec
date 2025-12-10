use crate::tuning::texel::TexelEntry;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Config for Gradient Descent
#[derive(Debug)]
pub struct GdParams {
    /// Usually (1.0 - 100.0) for AdaGrad
    pub learning_rate: f64,
    /// Sigmoid scaling factor (usually ~1.13)
    pub k: f64,
    /// How many times to loop through the dataset
    pub epochs: usize,
    /// Number of positions to process before updating weights
    pub batch_size: usize,
}

pub fn run_gd_tuning(
    entries: &[TexelEntry],
    initial_weights: Vec<f64>,
    feature_map: &[usize],
    params: GdParams,
) -> Vec<f64> {
    let mut weights = initial_weights;
    let num_params = weights.len();

    // AdaGrad: keep track of the sum of the squared gradients for each param
    let mut sum_squared_grad = vec![0.0; num_params];

    println!("Starting Gradient Descent (AdaGrad)...");

    for epoch in 0..params.epochs {
        // Process the dataset in batches to update weights frequently
        for batch in entries.chunks(params.batch_size) {
            // Calculate gradient for this batch
            // Use thread-local accumulator to parallelize gradient calc

            #[cfg(feature = "parallel")]
            let iter = batch.par_iter();
            #[cfg(not(feature = "parallel"))]
            let iter = batch.iter();

            // Helper closure the fold grad
            let fold_op = || vec![0.0; num_params];

            // Helper closure to process one entry
            let map_op = |mut grads: Vec<f64>, entry: &TexelEntry| {
                // Forward pass: calc eval
                let eval = entry.evaluate(&weights, feature_map);

                let sigmoid = 1.0 / (1.0 + (-params.k * eval / 400.0).exp());

                // Gradient term: (Result - Sigmoid) * Sigmoid * (1 - Sigmoid) * Scaling
                // We simplify the update rule direction.
                // The error term points towards the target.
                let error_term = (entry.result - sigmoid) * sigmoid * (1.0 - sigmoid);

                // Backward pass: distribute error to activate features

                // Standard Features
                for (trace_idx, &count) in entry.trace.features.iter().enumerate() {
                    if count != 0 {
                        let spsa_idx = feature_map[trace_idx];
                        let grad = error_term * count as f64;
                        // Update MG and EG gradients based on Phase
                        grads[spsa_idx] += grad * (1.0 - entry.phase);
                        grads[spsa_idx + 1] += grad * entry.phase;
                    }
                }

                grads
            };

            // Helper closure to reduce gradients
            let reduce_op = |mut a: Vec<f64>, b: Vec<f64>| {
                for i in 0..num_params {
                    a[i] += b[i];
                }
                a
            };

            // Execute Fold/Reduce based on feature flag
            #[cfg(feature = "parallel")]
            let batch_gradients = iter.fold(fold_op, map_op).reduce(fold_op, reduce_op);

            #[cfg(not(feature = "parallel"))]
            let batch_gradients = iter.fold(fold_op(), map_op);

            // Update Grads (AdaGrad)
            for i in 0..num_params {
                let gradient = batch_gradients[i];

                // Accumulate squared grad history
                sum_squared_grad[i] += gradient * gradient;

                // Calculate adaptive learning rate
                // epsilon 1e-8 prevents division by zero
                let adaptive_lr = params.learning_rate / (sum_squared_grad[i].sqrt() + 1e-8);

                // Update weights
                weights[i] += adaptive_lr * gradient;
            }
        }

        // Report Progress
        if epoch % 10 == 0 || epoch == params.epochs - 1 {
            let mse = crate::tuning::texel::calculate_mse(entries, &weights, feature_map, params.k);

            println!(
                "Epoch: {} / {} complete. MSE: {:.6}",
                epoch + 1,
                params.epochs,
                mse
            );
        }
    }

    weights
}

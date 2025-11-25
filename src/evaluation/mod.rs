use crate::{
    evaluation::accumulator::{EvalAccumulator, ScoreAccumulator, TraceAccumulator},
    prelude::*,
    tuning::{params::TunableParams, trace::EvalTrace},
};
use std::fmt::Debug;

pub mod score;

/// TODO: Remove all this and replace with pure functions
/// Weights will be held in TunableParams
/*


#### 3. Architectural Refactor
We refactored the evaluation system to separate **Logic** from **Data**.

*   **`TunableParams` (Data):**
    *   Moved all weights (Material, PSTs, Bonuses) into one struct.
    *   Converted all weights to `Score { mg, eg }` to allow Tapered Evaluation tuning.
    *   Flattened PST arrays to support `serde` serialization.
    *   Added `to_vector` / `from_vector` for the optimizer.

*   **`EvalTrace` (Data):**
    *   A struct that mirrors `TunableParams` but stores **Counts** (`i8`/`i16`) instead of Weights.
    *   Records *what* is on the board (e.g., "1 Bishop Pair", "Isolated Pawn on File 3").

*   **`EvalAccumulator` (Interface):**
    *   A trait that abstracts the evaluation process.
    *   **`ScoreAccumulator`**: Used during gameplay. Looks up weights in `TunableParams` and sums them into a `Score`.
    *   **`TraceAccumulator`**: Used during tuning. Increments counts in `EvalTrace`.

*   **`Evaluator` (Logic):**
    *   Refactored all evaluators (`Material`, `Position`, `KingSafety`, etc.) to use `eval_generic`.
    *   They no longer return scores directly; they call `acc.add_feature(...)`.
    *   This ensures the Engine and the Tuner always agree on the board state without duplicating code.

#### 4. Next Steps
1.  **Generate Dataset:** Write a binary that reads your 12M position file, runs `eval.trace()`, and saves the `EvalTrace` + `GameResult` to a binary file.
2.  **Offline Tuner:** Write a simple Gradient Descent program that loads the binary file and optimizes the weights in `TunableParams`.
3.  **Result:** You will be able to tune 400+ parameters in minutes rather than days.
*
*
use king_safety::eval_king_safety;
use material::eval_material;
use mobility::eval_mobility;
use pawn_structure::eval_pawn_structure;
use position::eval_position;

/// The main evaluation entry point for the Engine.
pub fn evaluate(board: &Board, params: &TunableParams) -> Score {
    let mut acc = ScoreAccumulator {
        params,
        score: Score::default(),
    };

    // Call functions directly.
    // The compiler can now INLINE all of this into one giant, optimized block of machine code.
    eval_material(board, &mut acc);
    eval_position(board, &mut acc);
    eval_pawn_structure(board, &mut acc);
    eval_mobility(board, &mut acc);
    eval_king_safety(board, &mut acc);

    // Side to move adjustment
    if board.stm == Side::White {
        acc.score
    } else {
        -acc.score
    }
}

/// The trace entry point for the Tuner.
pub fn trace(board: &Board, trace: &mut EvalTrace) {
    let mut acc = TraceAccumulator { trace };

    eval_material(board, &mut acc);
    eval_position(board, &mut acc);
    eval_pawn_structure(board, &mut acc);
    eval_mobility(board, &mut acc);
    eval_king_safety(board, &mut acc);
}
*
*/
pub mod accumulator;
pub mod king_safety;
pub mod material;
pub mod mobility;
pub mod pawn_structure;
pub mod position;

use king_safety::KingSafetyEvaluator;
use material::MaterialEvaluator;
use mobility::MobilityEvaluator;
use pawn_structure::PawnStructureEvaluator;
use position::PositionEvaluator;

pub trait Evaluator: Debug + Send + Sync {
    /// Core logic. Calculates featurres and adds them to the accumulator
    fn eval_generic(&self, board: &Board, acc: &mut dyn EvalAccumulator);
    /// Wrapper for playing. Calculating the final score from the board state
    fn evaluate(&self, board: &Board, params: &TunableParams) -> Score {
        let mut acc = ScoreAccumulator {
            params,
            score: Score::default(),
        };

        self.eval_generic(board, &mut acc);

        if board.stm == Side::White {
            acc.score
        } else {
            -acc.score
        }
    }
    /// Wrapper for tuning. Records the presence of features
    fn trace(&self, board: &Board, trace: &mut EvalTrace) {
        let mut acc = TraceAccumulator { trace };
        self.eval_generic(board, &mut acc);
    }
    fn name(&self) -> &str;
    fn clone_box(&self) -> Box<dyn Evaluator>;
}

#[derive(Debug, Default)]
pub struct CompositeEvaluator {
    name: String,
    evaluators: Vec<Box<dyn Evaluator>>,
    weights: Vec<i32>,
}

impl Clone for CompositeEvaluator {
    fn clone(&self) -> Self {
        let cloned_evals = self.evaluators.iter().map(|e| e.clone_box()).collect();
        Self {
            name: self.name().to_string(),
            evaluators: cloned_evals,
            weights: self.weights.clone(),
        }
    }
}

impl CompositeEvaluator {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            evaluators: Vec::new(),
            weights: Vec::new(),
        }
    }
    pub fn balanced() -> Self {
        let mut evaluator = CompositeEvaluator::new("Balanced");
        evaluator
            .add_evaluator(Box::new(MaterialEvaluator::new()), 5)
            .add_evaluator(Box::new(KingSafetyEvaluator::new()), 3)
            .add_evaluator(Box::new(PositionEvaluator::new()), 2)
            .add_evaluator(Box::new(PawnStructureEvaluator::new()), 2)
            .add_evaluator(Box::new(MobilityEvaluator::new()), 1);
        evaluator
    }

    pub fn with_params(params: &TunableParams) -> Self {
        let mut evaluator = CompositeEvaluator::new("Tunable");
        evaluator
            .add_evaluator(Box::new(MaterialEvaluator::with_params(params)), 5)
            .add_evaluator(Box::new(KingSafetyEvaluator::with_params(params)), 3)
            .add_evaluator(Box::new(PositionEvaluator::with_params(params)), 2)
            .add_evaluator(Box::new(PawnStructureEvaluator::with_params(params)), 2)
            .add_evaluator(Box::new(MobilityEvaluator::with_params(params)), 1);
        evaluator
    }

    pub fn add_evaluator(&mut self, evaluator: Box<dyn Evaluator>, weight: i32) -> &mut Self {
        self.evaluators.push(evaluator);
        self.weights.push(weight);
        self
    }

    pub fn print_eval_breakdown(&self, board: &Board, params: &TunableParams) {
        println!("+-------------------+---------+---------+");
        println!("|     Term          |    MG   |    EG   |");
        println!("+-------------------+---------+---------+");

        let mut total_mg = 0;
        let mut total_eg = 0;

        for eval in &self.evaluators {
            // We run evaluate() on the sub-evaluator.
            // Note: This returns STM score.
            let score = eval.evaluate(board, params);

            println!(
                "| {:<17} | {:>7} | {:>7} |",
                eval.name(),
                score.mg,
                score.eg
            );
            total_mg += score.mg;
            total_eg += score.eg;
        }

        println!("+-------------------+---------+---------+");
        println!("|     Total         | {:>7} | {:>7} |", total_mg, total_eg);
        println!("+-------------------+---------+---------+");
    }
}

impl Evaluator for CompositeEvaluator {
    fn eval_generic(&self, board: &Board, acc: &mut dyn EvalAccumulator) {
        for evaluator in &self.evaluators {
            evaluator.eval_generic(board, acc);
        }
    }
    fn evaluate(&self, board: &Board, params: &TunableParams) -> Score {
        let mut acc = ScoreAccumulator {
            params,
            score: Score::default(),
        };

        self.eval_generic(board, &mut acc);

        //  Jiggle logic
        let jiggle = (board.hash % 5) as i32 - 2;
        let final_score = Score::new(acc.score.mg + jiggle, acc.score.eg + jiggle);

        if board.stm == Side::White {
            final_score
        } else {
            -final_score
        }
    }
    // fn evaluate(&self, board: &Board) -> Score {
    //     let total_weight: i32 = self.weights.iter().sum();
    //     let score: Score = self
    //         .evaluators
    //         .iter()
    //         .zip(self.weights.iter())
    //         .map(|(evaluator, &weight)| evaluator.evaluate(board) * weight)
    //         .fold(Score::default(), |acc, score| acc + score);

    //     // NOTE: The jiggle might seem unusual as Evaluations are usually deterministic
    //     // But the jiggle here is based on the position's Zobrist hash,
    //     // so it is deterministic as well
    //     let jiggle = (board.hash % 5) as i32 - 2;
    //     if total_weight > 0 {
    //         Score::new(
    //             (score.mg / total_weight) + jiggle,
    //             (score.eg / total_weight) + jiggle,
    //         )
    //     } else {
    //         Score::default()
    //     }
    // }

    fn name(&self) -> &str {
        &self.name
    }

    fn clone_box(&self) -> Box<dyn Evaluator> {
        let cloned_evals: Vec<Box<dyn Evaluator>> = self
            .evaluators
            .iter()
            .map(|eval| eval.clone_box())
            .collect();
        Box::new(CompositeEvaluator {
            evaluators: cloned_evals,
            weights: self.weights.clone(),
            name: self.name().to_owned(),
        })
    }
}

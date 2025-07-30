use crate::prelude::*;
use std::fmt::Debug;

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
    fn evaluate(&self, board: &Board) -> i32;
    fn name(&self) -> &str;
}

#[derive(Debug, Default)]
pub struct CompositeEvaluator {
    name: String,
    evaluators: Vec<Box<dyn Evaluator>>,
    weights: Vec<f32>,
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
            .add_evaluator(Box::new(MaterialEvaluator::new()), 0.5)
            .add_evaluator(Box::new(PositionEvaluator::new()), 0.2)
            .add_evaluator(Box::new(MobilityEvaluator::new()), 0.1)
            .add_evaluator(Box::new(KingSafetyEvaluator::new()), 0.1)
            .add_evaluator(Box::new(PawnStructureEvaluator::new()), 0.1);
        evaluator
    }

    pub fn add_evaluator(&mut self, evaluator: Box<dyn Evaluator>, weight: f32) -> &mut Self {
        self.evaluators.push(evaluator);
        self.weights.push(weight);
        self
    }
}

impl Evaluator for CompositeEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        self.evaluators
            .iter()
            .zip(self.weights.iter())
            .map(|(evaluator, &weight)| evaluator.evaluate(board) as f32 * weight)
            .sum::<f32>() as i32
    }

    fn name(&self) -> &str {
        &self.name
    }

    // fn evaluate(&self, board: &Board) -> i32 {
    //     let mut score = 0;
    //     let mut per_eval_score = Vec::new();
    //     for (eval, weight) in self.evaluators.iter().zip(self.weights.iter()) {
    //         let s = eval.evaluate(board) as f32;
    //         let weighted_score = (s * weight);
    //         per_eval_score.push(weighted_score);
    //         score += weighted_score as i32;
    //     }
    //     per_eval_score
    //         .iter()
    //         .zip(self.evaluators.iter())
    //         .zip(self.weights.iter())
    //         .for_each(|((sc, eval), weigth)| {
    //             println!(
    //                 "{}: contribution: {}, weight: {}, score: {}",
    //                 eval.name(),
    //                 (sc / score as f32) * 100.0,
    //                 weigth,
    //                 sc
    //             )
    //         });
    //     score
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_evaluator() {
        let board = Board::new();

        let mut composite = CompositeEvaluator::new("Test Composite");
        composite
            .add_evaluator(Box::new(MaterialEvaluator::new()), 0.3)
            .add_evaluator(Box::new(PositionEvaluator::new()), 0.3)
            .add_evaluator(Box::new(MobilityEvaluator::new()), 0.1);

        let score = composite.evaluate(&board);

        // Initial position with our evaluators should be roughly balanced
        // Allow some small variation from position scoring
        assert!(score.abs() < 10);
    }
}

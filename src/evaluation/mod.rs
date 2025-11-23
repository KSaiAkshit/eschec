use crate::{prelude::*, tuning::params::TunableParams};
use std::fmt::Debug;

pub mod score;

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
    fn evaluate(&self, board: &Board) -> Score;
    fn name(&self) -> &str;
    fn clone_box(&self) -> Box<dyn Evaluator>;
    fn breakdown(&self, board: &Board) -> Option<(String, Score)> {
        Some((self.name().to_string(), self.evaluate(board)))
    }
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

    pub fn print_eval_breakdown(&self, board: &Board) {
        let breakdown = self.breakdown(board);
        println!("+-------------------+---------+---------+----------+");
        println!("|     Term          |    MG   |    EG   |  Weight  |");
        println!("+-------------------+---------+---------+----------+");
        let mut mg_total = 0;
        let mut eg_total = 0;
        let weight_total: i32 = self.weights.iter().sum();
        for (name, mg, eg, weight) in breakdown {
            println!("| {:<17} | {:>7} | {:>7} | {:>8.2} |", name, mg, eg, weight);
            mg_total += mg * weight;
            eg_total += eg * weight;
        }
        println!("+-------------------+---------+---------+----------+");
        println!(
            "|     Total         | {:>7.2} | {:>7.2} | {:>8.2} |",
            mg_total, eg_total, weight_total
        );
        println!("+-------------------+---------+---------+----------+");
    }

    fn breakdown(&self, board: &Board) -> Vec<(String, i32, i32, i32)> {
        self.evaluators
            .iter()
            .zip(self.weights.iter())
            .filter_map(|(eval, &weight)| {
                eval.breakdown(board)
                    .map(|(name, score)| (name, score.mg, score.eg, weight))
            })
            .collect()
    }
}

impl Evaluator for CompositeEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
        let mut material_score = Score::default();
        let mut positional_score = Score::default();
        let mut positional_total_weight = 0;

        for (evaluator, &weight) in self.evaluators.iter().zip(self.weights.iter()) {
            if evaluator.name() == "Material" {
                material_score = evaluator.evaluate(board);
            } else {
                positional_score += evaluator.evaluate(board) * weight;
                positional_total_weight += weight;
            }
        }

        if positional_total_weight > 0 {
            positional_score.mg /= positional_total_weight;
            positional_score.eg /= positional_total_weight;
        }

        let final_score = material_score + positional_score;

        let jiggle = (board.hash % 5) as i32 - 2;

        Score::new(final_score.mg + jiggle, final_score.eg + jiggle)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_evaluator() {
        let board = Board::new();

        let mut composite = CompositeEvaluator::new("Test Composite");
        composite
            .add_evaluator(Box::new(MaterialEvaluator::new()), 3)
            .add_evaluator(Box::new(PositionEvaluator::new()), 3)
            .add_evaluator(Box::new(MobilityEvaluator::new()), 1);

        let score = composite.evaluate(&board);

        // Initial position with our evaluators should be roughly balanced
        // Allow some small variation from position scoring
        assert!(score.mg < 10);
        assert!(score.eg < 10);
    }
}

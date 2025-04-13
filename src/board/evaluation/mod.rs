#![allow(unused)]
use super::*;

pub mod material;
pub mod position;

pub trait Evalutor {
    fn evaluate(&self, board: &Board) -> i32;
    fn name(&self) -> &str;
}

pub struct CompositeEvaluator {
    name: String,
    evaluators: Vec<Box<dyn Evalutor>>,
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

    pub fn add_evaluator(&mut self, evaluator: Box<dyn Evalutor>, weight: f32) -> &mut Self {
        self.evaluators.push(evaluator);
        self.weights.push(weight);
        self
    }
}

impl Evalutor for CompositeEvaluator {
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
}

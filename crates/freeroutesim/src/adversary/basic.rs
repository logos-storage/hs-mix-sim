use crate::adversary::Adversary;
use crate::path_sampler::{FixedHopSet, PathSampler, TimeBasedPathSampler};
use crate::topologygen::MixNode;

/// basic adversary tries to use sybil attacks and 
/// defines the probability of compromise and time needed to compromise.
#[derive(Debug, Default)]
pub struct BasicAdversary {

}

impl BasicAdversary {
    pub fn new() -> Self {
        Self{

        }
    }
}

impl Adversary for BasicAdversary {
    fn wins(&mut self, path: &[MixNode]) -> bool {
        todo!()
    }
}
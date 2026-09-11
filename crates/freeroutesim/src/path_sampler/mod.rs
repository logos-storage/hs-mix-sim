pub mod alpha_sticky;
pub mod bandwidth_random;
pub mod k_hops_fixed;
pub mod k_over_w;
pub mod random;

use crate::mixnet::{MixNode, Mixnet};

/// Select paths from one static mixnet for the lifetime of this sampler.
pub trait PathSampler {
    /// Sample a path from the run's static mixnet.
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode>;
    /// check the behavior for a certain hop
    #[allow(dead_code)]
    fn hops(&self) -> usize;
    #[allow(dead_code)]
    fn sampler_type(&self) -> &'static str;
}

pub struct FixedHop {
    position: usize,
    node: MixNode,
}

#[derive(Debug, Default)]
pub struct FixedHopSet {
    position: usize,
    nodes: Vec<MixNode>,
}

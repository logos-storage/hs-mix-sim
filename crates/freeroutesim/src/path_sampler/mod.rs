pub mod alpha_sticky;
pub mod bandwidth_random;
pub mod k_hops_fixed;
pub mod k_over_w;
pub mod random;

use crate::topologygen::{MixId, MixNode, Topology};

/// path sampler main job is to select a path from a given topology
pub trait PathSampler {
    /// sample a path using given topology and index
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode>;
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
    mix_ids: Vec<MixId>,
}

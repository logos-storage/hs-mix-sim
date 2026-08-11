pub mod alpha_sticky;
pub mod bandwidth_random;
pub mod guard;
pub mod k_hops_fixed;
pub mod k_over_w;
pub mod random;
pub mod vanguard;

use crate::topologygen::{MixNode, Topology};

/// path sampler main job is to select a path from a given topology
pub trait PathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode>;
    #[allow(dead_code)]
    fn hops(&self) -> usize;
    #[allow(dead_code)]
    fn sampler_type(&self) -> &'static str;
}

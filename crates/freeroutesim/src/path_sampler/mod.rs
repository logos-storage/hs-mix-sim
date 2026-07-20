pub mod bandwidth_random;
pub mod guard;
pub mod random;
pub mod vanguard;

/// path sampler main job is to select a path from a given topology
/// Uniform and bandwidth-weighted random samplers are available.
/// guard and vanguard samplers use persistent client-side selections.
///
use crate::topologygen::{MixNode, Topology};

pub trait PathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode>;
    #[allow(dead_code)]
    fn hops(&self) -> usize;
    #[allow(dead_code)]
    fn sampler_type(&self) -> &'static str;
}

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
    /// sample a path using given topology and index
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode>;
    /// check the behavior for a certain hop
    fn hop_behavior(&self, hop: usize) -> HopBehavior;
    /// peak to see the nodes on next hop, next hop would be the hop closer to the sender
    /// the sampler returns mix ids that are connect to given node `mix_node` on hop `hop`
    fn peak(&self, hop:usize, mix_node: u32) -> Vec<u32>;
    #[allow(dead_code)]
    fn hops(&self) -> usize;
    #[allow(dead_code)]
    fn sampler_type(&self) -> &'static str;
}

/// define the behavior of the hop
/// it is either random or "fixed" i.e., has some lifetime
/// this is useful when modeling the adversary
pub enum HopBehavior {
    Random,
    Fixed,
}


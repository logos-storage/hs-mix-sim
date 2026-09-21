//! local topology sampler with profiles.

mod fpoft;
pub mod profiles;
pub use fpoft::FPOFTSampler;
mod topology;
pub use topology::{ConnectionMode, FixedTopology, LayerConfig, NodeLifetime, NodeRotation};

use super::{Observation, TimeBasedPathSampler};
use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;

/// Configuration for one topology profile.
#[derive(Clone, Copy)]
pub struct TopologyProfile {
    pub name: &'static str,
    pub layers: &'static [LayerConfig],
    pub connections: ConnectionMode,
}

pub struct FixedTopologySampler {
    topology: FixedTopology,
    #[allow(dead_code)]
    name: &'static str,
}

impl FixedTopologySampler {
    pub fn new(profile: TopologyProfile, mixnet: &Mixnet) -> Self {
        Self {
            topology: FixedTopology::new(profile.layers, profile.connections, mixnet),
            name: profile.name,
        }
    }
}

impl PathSampler for FixedTopologySampler {
    fn sample_path(&mut self, _mixnet: &Mixnet) -> Vec<MixNode> {
        self.topology.sample_path()
    }
    fn hops(&self) -> usize {
        self.topology.hops()
    }
    fn sampler_type(&self) -> &'static str {
        self.name
    }
}

impl TimeBasedPathSampler for FixedTopologySampler {
    type Event = NodeRotation;
    fn next_events(&mut self, current_time: u64) -> Vec<(u64, Self::Event)> {
        self.topology.next_events(current_time)
    }
    fn handle_event(&mut self, current_time: u64, event: Self::Event, mixnet: &Mixnet) {
        self.topology.handle_event(current_time, event, mixnet);
    }
    fn peak(&self, chain: &[MixId]) -> Observation {
        self.topology.peak(chain)
    }
    fn node(&self, mix_id: MixId) -> Option<&MixNode> {
        self.topology.node(mix_id)
    }
}

/// A pool of active paths selected from one local topology.
#[derive(Clone, Copy)]
pub struct FPOFTProfile {
    pub name: &'static str,
    /// Layer sizes, connections, and independent node lifetimes.
    pub topology_profile: TopologyProfile,
    pub num_paths: usize,
    /// Every active path independently draws from this lifetime at each rotation.
    pub path_lifetime: super::Lifetime,
}

//! One topology sampler with named, hard-coded experiment presets.

mod fpoft;
pub mod presets;
pub use fpoft::FPOFTSampler;
mod topology;
pub use topology::{ConnectionMode, FixedTopology, LayerConfig, NodeLifetime, NodeRotation};

use super::{Observation, TimeBasedPathSampler};
use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;

/// All data identifying one experiment. Runtime behavior is shared by every preset.
#[derive(Clone, Copy)]
pub struct TopologyExperiment {
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
    pub fn new(experiment: TopologyExperiment, mixnet: &Mixnet) -> Self {
        Self {
            topology: FixedTopology::new(experiment.layers, experiment.connections, mixnet),
            name: experiment.name,
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
pub struct FPOFTExperiment {
    pub name: &'static str,
    /// Reuses layer sizes and connections; node lifetimes are disabled in FPOFT.
    pub topology_experiment: TopologyExperiment,
    pub num_paths: usize,
    /// Every active path independently draws from this lifetime at each rotation.
    pub path_lifetime: super::Lifetime,
}

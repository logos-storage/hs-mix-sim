//! Active paths over permanent topology nodes and connections.
//! Only paths have lifetimes in this sampler; topology node rotation is disabled.

use super::{FPOFTExperiment, FixedTopology, LayerConfig};
use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;
use crate::time_based_path_sampler::{Lifetime, Observation, TimeBasedPathSampler};
use rand::rngs::SmallRng;
use rand::seq::{SliceRandom, index};
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy)]
pub struct FPOFTEvent {
    slot: usize,
    expires_at: u64,
}

struct ActivePath {
    route_index: usize,
    expires_at: Option<u64>,
}

pub struct FPOFTSampler {
    topology: FixedTopology,
    #[allow(dead_code)]
    name: &'static str,
    /// All routes through the permanent topology; sampled paths store only indices.
    possible_paths: Vec<Vec<MixNode>>,
    active_paths: Vec<ActivePath>,
    path_lifetime: Lifetime,
    pending_events: Vec<(u64, FPOFTEvent)>,
    current_time: u64,
    rng: SmallRng,
}

impl FPOFTSampler {
    pub fn new(experiment: FPOFTExperiment, mixnet: &Mixnet) -> Self {
        assert!(
            experiment.num_paths > 0,
            "active path count must be positive"
        );
        experiment.path_lifetime.validate();
        let config = experiment.topology_experiment;
        // Reuse layer sizes and links, but disable every topology node lifetime.
        let layers: Vec<_> = config
            .layers
            .iter()
            .map(|layer| LayerConfig::new(layer.node_count, Lifetime::Never))
            .collect();
        let topology = FixedTopology::new(&layers, config.connections, mixnet);
        let possible_paths = topology.paths();
        assert!(
            experiment.num_paths <= possible_paths.len(),
            "active path count exceeds the topology's distinct path count"
        );
        let mut sampler = Self {
            topology,
            name: experiment.name,
            possible_paths,
            active_paths: Vec::with_capacity(experiment.num_paths),
            path_lifetime: experiment.path_lifetime,
            pending_events: Vec::new(),
            current_time: 0,
            rng: SmallRng::from_entropy(),
        };
        // Draw a subset without replacement; each slot has its own expiry draw.
        let selected = index::sample(
            &mut sampler.rng,
            sampler.possible_paths.len(),
            experiment.num_paths,
        );
        for (slot, route_index) in selected.into_iter().enumerate() {
            let path = sampler.make_active_path(slot, route_index, 0);
            sampler.active_paths.push(path);
        }
        sampler
    }

    fn make_active_path(&mut self, slot: usize, route_index: usize, time: u64) -> ActivePath {
        let expires_at = self.path_lifetime.sample_expiration(time, &mut self.rng);
        if let Some(expires_at) = expires_at {
            self.pending_events
                .push((expires_at, FPOFTEvent { slot, expires_at }));
        }
        ActivePath {
            route_index,
            expires_at,
        }
    }
}

impl PathSampler for FPOFTSampler {
    fn sample_path(&mut self, _mixnet: &Mixnet) -> Vec<MixNode> {
        let path = &self.active_paths[self.rng.gen_range(0..self.active_paths.len())];
        self.possible_paths[path.route_index].clone()
    }

    fn hops(&self) -> usize {
        self.topology.hops()
    }
    fn sampler_type(&self) -> &'static str {
        self.name
    }
}

impl TimeBasedPathSampler for FPOFTSampler {
    type Event = FPOFTEvent;

    fn next_events(&mut self, current_time: u64) -> Vec<(u64, Self::Event)> {
        assert!(
            current_time >= self.current_time,
            "sampler time cannot move backwards"
        );
        assert!(
            self.pending_events
                .iter()
                .all(|(time, _)| *time >= current_time),
            "path events must be collected before they expire"
        );
        self.current_time = current_time;
        std::mem::take(&mut self.pending_events)
    }

    fn handle_event(&mut self, current_time: u64, event: Self::Event, _mixnet: &Mixnet) {
        assert!(
            current_time >= self.current_time,
            "sampler time cannot move backwards"
        );
        let FPOFTEvent { slot, expires_at } = event;
        let Some(path) = self.active_paths.get(slot) else {
            return;
        };
        if path.expires_at != Some(expires_at) {
            return;
        }
        assert_eq!(
            current_time, expires_at,
            "path rotation must run at its scheduled time"
        );
        // Exclude routes used by other slots. The expired route can be drawn
        // again, including when the pool already contains every possible route.
        let mut available = vec![true; self.possible_paths.len()];
        for (other_slot, path) in self.active_paths.iter().enumerate() {
            if other_slot != slot {
                available[path.route_index] = false;
            }
        }
        let candidates: Vec<_> = available
            .iter()
            .enumerate()
            .filter_map(|(route, &free)| free.then_some(route))
            .collect();
        let route_index = *candidates.choose(&mut self.rng).unwrap();
        self.active_paths[slot] = self.make_active_path(slot, route_index, current_time);
        self.current_time = current_time;
    }

    fn peak(&self, chain: &[MixId]) -> Observation {
        if chain.len() > self.topology.hops() {
            return Observation::Unavailable;
        }
        let mut next_nodes = Vec::new();
        for active in &self.active_paths {
            let mut toward_sender = self.possible_paths[active.route_index].iter().rev();
            if chain
                .iter()
                .all(|id| toward_sender.next().is_some_and(|node| node.mix_id == *id))
            {
                if let Some(next) = toward_sender.next() {
                    next_nodes.push(next.mix_id);
                } else {
                    return Observation::ServiceIdentified;
                }
            }
        }
        next_nodes.sort_unstable();
        next_nodes.dedup();
        if next_nodes.is_empty() {
            Observation::Unavailable
        } else {
            Observation::Nodes(next_nodes)
        }
    }

    fn node(&self, mix_id: MixId) -> Option<&MixNode> {
        self.topology.node(mix_id)
    }
}
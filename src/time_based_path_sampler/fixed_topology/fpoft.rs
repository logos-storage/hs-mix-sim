//! Active Paths Over Fixed Topology (APOFT). 
//! Paths are just connections in the topology between node slots and 
//! these connections stay fixed, but node occupants may rotate.

use super::{FPOFTProfile, FixedTopology, NodeRotation};
use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;
use crate::time_based_path_sampler::{Lifetime, Observation, TimeBasedPathSampler};
use rand::rngs::SmallRng;
use rand::seq::{SliceRandom, index};
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy)]
pub enum FPOFTEvent {
    PathRotation { slot: usize, expires_at: u64 },
    NodeRotation(NodeRotation),
}

struct ActivePath {
    route_index: usize,
    expires_at: Option<u64>,
}

pub struct FPOFTSampler {
    topology: FixedTopology,
    #[allow(dead_code)]
    name: &'static str,
    /// Complete routes contain stable layer-local slots, no cached node identities.
    possible_paths: Vec<Vec<usize>>,
    active_paths: Vec<ActivePath>,
    path_lifetime: Lifetime,
    pending_events: Vec<(u64, FPOFTEvent)>,
    current_time: u64,
    rng: SmallRng,
}

impl FPOFTSampler {
    pub fn new(profile: FPOFTProfile, mixnet: &Mixnet) -> Self {
        assert!(profile.num_paths > 0, "active path count must be positive");
        profile.path_lifetime.validate();
        let config = profile.topology_profile;
        let topology = FixedTopology::new(config.layers, config.connections, mixnet);
        let possible_paths = topology.routes();
        assert!(
            profile.num_paths <= possible_paths.len(),
            "active path count exceeds the topology's distinct path count"
        );
        let mut sampler = Self {
            topology,
            name: profile.name,
            possible_paths,
            active_paths: Vec::with_capacity(profile.num_paths),
            path_lifetime: profile.path_lifetime,
            pending_events: Vec::new(),
            current_time: 0,
            rng: SmallRng::from_entropy(),
        };
        // Draw a subset without replacement; each slot has its own expiry draw.
        let selected = index::sample(
            &mut sampler.rng,
            sampler.possible_paths.len(),
            profile.num_paths,
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
                .push((expires_at, FPOFTEvent::PathRotation { slot, expires_at }));
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
        self.topology
            .resolve_route(&self.possible_paths[path.route_index])
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
        let mut events = std::mem::take(&mut self.pending_events);
        events.extend(
            self.topology
                .next_events(current_time)
                .into_iter()
                .map(|(time, event)| (time, FPOFTEvent::NodeRotation(event))),
        );
        assert!(events.iter().all(|(time, _)| *time >= current_time));
        events
    }

    fn handle_event(&mut self, current_time: u64, event: Self::Event, mixnet: &Mixnet) {
        assert!(
            current_time >= self.current_time,
            "sampler time cannot move backwards"
        );
        let (slot, expires_at) = match event {
            FPOFTEvent::PathRotation { slot, expires_at } => (slot, expires_at),
            FPOFTEvent::NodeRotation(event) => {
                self.topology.handle_event(current_time, event, mixnet);
                self.current_time = current_time;
                return;
            }
        };
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
            let mut toward_sender = self.possible_paths[active.route_index]
                .iter()
                .enumerate()
                .rev()
                .map(|(layer, &slot)| self.topology.node_at(layer, slot));
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

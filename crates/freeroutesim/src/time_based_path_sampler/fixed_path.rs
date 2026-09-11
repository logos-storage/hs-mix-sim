//! A pool of fixed paths with independent, event-driven rotation.
//!
//! Construct at time zero, enqueue `next_events(0)`, and deliver each rotation
//! through `handle_event` with the run's static mixnet. Drain `next_events`
//! after handling events to enqueue replacements. Path requests select from the
//! pool; they do not advance time or rotate paths themselves.

use super::{HopBehavior, Observation, TimeBasedPathSampler};
use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;
use rand::rngs::SmallRng;
use rand::seq::index;
use rand::{Rng, SeedableRng};

pub const FIXED_PATH_COUNT: usize = 5;
const MIN_LIFETIME_SECONDS: u64 = 60 * 60;
const MAX_LIFETIME_SECONDS: u64 = 48 * 60 * 60;

/// Identifies one path incarnation, so a duplicate event cannot rotate its replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathRotation {
    path_index: usize,
    expires_at: u64,
}

struct StoredPath {
    /// Mix hops ordered from sender to recipient.
    nodes: Vec<MixNode>,
    expires_at: u64,
}

pub struct FixedPathSampler {
    hops: usize,
    paths: Vec<StoredPath>,
    pending_events: Vec<(u64, PathRotation)>,
    current_time: u64,
    rng: SmallRng,
}

impl FixedPathSampler {
    /// Initialize five independently sampled paths at time zero. Nodes within
    /// each path are distinct; different paths may share nodes or be identical.
    pub fn new(hops: usize, mixnet: &Mixnet) -> Self {
        Self::with_rng(hops, mixnet, SmallRng::from_entropy())
    }

    fn with_rng(hops: usize, mixnet: &Mixnet, rng: SmallRng) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        let mut sampler = Self {
            hops,
            paths: Vec::with_capacity(FIXED_PATH_COUNT),
            pending_events: Vec::with_capacity(FIXED_PATH_COUNT),
            current_time: 0,
            rng,
        };
        for path_index in 0..FIXED_PATH_COUNT {
            let path = sampler.make_path(0, mixnet);
            sampler.queue_rotation(path_index, path.expires_at);
            sampler.paths.push(path);
        }
        sampler
    }

    fn make_path(&mut self, current_time: u64, mixnet: &Mixnet) -> StoredPath {
        let nodes = mixnet.nodes();
        assert!(self.hops <= nodes.len(), "not enough mix nodes for a path");
        let nodes = index::sample(&mut self.rng, nodes.len(), self.hops)
            .into_iter()
            .map(|i| nodes[i].clone())
            .collect();
        let expires_at = current_time
            .checked_add(sample_lifetime(&mut self.rng))
            .expect("path expiration timestamp overflowed");
        StoredPath { nodes, expires_at }
    }

    fn queue_rotation(&mut self, path_index: usize, expires_at: u64) {
        self.pending_events.push((
            expires_at,
            PathRotation {
                path_index,
                expires_at,
            },
        ));
    }
}

/// Maximum of two independent uniform draws (MAX(X,X') where X and X' are sampled uniformly)
fn sample_lifetime(rng: &mut impl Rng) -> u64 {
    let first = rng.gen_range(MIN_LIFETIME_SECONDS..=MAX_LIFETIME_SECONDS);
    let second = rng.gen_range(MIN_LIFETIME_SECONDS..=MAX_LIFETIME_SECONDS);
    first.max(second)
}

impl PathSampler for FixedPathSampler {
    fn sample_path(&mut self, _mixnet: &Mixnet) -> Vec<MixNode> {
        let path_index = self.rng.gen_range(0..self.paths.len());
        self.paths[path_index].nodes.clone()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "FixedPathSampler"
    }
}

impl TimeBasedPathSampler for FixedPathSampler {
    type Event = PathRotation;

    fn next_events(&mut self, current_time: u64) -> Vec<(u64, Self::Event)> {
        assert!(
            current_time >= self.current_time,
            "sampler time cannot move backwards"
        );
        assert!(
            self.pending_events
                .iter()
                .all(|(time, _)| *time >= current_time),
            "sampler events must be collected before they expire"
        );
        self.current_time = current_time;
        std::mem::take(&mut self.pending_events)
    }

    fn handle_event(&mut self, current_time: u64, event: Self::Event, mixnet: &Mixnet) {
        assert!(
            current_time >= self.current_time,
            "sampler time cannot move backwards"
        );
        if self.paths[event.path_index].expires_at != event.expires_at {
            return;
        }
        assert_eq!(
            current_time, event.expires_at,
            "rotation must run at its scheduled time"
        );
        let replacement = self.make_path(current_time, mixnet);
        self.queue_rotation(event.path_index, replacement.expires_at);
        self.paths[event.path_index] = replacement;
        self.current_time = current_time;
    }

    fn hop_behavior(&self, hop: usize) -> HopBehavior {
        assert!(hop < self.hops, "hop index is outside the path");
        HopBehavior::Fixed
    }

    fn peak(&self, node_chain: &[MixId]) -> Observation {
        if node_chain.len() > self.hops {
            return Observation::Unavailable;
        }
        let mut next_nodes = Vec::new();
        for path in &self.paths {
            let mut toward_sender = path.nodes.iter().rev();
            if node_chain
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
        self.paths
            .iter()
            .flat_map(|path| &path.nodes)
            .find(|node| node.mix_id == mix_id)
    }
}

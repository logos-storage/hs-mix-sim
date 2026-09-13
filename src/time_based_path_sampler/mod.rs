//! Time-based path samplers.

mod lifetime;
pub use lifetime::Lifetime;

pub mod fixed_path;
pub mod fixed_topology;

use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::path_sampler::PathSampler;

/// An observed, contiguous chain of mix IDs ordered from recipient toward sender.
/// The first ID is an exit; each appended ID is the next hop toward the service.
/// For example, `[exit, middle, entry]` describes a complete three-hop walk.
/// An empty chain requests the current exits. Keeping the full chain distinguishes
/// paths that share a mix at the same hop. This order is the reverse of sampled paths.
pub type PathChain = Vec<MixId>;

/// What an adversary can observe by following a chain toward the sender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    Unavailable,
    /// next mixes in the chain to explore, each extending the queried chain.
    Nodes(Vec<MixId>),
    ServiceIdentified,
}

pub trait TimeBasedPathSampler: PathSampler {
    /// Sampler-specific event, independent of the scheduling queue.
    type Event;

    /// Return new events to schedule as `(absolute_time_seconds, event)` pairs.
    ///
    /// Call at initialization with time zero, then after processing events that
    /// may change the sampler's schedule. Timestamps must be at least
    /// `current_time`; an empty return vector means there are no new events to schedule.
    /// Implementations must not return events already handed to the scheduler again
    fn next_events(&mut self, current_time: u64) -> Vec<(u64, Self::Event)>;

    /// Process a scheduled sampler event using the run's static mixnet.
    /// should be called before requesting paths at `current_time`.
    fn handle_event(&mut self, current_time: u64, event: Self::Event, mixnet: &Mixnet);

    /// Follow an observed chain from the recipient toward the sender, starting
    /// at the last mix hop. An empty chain observes the current exits. A partial
    /// chain observes the next adjacent mix IDs on paths matching the entire
    /// chain. Returned mix IDs must be unique. A complete
    /// matching chain identifies the service; an unknown chain is unavailable.
    fn peak(&self, node_chain: &[MixId]) -> Observation;

    /// Metadata for an observable node, including its initial malicious flag.
    /// this is mainly because we store ids instead of full node metadata.
    fn node(&self, mix_id: MixId) -> Option<&MixNode>;
}

//! Time-aware extensions to path sampling.

pub mod fixed_path;

use crate::path_sampler::PathSampler;
use crate::topologygen::{MixId, MixNode, Topology};

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
    /// Alternative next mixes to explore, each extending the queried chain.
    Nodes(Vec<MixId>),
    ServiceIdentified,
}

pub trait TimeBasedPathSampler: PathSampler {
    /// Sampler-specific event payload, independent of the scheduling queue.
    type Event;

    /// Return new events to schedule as `(absolute_time_seconds, event)` pairs.
    ///
    /// Call at initialization with time zero, then after processing events that
    /// may change the sampler's schedule. Timestamps must be at least
    /// `current_time`; an empty vector means there are no new events to schedule.
    /// Implementations must not return events already handed to the scheduler
    /// again. Vector order determines insertion order for simultaneous events.
    fn next_events(&mut self, current_time: u64) -> Vec<(u64, Self::Event)>;

    /// Process a scheduled sampler event using the topology at its timestamp.
    /// Process events at a timestamp before requesting paths at that time.
    fn handle_event(
        &mut self,
        current_time: u64,
        event: Self::Event,
        topology_index: usize,
        topology: &Topology,
    );

    /// Check the behavior of a particular hop.
    fn hop_behavior(&self, hop: usize) -> HopBehavior;

    /// Follow an observed chain from the recipient toward the sender, starting
    /// at the last mix hop. An empty chain observes the current exits. A partial
    /// chain observes the next adjacent mix IDs on paths matching the entire
    /// chain. Returned mix IDs must be unique and ordered deterministically. A complete
    /// matching chain identifies the service; an unknown chain is unavailable.
    /// The slice borrows a [`PathChain`] without requiring an owned vector.
    fn peak(&self, node_chain: &[MixId]) -> Observation;

    /// Metadata for an observable node, including its initial malicious flag.
    /// Stored paths retain their metadata until they rotate, even if a node is
    /// absent from a newer topology snapshot.
    fn node(&self, mix_id: MixId) -> Option<&MixNode>;
}

/// Whether a hop is random or fixed for some lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopBehavior {
    Random,
    Fixed,
}

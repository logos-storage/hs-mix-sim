//! Time-aware extensions to path sampling.

pub mod fixed_path;

use crate::path_sampler::PathSampler;
use crate::topologygen::Topology;

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
    /// at the last mix hop. Return only the next adjacent mix IDs on paths
    /// matching the entire chain, without duplicates. An empty, unknown, or
    /// complete chain has no next mix hop.
    fn peak(&self, node_chain: &[u32]) -> Vec<u32>;
}

/// Whether a hop is random or fixed for some lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopBehavior {
    Random,
    Fixed,
}

/// Defines when the node with this mix ID should expire, in simulated seconds.
#[derive(Debug, Default)]
pub struct RotatingNode {
    mix_id: u32,
    expires_at: u64,
}

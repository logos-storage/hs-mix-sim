//! Time-aware extensions to path sampling.

use crate::path_sampler::PathSampler;

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

    /// Check the behavior of a particular hop.
    fn hop_behavior(&self, hop: usize) -> HopBehavior;

    /// Peek at the next hop toward the sender, returning the mix IDs connected
    /// to `mix_node` at `hop`.
    fn peak(&self, hop: usize, mix_node: u32) -> Vec<u32>;
}

/// Whether a hop is random or fixed for some lifetime.
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

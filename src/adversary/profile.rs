//! Defines the outcome and duration of an attempt to compromise one node.
//!
//! An adversary profile specifies the cumulative probability of success at some
//! deadlines, measured from the start of the attempt. The walker samples the
//! outcome once per targeted node and stores it; checking the node again does
//! not resample the outcome or restart its timer.

use rand::Rng;

/// The probability that an attempt has succeeded by a given elapsed time.
///
/// Probabilities include success at all earlier milestones. For example, 75%
/// by day 14 after 50% by day 7 adds a 25% chance of success in the second interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompromiseMilestone {
    /// Deadline in seconds after the attempt starts.
    pub within_seconds: u64,
    /// Total probability of success by this deadline, between 0.0 and 1.0.
    pub cumulative_probability: f64,
}

/// Define rules for sampling whether an attempt succeeds and how long it takes.
///
/// For example, 50% by day 7 and 75% by day 14 means:
/// - 50% succeed between second 1 and the end of day 7.
/// - 25% succeed between the next second and the end of day 14.
/// - 25% never succeed.
///
/// All times are relative to the start of the attempt, not the simulation start.
/// An empty profile never succeeds, as needed by the Sybil-only adversary.
#[derive(Debug, Clone)]
pub struct CompromiseProfile {
    milestones: Vec<CompromiseMilestone>,
}

impl CompromiseProfile {
    /// Build a profile from milestones supplied in increasing time order.
    pub fn new(milestones: Vec<CompromiseMilestone>) -> Result<Self, &'static str> {
        let mut previous_time = 0;
        let mut previous_probability = 0.0;
        for milestone in &milestones {
            if milestone.within_seconds <= previous_time {
                return Err("compromise milestone times must be positive and increasing");
            }
            if !milestone.cumulative_probability.is_finite()
                || !(previous_probability..=1.0).contains(&milestone.cumulative_probability)
            {
                return Err("compromise probabilities must be cumulative and between zero and one");
            }
            previous_time = milestone.within_seconds;
            previous_probability = milestone.cumulative_probability;
        }
        Ok(Self { milestones })
    }

    /// Sample one attempt's outcome: a delay in seconds, or `None` for permanent failure.
    pub(super) fn sample_delay<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<u64> {
        // The first cumulative threshold above this draw selects the interval.
        let draw = rng.r#gen::<f64>();
        let mut previous_time = 0;
        for milestone in &self.milestones {
            if draw < milestone.cumulative_probability {
                return Some(rng.gen_range((previous_time + 1)..=milestone.within_seconds));
            }
            previous_time = milestone.within_seconds;
        }
        // A draw outside every success interval means this attempt never succeeds.
        None
    }
}

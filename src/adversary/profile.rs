use rand::Rng;

/// A cumulative chance of success by a duration after first discovery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompromiseMilestone {
    pub within_seconds: u64,
    pub cumulative_probability: f64,
}

/// An inclusive success-time interval and its unconditional probability mass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompromiseInterval {
    pub min_seconds: u64,
    pub max_seconds: u64,
    pub probability: f64,
}

/// A piecewise-uniform compromise-time distribution with optional permanent
/// failure. For example, 50% by day 7 and 75% by day 14 assigns 50% to days 0–7,
/// another 25% to days 7–14, and 25% to never succeeding.
#[derive(Debug, Clone)]
pub struct CompromiseProfile {
    milestones: Vec<CompromiseMilestone>,
}

impl CompromiseProfile {
    /// Times must be positive and strictly increasing. Probabilities must be
    /// finite, nondecreasing, and between zero and one. An empty profile never
    /// succeeds. Durations and draws use whole simulated seconds.
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

    /// Success intervals for reporting the exact sampled profile. Zero-probability
    /// plateaus are omitted; remaining probability means permanent failure.
    pub fn intervals(&self) -> Vec<CompromiseInterval> {
        let mut intervals = Vec::new();
        let (mut previous_time, mut previous_probability) = (0, 0.0);
        for milestone in &self.milestones {
            let probability = milestone.cumulative_probability - previous_probability;
            if probability > 0.0 {
                intervals.push(CompromiseInterval {
                    min_seconds: previous_time + 1,
                    max_seconds: milestone.within_seconds,
                    probability,
                });
            }
            previous_time = milestone.within_seconds;
            previous_probability = milestone.cumulative_probability;
        }
        intervals
    }

    pub(super) fn sample_delay<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<u64> {
        let draw = rng.r#gen::<f64>();
        let mut previous_time = 0;
        for milestone in &self.milestones {
            if draw < milestone.cumulative_probability {
                return Some(rng.gen_range((previous_time + 1)..=milestone.within_seconds));
            }
            previous_time = milestone.within_seconds;
        }
        None
    }
}

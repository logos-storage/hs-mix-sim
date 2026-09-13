//! compromise profiles sharing the existing persistent path walker.

use super::{Adversary, CompromiseMilestone, CompromiseProfile, PathWalker};

const DAY: u64 = 24 * 60 * 60;

/// One concrete type for the CLI's adversaries, independent of sampler choice.
#[derive(Debug)]
pub struct HiddenServiceAdversary {
    walker: PathWalker,
    profile: CompromiseProfile,
}

impl HiddenServiceAdversary {
    /// 50% succeed by day 15; the remainder never succeed.
    pub fn basic() -> Self {
        Self::from_milestones(&[(15 * DAY, 0.5)])
    }

    /// Only initially malicious nodes are controlled; no compromise events.
    pub fn sybil_only() -> Self {
        Self::from_milestones(&[])
    }

    /// 75% by day 15; the remaining 25% succeed by day 30.
    pub fn apt() -> Self {
        Self::from_milestones(&[(15 * DAY, 0.75), (30 * DAY, 1.0)])
    }

    /// 50% by day 2, another 25% by day 7, and 25% never succeed.
    pub fn fvey() -> Self {
        Self::from_milestones(&[(2 * DAY, 0.5), (7 * DAY, 0.75)])
    }

    /// 50% succeed uniformly between day 2 and day 14, including both endpoints.
    pub fn rubberhose1() -> Self {
        Self::between_days(2, 14)
    }

    /// 50% succeed uniformly between day 7 and day 21, including both endpoints.
    pub fn rubberhose2() -> Self {
        Self::between_days(7, 21)
    }

    fn between_days(min: u64, max: u64) -> Self {
        // A zero-probability plateau ends one second before the inclusive minimum.
        Self::from_milestones(&[(min * DAY - 1, 0.0), (max * DAY, 0.5)])
    }

    fn from_milestones(milestones: &[(u64, f64)]) -> Self {
        let profile = CompromiseProfile::new(
            milestones
                .iter()
                .map(
                    |&(within_seconds, cumulative_probability)| CompromiseMilestone {
                        within_seconds,
                        cumulative_probability,
                    },
                )
                .collect(),
        )
        .expect("built-in adversary profile must be valid");
        Self {
            walker: PathWalker::default(),
            profile,
        }
    }
}

impl Adversary for HiddenServiceAdversary {
    fn walker(&self) -> &PathWalker {
        &self.walker
    }
    fn walker_mut(&mut self) -> &mut PathWalker {
        &mut self.walker
    }
    fn compromise_profile(&self) -> &CompromiseProfile {
        &self.profile
    }
}

use super::{Adversary, CompromiseMilestone, CompromiseProfile, PathWalker};

pub const COMPROMISE_WINDOW_SECONDS: u64 = 15 * 24 * 60 * 60;

/// Uses existing Sybil control and attempts to compromise each discovered
/// honest node once: a 50% chance of success uniformly within 15 days,
/// otherwise permanent failure.
#[derive(Debug)]
pub struct BasicAdversary {
    walker: PathWalker,
    profile: CompromiseProfile,
}

impl Default for BasicAdversary {
    fn default() -> Self {
        Self::with_walker(PathWalker::default())
    }
}

impl BasicAdversary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_seed(seed: u64) -> Self {
        Self::with_walker(PathWalker::with_seed(seed))
    }

    fn with_walker(walker: PathWalker) -> Self {
        Self {
            walker,
            profile: CompromiseProfile::new(vec![CompromiseMilestone {
                within_seconds: COMPROMISE_WINDOW_SECONDS,
                cumulative_probability: 0.5,
            }])
            .expect("basic adversary has a valid compromise profile"),
        }
    }
}

impl Adversary for BasicAdversary {
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

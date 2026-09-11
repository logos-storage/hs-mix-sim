use super::{Adversary, CompromiseProfile, PathWalker};

/// Walks hidden-service paths using only mixes that are initially malicious.
/// The empty compromise profile always fails, so discovered honest mixes are
/// recorded as NeverSucceeds and no compromise completion events are scheduled.
#[derive(Debug)]
pub struct SybilOnlyAdversary {
    walker: PathWalker,
    profile: CompromiseProfile,
}

impl Default for SybilOnlyAdversary {
    fn default() -> Self {
        Self {
            walker: PathWalker::default(),
            profile: CompromiseProfile::new(Vec::new())
                .expect("an empty compromise profile is valid"),
        }
    }
}

impl SybilOnlyAdversary {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Adversary for SybilOnlyAdversary {
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

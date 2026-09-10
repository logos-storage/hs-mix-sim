use std::collections::HashMap;

use rand::{SeedableRng, rngs::SmallRng};

use super::CompromiseProfile;
use crate::topologygen::{MixId, MixNode};

/// Exactly one persistent outcome per node that required compromise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompromiseAttempt {
    Pending { completes_at: u64 },
    NeverSucceeds,
    Compromised { completed_at: u64 },
}

/// A completion event identifies the node and its scheduled completion time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompromiseEvent {
    pub mix_id: MixId,
    pub completes_at: u64,
}

/// Persistent control and attempts, independent of the sampler's rotations.
#[derive(Debug)]
pub struct PathWalker {
    attempts: HashMap<MixId, CompromiseAttempt>,
    events: Vec<(u64, CompromiseEvent)>,
    rng: SmallRng,
    compromises_done: u64,
    won: bool,
}

impl Default for PathWalker {
    fn default() -> Self {
        Self::with_rng(SmallRng::from_entropy())
    }
}

impl PathWalker {
    pub fn with_seed(seed: u64) -> Self {
        Self::with_rng(SmallRng::seed_from_u64(seed))
    }

    fn with_rng(rng: SmallRng) -> Self {
        Self {
            attempts: HashMap::new(),
            events: Vec::new(),
            rng,
            compromises_done: 0,
            won: false,
        }
    }

    pub fn attempts(&self) -> &HashMap<MixId, CompromiseAttempt> {
        &self.attempts
    }

    pub fn is_controlled(&self, node: &MixNode) -> bool {
        node.is_malicious
            || matches!(
                self.attempts.get(&node.mix_id),
                Some(CompromiseAttempt::Compromised { .. })
            )
    }

    pub fn compromises_done(&self) -> u64 {
        self.compromises_done
    }

    pub fn has_won(&self) -> bool {
        self.won
    }

    pub(super) fn mark_won(&mut self) {
        self.won = true;
    }

    pub(super) fn attempt_compromise(
        &mut self,
        mix_id: MixId,
        current_time: u64,
        profile: &CompromiseProfile,
    ) {
        if self.attempts.contains_key(&mix_id) {
            return;
        }

        let Some(delay) = profile.sample_delay(&mut self.rng) else {
            self.attempts
                .insert(mix_id, CompromiseAttempt::NeverSucceeds);
            return;
        };
        let completes_at = current_time
            .checked_add(delay)
            .expect("compromise completion exceeds u64 simulated seconds");
        self.attempts
            .insert(mix_id, CompromiseAttempt::Pending { completes_at });
        self.events.push((
            completes_at,
            CompromiseEvent {
                mix_id,
                completes_at,
            },
        ));
    }

    pub(super) fn next_events(&mut self, current_time: u64) -> Vec<(u64, CompromiseEvent)> {
        assert!(
            self.events.iter().all(|(time, _)| *time >= current_time),
            "compromise events must be collected before their scheduled time"
        );
        std::mem::take(&mut self.events)
    }

    pub(super) fn handle_event(&mut self, current_time: u64, event: CompromiseEvent) {
        if current_time < event.completes_at {
            return;
        }
        let Some(attempt) = self.attempts.get_mut(&event.mix_id) else {
            return;
        };
        if *attempt
            != (CompromiseAttempt::Pending {
                completes_at: event.completes_at,
            })
        {
            return;
        }
        *attempt = CompromiseAttempt::Compromised {
            completed_at: event.completes_at,
        };
        self.compromises_done += 1;
    }
}

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};

use super::{CompromiseBudget, CompromiseProfile};
use crate::mixnet::{MixId, MixNode};

/// possible compromise results
/// one outcome per node that required compromise.
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

/// path walker state
#[derive(Debug)]
pub struct PathWalker {
    attempts: HashMap<MixId, CompromiseAttempt>,
    budget: CompromiseBudget,
    attempts_by_layer: Vec<usize>,
    events: Vec<(u64, CompromiseEvent)>,
    rng: SmallRng,
    compromises_done: u64,
    has_won: bool,
}

impl Default for PathWalker {
    fn default() -> Self {
        Self::new(crate::params::COMPROMISE_BUDGET_PER_LAYER)
    }
}

impl PathWalker {
    pub fn new(budget: CompromiseBudget) -> Self {
        Self::with_rng(budget, SmallRng::from_entropy())
    }

    fn with_rng(budget: CompromiseBudget, rng: SmallRng) -> Self {
        Self {
            attempts: HashMap::new(),
            budget,
            attempts_by_layer: Vec::new(),
            events: Vec::new(),
            rng,
            compromises_done: 0,
            has_won: false,
        }
    }

    pub fn budget(&self) -> CompromiseBudget {
        self.budget
    }

    /// Uniform sampling without replacement among currently discovered nodes.
    /// Only nodes never previously attempted are eligible
    /// Initial Sybils are filtered by the walk.
    pub(super) fn attempt_discovered(
        &mut self,
        discovered: BTreeMap<usize, BTreeSet<MixId>>,
        current_time: u64,
        profile: &CompromiseProfile,
    ) {
        for (layer, nodes) in discovered {
            assert!(layer > 0, "layers are numbered from one");
            let used = self.attempts_by_layer.get(layer - 1).copied().unwrap_or(0);
            let remaining = self.budget.remaining(used);
            if remaining == 0 {
                continue;
            }
            let candidates: Vec<_> = nodes
                .into_iter()
                .filter(|id| !self.attempts.contains_key(id))
                .collect();
            let count = remaining.min(candidates.len());
            // Unlimited (or large) budget attacks every candidate. Otherwise
            // draw the complete subset before sampling any compromise outcomes.
            let selected = if count == candidates.len() {
                candidates
            } else {
                candidates
                    .choose_multiple(&mut self.rng, count)
                    .copied()
                    .collect()
            };
            for mix_id in selected {
                self.attempt_compromise(mix_id, layer, current_time, profile);
            }
        }
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
        self.has_won
    }

    pub(super) fn mark_won(&mut self) {
        self.has_won = true;
    }

    /// attempt to compromise a given mix node
    pub(super) fn attempt_compromise(
        &mut self,
        mix_id: MixId,
        layer: usize,
        current_time: u64,
        profile: &CompromiseProfile,
    ) {
        if self.attempts.contains_key(&mix_id) {
            return;
        }

        assert!(layer > 0, "layers are numbered from one");
        let used = self.attempts_by_layer.get(layer - 1).copied().unwrap_or(0);
        if self.budget.remaining(used) == 0 {
            return;
        }
        self.attempts_by_layer
            .resize(self.attempts_by_layer.len().max(layer), 0);

        self.attempts_by_layer[layer - 1] += 1;
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

//! Adversaries for sampled paths and event-driven hidden-service discovery.

pub mod basic;
pub mod sybil_only;
mod path_walker;
mod profile;

pub use path_walker::{ CompromiseEvent, PathWalker};
pub use profile::{CompromiseMilestone, CompromiseProfile};

use std::collections::{HashSet, VecDeque};

use crate::time_based_path_sampler::{Observation, PathChain, TimeBasedPathSampler};
use crate::mixnet::MixNode;

/// Defines when an adversary wins on a single sampled path.
pub trait PathAdversary {
    fn wins(&mut self, path: &[MixNode]) -> bool;
}

/// Wins when every node on a non-empty path is malicious.
#[derive(Debug, Default)]
pub struct SybilAdversary;

impl PathAdversary for SybilAdversary {
    fn wins(&mut self, path: &[MixNode]) -> bool {
        !path.is_empty() && path.iter().all(|node| node.is_malicious)
    }
}

/// Persistent hidden-service discovery with an implementation-defined
/// cumulative probability of compromise over time.
///
/// Implementations supply their state and compromise profile. The shared
/// methods walk currently observable paths, remember every attempted node, and
/// produce completion events. Chains always run from recipient toward sender.
pub trait Adversary {
    fn walker(&self) -> &PathWalker;
    fn walker_mut(&mut self) -> &mut PathWalker;
    fn compromise_profile(&self) -> &CompromiseProfile;

    /// Revisit the current paths from their exits, using all control acquired
    /// so far. Traversal is fresh each time; compromise attempts persist.
    fn wins<S: TimeBasedPathSampler>(&mut self, current_time: u64, sampler: &S) -> bool {
        if self.walker().has_won() {
            return true;
        }

        let profile = self.compromise_profile().clone();
        let walker = self.walker_mut();
        let exits = match sampler.peak(&[]) {
            Observation::Unavailable => return false,
            Observation::ServiceIdentified => {
                walker.mark_won();
                return true;
            }
            Observation::Nodes(exits) => exits,
        };
        let mut queue: VecDeque<PathChain> = VecDeque::new();
        let mut seen: HashSet<PathChain> = HashSet::new();
        for exit in exits {
            let chain = vec![exit];
            if seen.insert(chain.clone()) {
                queue.push_back(chain);
            }
        }

        while let Some(chain) = queue.pop_front() {
            let mix_id = *chain.last().expect("queued chains contain a node");
            let Some(node) = sampler.node(mix_id) else {
                continue;
            };
            if !walker.is_controlled(node) {
                walker.attempt_compromise(mix_id, current_time, &profile);
                continue;
            }

            match sampler.peak(&chain) {
                Observation::Unavailable => {}
                Observation::ServiceIdentified => {
                    walker.mark_won();
                    return true;
                }
                Observation::Nodes(nodes) => {
                    for next in nodes {
                        let mut extended = chain.clone();
                        extended.push(next);
                        if seen.insert(extended.clone()) {
                            queue.push_back(extended);
                        }
                    }
                }
            }
        }
        false
    }

    /// Drain only newly scheduled completion events, in discovery order.
    fn next_events(&mut self, current_time: u64) -> Vec<(u64, CompromiseEvent)> {
        self.walker_mut().next_events(current_time)
    }

    /// Complete a matching pending attempt at its scheduled time. Duplicate,
    /// stale, early, and unknown events do not grant control.
    fn handle_event(&mut self, current_time: u64, event: CompromiseEvent) {
        self.walker_mut().handle_event(current_time, event);
    }

    /// Completed compromises, excluding nodes controlled by their Sybil flag.
    fn compromises_done(&self) -> u64 {
        self.walker().compromises_done()
    }
}

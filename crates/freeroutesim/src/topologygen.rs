//! Runtime generator for free-route topology snapshots.
//!
//! It models churn, malicious node placement, and an authority/consensus-chosen
//! guard pool.

use crate::params::{
    DEFAULT_CHURN_RATE, DEFAULT_EPOCHS, DEFAULT_GUARD_BANDWIDTH_FRACTION,
    DEFAULT_MALICIOUS_BANDWIDTH_FRACTION, DEFAULT_MALICIOUS_NODE_FRACTION, DEFAULT_MIX_SIZE,
};
use rand::prelude::*;
use rand_distr::LogNormal;
use rand_distr::weighted_alias::WeightedAliasIndex;

#[derive(Debug, Clone)]
pub struct TopologyConfig {
    /// if guard mode is on/off
    pub guard_mode: bool,
    /// number of mix nodes in the mixnet
    pub mix_size: usize,
    /// number of epochs
    pub epochs: u32,
    /// fraction and bandwidth of malicious nodes
    pub malicious_node_fraction: f64,
    pub malicious_bandwidth_fraction: f64,
    pub churn_rate: f64,
    /// bandwidth for the guard pool
    pub guard_bandwidth_fraction: f64,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfig {
            guard_mode: false,
            mix_size: DEFAULT_MIX_SIZE,
            epochs: DEFAULT_EPOCHS,
            malicious_node_fraction: DEFAULT_MALICIOUS_NODE_FRACTION,
            malicious_bandwidth_fraction: DEFAULT_MALICIOUS_BANDWIDTH_FRACTION,
            churn_rate: DEFAULT_CHURN_RATE,
            guard_bandwidth_fraction: DEFAULT_GUARD_BANDWIDTH_FRACTION,
        }
    }
}

/// generate topologies
pub struct TopologyGenerator {
    pub config: TopologyConfig,
}

impl Default for TopologyGenerator {
    fn default() -> Self {
        Self {
            config: TopologyConfig::default(),
        }
    }
}

/// tags on the mix nodes, this can be extended later for more tag
/// which would probably mean more accurate simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixNodeTag {
    Guard,
}

/// represent a single mix node 
#[derive(Debug, Clone)]
pub struct MixNode {
    pub weight: f64,
    pub mix_id: u32,
    pub is_malicious: bool,
    pub tags: Vec<MixNodeTag>,
}

impl MixNode {
    pub fn has_tag(&self, tag: MixNodeTag) -> bool {
        self.tags.contains(&tag)
    }

    fn add_tag(&mut self, tag: MixNodeTag) {
        if !self.has_tag(tag) {
            self.tags.push(tag);
        }
    }

    fn remove_tag(&mut self, tag: MixNodeTag) {
        self.tags.retain(|existing| *existing != tag);
    }
}

/// this is the same as mix node but used only by the generator to mark as online/offline
#[derive(Debug, Clone)]
struct GeneratedMix {
    node: MixNode,
    online: bool,
}

/// One generated free-route topology snapshot for an epoch.
#[derive(Clone)]
pub struct Topology {
    /// list of online/active mix nodes
    active: Vec<MixNode>,
    /// Cached bandwidth sampler shared by every user of this snapshot.
    active_sampler: Option<WeightedAliasIndex<f64>>,
}

impl Topology {
    /// New topology snapshot containing all nodes that are online in this epoch.
    pub fn new(active: Vec<MixNode>) -> Self {
        let active_sampler = if active.is_empty() {
            None
        } else {
            Some(
                WeightedAliasIndex::new(
                    active
                        .iter()
                        .map(|node| node.weight.max(f64::EPSILON))
                        .collect(),
                )
                .expect("active mix-node weights should produce a valid sampler"),
            )
        };
        Topology {
            active,
            active_sampler,
        }
    }

    pub(crate) fn active(&self) -> &[MixNode] {
        &self.active
    }

    pub(crate) fn sample_active<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&MixNode> {
        self.active_sampler
            .as_ref()
            .map(|sampler| &self.active[sampler.sample(rng)])
    }

    pub(crate) fn guards(&self) -> impl Iterator<Item = &MixNode> {
        self.active
            .iter()
            .filter(|node| node.has_tag(MixNodeTag::Guard))
    }

    pub fn is_active(&self, mix_id: u32) -> bool {
        self.active
            .iter()
            .any(|node| node.mix_id == mix_id)
    }
}

impl Default for Topology {
    fn default() -> Self {
        Topology::new(Vec::new())
    }
}

impl TopologyGenerator {
    /// new topology generator with given config
    pub fn new(config: TopologyConfig) -> Self {
        Self { config }
    }

    /// generate all `epochs` topologies at once
    pub fn generate_topologies(&self) -> Vec<Topology> {
        let mut rng = thread_rng();
        let mut mixes = self.initial_mixes(&mut rng);
        let mut consensus = Consensus::default();
        let mut topologies = Vec::with_capacity(self.config.epochs as usize);

        for _ in 0..self.config.epochs {
            let guards = if self.config.guard_mode {
                consensus.build_guard_set(&mixes, &self.config, &mut rng)
            } else {
                Vec::new()
            };

            topologies.push(snapshot(&mixes, &guards));
            advance_epoch(&mut mixes, self.config.churn_rate, &mut rng);
        }

        topologies
    }

    fn initial_mixes<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<GeneratedMix> {
        let bandwidth_distribution =
            LogNormal::new(2.0, 1.0).expect("log-normal bandwidth distribution should be valid");
        let mut mixes = Vec::with_capacity(self.config.mix_size);

        for mixid in 0..self.config.mix_size {
            let weight: f64 = bandwidth_distribution.sample(rng);
            mixes.push(GeneratedMix {
                node: MixNode {
                    mix_id: mixid as u32,
                    weight: weight.max(0.01),
                    is_malicious: false,
                    tags: Vec::new(),
                },
                online: true,
            });
        }

        self.mark_malicious_nodes(&mut mixes, rng);
        mixes
    }

    fn mark_malicious_nodes<R: Rng + ?Sized>(&self, mixes: &mut [GeneratedMix], rng: &mut R) {
        let target_nodes = ((mixes.len() as f64)
            * self.config.malicious_node_fraction.clamp(0.0, 1.0))
        .ceil() as usize;
        let total_bandwidth: f64 = mixes.iter().map(|mix| mix.node.weight).sum();
        let target_bandwidth =
            total_bandwidth * self.config.malicious_bandwidth_fraction.clamp(0.0, 1.0);
        let mut indices: Vec<usize> = (0..mixes.len()).collect();
        indices.shuffle(rng);
        let mut selected_nodes = 0;
        let mut selected_bandwidth = 0.0;

        for index in indices {
            if selected_nodes >= target_nodes && selected_bandwidth >= target_bandwidth {
                break;
            }

            mixes[index].node.is_malicious = true;
            selected_nodes += 1;
            selected_bandwidth += mixes[index].node.weight;
        }
    }
}

fn snapshot(mixes: &[GeneratedMix], guards: &[u32]) -> Topology {
    let active = mixes
        .iter()
        .filter(|mix| mix.online)
        .map(|mix| {
            let mut node = mix.node.clone();
            node.remove_tag(MixNodeTag::Guard);
            if guards.contains(&node.mix_id) {
                node.add_tag(MixNodeTag::Guard);
            }
            node
        })
        .collect();

    Topology::new(active)
}

fn advance_epoch<R: Rng + ?Sized>(mixes: &mut [GeneratedMix], churn_rate: f64, rng: &mut R) {
    let churn_rate = churn_rate.clamp(0.0, 1.0);
    for mix in mixes {
        if mix.online {
            if rng.gen_bool(churn_rate) {
                mix.online = false;
            }
        } else if rng.gen_bool(churn_rate) {
            mix.online = true;
        }
    }
}

/// consensus decides which nodes are eligible to be guards
/// maintains states relating to guard nodes.
#[derive(Debug, Default)]
struct Consensus {
    /// Currently advertised guard nodes for this epoch.
    active_guards: Vec<u32>,
    /// General guards that are online but not currently needed to hit the
    /// active guard bandwidth target.
    backup_guards: Vec<u32>,
    /// General guards that are offline and may be removed if they stay unstable.
    offline_guards: Vec<u32>,
}

impl Consensus {
    /// Build the guard set for the current epoch.
    ///
    /// Selection preserves existing guard identities as much as possible:
    /// 1. Online active guards remain active. unavailable guards move offline.
    /// 2. Guards returning from the offline state become backups.
    /// 3. If active guard bandwidth is below the configured target, backups are
    ///    promoted first.
    /// 4. If backups are insufficient, new online nodes are selected by
    ///    bandwidth until the target is reached.
    /// 5. When active bandwidth is already sufficient, do nothing.
    fn build_guard_set<R: Rng + ?Sized>(
        &mut self,
        mixes: &[GeneratedMix],
        config: &TopologyConfig,
        rng: &mut R,
    ) -> Vec<u32> {
        let mut active = Vec::new();
        let mut backup = Vec::new();
        let mut offline = Vec::new();

        for guard in self.active_guards.drain(..) {
            if is_online(mixes, guard) {
                push_unique(&mut active, guard);
            } else {
                push_unique(&mut offline, guard);
            }
        }
        for guard in self.backup_guards.drain(..) {
            if is_online(mixes, guard) {
                push_unique(&mut backup, guard);
            } else {
                push_unique(&mut offline, guard);
            }
        }
        for guard in self.offline_guards.drain(..) {
            if is_online(mixes, guard) {
                push_unique(&mut backup, guard);
            } else {
                push_unique(&mut offline, guard);
            }
        }

        let total_online_bandwidth: f64 = mixes
            .iter()
            .filter(|mix| mix.online)
            .map(|mix| mix.node.weight)
            .sum();
        let guard_target = total_online_bandwidth * config.guard_bandwidth_fraction.clamp(0.0, 1.0);

        if guard_target <= 0.0 {
            self.active_guards.clear();
            self.backup_guards.clear();
            self.offline_guards.clear();
            return Vec::new();
        }

        promote_until_target(mixes, &mut active, &mut backup, guard_target, rng);

        if guard_bandwidth(mixes, &active) < guard_target {
            let known: Vec<u32> = active
                .iter()
                .chain(&backup)
                .chain(&offline)
                .copied()
                .collect();
            let mut candidates: Vec<u32> = mixes
                .iter()
                .filter(|mix| mix.online && !known.contains(&mix.node.mix_id))
                .map(|mix| mix.node.mix_id)
                .collect();
            promote_until_target(mixes, &mut active, &mut candidates, guard_target, rng);
        }

        self.active_guards = active;
        self.backup_guards = backup;
        self.offline_guards = offline;
        self.active_guards.clone()
    }
}

fn promote_until_target<R: Rng + ?Sized>(
    mixes: &[GeneratedMix],
    active: &mut Vec<u32>,
    candidates: &mut Vec<u32>,
    target: f64,
    rng: &mut R,
) {
    while guard_bandwidth(mixes, active) < target && !candidates.is_empty() {
        let position = weighted_candidate_position(mixes, candidates, rng);
        active.push(candidates.swap_remove(position));
    }
}

fn weighted_candidate_position<R: Rng + ?Sized>(
    mixes: &[GeneratedMix],
    candidates: &[u32],
    rng: &mut R,
) -> usize {
    let total_weight: f64 = candidates
        .iter()
        .filter_map(|id| find_mix(mixes, *id))
        .map(|mix| mix.node.weight)
        .sum();
    let mut target = rng.gen_range(0.0..total_weight);

    for (position, id) in candidates.iter().enumerate() {
        if let Some(mix) = find_mix(mixes, *id) {
            target -= mix.node.weight;
            if target <= 0.0 {
                return position;
            }
        }
    }

    candidates.len() - 1
}

fn guard_bandwidth(mixes: &[GeneratedMix], guards: &[u32]) -> f64 {
    guards
        .iter()
        .filter_map(|id| find_mix(mixes, *id))
        .filter(|mix| mix.online)
        .map(|mix| mix.node.weight)
        .sum()
}

fn find_mix(mixes: &[GeneratedMix], mix_id: u32) -> Option<&GeneratedMix> {
    mixes.iter().find(|mix| mix.node.mix_id == mix_id)
}

fn is_online(mixes: &[GeneratedMix], mix_id: u32) -> bool {
    find_mix(mixes, mix_id).is_some_and(|mix| mix.online)
}

fn push_unique(guards: &mut Vec<u32>, guard: u32) {
    if !guards.contains(&guard) {
        guards.push(guard);
    }
}


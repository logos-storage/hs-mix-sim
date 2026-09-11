//! Runtime generator for free-route topology snapshots.
//!
//! It models churn and malicious node placement.

use crate::params::{
    DEFAULT_CHURN_RATE, DEFAULT_EPOCHS, DEFAULT_MALICIOUS_BANDWIDTH_FRACTION,
    DEFAULT_MALICIOUS_NODE_FRACTION, DEFAULT_MIX_SIZE,
};
use rand::prelude::*;
use rand_distr::LogNormal;
use rand_distr::weighted_alias::WeightedAliasIndex;

/// Stable identity of a mix node, shared across topology snapshots, samplers,
/// and adversary state. A mix keeps this ID when it goes offline or returns.
pub type MixId = u32;

#[derive(Debug, Clone)]
pub struct TopologyConfig {
    /// number of mix nodes in the mixnet
    pub mix_size: usize,
    /// number of epochs
    pub epochs: u32,
    /// fraction and bandwidth of malicious nodes
    pub malicious_node_fraction: f64,
    pub malicious_bandwidth_fraction: f64,
    pub churn_rate: f64,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfig {
            mix_size: DEFAULT_MIX_SIZE,
            epochs: DEFAULT_EPOCHS,
            malicious_node_fraction: DEFAULT_MALICIOUS_NODE_FRACTION,
            malicious_bandwidth_fraction: DEFAULT_MALICIOUS_BANDWIDTH_FRACTION,
            churn_rate: DEFAULT_CHURN_RATE,
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

/// Represent a single mix node.
#[derive(Debug, Clone)]
pub struct MixNode {
    pub weight: f64,
    pub mix_id: MixId,
    pub is_malicious: bool,
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

    pub fn is_active(&self, mix_id: MixId) -> bool {
        self.active.iter().any(|node| node.mix_id == mix_id)
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
        let mut topologies = Vec::with_capacity(self.config.epochs as usize);

        for _ in 0..self.config.epochs {
            topologies.push(snapshot(&mixes));
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
                    mix_id: mixid as MixId,
                    weight: weight.max(0.01),
                    is_malicious: false,
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

fn snapshot(mixes: &[GeneratedMix]) -> Topology {
    let active = mixes
        .iter()
        .filter(|mix| mix.online)
        .map(|mix| mix.node.clone())
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

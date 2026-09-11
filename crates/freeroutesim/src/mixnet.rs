//! Generates one static free-route mixnet shared by all users in a run.

use crate::params::{
    DEFAULT_MALICIOUS_BANDWIDTH_FRACTION, DEFAULT_MALICIOUS_NODE_FRACTION, DEFAULT_MIX_SIZE,
};
use rand::prelude::*;
use rand_distr::LogNormal;
use rand_distr::weighted_alias::WeightedAliasIndex;

/// Stable identity of a mix node, shared by the mixnet, samplers, and adversaries.
pub type MixId = u32;

#[derive(Debug, Clone)]
pub struct MixnetConfig {
    /// number of mix nodes in the mixnet
    pub mix_size: usize,
    /// fraction and bandwidth of malicious nodes
    pub malicious_node_fraction: f64,
    pub malicious_bandwidth_fraction: f64,
}

impl Default for MixnetConfig {
    fn default() -> Self {
        MixnetConfig {
            mix_size: DEFAULT_MIX_SIZE,
            malicious_node_fraction: DEFAULT_MALICIOUS_NODE_FRACTION,
            malicious_bandwidth_fraction: DEFAULT_MALICIOUS_BANDWIDTH_FRACTION,
        }
    }
}

/// Generates a static mix network with bandwidth weights and malicious flags.
pub struct MixnetGenerator {
    pub config: MixnetConfig,
}

impl Default for MixnetGenerator {
    fn default() -> Self {
        Self {
            config: MixnetConfig::default(),
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

/// A static mix network. Every node remains available throughout the simulation.
#[derive(Clone)]
pub struct Mixnet {
    /// All mix nodes in the network.
    nodes: Vec<MixNode>,
    /// Cached bandwidth sampler shared by every user.
    bandwidth_sampler: Option<WeightedAliasIndex<f64>>,
}

impl Mixnet {
    /// Construct a mixnet from its mix nodes.
    pub fn new(nodes: Vec<MixNode>) -> Self {
        let bandwidth_sampler = if nodes.is_empty() {
            None
        } else {
            Some(
                WeightedAliasIndex::new(
                    nodes
                        .iter()
                        .map(|node| node.weight.max(f64::EPSILON))
                        .collect(),
                )
                .expect("mix-node weights should produce a valid sampler"),
            )
        };
        Mixnet {
            nodes,
            bandwidth_sampler,
        }
    }

    pub(crate) fn nodes(&self) -> &[MixNode] {
        &self.nodes
    }

    pub(crate) fn sample_by_bandwidth<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&MixNode> {
        self.bandwidth_sampler
            .as_ref()
            .map(|sampler| &self.nodes[sampler.sample(rng)])
    }
}

impl Default for Mixnet {
    fn default() -> Self {
        Mixnet::new(Vec::new())
    }
}

impl MixnetGenerator {
    /// new mixnet generator with given config
    pub fn new(config: MixnetConfig) -> Self {
        Self { config }
    }

    /// Generate one mixnet, independent of the simulation duration.
    pub fn generate_mixnet(&self) -> Mixnet {
        Mixnet::new(self.initial_mixes(&mut thread_rng()))
    }

    fn initial_mixes<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<MixNode> {
        let bandwidth_distribution =
            LogNormal::new(2.0, 1.0).expect("log-normal bandwidth distribution should be valid");
        let mut mixes = Vec::with_capacity(self.config.mix_size);

        for mixid in 0..self.config.mix_size {
            let weight: f64 = bandwidth_distribution.sample(rng);
            mixes.push(MixNode {
                mix_id: mixid as MixId,
                weight: weight.max(0.01),
                is_malicious: false,
            });
        }

        self.mark_malicious_nodes(&mut mixes, rng);
        mixes
    }

    fn mark_malicious_nodes<R: Rng + ?Sized>(&self, mixes: &mut [MixNode], rng: &mut R) {
        let target_nodes = ((mixes.len() as f64)
            * self.config.malicious_node_fraction.clamp(0.0, 1.0))
        .ceil() as usize;
        let total_bandwidth: f64 = mixes.iter().map(|mix| mix.weight).sum();
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

            mixes[index].is_malicious = true;
            selected_nodes += 1;
            selected_bandwidth += mixes[index].weight;
        }
    }
}

//! Generates one static free-route mixnet shared by all users in a run.

use crate::params::{DEFAULT_MALICIOUS_NODE_FRACTION, DEFAULT_MIX_SIZE};
use rand::prelude::*;
use rand::seq::index;

/// Stable identity of a mix node, shared by the mixnet, samplers, and adversaries.
pub type MixId = u32;

#[derive(Debug, Clone)]
pub struct MixnetConfig {
    /// Number of mix nodes in the mixnet.
    pub mix_size: usize,
    /// Fraction of nodes initially controlled by the adversary.
    pub malicious_node_fraction: f64,
}

impl Default for MixnetConfig {
    fn default() -> Self {
        Self {
            mix_size: DEFAULT_MIX_SIZE,
            malicious_node_fraction: DEFAULT_MALICIOUS_NODE_FRACTION,
        }
    }
}

/// Generates a static mixnet with randomly assigned malicious flags.
#[derive(Default)]
pub struct MixnetGenerator {
    pub config: MixnetConfig,
}

/// A single mix node and its initial adversary control status.
#[derive(Debug, Clone)]
pub struct MixNode {
    pub mix_id: MixId,
    pub is_malicious: bool,
}

/// A static mixnet. Every node remains available throughout the simulation.
#[derive(Clone, Default)]
pub struct Mixnet {
    nodes: Vec<MixNode>,
}

impl Mixnet {
    pub fn new(nodes: Vec<MixNode>) -> Self {
        Self { nodes }
    }

    pub(crate) fn nodes(&self) -> &[MixNode] {
        &self.nodes
    }

    /// Select distinct existing nodes with a controlled malicious fraction.
    /// `malicious_fraction` is in [0, 1] (0.02 means 2%) and must not exceed
    /// the actual malicious fraction of this mixnet. Fractional node counts
    /// round down, so the returned fraction never exceeds the requested one.
    /// Selection is uniform within each group, then the result is shuffled.
    /// IDs and malicious flags are preserved; the shared mixnet is not modified.
    /// Returns an error for invalid inputs or insufficient nodes of either kind.
    /// This helper is available for future samplers; current samplers use all nodes.
    #[allow(dead_code)]
    pub fn sample_subset(
        &self,
        size: usize,
        malicious_fraction: f64,
    ) -> Result<Vec<MixNode>, &'static str> {
        if !(0.0..=1.0).contains(&malicious_fraction) {
            return Err("malicious fraction must be finite and between 0 and 1");
        }
        if size > self.nodes.len() {
            return Err("subset size exceeds the mixnet size");
        }
        let (malicious, honest): (Vec<_>, Vec<_>) =
            self.nodes.iter().partition(|node| node.is_malicious);
        let global_fraction = if self.nodes.is_empty() {
            0.0
        } else {
            malicious.len() as f64 / self.nodes.len() as f64
        };
        if malicious_fraction > global_fraction {
            return Err("requested malicious fraction exceeds global mixnet control");
        }

        // Start at the nearest integer, then enforce the fraction cap. This
        // avoids losing a node to multiplication rounding (e.g. 100 * 0.29).
        let mut malicious_count = (size as f64 * malicious_fraction).round() as usize;
        if malicious_count > 0 && malicious_count as f64 / size as f64 > malicious_fraction {
            malicious_count -= 1;
        }
        let honest_count = size - malicious_count;
        if malicious_count > malicious.len() || honest_count > honest.len() {
            return Err("not enough honest or malicious nodes for the requested subset");
        }

        let mut rng = thread_rng();
        let mut subset: Vec<MixNode> = malicious
            .choose_multiple(&mut rng, malicious_count)
            .chain(honest.choose_multiple(&mut rng, honest_count))
            .map(|node| (**node).clone())
            .collect();
        subset.shuffle(&mut rng);
        Ok(subset)
    }
}

impl MixnetGenerator {
    pub fn new(config: MixnetConfig) -> Self {
        Self { config }
    }

    /// Generate one mixnet, independent of the simulation duration.
    /// The malicious count is ceil(mix_size * malicious_node_fraction).
    pub fn generate_mixnet(&self) -> Mixnet {
        let mut nodes: Vec<_> = (0..self.config.mix_size)
            .map(|mix_id| MixNode {
                mix_id: mix_id as MixId,
                is_malicious: false,
            })
            .collect();
        let malicious_count = (nodes.len() as f64
            * self.config.malicious_node_fraction.clamp(0.0, 1.0))
        .ceil() as usize;
        for position in index::sample(&mut thread_rng(), nodes.len(), malicious_count) {
            nodes[position].is_malicious = true;
        }
        Mixnet::new(nodes)
    }
}

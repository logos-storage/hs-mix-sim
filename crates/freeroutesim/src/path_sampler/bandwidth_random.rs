use crate::params::DEFAULT_PATH_HOPS;
use crate::path_sampler::PathSampler;
use crate::topologygen::{MixId, MixNode, Topology};
use rand::thread_rng;
use rand_distr::Distribution;
use rand_distr::weighted_alias::WeightedAliasIndex;
use std::collections::HashSet;

pub struct BandwidthRandomPathSampler {
    hops: usize,
    current_topology_index: Option<usize>,
}

impl BandwidthRandomPathSampler {
    pub fn new(hops: usize) -> Self {
        Self {
            hops,
            current_topology_index: None,
        }
    }
}

impl Default for BandwidthRandomPathSampler {
    fn default() -> Self {
        Self::new(DEFAULT_PATH_HOPS)
    }
}

impl PathSampler for BandwidthRandomPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.current_topology_index = Some(topology_index);
        sample_bandwidth_path_with_fixed_hops(topology, self.hops, &[])
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "BandwidthRandomPathSampler"
    }
}

/// Fill non-fixed positions from the active-node bandwidth sampler.
pub(crate) fn sample_bandwidth_path_with_fixed_hops(
    topology: &Topology,
    hops: usize,
    fixed_hops: &[(usize, MixNode)],
) -> Vec<MixNode> {
    let active = topology.active();
    assert!(
        hops <= active.len(),
        "cannot sample a {hops}-hop path from {} active mix nodes",
        active.len()
    );

    let mut path = vec![None; hops];
    let mut used = HashSet::with_capacity(hops);
    let mut rng = thread_rng();
    for (hop, node) in fixed_hops {
        assert!(*hop < hops, "fixed hop {hop} is outside a {hops}-hop path");
        assert!(path[*hop].is_none(), "hop {hop} was fixed more than once");
        assert!(
            used.insert(node.mix_id),
            "mix {} was fixed at more than one hop",
            node.mix_id
        );
        path[*hop] = Some(node.clone());
    }

    for hop in &mut path {
        if hop.is_none() {
            loop {
                let sampled = topology
                    .sample_active(&mut rng)
                    .expect("active topology should contain at least one node");
                if used.insert(sampled.mix_id) {
                    *hop = Some(sampled.clone());
                    break;
                }
            }
        }
    }

    path.into_iter().map(Option::unwrap).collect()
}

/// Sample up to `count` bandwidth-weighted nodes without replacement.
pub(crate) fn sample_weighted_unique<'a>(
    nodes: impl IntoIterator<Item = &'a MixNode>,
    count: usize,
    excluded: &HashSet<MixId>,
) -> Vec<MixNode> {
    let mut candidates: Vec<&MixNode> = nodes
        .into_iter()
        .filter(|node| !excluded.contains(&node.mix_id))
        .collect();
    let mut sampled = Vec::with_capacity(count.min(candidates.len()));
    let mut rng = thread_rng();

    while sampled.len() < count && !candidates.is_empty() {
        let weights = candidates
            .iter()
            .map(|node| node.weight.max(f64::EPSILON))
            .collect();
        let sampler = WeightedAliasIndex::new(weights)
            .expect("mix-node weights should produce a valid sampler");
        let position = sampler.sample(&mut rng);
        sampled.push(candidates.swap_remove(position).clone());
    }

    sampled
}

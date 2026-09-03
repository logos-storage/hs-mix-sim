use crate::path_sampler::{HopBehavior, PathSampler};
use crate::topologygen::{MixNode, Topology};
use rand::seq::{IteratorRandom, index};
use rand::thread_rng;
use std::collections::HashMap;

/// K/W path sampler.
///
/// Each logical hop position gets a session-persistent pool of `k` uniformly
/// selected candidates. Every path independently chooses one node from each pool.
pub struct KOverWPathSampler {
    hops: usize,
    k: usize,
    candidate_pools: Vec<Vec<u32>>,
}

impl KOverWPathSampler {
    pub fn new(hops: usize, k: usize) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        assert!(k > 0, "K/W needs at least one candidate per hop");

        Self {
            hops,
            k,
            candidate_pools: Vec::new(),
        }
    }

    fn initialize(&mut self, topology: &Topology) {
        if !self.candidate_pools.is_empty() {
            return;
        }

        let required_candidates = self
            .hops
            .checked_mul(self.k)
            .expect("K/W candidate count overflowed");
        assert!(
            required_candidates <= topology.active().len(),
            "K/W needs {required_candidates} distinct candidates but the topology has only {} active nodes",
            topology.active().len()
        );

        let mut rng = thread_rng();
        let candidates: Vec<u32> =
            index::sample(&mut rng, topology.active().len(), required_candidates)
                .into_iter()
                .map(|node_index| topology.active()[node_index].mix_id)
                .collect();
        self.candidate_pools = candidates
            .chunks_exact(self.k)
            .map(|pool| pool.to_vec())
            .collect();
    }
}

impl PathSampler for KOverWPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.initialize(topology);

        let active_by_id: HashMap<u32, &MixNode> = topology
            .active()
            .iter()
            .map(|node| (node.mix_id, node))
            .collect();
        let mut rng = thread_rng();
        self.candidate_pools
            .iter()
            .enumerate()
            .map(|(hop, pool)| {
                pool.iter()
                    .filter_map(|mix_id| active_by_id.get(mix_id).copied())
                    .choose(&mut rng)
                    .unwrap_or_else(|| {
                        panic!(
                            "K/W hop {hop} has no active candidates in topology {topology_index}"
                        )
                    })
                    .clone()
            })
            .collect()
    }

    fn hop_behavior(&self, hop: usize) -> HopBehavior {
        assert!(
            hop < self.hops,
            "hop {hop} is outside a {}-hop path",
            self.hops
        );
        HopBehavior::Fixed
    }

    fn peak(&self, hop: usize, mix_node: u32) -> Vec<u32> {
        assert!(
            hop < self.hops,
            "hop {hop} is outside a {}-hop path",
            self.hops
        );
        if hop == 0
            || !self
                .candidate_pools
                .get(hop)
                .is_some_and(|pool| pool.contains(&mix_node))
        {
            return Vec::new();
        }

        self.candidate_pools[hop - 1].clone()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "KOverWPathSampler"
    }
}

#[cfg(test)]
mod tests {
    use super::KOverWPathSampler;
    use crate::path_sampler::{HopBehavior, PathSampler};
    use crate::topologygen::{MixNode, Topology};

    fn topology(ids: &[u32]) -> Topology {
        Topology::new(
            ids.iter()
                .map(|mix_id| MixNode {
                    weight: 1.0,
                    mix_id: *mix_id,
                    is_malicious: false,
                    tags: Vec::new(),
                })
                .collect(),
        )
    }

    fn sampler() -> KOverWPathSampler {
        KOverWPathSampler {
            hops: 2,
            k: 2,
            candidate_pools: vec![vec![10, 11], vec![20, 21]],
        }
    }

    #[test]
    fn samples_active_candidates_by_mix_id_across_topologies() {
        let mut sampler = sampler();
        let path = sampler.sample_path(7, &topology(&[11, 21, 99]));
        let path_ids: Vec<u32> = path.into_iter().map(|node| node.mix_id).collect();

        assert_eq!(path_ids, vec![11, 21]);
    }

    #[test]
    fn fixed_hop_peaks_at_the_previous_candidate_pool() {
        let sampler = sampler();

        assert!(matches!(sampler.hop_behavior(0), HopBehavior::Fixed));
        assert_eq!(sampler.peak(1, 20), vec![10, 11]);
        assert!(sampler.peak(0, 10).is_empty());
        assert!(sampler.peak(1, 99).is_empty());
    }

    #[test]
    #[should_panic(expected = "K/W hop 1 has no active candidates in topology 8")]
    fn fails_clearly_when_a_hop_has_no_active_candidates() {
        let mut sampler = sampler();
        sampler.sample_path(8, &topology(&[10]));
    }
}

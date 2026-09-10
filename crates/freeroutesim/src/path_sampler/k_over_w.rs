use crate::path_sampler::{FixedHopSet, PathSampler};
use crate::topologygen::{MixId, MixNode, Topology};
use rand::seq::{IteratorRandom, index};
use rand::thread_rng;
use std::collections::HashSet;

/// K/W path sampler.
///
/// Each logical hop position gets a session-persistent pool of `k` uniformly
/// selected candidates. Every path independently chooses one node from each pool.
pub struct KOverWPathSampler {
    hops: usize,
    k: usize,
    fixed_hops: usize,
    candidate_pools: Vec<FixedHopSet>,
}

impl KOverWPathSampler {
    pub fn new(hops: usize, k: usize) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        assert!(k > 0, "K/W needs at least one candidate per hop");

        Self {
            hops,
            k,
            fixed_hops: hops,
            candidate_pools: Vec::new(),
        }
    }

    pub fn new_with_fixed_hops(hops: usize, k: usize, fixed_hops: usize) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        assert!(k > 0, "K/W needs at least one candidate per hop");
        assert!(
            fixed_hops <= hops,
            "the number of fixed hops must not exceed the path length"
        );

        Self {
            hops,
            k,
            fixed_hops,
            candidate_pools: Vec::new(),
        }
    }

    fn initialize(&mut self, topology: &Topology) {
        if self.candidate_pools.len() == self.fixed_hops {
            return;
        }

        let required_candidates = self
            .fixed_hops
            .checked_mul(self.k)
            .expect("K/W candidate count overflowed");
        assert!(
            required_candidates <= topology.active().len(),
            "K/W needs {required_candidates} distinct candidates but the topology has only {} active nodes",
            topology.active().len()
        );

        let mut rng = thread_rng();
        let mut fixed_positions = index::sample(&mut rng, self.hops, self.fixed_hops).into_vec();
        fixed_positions.sort_unstable();

        let candidates: Vec<MixId> =
            index::sample(&mut rng, topology.active().len(), required_candidates)
                .into_iter()
                .map(|node_index| topology.active()[node_index].mix_id)
                .collect();
        self.candidate_pools = fixed_positions
            .into_iter()
            .zip(candidates.chunks_exact(self.k))
            .map(|(position, pool)| FixedHopSet {
                position,
                mix_ids: pool.to_vec(),
            })
            .collect();
    }
}

impl PathSampler for KOverWPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.initialize(topology);

        let active = topology.active();
        assert!(
            self.hops <= active.len(),
            "cannot sample a {}-hop path from {} active mix nodes",
            self.hops,
            active.len()
        );

        let mut rng = thread_rng();
        let mut path = vec![None; self.hops];
        let mut used = HashSet::with_capacity(self.hops);

        for fixed_hop in &self.candidate_pools {
            let node = fixed_hop
                .mix_ids
                .iter()
                .filter_map(|mix_id| active.iter().find(|node| node.mix_id == *mix_id))
                .choose(&mut rng)
                .unwrap_or_else(|| {
                    panic!(
                        "K/W hop {} has no active candidates in topology {topology_index}",
                        fixed_hop.position
                    )
                });

            used.insert(node.mix_id);
            path[fixed_hop.position] = Some(node.clone());
        }

        for hop in &mut path {
            if hop.is_some() {
                continue;
            }

            let node = active
                .iter()
                .filter(|node| !used.contains(&node.mix_id))
                .choose(&mut rng)
                .expect("an unused active mix node should be available");
            used.insert(node.mix_id);
            *hop = Some(node.clone());
        }

        path.into_iter().map(Option::unwrap).collect()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "KOverWPathSampler"
    }
}

use crate::mixnet::{MixNode, Mixnet};
use crate::path_sampler::{FixedHopSet, PathSampler};
use rand::seq::{IteratorRandom, SliceRandom, index};
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

    fn initialize(&mut self, mixnet: &Mixnet) {
        if self.candidate_pools.len() == self.fixed_hops {
            return;
        }

        let required_candidates = self
            .fixed_hops
            .checked_mul(self.k)
            .expect("K/W candidate count overflowed");
        assert!(
            required_candidates <= mixnet.nodes().len(),
            "K/W needs {required_candidates} distinct candidates but the mixnet has only {} mix nodes",
            mixnet.nodes().len()
        );

        let mut rng = thread_rng();
        let mut fixed_positions = index::sample(&mut rng, self.hops, self.fixed_hops).into_vec();
        fixed_positions.sort_unstable();

        let candidates: Vec<MixNode> =
            index::sample(&mut rng, mixnet.nodes().len(), required_candidates)
                .into_iter()
                .map(|node_index| mixnet.nodes()[node_index].clone())
                .collect();
        self.candidate_pools = fixed_positions
            .into_iter()
            .zip(candidates.chunks_exact(self.k))
            .map(|(position, pool)| FixedHopSet {
                position,
                nodes: pool.to_vec(),
            })
            .collect();
    }
}

impl PathSampler for KOverWPathSampler {
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode> {
        self.initialize(mixnet);

        let nodes = mixnet.nodes();
        assert!(
            self.hops <= nodes.len(),
            "cannot sample a {}-hop path from {} mix nodes",
            self.hops,
            nodes.len()
        );

        let mut rng = thread_rng();
        let mut path = vec![None; self.hops];
        let mut used = HashSet::with_capacity(self.hops);

        for fixed_hop in &self.candidate_pools {
            let node = fixed_hop
                .nodes
                .choose(&mut rng)
                .expect("K/W candidate pools are nonempty");

            used.insert(node.mix_id);
            path[fixed_hop.position] = Some(node.clone());
        }

        for hop in &mut path {
            if hop.is_some() {
                continue;
            }

            let node = nodes
                .iter()
                .filter(|node| !used.contains(&node.mix_id))
                .choose(&mut rng)
                .expect("an unused mix node should be available");
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

use crate::path_sampler::PathSampler;
use crate::topologygen::{MixNode, Topology};
use rand::Rng;
use rand::seq::index;
use rand::thread_rng;

/// K/W path sampler.
///
/// Each logical hop position gets a session-persistent pool of `k` uniformly
/// selected candidates. Every path independently chooses one node from each pool.
pub struct KOverWPathSampler {
    hops: usize,
    k: usize,
    candidate_pools: Vec<Vec<usize>>,
    topology_index: Option<usize>,
}

impl KOverWPathSampler {
    pub fn new(hops: usize, k: usize) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        assert!(k > 0, "K/W needs at least one candidate per hop");

        Self {
            hops,
            k,
            candidate_pools: Vec::new(),
            topology_index: None,
        }
    }

    fn initialize(&mut self, topology_index: usize, topology: &Topology) {
        if let Some(initial_topology_index) = self.topology_index {
            assert_eq!(
                topology_index, initial_topology_index,
                "K/W sampler state cannot cross topology snapshots"
            );
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
        let candidates =
            index::sample(&mut rng, topology.active().len(), required_candidates).into_vec();
        self.candidate_pools = candidates
            .chunks_exact(self.k)
            .map(|pool| pool.to_vec())
            .collect();
        self.topology_index = Some(topology_index);
    }
}

impl PathSampler for KOverWPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.initialize(topology_index, topology);

        let active = topology.active();
        let mut rng = thread_rng();
        self.candidate_pools
            .iter()
            .map(|pool| active[pool[rng.gen_range(0..pool.len())]].clone())
            .collect()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "KOverWPathSampler"
    }
}


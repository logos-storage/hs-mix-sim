use crate::mixnet::{MixNode, Mixnet};
use crate::path_sampler::{FixedHop, PathSampler};
use rand::Rng;
use rand::seq::index;
use rand::thread_rng;
use std::collections::HashSet;

/// K-Hops Fixed (K-HF) path sampler.
///
/// A random subset of `fixed_hops` path positions is chosen once for this
/// sampler. One node is fixed at each selected position and reused for every
/// path for the entire run.
/// Nodes at all remaining positions are sampled independently and uniformly.
pub struct KHopsFixedPathSampler {
    hops: usize,
    num_fixed_hops: usize,
    fixed_hops: Vec<FixedHop>,
}

impl KHopsFixedPathSampler {
    pub fn new(hops: usize, fixed_hops: usize) -> Self {
        assert!(
            fixed_hops <= hops,
            "the number of fixed hops must not exceed the path length"
        );

        Self {
            hops,
            num_fixed_hops: fixed_hops,
            fixed_hops: Vec::new(),
        }
    }

    fn initialize(&mut self, mixnet: &Mixnet) {
        if self.fixed_hops.len() == self.num_fixed_hops {
            return;
        }

        let nodes = mixnet.nodes();
        let mut rng = thread_rng();
        let mut fixed_positions =
            index::sample(&mut rng, self.hops, self.num_fixed_hops).into_vec();
        fixed_positions.sort_unstable();

        let sampled_nodes = index::sample(&mut rng, nodes.len(), self.num_fixed_hops);

        self.fixed_hops = fixed_positions
            .into_iter()
            .zip(sampled_nodes)
            .map(|(position, node_index)| FixedHop {
                position,
                node: nodes[node_index].clone(),
            })
            .collect();
    }
}

impl PathSampler for KHopsFixedPathSampler {
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode> {
        let nodes = mixnet.nodes();

        assert!(
            self.hops <= nodes.len(),
            "cannot sample a {}-hop path from {} mix nodes",
            self.hops,
            nodes.len()
        );
        self.initialize(mixnet);

        let mut path = vec![None; self.hops];
        let mut used = HashSet::with_capacity(self.hops);

        for fixed_hop in &self.fixed_hops {
            used.insert(fixed_hop.node.mix_id);
            path[fixed_hop.position] = Some(fixed_hop.node.clone());
        }

        let mut rng = thread_rng();
        for hop in &mut path {
            if hop.is_some() {
                continue;
            }

            loop {
                let node = &nodes[rng.gen_range(0..nodes.len())];
                if used.insert(node.mix_id) {
                    *hop = Some(node.clone());
                    break;
                }
            }
        }

        path.into_iter().map(Option::unwrap).collect()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "KHopsFixedPathSampler"
    }
}

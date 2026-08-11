use crate::path_sampler::PathSampler;
use crate::topologygen::{MixNode, Topology};
use rand::Rng;
use rand::seq::index;
use rand::thread_rng;
use std::collections::HashSet;

/// K-Hops Fixed (K-HF) path sampler.
///
/// A random subset of `fixed_hops` path positions is chosen once for this
/// sampler. One node is fixed at each selected position and reused for every
/// path. If churn takes a fixed node offline, only that node is replaced.
/// Nodes at all remaining positions are sampled independently and uniformly.
pub struct KHopsFixedPathSampler {
    hops: usize,
    fixed_hops: usize,
    fixed_positions: Vec<usize>,
    fixed_nodes: Vec<MixNode>,
    current_topology_index: Option<usize>,
}

impl KHopsFixedPathSampler {
    pub fn new(hops: usize, fixed_hops: usize) -> Self {
        assert!(
            fixed_hops <= hops,
            "the number of fixed hops must not exceed the path length"
        );

        Self {
            hops,
            fixed_hops,
            fixed_positions: Vec::new(),
            fixed_nodes: Vec::new(),
            current_topology_index: None,
        }
    }

    fn initialize(&mut self, topology_index: usize, topology: &Topology) {
        if self.current_topology_index.is_some() {
            return;
        }

        let active = topology.active();
        let mut rng = thread_rng();
        self.fixed_positions = index::sample(&mut rng, self.hops, self.fixed_hops).into_vec();
        self.fixed_positions.sort_unstable();

        let sampled_nodes = index::sample(&mut rng, active.len(), self.fixed_hops);
        self.fixed_nodes = sampled_nodes
            .into_iter()
            .map(|node_index| active[node_index].clone())
            .collect();
        self.current_topology_index = Some(topology_index);
    }

    fn update_fixed_nodes(&mut self, topology_index: usize, topology: &Topology) {
        if self.current_topology_index == Some(topology_index) {
            return;
        }

        let active = topology.active();
        let mut used = HashSet::with_capacity(self.fixed_hops);
        let mut offline = Vec::new();
        let mut rng = thread_rng();

        for (selection_index, selected) in self.fixed_nodes.iter_mut().enumerate() {
            if let Some(node) = active.iter().find(|node| node.mix_id == selected.mix_id) {
                *selected = node.clone();
                used.insert(node.mix_id);
            } else {
                offline.push(selection_index);
            }
        }

        for selection_index in offline {
            loop {
                let replacement = &active[rng.gen_range(0..active.len())];
                if used.insert(replacement.mix_id) {
                    self.fixed_nodes[selection_index] = replacement.clone();
                    break;
                }
            }
        }

        self.current_topology_index = Some(topology_index);
    }
}

impl PathSampler for KHopsFixedPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        let active = topology.active();
        assert!(
            self.hops <= active.len(),
            "cannot sample a {}-hop path from {} active mix nodes",
            self.hops,
            active.len()
        );

        self.initialize(topology_index, topology);
        self.update_fixed_nodes(topology_index, topology);

        let mut path = vec![None; self.hops];
        let mut used = HashSet::with_capacity(self.hops);

        for (&position, node) in self.fixed_positions.iter().zip(&self.fixed_nodes) {
            used.insert(node.mix_id);
            path[position] = Some(node.clone());
        }

        let mut rng = thread_rng();
        for hop in &mut path {
            if hop.is_some() {
                continue;
            }

            loop {
                let node = &active[rng.gen_range(0..active.len())];
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
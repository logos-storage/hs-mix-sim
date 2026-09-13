//! A user's persistent layered topology, independent of the path-sampler traits.

use crate::mixnet::{MixId, MixNode, Mixnet};
use crate::time_based_path_sampler::Observation;
use rand::rngs::SmallRng;
use rand::seq::{SliceRandom, index};
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

/// Node lifetime/rotation for a layer. Each rotating node independently samples its duration.
#[derive(Debug, Clone, Copy)]
pub enum NodeLifetime {
    #[allow(dead_code)]
    Never,
    MaxOfTwoUniform {
        min_seconds: u64,
        max_seconds: u64,
    },
}

impl NodeLifetime {
    /// bounds in seconds, or None for a node that never expires.
    pub fn bounds(self) -> Option<(u64, u64)> {
        match self {
            Self::Never => None,
            Self::MaxOfTwoUniform {
                min_seconds,
                max_seconds,
            } => Some((min_seconds, max_seconds)),
        }
    }

    fn sample_expiration(self, current_time: u64, rng: &mut impl Rng) -> Option<u64> {
        self.bounds().map(|(min, max)| {
            let first = rng.gen_range(min..=max);
            let second = rng.gen_range(min..=max);
            current_time
                .checked_add(first.max(second))
                .expect("node expiration timestamp overflowed")
        })
    }
}

/// Configuration of one layer. Layers are ordered from service/sender to recipient;
/// layer 1 (index 0) is nearest the service and the last layer contains the exits.
#[derive(Debug, Clone, Copy)]
pub struct LayerConfig {
    pub node_count: usize,
    /// Shared lifetime policy for all nodes in this layer.
    pub lifetime: NodeLifetime,
}

impl LayerConfig {
    pub const fn new(node_count: usize, lifetime: NodeLifetime) -> Self {
        Self {
            node_count,
            lifetime,
        }
    }
}

/// How adjacent layers are connected. Every selected node belongs to a complete path.
#[derive(Debug, Clone, Copy)]
pub enum ConnectionMode {
    /// Connect each node to every node in the next layer.
    #[allow(dead_code)]
    Mesh,
    /// Exactly d distinct outgoing links per non-exit node.
    Degree(usize),
}

impl ConnectionMode {
    /// Reject impossible configurations before sampling any connections.
    pub fn validate_layers(self, layers: &[LayerConfig]) {
        assert!(!layers.is_empty(), "a topology needs at least one layer");
        assert!(
            layers.iter().all(|layer| layer.node_count > 0),
            "each layer needs at least one node"
        );
        if let Self::Degree(d) = self {
            assert!(d > 0, "topology degree must be positive");
            for (index, pair) in layers.windows(2).enumerate() {
                let (source, next) = (pair[0].node_count, pair[1].node_count);
                assert!(
                    d <= next,
                    "degree {d} exceeds the {next} nodes in layer {}",
                    index + 2
                );
                // Equivalent to source*d >= next without multiplication overflow.
                assert!(
                    source >= next.div_ceil(d),
                    "layer {} has {source} nodes at degree {d}, insufficient to cover {next} nodes in layer {}",
                    index + 1,
                    index + 2
                );
            }
        }
    }

    fn connect(self, source_size: usize, next_size: usize, rng: &mut impl Rng) -> Vec<Vec<usize>> {
        if matches!(self, Self::Mesh) {
            return vec![(0..next_size).collect(); source_size];
        }
        let Self::Degree(d) = self else {
            unreachable!()
        };
        let mut sources: Vec<_> = (0..source_size).collect();
        let mut unused: Vec<_> = (0..next_size).collect();
        sources.shuffle(rng);
        unused.shuffle(rng);
        let mut links = vec![Vec::new(); source_size];
        for source in sources {
            let peers = &mut links[source];
            // Prefer nodes with no incoming connection yet.
            while peers.len() < d {
                let Some(next) = unused.pop() else {
                    break;
                };
                peers.push(next);
            }
            if peers.len() < d {
                // All destinations are now covered. Fill randomly, without repeating
                // an outgoing neighbor for this source. Other sources may reuse it.
                let available: Vec<_> = (0..next_size)
                    .filter(|next| !peers.contains(next))
                    .collect();
                let needed = d - peers.len();
                peers.extend(available.choose_multiple(rng, needed).copied());
            }
            peers.sort_unstable();
        }
        assert!(
            unused.is_empty(),
            "connection generation left uncovered nodes"
        );
        links
    }
}

/// Defines node rotation. `layer_index` and `node_index` refers to the exact node
/// in the local topology that we are defining rotation for in here. 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRotation {
    layer_index: usize,
    node_index: usize,
    expires_at: u64,
}

struct StoredNode {
    node: MixNode,
    /// None means no expiration, not a timestamp far in the future.
    expires_at: Option<u64>,
}

struct Layer {
    config: LayerConfig,
    nodes: Vec<StoredNode>,
}

/// Every node belongs to a complete path through the configured layer connections. Paths are generated
/// on demand, so the topology stores nodes and links rather than complete paths.
/// Membership and individual lifetimes persist until their rotation events run.
/// Initial malicious flags are copied from the global mixnet
pub struct FixedTopology {
    layers: Vec<Layer>,
    occupied: HashSet<MixId>,
    /// Links use stable layer-local slot indices; replacements inherit their links.
    connections: Vec<Vec<Vec<usize>>>,
    pending_events: Vec<(u64, NodeRotation)>,
    current_time: u64,
    rng: SmallRng,
}

impl FixedTopology {

    pub fn new(config: &[LayerConfig], connections: ConnectionMode, mixnet: &Mixnet) -> Self {
        Self::with_rng(config, connections, mixnet, SmallRng::from_entropy())
    }

    fn with_rng(
        config: &[LayerConfig],
        connections: ConnectionMode,
        mixnet: &Mixnet,
        mut rng: SmallRng,
    ) -> Self {
        connections.validate_layers(config);
        let count = config.iter().fold(0usize, |total, layer| {
            assert!(layer.node_count > 0, "each layer needs at least one node");
            if let Some((min, max)) = layer.lifetime.bounds() {
                assert!(
                    min > 0 && min <= max,
                    "layer lifetime bounds must be positive and ordered"
                );
            }
            total
                .checked_add(layer.node_count)
                .expect("topology node count overflowed")
        });
        assert!(
            count <= mixnet.nodes().len(),
            "not enough mix nodes for topology"
        );
        if config.iter().any(|layer| layer.lifetime.bounds().is_some()) {
            assert!(
                count < mixnet.nodes().len(),
                "topology needs at least one spare mixnet node for rotation"
            );
        }
        assert_eq!(
            mixnet
                .nodes()
                .iter()
                .map(|node| node.mix_id)
                .collect::<HashSet<_>>()
                .len(),
            mixnet.nodes().len(),
            "mixnet IDs must be unique"
        );
        let mut selected = index::sample(&mut rng, mixnet.nodes().len(), count).into_iter();
        let mut topology = Self {
            layers: Vec::with_capacity(config.len()),
            occupied: HashSet::with_capacity(count),
            pending_events: Vec::with_capacity(count),
            current_time: 0,
            rng,
            connections: Vec::new(),
        };
        for (layer_index, &config) in config.iter().enumerate() {
            let mut nodes = Vec::with_capacity(config.node_count);
            for node_index in 0..config.node_count {
                let node = mixnet.nodes()[selected.next().unwrap()].clone();
                let expires_at = config.lifetime.sample_expiration(0, &mut topology.rng);
                topology.occupied.insert(node.mix_id);
                if let Some(time) = expires_at {
                    topology.queue_rotation(layer_index, node_index, time);
                }
                nodes.push(StoredNode { node, expires_at });
            }
            topology.layers.push(Layer { config, nodes });
        }
        // Cover every next-layer node. Inductively, all nodes are reachable from
        // the first layer and have a continuation to an exit. Rotations preserve links.
        for pair in config.windows(2) {
            topology.connections.push(connections.connect(
                pair[0].node_count,
                pair[1].node_count,
                &mut topology.rng,
            ));
        }
        topology
    }

    pub fn hops(&self) -> usize {
        self.layers.len()
    }

    /// Uniformly choose a first-layer node, then one outgoing link at each hop.
    pub fn sample_path(&mut self) -> Vec<MixNode> {
        let mut node_index = self.rng.gen_range(0..self.layers[0].nodes.len());
        let mut path = Vec::with_capacity(self.hops());
        for (layer_index, layer) in self.layers.iter().enumerate() {
            path.push(layer.nodes[node_index].node.clone());
            if let Some(links) = self.connections.get(layer_index) {
                let peers = &links[node_index];
                node_index = peers[self.rng.gen_range(0..peers.len())];
            }
        }
        path
    }

    /// Enumerate the set P of valid paths, in sender-to-recipient order, without
    /// duplicates. Degree(d) gives |P| = first_layer_size * d^(hops-1); Mesh gives
    /// the product of layer sizes.
    #[allow(dead_code)]
    pub fn paths(&self) -> Vec<Vec<MixNode>> {
        fn visit(
            topology: &FixedTopology,
            layer: usize,
            node: usize,
            path: &mut Vec<MixNode>,
            paths: &mut Vec<Vec<MixNode>>,
        ) {
            path.push(topology.layers[layer].nodes[node].node.clone());
            if let Some(links) = topology.connections.get(layer) {
                for &peer in &links[node] {
                    visit(topology, layer + 1, peer, path, paths);
                }
            } else {
                paths.push(path.clone());
            }
            path.pop();
        }
        let mut paths = Vec::new();
        let mut path = Vec::with_capacity(self.hops());
        for node in 0..self.layers[0].nodes.len() {
            visit(self, 0, node, &mut path, &mut paths);
        }
        paths
    }

    pub fn node(&self, mix_id: MixId) -> Option<&MixNode> {
        self.layers
            .iter()
            .flat_map(|layer| &layer.nodes)
            .map(|stored| &stored.node)
            .find(|node| node.mix_id == mix_id)
    }

    /// Validate the entire recipient-to-sender chain against the configured
    /// graph. Reveal only neighbors extending this chain.
    pub fn peak(&self, chain: &[MixId]) -> Observation {
        if chain.len() > self.layers.len() {
            return Observation::Unavailable;
        }
        let mut toward_recipient = None;
        for (offset, id) in chain.iter().enumerate() {
            let layer_index = self.layers.len() - offset - 1;
            let Some(slot) = self.layers[layer_index]
                .nodes
                .iter()
                .position(|stored| stored.node.mix_id == *id)
            else {
                return Observation::Unavailable;
            };
            if let Some(next_slot) = toward_recipient {
                if !self.connections[layer_index][slot].contains(&next_slot) {
                    return Observation::Unavailable;
                }
            }
            toward_recipient = Some(slot);
        }
        if chain.len() == self.layers.len() {
            return Observation::ServiceIdentified;
        }
        let layer_index = self.layers.len() - chain.len() - 1;
        let mut nodes: Vec<_> = self.layers[layer_index]
            .nodes
            .iter()
            .enumerate()
            .filter(|(slot, _)| {
                toward_recipient
                    .is_none_or(|next| self.connections[layer_index][*slot].contains(&next))
            })
            .map(|(_, stored)| stored.node.mix_id)
            .collect();
        nodes.sort_unstable();
        Observation::Nodes(nodes)
    }

    pub fn next_events(&mut self, current_time: u64) -> Vec<(u64, NodeRotation)> {
        assert!(
            current_time >= self.current_time,
            "topology time cannot move backwards"
        );
        assert!(
            self.pending_events
                .iter()
                .all(|(time, _)| *time >= current_time),
            "topology events must be collected before they expire"
        );
        self.current_time = current_time;
        std::mem::take(&mut self.pending_events)
    }

    /// Replace only the expired slot, using the same layer's lifetime range.
    /// The global mixnet must remain the same static network supplied at initialization.
    pub fn handle_event(&mut self, current_time: u64, event: NodeRotation, mixnet: &Mixnet) {
        assert!(
            current_time >= self.current_time,
            "topology time cannot move backwards"
        );
        let Some(layer) = self.layers.get(event.layer_index) else {
            return;
        };
        let Some(stored) = layer.nodes.get(event.node_index) else {
            return;
        };
        if stored.expires_at != Some(event.expires_at) {
            return;
        }
        assert_eq!(
            current_time, event.expires_at,
            "node rotation must run at its scheduled time"
        );
        let old_id = stored.node.mix_id;
        let expires_at = layer
            .config
            .lifetime
            .sample_expiration(current_time, &mut self.rng)
            .expect("a rotating node must have a finite lifetime");
        // Rejection sampling is uniform over nodes outside the topology. Small
        // local topologies usually need just one draw; no full network scan is needed.
        let replacement = loop {
            let candidate = &mixnet.nodes()[self.rng.gen_range(0..mixnet.nodes().len())];
            if !self.occupied.contains(&candidate.mix_id) {
                break candidate.clone();
            }
        };
        self.occupied.remove(&old_id);
        self.occupied.insert(replacement.mix_id);
        self.layers[event.layer_index].nodes[event.node_index] = StoredNode {
            node: replacement,
            expires_at: Some(expires_at),
        };
        self.queue_rotation(event.layer_index, event.node_index, expires_at);
        self.current_time = current_time;
    }

    fn queue_rotation(&mut self, layer_index: usize, node_index: usize, expires_at: u64) {
        self.pending_events.push((
            expires_at,
            NodeRotation {
                layer_index,
                node_index,
                expires_at,
            },
        ));
    }
}

//! Hard-coded experiment data, selected using --topology-preset.
//! Names list node counts from service to exits, followed by D<degree> or M for mesh.

use super::{ConnectionMode, LayerConfig, NodeLifetime, TopologyExperiment};

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;
const SHORT_ROTATION: NodeLifetime = NodeLifetime::MaxOfTwoUniform {
    min_seconds: HOUR,
    max_seconds: 48 * HOUR,
};
const UNIFORM_LAYER: LayerConfig = LayerConfig::new(5, SHORT_ROTATION);
const LAYERS_5_5_5: [LayerConfig; 3] = [UNIFORM_LAYER; 3];
const LAYERS_5_5_5_5: [LayerConfig; 4] = [UNIFORM_LAYER; 4];

const fn vanguard_layers(exit_nodes: usize) -> [LayerConfig; 3] {
    [
        LayerConfig::new(
            2,
            NodeLifetime::MaxOfTwoUniform {
                min_seconds: 90 * DAY,
                max_seconds: 120 * DAY,
            },
        ),
        LayerConfig::new(
            4,
            NodeLifetime::MaxOfTwoUniform {
                min_seconds: 30 * DAY,
                max_seconds: 60 * DAY,
            },
        ),
        LayerConfig::new(exit_nodes, SHORT_ROTATION),
    ]
}

pub const TOPOLOGY_2_4_6_M: TopologyExperiment = TopologyExperiment {
    name: "2_4_6_M",
    layers: &vanguard_layers(6),
    connections: ConnectionMode::Mesh,
};
pub const TOPOLOGY_2_4_8_M: TopologyExperiment = TopologyExperiment {
    name: "2_4_8_M",
    layers: &vanguard_layers(8),
    connections: ConnectionMode::Mesh,
};

pub const TOPOLOGY_5_5_5_M: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_M",
    layers: &LAYERS_5_5_5,
    connections: ConnectionMode::Mesh,
};
pub const TOPOLOGY_5_5_5_D2: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_D2",
    layers: &LAYERS_5_5_5,
    connections: ConnectionMode::Degree(2),
};
pub const TOPOLOGY_5_5_5_D3: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_D3",
    layers: &LAYERS_5_5_5,
    connections: ConnectionMode::Degree(3),
};
pub const TOPOLOGY_5_5_5_5_M: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_5_M",
    layers: &LAYERS_5_5_5_5,
    connections: ConnectionMode::Mesh,
};
pub const TOPOLOGY_5_5_5_5_D2: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_5_D2",
    layers: &LAYERS_5_5_5_5,
    connections: ConnectionMode::Degree(2),
};
pub const TOPOLOGY_5_5_5_5_D3: TopologyExperiment = TopologyExperiment {
    name: "5_5_5_5_D3",
    layers: &LAYERS_5_5_5_5,
    connections: ConnectionMode::Degree(3),
};

/// Default when --mode fixed-topology is used without --topology-preset.
pub const DEFAULT_EXPERIMENT: TopologyExperiment = TOPOLOGY_5_5_5_5_D2;

/// The CLI choices and sampler tests both use this catalog.
pub const ALL_EXPERIMENTS: &[TopologyExperiment] = &[
    TOPOLOGY_2_4_6_M,
    TOPOLOGY_2_4_8_M,
    TOPOLOGY_5_5_5_M,
    TOPOLOGY_5_5_5_D2,
    TOPOLOGY_5_5_5_D3,
    TOPOLOGY_5_5_5_5_M,
    TOPOLOGY_5_5_5_5_D2,
    TOPOLOGY_5_5_5_5_D3,
];

//! hidden-service topology profiles:
//! Vanguard profiles sample directly from a mes
//! LITE
//! STANDARD
//! STRICT

use super::{ConnectionMode, FPOFTProfile, LayerConfig, NodeLifetime, TopologyProfile};

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;
const SHORT_ROTATION: NodeLifetime = NodeLifetime::MaxOfTwoUniform {
    min_seconds: HOUR,
    max_seconds: 48 * HOUR,
};
const UNIFORM_LAYER: LayerConfig = LayerConfig::new(5, SHORT_ROTATION);

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

pub const VANGUARD1: TopologyProfile = TopologyProfile {
    name: "vanguard1",
    layers: &vanguard_layers(6),
    connections: ConnectionMode::Mesh,
};
pub const VANGUARD2: TopologyProfile = TopologyProfile {
    name: "vanguard2",
    layers: &vanguard_layers(8),
    connections: ConnectionMode::Mesh,
};

/// Default when --mode fixed-topology is used without --topology-profile.
pub const DEFAULT_PROFILE: TopologyProfile = VANGUARD1;
pub const ALL_PROFILES: &[TopologyProfile] = &[VANGUARD1, VANGUARD2];

/// Active-path profiles explicitly specify node lifetimes as well as path
/// lifetimes. The topology covers the complete path, including any rotating layer.
const fn active_paths(
    name: &'static str,
    layers: &'static [LayerConfig],
    connections: ConnectionMode,
) -> FPOFTProfile {
    FPOFTProfile {
        name,
        topology_profile: TopologyProfile {
            name,
            layers,
            connections,
        },
        num_paths: 5,
        path_lifetime: SHORT_ROTATION,
    }
}

const PERMANENT: LayerConfig = LayerConfig::new(5, NodeLifetime::Never);

/// Two fixed mesh layers, five independently rotating complete routes.
pub const LITE: FPOFTProfile = active_paths("LITE", &[PERMANENT; 2], ConnectionMode::Mesh);
/// Two fixed layers and a rotating final layer, all with degree 3.
pub const STANDARD: FPOFTProfile = active_paths(
    "STANDARD",
    &[PERMANENT, PERMANENT, UNIFORM_LAYER],
    ConnectionMode::Degree(3),
);
/// Three fixed layers and a rotating final layer, all with degree 3
pub const STRICT: FPOFTProfile = active_paths(
    "STRICT",
    &[PERMANENT, PERMANENT, PERMANENT, UNIFORM_LAYER],
    ConnectionMode::Degree(3),
);

pub const DEFAULT_FPOFT_PROFILE: FPOFTProfile = STANDARD;
pub const ALL_FPOFT_PROFILES: &[FPOFTProfile] = &[LITE, STANDARD, STRICT];

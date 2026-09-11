//! Constants and parameters used throughout the simulator.

pub const DEFAULT_PATH_HOPS: usize = 3;

pub const DEFAULT_MIX_SIZE: usize = 1000;
pub const DEFAULT_EPOCHS: u32 = 1;
pub const DEFAULT_MALICIOUS_NODE_FRACTION: f64 = 0.10;
pub const DEFAULT_MALICIOUS_BANDWIDTH_FRACTION: f64 = 0.10;
/// churn rate is a single value for now, but we can extend this later for
/// different rates for joining nodes and leaving node (honest & malicious as well)
pub const DEFAULT_CHURN_RATE: f64 = 0.03;

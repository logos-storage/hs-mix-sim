//! Constants and parameters used throughout the simulator.

pub const DEFAULT_PATH_HOPS: usize = 3;

pub const DEFAULT_MIX_SIZE: usize = 1000;
pub const DEFAULT_MALICIOUS_NODE_FRACTION: f64 = 0.10;
/// Output resolution only; it does not advance simulated time.
pub const DEFAULT_CSV_INTERVAL_SECONDS: u32 = 3600;

/// Maximum compromise attempts per layer for each hidden-service run.
/// Change to CompromiseBudget::Unlimited for Unlimited compromise attempts.
/// Sybils are free.
pub const COMPROMISE_BUDGET_PER_LAYER: crate::adversary::CompromiseBudget =
    crate::adversary::CompromiseBudget::Limited(1);

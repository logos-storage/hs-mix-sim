//! Lifetime budget on number of compromise targets, independently for each layer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompromiseBudget {
    /// At most this many attempts per layer over the whole user simulation.
    /// Failures, pending attempts, all consume the budget.
    Limited(usize),
    #[allow(dead_code)]
    Unlimited,
}

impl CompromiseBudget {
    pub(super) fn remaining(self, attempted: usize) -> usize {
        match self {
            Self::Limited(limit) => limit.saturating_sub(attempted),
            Self::Unlimited => usize::MAX,
        }
    }
}

impl std::fmt::Display for CompromiseBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limited(limit) => write!(f, "{limit}"),
            Self::Unlimited => f.write_str("unlimited"),
        }
    }
}

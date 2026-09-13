//! lifetime for independently rotating nodes or paths.

use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub enum Lifetime {
    /// No expiry timestamp and no scheduled rotation event.
    Never,
    /// Maximum of two independent, inclusive uniform draws, in seconds.
    MaxOfTwoUniform { min_seconds: u64, max_seconds: u64 },
}

impl Lifetime {
    pub fn bounds(self) -> Option<(u64, u64)> {
        match self {
            Self::Never => None,
            Self::MaxOfTwoUniform {
                min_seconds,
                max_seconds,
            } => Some((min_seconds, max_seconds)),
        }
    }

    pub(crate) fn validate(self) {
        if let Some((min, max)) = self.bounds() {
            assert!(
                min > 0 && min <= max,
                "lifetime bounds must be positive and ordered"
            );
        }
    }

    pub(crate) fn sample_expiration(self, current_time: u64, rng: &mut impl Rng) -> Option<u64> {
        self.validate();
        self.bounds().map(|(min, max)| {
            let first = rng.gen_range(min..=max);
            let second = rng.gen_range(min..=max);
            current_time
                .checked_add(first.max(second))
                .expect("expiration timestamp overflowed")
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rate {
    pub operations: u64,
    pub elapsed_us: u64,
}

impl Rate {
    #[must_use]
    pub const fn per_second(self) -> u64 {
        if self.elapsed_us == 0 {
            0
        } else {
            self.operations.saturating_mul(1_000_000) / self.elapsed_us
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_rate_and_handles_zero_time() {
        assert_eq!(
            Rate {
                operations: 1_000,
                elapsed_us: 250
            }
            .per_second(),
            4_000_000
        );
        assert_eq!(
            Rate {
                operations: 1,
                elapsed_us: 0
            }
            .per_second(),
            0
        );
    }

    #[test]
    fn saturates_before_division() {
        assert_eq!(
            Rate {
                operations: u64::MAX,
                elapsed_us: 1_000_000
            }
            .per_second(),
            u64::MAX / 1_000_000
        );
    }
}

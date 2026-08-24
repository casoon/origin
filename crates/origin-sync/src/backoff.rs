use time::Duration;

/// Exponential backoff with a cap and jitter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Backoff {
    /// Delay after the first failure.
    pub base: Duration,
    /// Upper bound, however many failures accumulate.
    pub max: Duration,
    /// Growth per failure.
    pub multiplier: u32,
    /// Fraction of the delay that is randomised, 0.0 to 1.0.
    pub jitter: f64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base: Duration::seconds(30),
            max: Duration::minutes(30),
            multiplier: 2,
            // ±20 %: enough to keep several targets that failed together from
            // retrying in lockstep and hammering a recovering service.
            jitter: 0.2,
        }
    }
}

impl Backoff {
    /// Delay after `failures` consecutive failures.
    ///
    /// `random` is a value in `0.0..=1.0` supplied by the caller, which keeps this a
    /// pure function — jitter is otherwise untestable.
    pub fn delay_for(&self, failures: u32, random: f64) -> Duration {
        if failures == 0 {
            return Duration::ZERO;
        }

        let exponent = failures.saturating_sub(1);
        // Saturating: 2^32 seconds overflows long before the cap matters.
        let factor = (self.multiplier as u64).saturating_pow(exponent.min(32));
        let raw = self
            .base
            .saturating_mul(factor.min(i32::MAX as u64) as i32)
            .min(self.max);

        if self.jitter <= 0.0 {
            return raw;
        }

        let jitter = self.jitter.clamp(0.0, 1.0);
        let random = random.clamp(0.0, 1.0);
        // Spread symmetrically around the raw delay: [1-j, 1+j].
        let scale = 1.0 - jitter + 2.0 * jitter * random;

        let seconds = (raw.as_seconds_f64() * scale).max(0.0);
        Duration::seconds_f64(seconds).min(self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No jitter, so the growth curve itself is visible.
    const PLAIN: Backoff = Backoff {
        base: Duration::seconds(30),
        max: Duration::minutes(30),
        multiplier: 2,
        jitter: 0.0,
    };

    #[test]
    fn a_healthy_target_waits_not_at_all() {
        assert_eq!(PLAIN.delay_for(0, 0.5), Duration::ZERO);
    }

    #[test]
    fn the_delay_doubles_per_failure() {
        assert_eq!(PLAIN.delay_for(1, 0.5), Duration::seconds(30));
        assert_eq!(PLAIN.delay_for(2, 0.5), Duration::minutes(1));
        assert_eq!(PLAIN.delay_for(3, 0.5), Duration::minutes(2));
        assert_eq!(PLAIN.delay_for(4, 0.5), Duration::minutes(4));
    }

    #[test]
    fn the_cap_holds_however_long_the_outage_lasts() {
        assert_eq!(PLAIN.delay_for(50, 0.5), Duration::minutes(30));
        assert_eq!(PLAIN.delay_for(u32::MAX, 0.5), Duration::minutes(30));
    }

    #[test]
    fn jitter_spreads_symmetrically_around_the_delay() {
        let backoff = Backoff {
            jitter: 0.2,
            ..PLAIN
        };

        assert_eq!(backoff.delay_for(1, 0.5), Duration::seconds(30));
        assert_eq!(backoff.delay_for(1, 0.0), Duration::seconds(24)); // −20 %
        assert_eq!(backoff.delay_for(1, 1.0), Duration::seconds(36)); // +20 %
    }

    #[test]
    fn jitter_never_pushes_a_delay_past_the_cap() {
        let backoff = Backoff {
            jitter: 0.5,
            ..PLAIN
        };
        assert!(backoff.delay_for(20, 1.0) <= Duration::minutes(30));
    }

    #[test]
    fn a_random_value_out_of_range_is_clamped_rather_than_trusted() {
        let backoff = Backoff {
            jitter: 0.2,
            ..PLAIN
        };
        assert_eq!(backoff.delay_for(1, 5.0), backoff.delay_for(1, 1.0));
        assert_eq!(backoff.delay_for(1, -5.0), backoff.delay_for(1, 0.0));
    }
}

use std::time::Duration;

/// Politique de retry avec backoff exponentiel.
/// Le délai double à chaque tentative jusqu'au plafond.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,      // nombre max de retries
    pub base_delay: Duration,  // délai initial
    pub max_delay: Duration,   // plafond du backoff
}

impl RetryPolicy {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            base_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(60),
        }
    }

    pub fn no_retry() -> Self {
        Self::new(0) // désactive les retries
    }

    /// Retourne le délai pour une tentative donnée (1-indexé)
    pub fn delay_for(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO; // cas invalide
        }

        // facteur exponentiel (2^(attempt-1))
        let factor = 1u64 << (attempt - 1).min(31);
        let millis = self.base_delay.as_millis() as u64;
        let raw = millis.saturating_mul(factor);
        let capped = raw.min(self.max_delay.as_millis() as u64);

        Duration::from_millis(capped)
    }

    /// Vérifie si un retry est encore autorisé
    pub fn should_retry(&self, attempts_done: u32) -> bool {
        attempts_done < self.max_retries
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_retry() {
        let policy = RetryPolicy::no_retry();
        assert!(!policy.should_retry(0));
    }

    #[test]
    fn test_should_retry_within_limit() {
        let policy = RetryPolicy::new(3);
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }

    #[test]
    fn test_backoff_doubles() {
        let policy = RetryPolicy::new(5);
        let d1 = policy.delay_for(1);
        let d2 = policy.delay_for(2);
        let d3 = policy.delay_for(3);

        assert_eq!(d2, d1 * 2);
        assert_eq!(d3, d2 * 2);
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let policy = RetryPolicy::new(10);
        let d_high = policy.delay_for(10);
        assert_eq!(d_high, policy.max_delay); // plafonnement
    }

    #[test]
    fn test_delay_for_attempt_zero() {
        let policy = RetryPolicy::new(3);
        assert_eq!(policy.delay_for(0), Duration::ZERO);
    }
}
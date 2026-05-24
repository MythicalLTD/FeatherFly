use std::time::{Duration, Instant};

/// Exponential backoff modeled after Pterodactyl Wings `remote/http.go`.
pub struct ExponentialBackoff {
    current: Duration,
    max_interval: Duration,
    max_elapsed: Duration,
    max_retries: u32,
    started: Instant,
    attempt: u32,
}

impl ExponentialBackoff {
    #[must_use]
    pub fn wings(retry_limit: u32) -> Self {
        Self {
            current: Duration::from_millis(500),
            max_interval: Duration::from_secs(12),
            max_elapsed: Duration::from_secs(30),
            max_retries: retry_limit,
            started: Instant::now(),
            attempt: 0,
        }
    }

    /// Returns the delay before the next attempt, or `None` when exhausted.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.max_retries > 0 && self.attempt >= self.max_retries {
            return None;
        }
        if self.max_retries == 0 && self.started.elapsed() >= self.max_elapsed {
            return None;
        }

        let delay = self.current;
        self.attempt += 1;
        self.current = self.current.saturating_mul(2).min(self.max_interval);
        Some(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_at_retry_limit() {
        let mut b = ExponentialBackoff::wings(3);
        assert!(b.next_delay().is_some());
        assert!(b.next_delay().is_some());
        assert!(b.next_delay().is_some());
        assert!(b.next_delay().is_none());
    }
}

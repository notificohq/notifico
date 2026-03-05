use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use uuid::Uuid;

/// In-memory sliding-window rate limiter keyed by API key ID.
pub struct RateLimiter {
    /// Max requests per window
    max_requests: u32,
    /// Window duration in seconds
    window_secs: u64,
    /// Per-key request timestamps
    entries: Mutex<HashMap<Uuid, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a request is allowed. Returns Ok(()) if allowed,
    /// Err(retry_after_secs) if rate limited.
    pub fn check(&self, key_id: Uuid) -> Result<(), u64> {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        let mut entries = self.entries.lock().unwrap();
        let timestamps = entries.entry(key_id).or_default();

        // Remove expired entries
        timestamps.retain(|t| now.duration_since(*t) < window);

        if timestamps.len() >= self.max_requests as usize {
            // Calculate retry-after: time until the oldest entry expires
            let oldest = timestamps[0];
            let elapsed = now.duration_since(oldest);
            let retry_after = self.window_secs.saturating_sub(elapsed.as_secs());
            Err(retry_after.max(1))
        } else {
            timestamps.push(now);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_within_limit() {
        let limiter = RateLimiter::new(3, 60);
        let key = Uuid::now_v7();

        assert!(limiter.check(key).is_ok());
        assert!(limiter.check(key).is_ok());
        assert!(limiter.check(key).is_ok());
    }

    #[test]
    fn rejects_over_limit() {
        let limiter = RateLimiter::new(2, 60);
        let key = Uuid::now_v7();

        assert!(limiter.check(key).is_ok());
        assert!(limiter.check(key).is_ok());
        let result = limiter.check(key);
        assert!(result.is_err());
        assert!(result.unwrap_err() >= 1);
    }
}

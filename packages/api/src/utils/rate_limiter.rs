use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Simple in-memory rate limiter for user creation
#[derive(Debug, Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<DateTime<Utc>>>>>,
    max_requests: u32,
    window_seconds: i64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: u32, window_seconds: i64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window_seconds,
        }
    }

    /// Check if a request is allowed for a given identifier
    pub async fn check_rate_limit(&self, identifier: &str) -> Result<(), String> {
        let mut requests = self.requests.write().await;
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.window_seconds);

        // Get or create the request history for this identifier
        let user_requests = requests
            .entry(identifier.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests outside the window
        user_requests.retain(|&timestamp| timestamp > window_start);

        // Check if we're at the limit
        if user_requests.len() >= self.max_requests as usize {
            let oldest_request = user_requests.first().unwrap();
            let reset_time = *oldest_request + Duration::seconds(self.window_seconds);
            let wait_seconds = (reset_time - now).num_seconds();

            warn!(
                "Rate limit exceeded for {}: {} requests in last {} seconds. Reset in {} seconds",
                identifier,
                user_requests.len(),
                self.window_seconds,
                wait_seconds
            );

            return Err(format!(
                "Rate limit exceeded. Maximum {} requests per {} seconds. Please try again in {} seconds.",
                self.max_requests, self.window_seconds, wait_seconds
            ));
        }

        // Record this request
        user_requests.push(now);

        info!(
            "Request allowed for {}: {}/{} requests in current window",
            identifier,
            user_requests.len(),
            self.max_requests
        );

        Ok(())
    }

    /// Get current usage statistics for an identifier
    pub async fn get_usage_stats(&self, identifier: &str) -> (usize, u32) {
        let requests = self.requests.read().await;
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.window_seconds);

        if let Some(user_requests) = requests.get(identifier) {
            let recent_requests = user_requests
                .iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count();
            (recent_requests, self.max_requests)
        } else {
            (0, self.max_requests)
        }
    }

    /// Reset the rate limit for a specific identifier (admin function)
    pub async fn reset_for_user(&self, identifier: &str) {
        let mut requests = self.requests.write().await;
        requests.remove(identifier);
        info!("Rate limit reset for identifier: {}", identifier);
    }

    /// Cleanup old entries to prevent memory leaks
    pub async fn cleanup(&self) {
        let mut requests = self.requests.write().await;
        let now = Utc::now();
        let cutoff = now - Duration::seconds(self.window_seconds * 2); // Keep entries for 2x window time

        requests.retain(|_, timestamps| {
            timestamps.retain(|&timestamp| timestamp > cutoff);
            !timestamps.is_empty() // Remove empty entries
        });
    }
}

/// Rate limiter specifically for user creation
pub struct UserCreationRateLimiter {
    limiter: RateLimiter,
}

impl UserCreationRateLimiter {
    /// Create a new user creation rate limiter
    ///
    /// Default: 10 users per hour per school manager
    pub fn new() -> Self {
        Self {
            limiter: RateLimiter::new(10, 3600), // 10 requests per hour (3600 seconds)
        }
    }

    /// Create a custom user creation rate limiter
    pub fn with_limits(max_users: u32, window_hours: u32) -> Self {
        Self {
            limiter: RateLimiter::new(max_users, (window_hours * 3600) as i64),
        }
    }

    /// Check if a school manager can create a user
    pub async fn can_create_user(&self, school_manager_id: &Uuid) -> Result<(), String> {
        let identifier = format!("user_creation:{}", school_manager_id);
        self.limiter.check_rate_limit(&identifier).await
    }

    /// Get user creation statistics for a school manager
    pub async fn get_creation_stats(&self, school_manager_id: &Uuid) -> (usize, u32) {
        let identifier = format!("user_creation:{}", school_manager_id);
        self.limiter.get_usage_stats(&identifier).await
    }

    /// Reset rate limit for a school manager (admin function)
    pub async fn reset_manager_limit(&self, school_manager_id: &Uuid) {
        let identifier = format!("user_creation:{}", school_manager_id);
        self.limiter.reset_for_user(&identifier).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2, 1); // 2 requests per second

        let identifier = "test_user";

        // First request should succeed
        assert!(limiter.check_rate_limit(identifier).await.is_ok());

        // Second request should succeed
        assert!(limiter.check_rate_limit(identifier).await.is_ok());

        // Third request should fail
        assert!(limiter.check_rate_limit(identifier).await.is_err());

        // Wait for window to pass
        sleep(TokioDuration::from_millis(1100)).await;

        // Should work again after window reset
        assert!(limiter.check_rate_limit(identifier).await.is_ok());
    }

    #[tokio::test]
    async fn test_user_creation_rate_limiter() {
        let limiter = UserCreationRateLimiter::with_limits(3, 1); // 3 users per hour
        let manager_id = Uuid::new_v4();

        // First 3 requests should succeed
        for i in 0..3 {
            assert!(
                limiter.can_create_user(&manager_id).await.is_ok(),
                "Request {} should succeed",
                i + 1
            );
        }

        // 4th request should fail
        assert!(limiter.can_create_user(&manager_id).await.is_err());

        // Check stats
        let (used, max) = limiter.get_creation_stats(&manager_id).await;
        assert_eq!(used, 3);
        assert_eq!(max, 3);
    }
}

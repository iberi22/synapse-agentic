//! Rate limiting implementations.

use crate::channels::domain::Channel;
use crate::channels::ports::RateLimiter;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Token bucket rate limiter.
pub struct TokenBucketLimiter {
    /// Maximum tokens in bucket.
    pub capacity: u32,
    /// Tokens added per second.
    pub refill_rate: f64,
    /// Current bucket state per channel.
    buckets: Mutex<HashMap<Channel, BucketState>>,
}

struct BucketState {
    tokens: f64,
    last_update: Instant,
}

impl TokenBucketLimiter {
    /// Creates a new token bucket limiter.
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Creates with channel-specific defaults.
    pub fn for_channel(channel: Channel) -> Self {
        match channel {
            Channel::Slack => Self::new(50, 1.0), // 50 burst, 1/sec sustained
            Channel::Discord => Self::new(30, 0.5), // 30 burst, 0.5/sec
            Channel::Telegram => Self::new(30, 1.0), // 30/sec limit
            Channel::Teams => Self::new(20, 0.5), // Conservative
            Channel::WebSocket => Self::new(100, 10.0), // High throughput
            Channel::Webhook => Self::new(10, 0.2), // Low rate
            Channel::Email => Self::new(5, 0.05), // Very low
            Channel::Custom => Self::new(20, 1.0), // Reasonable default
        }
    }

    fn get_or_create_bucket(&self, channel: Channel) -> (f64, Instant) {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(channel).or_insert_with(|| BucketState {
            tokens: self.capacity as f64,
            last_update: Instant::now(),
        });
        (bucket.tokens, bucket.last_update)
    }

    fn update_bucket(&self, channel: Channel, tokens: f64, now: Instant) {
        let mut buckets = self.buckets.lock().unwrap();
        if let Some(bucket) = buckets.get_mut(&channel) {
            bucket.tokens = tokens;
            bucket.last_update = now;
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucketLimiter {
    async fn check(&self, channel: Channel) -> Result<(), Duration> {
        let (current_tokens, last_update) = self.get_or_create_bucket(channel);
        let now = Instant::now();
        let elapsed = now.duration_since(last_update).as_secs_f64();

        // Add tokens based on elapsed time
        let new_tokens = (current_tokens + elapsed * self.refill_rate).min(self.capacity as f64);

        if new_tokens >= 1.0 {
            Ok(())
        } else {
            // Calculate wait time for 1 token
            let tokens_needed = 1.0 - new_tokens;
            let wait_secs = tokens_needed / self.refill_rate;
            Err(Duration::from_secs_f64(wait_secs))
        }
    }

    async fn acquire(&self, channel: Channel) -> Result<(), Duration> {
        let (current_tokens, last_update) = self.get_or_create_bucket(channel);
        let now = Instant::now();
        let elapsed = now.duration_since(last_update).as_secs_f64();

        // Add tokens based on elapsed time
        let new_tokens = (current_tokens + elapsed * self.refill_rate).min(self.capacity as f64);

        if new_tokens >= 1.0 {
            // Consume one token
            self.update_bucket(channel, new_tokens - 1.0, now);
            Ok(())
        } else {
            // Calculate wait time
            let tokens_needed = 1.0 - new_tokens;
            let wait_secs = tokens_needed / self.refill_rate;
            Err(Duration::from_secs_f64(wait_secs))
        }
    }

    async fn release(&self, channel: Channel) {
        // In token bucket, we don't "release" - tokens refill over time
        // But we can optionally add back a token for cancelled operations
        let (current_tokens, _) = self.get_or_create_bucket(channel);
        let new_tokens = (current_tokens + 1.0).min(self.capacity as f64);
        self.update_bucket(channel, new_tokens, Instant::now());
    }

    fn remaining(&self, channel: Channel) -> u32 {
        let (current_tokens, last_update) = self.get_or_create_bucket(channel);
        let elapsed = Instant::now().duration_since(last_update).as_secs_f64();
        let tokens = (current_tokens + elapsed * self.refill_rate).min(self.capacity as f64);
        tokens.floor() as u32
    }
}

/// Sliding window rate limiter.
pub struct SlidingWindowLimiter {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Window duration.
    pub window: Duration,
    /// Request timestamps per channel.
    timestamps: Mutex<HashMap<Channel, Vec<Instant>>>,
}

impl SlidingWindowLimiter {
    /// Creates a new sliding window limiter.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            timestamps: Mutex::new(HashMap::new()),
        }
    }

    /// Creates with requests per minute.
    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(60))
    }

    /// Creates with requests per second.
    pub fn per_second(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(1))
    }

    fn cleanup_old_timestamps(&self, channel: Channel, now: Instant) -> Vec<Instant> {
        let mut timestamps = self.timestamps.lock().unwrap();
        let window_start = now - self.window;

        timestamps
            .entry(channel)
            .or_insert_with(Vec::new)
            .retain(|&t| t > window_start);

        timestamps.get(&channel).cloned().unwrap_or_default()
    }
}

#[async_trait]
impl RateLimiter for SlidingWindowLimiter {
    async fn check(&self, channel: Channel) -> Result<(), Duration> {
        let now = Instant::now();
        let recent = self.cleanup_old_timestamps(channel, now);

        if recent.len() < self.max_requests as usize {
            Ok(())
        } else {
            // Oldest request will expire at window_start + window
            if let Some(&oldest) = recent.first() {
                let expires_at = oldest + self.window;
                if expires_at > now {
                    return Err(expires_at - now);
                }
            }
            Ok(())
        }
    }

    async fn acquire(&self, channel: Channel) -> Result<(), Duration> {
        let now = Instant::now();
        let recent = self.cleanup_old_timestamps(channel, now);

        if recent.len() < self.max_requests as usize {
            let mut timestamps = self.timestamps.lock().unwrap();
            timestamps.entry(channel).or_insert_with(Vec::new).push(now);
            Ok(())
        } else {
            // Calculate wait time
            if let Some(&oldest) = recent.first() {
                let expires_at = oldest + self.window;
                if expires_at > now {
                    return Err(expires_at - now);
                }
            }
            // Should not happen after cleanup, but handle gracefully
            Ok(())
        }
    }

    async fn release(&self, channel: Channel) {
        // Remove the most recent timestamp (for cancelled operations)
        let mut timestamps = self.timestamps.lock().unwrap();
        if let Some(ts) = timestamps.get_mut(&channel) {
            ts.pop();
        }
    }

    fn remaining(&self, channel: Channel) -> u32 {
        let now = Instant::now();
        let recent = self.cleanup_old_timestamps(channel, now);
        self.max_requests.saturating_sub(recent.len() as u32)
    }
}

/// Composite rate limiter combining multiple strategies.
pub struct CompositeLimiter {
    limiters: Vec<Box<dyn RateLimiter + Send + Sync>>,
}

impl CompositeLimiter {
    /// Creates an empty composite limiter.
    pub fn new() -> Self {
        Self {
            limiters: Vec::new(),
        }
    }

    /// Adds a limiter to the composite.
    pub fn add<L: RateLimiter + Send + Sync + 'static>(mut self, limiter: L) -> Self {
        self.limiters.push(Box::new(limiter));
        self
    }
}

impl Default for CompositeLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateLimiter for CompositeLimiter {
    async fn check(&self, channel: Channel) -> Result<(), Duration> {
        let mut max_wait = Duration::ZERO;

        for limiter in &self.limiters {
            if let Err(wait) = limiter.check(channel).await {
                max_wait = max_wait.max(wait);
            }
        }

        if max_wait > Duration::ZERO {
            Err(max_wait)
        } else {
            Ok(())
        }
    }

    async fn acquire(&self, channel: Channel) -> Result<(), Duration> {
        // First check all
        self.check(channel).await?;

        // Then acquire from all
        for limiter in &self.limiters {
            if let Err(wait) = limiter.acquire(channel).await {
                // Rollback previous acquisitions
                for prev in &self.limiters {
                    prev.release(channel).await;
                    if std::ptr::eq(prev.as_ref(), limiter.as_ref()) {
                        break;
                    }
                }
                return Err(wait);
            }
        }
        Ok(())
    }

    async fn release(&self, channel: Channel) {
        for limiter in &self.limiters {
            limiter.release(channel).await;
        }
    }

    fn remaining(&self, channel: Channel) -> u32 {
        self.limiters
            .iter()
            .map(|l| l.remaining(channel))
            .min()
            .unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let limiter = TokenBucketLimiter::new(5, 1.0);

        // Should have capacity available
        assert!(limiter.check(Channel::Slack).await.is_ok());
        assert_eq!(limiter.remaining(Channel::Slack), 5);
    }

    #[tokio::test]
    async fn test_token_bucket_acquire() {
        let limiter = TokenBucketLimiter::new(3, 1.0);

        // Acquire all tokens
        assert!(limiter.acquire(Channel::Slack).await.is_ok());
        assert!(limiter.acquire(Channel::Slack).await.is_ok());
        assert!(limiter.acquire(Channel::Slack).await.is_ok());

        // Next should fail
        let result = limiter.acquire(Channel::Slack).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sliding_window_basic() {
        let limiter = SlidingWindowLimiter::per_second(5);

        assert!(limiter.check(Channel::Discord).await.is_ok());
        assert_eq!(limiter.remaining(Channel::Discord), 5);
    }

    #[tokio::test]
    async fn test_sliding_window_acquire() {
        let limiter = SlidingWindowLimiter::new(2, Duration::from_secs(10));

        assert!(limiter.acquire(Channel::Telegram).await.is_ok());
        assert!(limiter.acquire(Channel::Telegram).await.is_ok());

        // Third should fail
        let result = limiter.acquire(Channel::Telegram).await;
        assert!(result.is_err());
        assert_eq!(limiter.remaining(Channel::Telegram), 0);
    }

    #[tokio::test]
    async fn test_composite_limiter() {
        let limiter = CompositeLimiter::new()
            .add(TokenBucketLimiter::new(10, 1.0))
            .add(SlidingWindowLimiter::per_second(5));

        // Should use minimum of both
        assert!(limiter.check(Channel::Slack).await.is_ok());
        assert_eq!(limiter.remaining(Channel::Slack), 5); // Min of 10 and 5
    }

    #[test]
    fn test_channel_specific_defaults() {
        let slack = TokenBucketLimiter::for_channel(Channel::Slack);
        assert_eq!(slack.capacity, 50);

        let email = TokenBucketLimiter::for_channel(Channel::Email);
        assert_eq!(email.capacity, 5);
    }

    #[tokio::test]
    async fn test_release_restores_capacity() {
        let limiter = TokenBucketLimiter::new(2, 0.0); // No refill

        limiter.acquire(Channel::Slack).await.ok();
        limiter.acquire(Channel::Slack).await.ok();
        assert_eq!(limiter.remaining(Channel::Slack), 0);

        limiter.release(Channel::Slack).await;
        assert_eq!(limiter.remaining(Channel::Slack), 1);
    }
}

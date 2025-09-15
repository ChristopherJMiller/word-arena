use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    tokens: u32,
    max_tokens: u32,
    refill_rate: Duration,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            tokens: 30, // Start with full bucket
            max_tokens: 30, // Max 30 tokens
            refill_rate: Duration::from_secs(2), // Refill 1 token every 2 seconds
            last_refill: Instant::now(),
        }
    }
    
    pub fn new_with_limits(max_tokens: u32, refill_rate: Duration) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }
    
    pub async fn check_rate_limit(&mut self) -> bool {
        self.refill_tokens();
        
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
    
    fn refill_tokens(&mut self) {
        let now = Instant::now();
        let time_passed = now.duration_since(self.last_refill);
        
        if time_passed >= self.refill_rate {
            let tokens_to_add = (time_passed.as_secs() / self.refill_rate.as_secs()) as u32;
            self.tokens = (self.tokens + tokens_to_add).min(self.max_tokens);
            self.last_refill = now;
        }
    }
    
    pub fn get_remaining_tokens(&mut self) -> u32 {
        self.refill_tokens();
        self.tokens
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
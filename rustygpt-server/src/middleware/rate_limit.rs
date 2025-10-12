use std::{collections::HashMap, sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::State,
    http::{self, Method, Request},
    middleware::Next,
    response::Response,
};
use tokio::sync::Mutex;

use crate::http::error::{ApiError, AppResult};
use shared::config::server::Config;

#[derive(Clone)]
pub struct RateLimitState {
    store: Arc<Mutex<HashMap<String, Bucket>>>,
    auth_strategy: Strategy,
    default_strategy: Strategy,
}

impl RateLimitState {
    pub fn from_config(config: &Config) -> Self {
        let default_refill = f64::from(config.rate_limits.default_rps.max(0.1_f32));
        let default = Strategy::new(config.rate_limits.burst as f64, default_refill);

        let auth_per_min = config.rate_limits.auth_login_per_ip_per_min.max(1) as f64;
        let auth = Strategy::new(auth_per_min, auth_per_min / 60.0);

        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            auth_strategy: auth,
            default_strategy: default,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Strategy {
    capacity: f64,
    refill_per_sec: f64,
}

impl Strategy {
    fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            capacity: capacity.max(1.0),
            refill_per_sec: refill_per_sec.max(0.1),
        }
    }
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
    strategy: Strategy,
}

impl Bucket {
    fn new(strategy: Strategy) -> Self {
        Self {
            tokens: strategy.capacity,
            last_refill: Instant::now(),
            strategy,
        }
    }

    fn take(&mut self) -> RateLimitOutcome {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            let remaining = self.tokens.floor() as u32;
            let deficit = (self.strategy.capacity - self.tokens).max(0.0);
            let reset_after = if self.tokens >= self.strategy.capacity {
                0
            } else {
                (deficit / self.strategy.refill_per_sec).ceil() as u64
            };

            RateLimitOutcome::Allowed {
                limit: self.strategy.capacity as u32,
                remaining,
                reset_after,
            }
        } else {
            let needed = 1.0 - self.tokens;
            let retry_after = (needed / self.strategy.refill_per_sec).ceil() as u64;
            RateLimitOutcome::Denied { retry_after }
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }

        self.tokens =
            (self.tokens + elapsed * self.strategy.refill_per_sec).min(self.strategy.capacity);
        self.last_refill = now;
    }
}

enum RateLimitOutcome {
    Allowed {
        limit: u32,
        remaining: u32,
        reset_after: u64,
    },
    Denied {
        retry_after: u64,
    },
}

pub async fn enforce_rate_limits(
    State(state): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let key = determine_key(request.uri().path());
    let outcome = acquire(&state, &key).await;

    match outcome {
        RateLimitOutcome::Allowed {
            limit,
            remaining,
            reset_after,
        } => {
            let mut response = next.run(request).await;
            attach_rate_limit_headers(&mut response, limit, remaining, reset_after);
            Ok(response)
        }
        RateLimitOutcome::Denied { retry_after } => Err(ApiError::too_many_requests(
            "rate limit exceeded",
        )
        .with_details(serde_json::json!({
            "retry_after_seconds": retry_after
        }))),
    }
}

async fn acquire(state: &RateLimitState, key: &str) -> RateLimitOutcome {
    let strategy = if key.starts_with("/api/auth") {
        state.auth_strategy
    } else {
        state.default_strategy
    };

    let mut guard = state.store.lock().await;
    let bucket = guard
        .entry(key.to_string())
        .or_insert_with(|| Bucket::new(strategy));
    bucket.strategy = strategy;
    bucket.take()
}

fn determine_key(path: &str) -> String {
    path.to_string()
}

fn attach_rate_limit_headers(
    response: &mut Response,
    limit: u32,
    remaining: u32,
    reset_after: u64,
) {
    const LIMIT: &str = "RateLimit-Limit";
    const REMAINING: &str = "RateLimit-Remaining";
    const RESET: &str = "RateLimit-Reset";

    let headers = response.headers_mut();
    headers.insert(LIMIT, header_value(limit));
    headers.insert(REMAINING, header_value(remaining));
    headers.insert(RESET, header_value(reset_after));
}

fn header_value<T: ToString>(value: T) -> http::HeaderValue {
    http::HeaderValue::from_str(&value.to_string())
        .unwrap_or_else(|_| http::HeaderValue::from_static("0"))
}

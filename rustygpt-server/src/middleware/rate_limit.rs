use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::State,
    http::{self, Method, Request},
    middleware::Next,
    response::Response,
};
use metrics::{counter, gauge};
use serde_json::Value;
use sqlx::PgPool;
use tokio::{
    sync::{Mutex, RwLock},
    time::{self, MissedTickBehavior},
};
use tracing::warn;

use crate::http::error::{ApiError, AppResult};
use shared::config::server::Config;
use uuid::Uuid;

#[derive(Clone)]
pub struct RateLimitState {
    store: Arc<Mutex<HashMap<String, Bucket>>>,
    assignments: Arc<RwLock<Vec<RouteStrategy>>>,
    pool: Option<PgPool>,
    auth_strategy: Strategy,
    default_strategy: Strategy,
}

impl RateLimitState {
    pub fn new(config: &Config, pool: Option<PgPool>) -> Self {
        let default_refill = f64::from(config.rate_limits.default_rps.max(0.1_f32));
        let default = Strategy::new(config.rate_limits.burst as f64, default_refill);

        let auth_per_min = config.rate_limits.auth_login_per_ip_per_min.max(1) as f64;
        let auth = Strategy::new(auth_per_min, auth_per_min / 60.0);

        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(Vec::new())),
            pool,
            auth_strategy: auth,
            default_strategy: default,
        }
    }

    pub async fn reload_from_db(&self) -> Result<(), sqlx::Error> {
        let pool = match &self.pool {
            Some(pool) => pool.clone(),
            None => return Ok(()),
        };

        let profiles = sqlx::query_as::<_, DbProfile>(
            "SELECT profile_id, algorithm, params FROM rustygpt.sp_limits_list_profiles()",
        )
        .fetch_all(&pool)
        .await?;

        let mut profile_map = HashMap::new();
        for profile in profiles {
            if let Some(strategy) = strategy_from_profile(&profile.algorithm, &profile.params) {
                profile_map.insert(profile.profile_id, strategy);
            } else {
                warn!(profile_id = %profile.profile_id, algorithm = %profile.algorithm, "unsupported rate limit algorithm");
            }
        }

        let assignments = sqlx::query_as::<_, DbAssignment>(
            "SELECT assignment_id, profile_id, profile_name, method, path_pattern FROM rustygpt.sp_limits_list_assignments()",
        )
        .fetch_all(&pool)
        .await?;

        let mut routes = Vec::with_capacity(assignments.len());
        for assignment in assignments.iter() {
            let Some(strategy) = profile_map.get(&assignment.profile_id) else {
                warn!(assignment_id = %assignment.assignment_id, profile_id = %assignment.profile_id, "missing profile for assignment");
                continue;
            };

            let method = match Method::from_bytes(assignment.method.as_bytes()) {
                Ok(method) => method,
                Err(_) => {
                    warn!(assignment_id = %assignment.assignment_id, method = %assignment.method, "invalid HTTP method in rate limit assignment");
                    continue;
                }
            };

            routes.push(RouteStrategy {
                method,
                pattern: assignment.path_pattern.clone(),
                bucket_key: assignment.path_pattern.clone(),
                strategy: *strategy,
                profile: assignment.profile_name.clone(),
            });
        }

        gauge!("rustygpt_limits_profiles").set(profile_map.len() as f64);
        gauge!("rustygpt_limits_assignments").set(routes.len() as f64);

        let mut guard = self.assignments.write().await;
        *guard = routes;

        Ok(())
    }

    async fn select_strategy(&self, method: &Method, path: &str) -> AppliedStrategy {
        let assignments = self.assignments.read().await;
        if let Some(route) = assignments
            .iter()
            .find(|route| route.method == *method && route.matches(path))
        {
            return AppliedStrategy {
                strategy: route.strategy,
                bucket_key: route.bucket_key.clone(),
                profile: route.profile.clone(),
            };
        }

        if path.starts_with("/api/auth/login") {
            AppliedStrategy {
                strategy: self.auth_strategy,
                bucket_key: "auth_login".to_string(),
                profile: "auth_login".to_string(),
            }
        } else {
            AppliedStrategy {
                strategy: self.default_strategy,
                bucket_key: path.to_string(),
                profile: "default".to_string(),
            }
        }
    }

    pub fn spawn_auto_refresh(self: Arc<Self>, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                ticker.tick().await;
                if let Err(err) = self.reload_from_db().await {
                    warn!(error = %err, "failed to refresh rate limit configuration from database");
                }
            }
        })
    }
}

#[derive(Clone)]
struct RouteStrategy {
    method: Method,
    pattern: String,
    bucket_key: String,
    strategy: Strategy,
    profile: String,
}

impl RouteStrategy {
    fn matches(&self, path: &str) -> bool {
        pattern_matches(&self.pattern, path)
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

#[derive(Clone)]
struct AppliedStrategy {
    strategy: Strategy,
    bucket_key: String,
    profile: String,
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

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let applied = state.select_strategy(&method, &path).await;
    let key = format!("{} {}", method.as_str(), applied.bucket_key);
    let outcome = acquire(&state, &key, applied.strategy).await;

    match outcome {
        RateLimitOutcome::Allowed {
            limit,
            remaining,
            reset_after,
        } => {
            counter!(
                "http_rate_limit_requests_total",
                "key" => key.clone(),
                "profile" => applied.profile.clone(),
                "result" => "allowed"
            )
            .increment(1);
            gauge!(
                "http_rate_limit_remaining",
                "key" => key.clone(),
                "profile" => applied.profile.clone()
            )
            .set(remaining as f64);
            gauge!(
                "http_rate_limit_reset_seconds",
                "key" => key.clone(),
                "profile" => applied.profile.clone()
            )
            .set(reset_after as f64);
            let mut response = next.run(request).await;
            attach_rate_limit_headers(
                &mut response,
                limit,
                remaining,
                reset_after,
                &applied.profile,
            );
            Ok(response)
        }
        RateLimitOutcome::Denied { retry_after } => {
            counter!(
                "http_rate_limit_requests_total",
                "key" => key.clone(),
                "profile" => applied.profile.clone(),
                "result" => "denied"
            )
            .increment(1);
            gauge!(
                "http_rate_limit_reset_seconds",
                "key" => key.clone(),
                "profile" => applied.profile.clone()
            )
            .set(retry_after as f64);
            Err(ApiError::too_many_requests("rate limit exceeded")
                .with_details(serde_json::json!({ "retry_after_seconds": retry_after })))
        }
    }
}

async fn acquire(state: &RateLimitState, key: &str, strategy: Strategy) -> RateLimitOutcome {
    let mut guard = state.store.lock().await;
    let bucket = guard
        .entry(key.to_string())
        .or_insert_with(|| Bucket::new(strategy));
    bucket.strategy = strategy;
    bucket.take()
}

fn attach_rate_limit_headers(
    response: &mut Response,
    limit: u32,
    remaining: u32,
    reset_after: u64,
    profile: &str,
) {
    const LIMIT: &str = "RateLimit-Limit";
    const REMAINING: &str = "RateLimit-Remaining";
    const RESET: &str = "RateLimit-Reset";
    const PROFILE: &str = "X-RateLimit-Profile";

    let headers = response.headers_mut();
    headers.insert(LIMIT, header_value(limit));
    headers.insert(REMAINING, header_value(remaining));
    headers.insert(RESET, header_value(reset_after));
    headers.insert(PROFILE, header_value(profile));
}

fn header_value<T: ToString>(value: T) -> http::HeaderValue {
    http::HeaderValue::from_str(&value.to_string())
        .unwrap_or_else(|_| http::HeaderValue::from_static("0"))
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let pattern = pattern.trim_end_matches('/');
    let path = path.trim_end_matches('/');

    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path.starts_with(prefix);
    }

    let pattern_segments: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let path_segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if pattern_segments.len() != path_segments.len() {
        return false;
    }

    for (pat, seg) in pattern_segments.iter().zip(path_segments.iter()) {
        if *pat == "*" {
            continue;
        }
        if pat.starts_with(':') {
            continue;
        }
        if pat != seg {
            return false;
        }
    }

    true
}

fn strategy_from_profile(algorithm: &str, params: &Value) -> Option<Strategy> {
    match algorithm {
        "gcra" => {
            let requests_per_second = params
                .get("requests_per_second")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let burst = params
                .get("burst")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .max(1) as f64;
            Some(Strategy::new(burst, requests_per_second.max(0.1)))
        }
        _ => None,
    }
}

#[derive(sqlx::FromRow)]
struct DbProfile {
    profile_id: Uuid,
    algorithm: String,
    params: Value,
}

#[derive(sqlx::FromRow)]
struct DbAssignment {
    assignment_id: Uuid,
    profile_id: Uuid,
    profile_name: String,
    method: String,
    path_pattern: String,
}

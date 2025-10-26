use std::{collections::HashMap, fmt, net::IpAddr, str::FromStr, sync::Arc};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use serde_json::Value as JsonValue;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, pool::PoolConnection, types::Json};
use thiserror::Error;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use shared::{
    config::server::{Config, CookieSameSite},
    models::UserRole,
};

const LOGIN_METRIC_NAME: &str = "rustygpt_auth_logins_total";
const ROTATION_METRIC_NAME: &str = "rustygpt_auth_session_rotations_total";
const ACTIVE_SESSIONS_METRIC_NAME: &str = "rustygpt_auth_active_sessions";
const ALL_ROLES: &[UserRole] = &[UserRole::Admin, UserRole::Member, UserRole::ReadOnly];

/// Errors produced by the session subsystem.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("password hashing failed: {0}")]
    PasswordHash(String),
    #[error("password verification failed")]
    InvalidCredentials,
    #[error("user account disabled")]
    DisabledUser,
    #[error("session expired")]
    SessionExpired,
    #[error("session absolute lifetime exceeded")]
    AbsoluteExpired,
    #[error("session rotation required")]
    RotationRequired,
    #[error("suspicious session activity")]
    SuspiciousActivity,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("time conversion error: {0}")]
    TimeConversion(String),
}

/// Metadata captured when issuing or refreshing a session.
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub fingerprint: Option<String>,
}

impl SessionMetadata {
    #[must_use]
    pub fn with_user_agent<T: Into<Option<String>>>(mut self, value: T) -> Self {
        self.user_agent = value.into();
        self
    }

    #[must_use]
    pub fn with_ip(mut self, value: Option<IpAddr>) -> Self {
        self.ip = value.map(|addr| addr.to_string());
        self
    }

    #[must_use]
    pub fn with_ip_str<T: Into<Option<String>>>(mut self, value: T) -> Self {
        self.ip = value.into();
        self
    }

    #[must_use]
    pub fn with_fingerprint<T: Into<Option<String>>>(mut self, value: T) -> Self {
        self.fingerprint = value.into();
        self
    }

    #[must_use]
    pub fn as_json(&self) -> serde_json::Value {
        json!({
            "user_agent": self.user_agent,
            "ip": self.ip,
            "fingerprint": self.fingerprint,
        })
    }

    #[must_use]
    pub fn from_stored(
        user_agent: Option<String>,
        ip: Option<String>,
        client_meta: Option<&JsonValue>,
    ) -> Self {
        let fingerprint = client_meta
            .and_then(|meta| meta.get("fingerprint"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        Self::default()
            .with_user_agent(user_agent)
            .with_ip_str(ip)
            .with_fingerprint(fingerprint)
    }

    #[must_use]
    pub fn suspicious_mismatch(&self, current: &Self) -> bool {
        fn changed<F>(stored: &Option<String>, current: &Option<String>, cmp: F) -> bool
        where
            F: Fn(&str, &str) -> bool,
        {
            match (stored.as_ref(), current.as_ref()) {
                (Some(stored), Some(now)) => cmp(stored, now),
                _ => false,
            }
        }

        let ua_changed = changed(&self.user_agent, &current.user_agent, |a, b| a != b);
        let ip_changed = changed(&self.ip, &current.ip, |a, b| a != b);
        let fingerprint_changed = changed(&self.fingerprint, &current.fingerprint, |a, b| a != b);

        ua_changed || ip_changed || fingerprint_changed
    }
}

/// Authenticated user details attached to the request context.
#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: Option<String>,
    pub roles: Vec<UserRole>,
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub absolute_expires_at: DateTime<Utc>,
}

/// Session issuance output containing the raw token and encoded cookie.
#[derive(Debug, Clone)]
pub struct SessionBundle {
    pub session_cookie: Cookie<'static>,
    pub csrf_token: String,
    pub csrf_cookie: Cookie<'static>,
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub absolute_expires_at: DateTime<Utc>,
}

/// Successful validation result used by the auth middleware.
#[derive(Debug, Clone)]
pub struct SessionValidation {
    pub user: SessionUser,
    pub bundle: Option<SessionBundle>,
    pub rotated: bool,
}

/// Database-backed session manager.
#[derive(Clone)]
pub struct SessionService {
    pool: PgPool,
    config: Arc<Config>,
}

impl SessionService {
    pub const fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    fn rotation_threshold(&self) -> Duration {
        let idle = self.config.session.idle_seconds.max(1);
        let threshold = (idle / 2).max(1);
        Duration::seconds(threshold as i64)
    }

    fn build_cookie(
        &self,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Cookie<'static>, SessionError> {
        let expires_utc =
            OffsetDateTime::from_unix_timestamp(expires_at.timestamp()).map_err(|err| {
                SessionError::TimeConversion(format!("failed to convert cookie expiry: {err}"))
            })?;
        let max_age = (expires_utc - OffsetDateTime::now_utc()).max(TimeDuration::seconds(0));
        let same_site = Self::map_same_site(self.config.security.cookie.same_site);

        let mut builder = Cookie::build((
            self.config.session.session_cookie_name.clone(),
            token.to_owned(),
        ))
        .path("/")
        .http_only(true)
        .secure(self.config.security.cookie.secure)
        .same_site(same_site)
        .max_age(max_age)
        .expires(expires_utc);

        if let Some(domain) = &self.config.security.cookie.domain {
            builder = builder.domain(domain.clone());
        }

        Ok(builder.build())
    }

    fn build_csrf_cookie(
        &self,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Cookie<'static>, SessionError> {
        let expires_utc =
            OffsetDateTime::from_unix_timestamp(expires_at.timestamp()).map_err(|err| {
                SessionError::TimeConversion(format!("failed to convert CSRF cookie expiry: {err}"))
            })?;
        let max_age = (expires_utc - OffsetDateTime::now_utc()).max(TimeDuration::seconds(0));
        let mut builder = Cookie::build((
            self.config.session.csrf_cookie_name.clone(),
            token.to_owned(),
        ))
        .path("/")
        .http_only(false)
        .secure(self.config.security.cookie.secure)
        .same_site(SameSite::Strict)
        .max_age(max_age)
        .expires(expires_utc);

        if let Some(domain) = &self.config.security.cookie.domain {
            builder = builder.domain(domain.clone());
        }

        Ok(builder.build())
    }

    const fn map_same_site(value: CookieSameSite) -> SameSite {
        match value {
            CookieSameSite::Lax => SameSite::Lax,
            CookieSameSite::Strict => SameSite::Strict,
            CookieSameSite::None => SameSite::None,
        }
    }

    fn new_token() -> Result<(String, Vec<u8>), SessionError> {
        let mut raw = [0u8; 32];
        OsRng.fill_bytes(&mut raw);
        let token = URL_SAFE_NO_PAD.encode(raw);
        let hash = Sha256::digest(token.as_bytes());
        Ok((token, hash.to_vec()))
    }

    fn new_csrf_token() -> String {
        let mut raw = [0u8; 16];
        OsRng.fill_bytes(&mut raw);
        URL_SAFE_NO_PAD.encode(raw)
    }

    fn hash_for_token(token: &str) -> Vec<u8> {
        Sha256::digest(token.as_bytes()).to_vec()
    }

    async fn acquire_connection(&self) -> Result<PoolConnection<Postgres>, SessionError> {
        self.pool.acquire().await.map_err(SessionError::Database)
    }

    fn max_sessions_per_user(&self) -> Option<i32> {
        self.config
            .session
            .max_sessions_per_user
            .map(|value| value as i32)
    }

    #[instrument(skip(self, password), fields(identifier = %identifier))]
    pub async fn authenticate(
        &self,
        identifier: &str,
        password: &str,
        metadata: &SessionMetadata,
    ) -> Result<(SessionUser, SessionBundle), SessionError> {
        let mut conn = match self.acquire_connection().await {
            Ok(conn) => conn,
            Err(err) => {
                record_login_metric("error");
                return Err(err);
            }
        };

        let record = sqlx::query_as::<_, CredentialRow>(
            "SELECT id,
                    email::TEXT AS email,
                    username::TEXT AS username,
                    display_name,
                    password_hash,
                    disabled_at
             FROM rustygpt.users
             WHERE email = $1::citext OR username = $1::citext",
        )
        .bind(identifier)
        .fetch_optional(conn.as_mut())
        .await
        .map_err(|err| {
            record_login_metric("error");
            err
        })?;

        let row = match record {
            Some(row) => row,
            None => {
                record_login_metric("invalid_credentials");
                return Err(SessionError::InvalidCredentials);
            }
        };

        if row.disabled_at.is_some() {
            record_login_metric("disabled");
            return Err(SessionError::DisabledUser);
        }

        if let Err(err) = verify_password(&row.password_hash, password) {
            record_login_metric("invalid_credentials");
            return Err(err);
        }

        let roles = match self.load_roles(&mut conn, row.id).await {
            Ok(roles) => roles,
            Err(err) => {
                record_login_metric("error");
                return Err(err);
            }
        };

        let bundle = match self
            .issue_session(&mut conn, row.id, &roles, metadata)
            .await
        {
            Ok(bundle) => bundle,
            Err(err) => {
                record_login_metric("error");
                return Err(err);
            }
        };

        record_login_metric("success");
        if let Err(err) = self.refresh_active_session_metrics(&mut conn).await {
            warn!(error = %err, "failed to refresh active session metrics after login");
        }

        let user = SessionUser {
            id: row.id,
            email: row.email,
            username: row.username,
            display_name: row.display_name.clone(),
            roles,
            session_id: bundle.session_id,
            issued_at: bundle.issued_at,
            expires_at: bundle.expires_at,
            absolute_expires_at: bundle.absolute_expires_at,
        };

        Ok((user, bundle))
    }

    #[instrument(skip(self, token, metadata))]
    pub async fn validate_session(
        &self,
        token: &str,
        metadata: &SessionMetadata,
    ) -> Result<Option<SessionValidation>, SessionError> {
        if token.trim().is_empty() {
            return Ok(None);
        }

        let mut conn = self.acquire_connection().await?;
        let hash = Self::hash_for_token(token);

        let session = sqlx::query_as::<_, ActiveSessionRow>(
            "SELECT s.id AS session_id,
                    s.user_id,
                    s.issued_at,
                    s.expires_at,
                    s.absolute_expires_at,
                    s.requires_rotation,
                    s.roles_snapshot,
                    s.user_agent,
                    s.ip::TEXT AS ip,
                    s.client_meta,
                    u.email::TEXT AS email,
                    u.username::TEXT AS username,
                    u.display_name,
                    u.disabled_at
             FROM rustygpt.user_sessions s
             JOIN rustygpt.users u ON u.id = s.user_id
             WHERE s.token_hash = $1
               AND s.revoked_at IS NULL
             FOR UPDATE",
        )
        .bind(hash.clone())
        .fetch_optional(conn.as_mut())
        .await?;

        let Some(mut row) = session else {
            return Ok(None);
        };

        if row.disabled_at.is_some() {
            self.logout_session(&mut conn, row.session_id, Some("disabled"))
                .await?;
            return Err(SessionError::DisabledUser);
        }

        let now = Utc::now();
        if row.expires_at <= now {
            self.logout_session(&mut conn, row.session_id, Some("idle_expired"))
                .await?;
            return Err(SessionError::SessionExpired);
        }

        if row.absolute_expires_at <= now {
            self.logout_session(&mut conn, row.session_id, Some("absolute_expired"))
                .await?;
            return Err(SessionError::AbsoluteExpired);
        }

        if self.config.auth.suspicious_check {
            let stored_metadata = SessionMetadata::from_stored(
                row.user_agent.clone(),
                row.ip.clone(),
                row.client_meta.as_ref().map(|meta| &meta.0),
            );

            if stored_metadata.suspicious_mismatch(metadata) {
                self.flag_session_rotation(&mut conn, row.session_id)
                    .await?;
                return Err(SessionError::SuspiciousActivity);
            }
        }

        let roles = self.load_roles(&mut conn, row.user_id).await?;
        let snapshot_mismatch = roles_snapshot_changed(&roles, &row.roles_snapshot);

        let threshold = self.rotation_threshold();
        let rotation_cause = if row.requires_rotation {
            Some(RotationCause::RequiresRotation)
        } else if snapshot_mismatch {
            Some(RotationCause::RoleChange)
        } else if (row.expires_at - now) <= threshold {
            Some(RotationCause::IdleRefresh)
        } else {
            None
        };

        let (rotated, bundle) = if let Some(cause) = rotation_cause {
            let rotated_bundle = self
                .rotate_session(
                    &mut conn,
                    row.session_id,
                    row.user_id,
                    &roles,
                    metadata,
                    cause,
                )
                .await?;

            row.session_id = rotated_bundle.session_id;
            row.issued_at = rotated_bundle.issued_at;
            row.expires_at = rotated_bundle.expires_at;
            row.absolute_expires_at = rotated_bundle.absolute_expires_at;
            (true, Some(rotated_bundle))
        } else {
            self.touch_session(&mut conn, row.session_id, metadata)
                .await?;
            (false, None)
        };

        drop(conn);

        let user = SessionUser {
            id: row.user_id,
            email: row.email,
            username: row.username,
            display_name: row.display_name.clone(),
            roles,
            session_id: row.session_id,
            issued_at: row.issued_at,
            expires_at: row.expires_at,
            absolute_expires_at: row.absolute_expires_at,
        };

        Ok(Some(SessionValidation {
            user,
            bundle,
            rotated,
        }))
    }

    pub async fn refresh_session(
        &self,
        token: &str,
        metadata: &SessionMetadata,
    ) -> Result<Option<(SessionUser, SessionBundle)>, SessionError> {
        let validation = match self.validate_session(token, metadata).await? {
            Some(value) => value,
            None => return Ok(None),
        };

        let SessionValidation {
            mut user, bundle, ..
        } = validation;
        if let Some(bundle) = bundle {
            user.session_id = bundle.session_id;
            user.issued_at = bundle.issued_at;
            user.expires_at = bundle.expires_at;
            user.absolute_expires_at = bundle.absolute_expires_at;
            return Ok(Some((user, bundle)));
        }

        let mut conn = self.acquire_connection().await?;
        let rotated_bundle = self
            .rotate_session(
                &mut conn,
                user.session_id,
                user.id,
                &user.roles,
                metadata,
                RotationCause::ExplicitRefresh,
            )
            .await?;
        drop(conn);

        user.session_id = rotated_bundle.session_id;
        user.issued_at = rotated_bundle.issued_at;
        user.expires_at = rotated_bundle.expires_at;
        user.absolute_expires_at = rotated_bundle.absolute_expires_at;

        Ok(Some((user, rotated_bundle)))
    }

    pub async fn revoke_session_by_id(
        &self,
        session_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), SessionError> {
        let mut conn = self.acquire_connection().await?;
        self.logout_session(&mut conn, session_id, reason).await
    }

    #[instrument(skip(self, reason), fields(user_id = %user_id))]
    pub async fn mark_user_for_rotation(
        &self,
        user_id: Uuid,
        reason: &str,
    ) -> Result<i64, SessionError> {
        let mut conn = self.acquire_connection().await?;
        let updated = sqlx::query_scalar::<_, i64>("SELECT rustygpt.sp_auth_mark_rotation($1, $2)")
            .bind(user_id)
            .bind(reason)
            .fetch_one(conn.as_mut())
            .await?;

        Ok(updated)
    }

    #[instrument(skip(self, roles, metadata))]
    async fn issue_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        user_id: Uuid,
        roles: &[UserRole],
        metadata: &SessionMetadata,
    ) -> Result<SessionBundle, SessionError> {
        let (token, hash) = Self::new_token()?;
        let csrf_token = Self::new_csrf_token();
        let roles_text: Vec<String> = roles.iter().map(|role| role.as_str().to_string()).collect();
        let client_meta = Json(metadata.as_json());
        let idle_seconds = self.config.session.idle_seconds as i32;
        let absolute_seconds = self.config.session.absolute_seconds as i32;

        let record = sqlx::query_as::<_, SessionLoginRow>(
            "SELECT session_id,
                    issued_at,
                    expires_at,
                    absolute_expires_at
             FROM rustygpt.sp_auth_login(
                 $1,
                 $2,
                 $3,
                 $4,
                 $5,
                 $6::TEXT[],
                 $7,
                 $8,
                 $9
             )",
        )
        .bind(user_id)
        .bind(&hash)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
        .bind(client_meta)
        .bind(&roles_text)
        .bind(idle_seconds)
        .bind(absolute_seconds)
        .bind(self.max_sessions_per_user())
        .fetch_one(conn.as_mut())
        .await?;

        debug!(
            user_id = %user_id,
            session_id = %record.session_id,
            "issued new session"
        );

        let session_cookie = self.build_cookie(&token, record.expires_at)?;
        let csrf_cookie = self.build_csrf_cookie(&csrf_token, record.expires_at)?;

        Ok(SessionBundle {
            session_cookie,
            csrf_token,
            csrf_cookie,
            session_id: record.session_id,
            issued_at: record.issued_at,
            expires_at: record.expires_at,
            absolute_expires_at: record.absolute_expires_at,
        })
    }

    #[instrument(skip(self, roles, metadata))]
    async fn rotate_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
        user_id: Uuid,
        roles: &[UserRole],
        metadata: &SessionMetadata,
        cause: RotationCause,
    ) -> Result<SessionBundle, SessionError> {
        let (token, hash) = Self::new_token()?;
        let csrf_token = Self::new_csrf_token();
        let roles_text: Vec<String> = roles.iter().map(|role| role.as_str().to_string()).collect();
        let client_meta = Json(metadata.as_json());
        let idle_seconds = self.config.session.idle_seconds as i32;

        let record = sqlx::query_as::<_, SessionRefreshRow>(
            "SELECT next_session_id,
                    user_id,
                    issued_at,
                    expires_at,
                    absolute_expires_at
             FROM rustygpt.sp_auth_refresh(
                 $1,
                 $2,
                 $3,
                 $4,
                 $5,
                 $6::TEXT[],
                 $7
             )",
        )
        .bind(session_id)
        .bind(&hash)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
        .bind(client_meta)
        .bind(&roles_text)
        .bind(idle_seconds)
        .fetch_one(conn.as_mut())
        .await?;

        if record.user_id != user_id {
            return Err(SessionError::RotationRequired);
        }

        debug!(
            old_session_id = %session_id,
            new_session_id = %record.next_session_id,
            reason = %cause,
            "rotated session"
        );

        record_rotation_metric(cause);
        if let Err(err) = self.refresh_active_session_metrics(conn).await {
            warn!(
                error = %err,
                session_id = %record.next_session_id,
                "failed to refresh active session metrics after rotation"
            );
        }

        let session_cookie = self.build_cookie(&token, record.expires_at)?;
        let csrf_cookie = self.build_csrf_cookie(&csrf_token, record.expires_at)?;

        Ok(SessionBundle {
            session_cookie,
            csrf_token,
            csrf_cookie,
            session_id: record.next_session_id,
            issued_at: record.issued_at,
            expires_at: record.expires_at,
            absolute_expires_at: record.absolute_expires_at,
        })
    }

    async fn touch_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
        metadata: &SessionMetadata,
    ) -> Result<(), SessionError> {
        let client_meta = Json(metadata.as_json());
        sqlx::query(
            "UPDATE rustygpt.user_sessions
             SET last_seen_at = now(),
                 user_agent = COALESCE($2, user_agent),
                 ip = COALESCE($3::inet, ip),
                 client_meta = $4
             WHERE id = $1",
        )
        .bind(session_id)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
        .bind(client_meta)
        .execute(conn.as_mut())
        .await?;
        Ok(())
    }

    async fn flag_session_rotation(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
    ) -> Result<(), SessionError> {
        sqlx::query("UPDATE rustygpt.user_sessions SET requires_rotation = TRUE WHERE id = $1")
            .bind(session_id)
            .execute(conn.as_mut())
            .await?;
        Ok(())
    }

    async fn logout_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), SessionError> {
        sqlx::query("CALL rustygpt.sp_auth_logout($1, $2)")
            .bind(session_id)
            .bind(reason.unwrap_or("logout"))
            .execute(conn.as_mut())
            .await?;
        if let Err(err) = self.refresh_active_session_metrics(conn).await {
            warn!(
                error = %err,
                session_id = %session_id,
                "failed to refresh active session metrics after logout"
            );
        }
        Ok(())
    }

    async fn refresh_active_session_metrics(
        &self,
        conn: &mut PoolConnection<Postgres>,
    ) -> Result<(), SessionError> {
        let rows = sqlx::query(
            r#"
            SELECT role, COUNT(*)::BIGINT AS count
            FROM (
                SELECT COALESCE(role_text, 'member') AS role
                FROM rustygpt.user_sessions s
                CROSS JOIN LATERAL unnest(
                    CASE
                        WHEN array_length(s.roles_snapshot, 1) > 0 THEN s.roles_snapshot
                        ELSE ARRAY['member']
                    END
                ) AS role(role_text)
                WHERE s.revoked_at IS NULL
            ) stats
            GROUP BY role
            "#,
        )
        .fetch_all(conn.as_mut())
        .await
        .map_err(SessionError::Database)?;

        let mut counts: HashMap<String, i64> = rows
            .into_iter()
            .filter_map(|row| {
                let role: Option<String> = row.try_get("role").ok();
                let count: Option<i64> = row.try_get("count").ok();
                role.zip(count)
            })
            .collect();

        for role in ALL_ROLES {
            let value = counts.remove(role.as_str()).unwrap_or(0);
            metrics::gauge!(
                ACTIVE_SESSIONS_METRIC_NAME,
                "role" => role.as_str().to_string()
            )
            .set(value as f64);
        }

        Ok(())
    }

    async fn load_roles(
        &self,
        conn: &mut PoolConnection<Postgres>,
        user_id: Uuid,
    ) -> Result<Vec<UserRole>, SessionError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT role::TEXT FROM rustygpt.user_roles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(conn.as_mut())
        .await?;

        let mut roles = Vec::with_capacity(rows.len().max(1));
        for role in rows {
            match UserRole::from_str(&role) {
                Ok(parsed) => roles.push(parsed),
                Err(_) => warn!(user_id = %user_id, role = %role, "unknown user role in database"),
            }
        }

        if roles.is_empty() {
            roles.push(UserRole::Member);
        }

        Ok(roles)
    }
}

fn roles_snapshot_changed(roles: &[UserRole], snapshot: &Option<Vec<String>>) -> bool {
    snapshot
        .as_ref()
        .map(|stored| {
            let current: Vec<String> = roles.iter().map(|role| role.as_str().to_string()).collect();
            &current != stored
        })
        .unwrap_or(false)
}

fn record_login_metric(result: &'static str) {
    metrics::counter!(
        LOGIN_METRIC_NAME,
        "result" => result.to_string()
    )
    .increment(1);
}

fn record_rotation_metric(cause: RotationCause) {
    metrics::counter!(
        ROTATION_METRIC_NAME,
        "reason" => cause.as_str().to_string()
    )
    .increment(1);
}

#[derive(Debug, Clone, Copy)]
enum RotationCause {
    RequiresRotation,
    RoleChange,
    IdleRefresh,
    ExplicitRefresh,
}

impl RotationCause {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RequiresRotation => "requires_rotation",
            Self::RoleChange => "role_change",
            Self::IdleRefresh => "idle_refresh",
            Self::ExplicitRefresh => "explicit_refresh",
        }
    }
}

impl fmt::Display for RotationCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Compute an Argon2id password hash.
pub fn hash_password(password: &str) -> Result<String, SessionError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| SessionError::PasswordHash(err.to_string()))
}

/// Verify a password against an encoded Argon2id hash.
pub fn verify_password(hash: &str, candidate: &str) -> Result<(), SessionError> {
    let parsed =
        PasswordHash::new(hash).map_err(|err| SessionError::PasswordHash(err.to_string()))?;
    let argon2 = Argon2::default();
    argon2
        .verify_password(candidate.as_bytes(), &parsed)
        .map_err(|_| SessionError::InvalidCredentials)
}

#[derive(sqlx::FromRow)]
struct CredentialRow {
    id: Uuid,
    email: String,
    username: String,
    display_name: Option<String>,
    password_hash: String,
    disabled_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct ActiveSessionRow {
    session_id: Uuid,
    user_id: Uuid,
    issued_at: DateTime<Utc>,
    email: String,
    username: String,
    display_name: Option<String>,
    expires_at: DateTime<Utc>,
    absolute_expires_at: DateTime<Utc>,
    requires_rotation: bool,
    roles_snapshot: Option<Vec<String>>,
    user_agent: Option<String>,
    ip: Option<String>,
    client_meta: Option<Json<JsonValue>>,
    disabled_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct SessionLoginRow {
    session_id: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    absolute_expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SessionRefreshRow {
    next_session_id: Uuid,
    user_id: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    absolute_expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{config::server::Profile, models::UserRole};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    fn service_with_config(config: Config) -> SessionService {
        let pool = PgPoolOptions::new().connect_lazy_with(
            PgConnectOptions::new()
                .host("localhost")
                .username("postgres")
                .password("postgres")
                .database("postgres"),
        );
        SessionService::new(pool, Arc::new(config))
    }

    #[tokio::test]
    async fn build_cookie_honours_security_settings() {
        let mut config = Config::default_for_profile(Profile::Test);
        config.security.cookie.secure = true;
        config.security.cookie.same_site = CookieSameSite::Strict;
        config.security.cookie.domain = Some("example.com".into());

        let service = service_with_config(config.clone());
        let expires_at = Utc::now() + Duration::hours(1);
        let cookie = service
            .build_cookie("token", expires_at)
            .expect("cookie built");

        assert!(cookie.secure().unwrap());
        assert_eq!(cookie.same_site().unwrap(), SameSite::Strict);
        assert_eq!(cookie.domain(), config.security.cookie.domain.as_deref());
    }

    #[tokio::test]
    async fn csrf_cookie_is_strict_and_secure() {
        let mut config = Config::default_for_profile(Profile::Test);
        config.security.cookie.secure = true;
        let service = service_with_config(config);
        let expires_at = Utc::now() + Duration::hours(1);

        let csrf_cookie = service
            .build_csrf_cookie("csrf", expires_at)
            .expect("csrf cookie");

        assert!(csrf_cookie.secure().unwrap());
        assert_eq!(csrf_cookie.same_site().unwrap(), SameSite::Strict);
    }

    #[test]
    fn map_same_site_covers_variants() {
        assert_eq!(
            SessionService::map_same_site(CookieSameSite::Lax),
            SameSite::Lax
        );
        assert_eq!(
            SessionService::map_same_site(CookieSameSite::Strict),
            SameSite::Strict
        );
        assert_eq!(
            SessionService::map_same_site(CookieSameSite::None),
            SameSite::None
        );
    }

    #[test]
    fn privilege_change_forces_rotation_on_next_request() {
        let roles = vec![UserRole::Admin];
        let snapshot = Some(vec!["member".to_string()]);
        assert!(roles_snapshot_changed(&roles, &snapshot));
    }

    #[test]
    fn suspicious_ua_ip_triggers_refresh_if_enabled() {
        let stored =
            SessionMetadata::from_stored(Some("Agent/1.0".into()), Some("10.0.0.1".into()), None);
        let current = SessionMetadata::default()
            .with_user_agent(Some("Agent/2.0".into()))
            .with_ip(Some("10.0.0.2".parse().unwrap()));

        assert!(stored.suspicious_mismatch(&current));
    }
}

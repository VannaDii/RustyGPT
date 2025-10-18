use std::{net::IpAddr, str::FromStr, sync::Arc};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, pool::PoolConnection, types::Json};
use thiserror::Error;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

use shared::{
    config::server::{Config, CookieSameSite},
    models::UserRole,
};

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

    #[allow(dead_code)]
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
}

/// Authenticated user details attached to the request context.
#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SessionBundle {
    pub token: String,
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
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
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
        let same_site = self.map_same_site(self.config.security.cookie.same_site);

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

    fn map_same_site(&self, value: CookieSameSite) -> SameSite {
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
        let mut conn = self.acquire_connection().await?;
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
        .await?;

        let row = match record {
            Some(row) => row,
            None => return Err(SessionError::InvalidCredentials),
        };

        if row.disabled_at.is_some() {
            return Err(SessionError::DisabledUser);
        }

        verify_password(&row.password_hash, password)?;

        let roles = self.load_roles(&mut conn, row.id).await?;
        let bundle = self
            .issue_session(&mut conn, row.id, &roles, metadata)
            .await?;

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

        let roles = self.load_roles(&mut conn, row.user_id).await?;
        let snapshot_mismatch = row
            .roles_snapshot
            .as_ref()
            .map(|snapshot| {
                let current: Vec<String> =
                    roles.iter().map(|role| role.as_str().to_string()).collect();
                &current != snapshot
            })
            .unwrap_or(false);

        let threshold = self.rotation_threshold();
        let needs_rotation =
            row.requires_rotation || snapshot_mismatch || (row.expires_at - now) <= threshold;

        let mut rotated = false;
        let mut bundle = None;

        if needs_rotation {
            let rotated_bundle = self
                .rotate_session(&mut conn, row.session_id, row.user_id, &roles, metadata)
                .await?;

            row.session_id = rotated_bundle.session_id;
            row.issued_at = rotated_bundle.issued_at;
            row.expires_at = rotated_bundle.expires_at;
            row.absolute_expires_at = rotated_bundle.absolute_expires_at;
            rotated = true;
            bundle = Some(rotated_bundle);
        } else {
            self.touch_session(&mut conn, row.session_id, metadata)
                .await?;
        }

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
            .rotate_session(&mut conn, user.session_id, user.id, &user.roles, metadata)
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
            token,
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
            "rotated session"
        );

        let session_cookie = self.build_cookie(&token, record.expires_at)?;
        let csrf_cookie = self.build_csrf_cookie(&csrf_token, record.expires_at)?;

        Ok(SessionBundle {
            token,
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
#[allow(dead_code)]
pub fn verify_password(hash: &str, candidate: &str) -> Result<(), SessionError> {
    let parsed =
        PasswordHash::new(hash).map_err(|err| SessionError::PasswordHash(err.to_string()))?;
    let argon2 = Argon2::default();
    argon2
        .verify_password(candidate.as_bytes(), &parsed)
        .map_err(|_| SessionError::InvalidCredentials)
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
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

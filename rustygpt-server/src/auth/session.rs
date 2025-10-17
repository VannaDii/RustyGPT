use std::{net::IpAddr, str::FromStr, sync::Arc};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::Cookie;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, pool::PoolConnection};
use thiserror::Error;
use time::{Duration as TimeDuration, OffsetDateTime};
use tracing::{instrument, warn};
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
}

/// Authenticated user details attached to the request context.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub roles: Vec<UserRole>,
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Session issuance output containing the raw token and encoded cookie.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SessionCookie {
    pub token: String,
    pub cookie: Cookie<'static>,
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Successful validation result used by the auth middleware.
#[derive(Debug, Clone)]
pub struct SessionValidation {
    pub user: SessionUser,
    pub refresh_cookie: Option<Cookie<'static>>,
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

    fn ttl_duration(&self) -> Duration {
        let ttl = self.config.session.ttl_seconds.max(1);
        Duration::seconds(ttl as i64)
    }

    fn rotation_threshold(&self) -> Duration {
        let ttl = self.config.session.ttl_seconds.max(1);
        let threshold = (ttl / 2).max(1);
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
        let same_site = match self.config.security.cookie.same_site {
            CookieSameSite::Lax => cookie::SameSite::Lax,
            CookieSameSite::Strict => cookie::SameSite::Strict,
            CookieSameSite::None => cookie::SameSite::None,
        };

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

    fn new_token() -> Result<(String, Vec<u8>), SessionError> {
        let mut raw = [0u8; 32];
        OsRng.fill_bytes(&mut raw);
        let token = URL_SAFE_NO_PAD.encode(raw);
        let hash = Sha256::digest(token.as_bytes());
        Ok((token, hash.to_vec()))
    }

    fn hash_for_token(token: &str) -> Vec<u8> {
        Sha256::digest(token.as_bytes()).to_vec()
    }

    async fn acquire_connection(&self) -> Result<PoolConnection<Postgres>, SessionError> {
        self.pool.acquire().await.map_err(SessionError::Database)
    }

    #[instrument(skip(self, password), fields(identifier = %identifier))]
    pub async fn authenticate(
        &self,
        identifier: &str,
        password: &str,
        metadata: &SessionMetadata,
    ) -> Result<(SessionUser, SessionCookie), SessionError> {
        let mut conn = self.acquire_connection().await?;
        let record = sqlx::query_as::<_, CredentialRow>(
            "SELECT id, email::TEXT AS email, username::TEXT AS username, password_hash \
             FROM rustygpt.users \
             WHERE email = $1::citext OR username = $1::citext",
        )
        .bind(identifier)
        .fetch_optional(conn.as_mut())
        .await?;

        let row = match record {
            Some(row) => row,
            None => return Err(SessionError::InvalidCredentials),
        };

        verify_password(&row.password_hash, password)?;

        let roles = self.load_roles(&mut conn, row.id).await?;
        let session_cookie = self.issue_session(&mut conn, row.id, metadata).await?;

        drop(conn);

        let user = SessionUser {
            id: row.id,
            email: row.email,
            username: row.username,
            roles,
            session_id: session_cookie.session_id,
            expires_at: session_cookie.expires_at,
        };

        Ok((user, session_cookie))
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
                    s.expires_at,
                    u.email::TEXT AS email,
                    u.username::TEXT AS username
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

        let now = Utc::now();
        if row.expires_at <= now {
            sqlx::query("DELETE FROM rustygpt.user_sessions WHERE id = $1")
                .bind(row.session_id)
                .execute(conn.as_mut())
                .await?;
            return Ok(None);
        }

        let mut refresh_cookie = None;
        if row.expires_at - now <= self.rotation_threshold() {
            let rotated = self
                .rotate_session(&mut conn, row.session_id, row.user_id, metadata)
                .await?;
            refresh_cookie = Some(rotated.cookie.clone());
            row.expires_at = rotated.expires_at;
        } else {
            self.touch_session(&mut conn, row.session_id, metadata)
                .await?;
        }

        let roles = self.load_roles(&mut conn, row.user_id).await?;
        drop(conn);

        let user = SessionUser {
            id: row.user_id,
            email: row.email,
            username: row.username,
            roles,
            session_id: row.session_id,
            expires_at: row.expires_at,
        };

        Ok(Some(SessionValidation {
            user,
            refresh_cookie,
        }))
    }

    #[instrument(skip(self, metadata))]
    async fn issue_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        user_id: Uuid,
        metadata: &SessionMetadata,
    ) -> Result<SessionCookie, SessionError> {
        let (token, hash) = Self::new_token()?;
        let expires_at = Utc::now() + self.ttl_duration();

        let inserted = sqlx::query_as::<_, SessionInsertRow>(
            "INSERT INTO rustygpt.user_sessions (user_id, token_hash, expires_at, user_agent, ip) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, expires_at",
        )
        .bind(user_id)
        .bind(hash)
        .bind(expires_at)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
        .fetch_one(conn.as_mut())
        .await?;

        let cookie = self.build_cookie(&token, expires_at)?;

        Ok(SessionCookie {
            token,
            cookie,
            session_id: inserted.id,
            expires_at: inserted.expires_at,
        })
    }

    #[instrument(skip(self, metadata))]
    async fn rotate_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
        user_id: Uuid,
        metadata: &SessionMetadata,
    ) -> Result<SessionCookie, SessionError> {
        let (token, hash) = Self::new_token()?;
        let expires_at = Utc::now() + self.ttl_duration();

        let updated = sqlx::query_as::<_, SessionUpdateRow>(
            "UPDATE rustygpt.user_sessions
             SET token_hash = $1,
                 expires_at = $2,
                 last_seen_at = now(),
                 rotated_at = now(),
                 rotated_by = $3,
                 user_agent = COALESCE($4, user_agent),
                 ip = COALESCE($5::inet, ip)
             WHERE id = $6
             RETURNING expires_at",
        )
        .bind(hash)
        .bind(expires_at)
        .bind(user_id)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
        .bind(session_id)
        .fetch_one(conn.as_mut())
        .await?;

        let cookie = self.build_cookie(&token, expires_at)?;

        Ok(SessionCookie {
            token,
            cookie,
            session_id,
            expires_at: updated.expires_at,
        })
    }

    async fn touch_session(
        &self,
        conn: &mut PoolConnection<Postgres>,
        session_id: Uuid,
        metadata: &SessionMetadata,
    ) -> Result<(), SessionError> {
        sqlx::query(
            "UPDATE rustygpt.user_sessions
             SET last_seen_at = now(),
                 user_agent = COALESCE($2, user_agent),
                 ip = COALESCE($3::inet, ip)
             WHERE id = $1",
        )
        .bind(session_id)
        .bind(metadata.user_agent.as_deref())
        .bind(metadata.ip.as_deref())
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
    password_hash: String,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct SessionInsertRow {
    id: Uuid,
    expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SessionUpdateRow {
    expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ActiveSessionRow {
    session_id: Uuid,
    user_id: Uuid,
    email: String,
    username: String,
    expires_at: DateTime<Utc>,
}

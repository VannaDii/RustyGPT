//! Central configuration loader for RustyGPT backends, CLI, and supporting tools.
//!
//! The configuration system follows a layered approach:
//!   1. Built-in defaults per [`Profile`]
//!   2. Optional `config.toml` / `config.yaml` / `config.json`
//!   3. Environment overrides via hierarchical keys (e.g. `RUSTYGPT_SERVER__PORT`)
//!   4. CLI overrides (such as `--port`)
//!
//! Required values are validated eagerly. Optional modules such as Auth V1 or SSE will
//! disable themselves automatically when their prerequisites are missing, emitting a
//! single warning line via `tracing`.

use directories::BaseDirs;
use serde::{Deserialize, Serialize, ser::Serializer};
use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;
use tracing::warn;
use url::Url;

use crate::config::llm::LLMConfiguration;

const CONFIG_SEARCH_LOCATIONS: &[&str] =
    &["config.toml", "config.yaml", "config.yml", "config.json"];

/// Active execution profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Dev,
    Test,
    Prod,
}

impl Profile {
    #[inline]
    pub const fn is_prod(self) -> bool {
        matches!(self, Self::Prod)
    }

    #[inline]
    pub const fn is_dev(self) -> bool {
        matches!(self, Self::Dev)
    }

    fn default_host(self) -> String {
        match self {
            Profile::Dev | Profile::Test => "127.0.0.1".into(),
            Profile::Prod => "0.0.0.0".into(),
        }
    }

    const fn default_port(self) -> u16 {
        match self {
            Profile::Dev => 8080,
            Profile::Test => 18080,
            Profile::Prod => 443,
        }
    }

    fn default_static_dir(self) -> PathBuf {
        match self {
            Profile::Dev => PathBuf::from("../rustygpt-web/dist"),
            Profile::Test => PathBuf::from("../rustygpt-web/dist"),
            Profile::Prod => PathBuf::from("./public"),
        }
    }

    fn default_spa_index(self) -> PathBuf {
        match self {
            Profile::Dev | Profile::Test => PathBuf::from("../rustygpt-web/dist/index.html"),
            Profile::Prod => PathBuf::from("./public/index.html"),
        }
    }

    fn default_base_scheme(self) -> &'static str {
        match self {
            Profile::Prod => "https",
            Profile::Dev | Profile::Test => "http",
        }
    }
}

impl FromStr for Profile {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "dev" | "development" => Ok(Profile::Dev),
            "test" | "testing" => Ok(Profile::Test),
            "prod" | "production" => Ok(Profile::Prod),
            other => Err(ConfigError::InvalidProfile(other.to_string())),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::Dev => f.write_str("dev"),
            Profile::Test => f.write_str("test"),
            Profile::Prod => f.write_str("prod"),
        }
    }
}

/// Logging configuration.
#[derive(Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
}

impl fmt::Debug for LoggingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoggingConfig")
            .field("level", &self.level)
            .field("format", &self.format)
            .finish()
    }
}

/// Output format for logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Text,
    Json,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Text
    }
}

/// Server configuration block.
#[derive(Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub public_base_url: Url,
    pub cors: CorsConfig,
    pub request_id_header: String,
}

impl fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("public_base_url", &self.public_base_url)
            .field("cors", &self.cors)
            .field("request_id_header", &self.request_id_header)
            .finish()
    }
}

/// CORS behaviour.
#[derive(Serialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
    pub max_age_seconds: u64,
}

impl fmt::Debug for CorsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CorsConfig")
            .field("allowed_origins", &self.allowed_origins)
            .field("allow_credentials", &self.allow_credentials)
            .field("max_age_seconds", &self.max_age_seconds)
            .finish()
    }
}

impl CorsConfig {
    fn defaults(profile: Profile) -> Self {
        let allowed_origins = match profile {
            Profile::Dev => vec![
                "http://localhost:3000".into(),
                "http://127.0.0.1:3000".into(),
                "http://localhost:4173".into(),
            ],
            Profile::Test => vec!["http://localhost:4173".into()],
            Profile::Prod => Vec::new(),
        };

        Self {
            allowed_origins,
            allow_credentials: false,
            max_age_seconds: 600,
        }
    }
}

/// Security configuration (headers, cookies, CSRF).
#[derive(Serialize, Clone)]
pub struct SecurityConfig {
    pub hsts: HstsConfig,
    pub cookie: CookieConfig,
    pub csrf: CsrfConfig,
}

impl fmt::Debug for SecurityConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecurityConfig")
            .field("hsts", &self.hsts)
            .field("cookie", &self.cookie)
            .field("csrf", &self.csrf)
            .finish()
    }
}

impl SecurityConfig {
    fn defaults(profile: Profile) -> Self {
        Self {
            hsts: HstsConfig {
                enabled: profile.is_prod(),
                max_age_seconds: 63_072_000,
                include_subdomains: true,
                preload: false,
            },
            cookie: CookieConfig {
                domain: None,
                secure: profile.is_prod(),
                same_site: CookieSameSite::Lax,
            },
            csrf: CsrfConfig::default(),
        }
    }
}

/// HTTP Strict Transport Security.
#[derive(Serialize, Clone)]
pub struct HstsConfig {
    pub enabled: bool,
    pub max_age_seconds: u64,
    pub include_subdomains: bool,
    pub preload: bool,
}

impl fmt::Debug for HstsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HstsConfig")
            .field("enabled", &self.enabled)
            .field("max_age_seconds", &self.max_age_seconds)
            .field("include_subdomains", &self.include_subdomains)
            .field("preload", &self.preload)
            .finish()
    }
}

/// Cookie behaviour shared across session + CSRF.
#[derive(Serialize, Clone)]
pub struct CookieConfig {
    pub domain: Option<String>,
    pub secure: bool,
    pub same_site: CookieSameSite,
}

impl fmt::Debug for CookieConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CookieConfig")
            .field("domain", &self.domain)
            .field("secure", &self.secure)
            .field("same_site", &self.same_site)
            .finish()
    }
}

/// CSRF guard configuration.
#[derive(Serialize, Clone)]
pub struct CsrfConfig {
    pub cookie_name: String,
    pub header_name: String,
    pub enabled: bool,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            cookie_name: "CSRF-TOKEN".into(),
            header_name: "X-CSRF-TOKEN".into(),
            enabled: true,
        }
    }
}

impl fmt::Debug for CsrfConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CsrfConfig")
            .field("cookie_name", &self.cookie_name)
            .field("header_name", &self.header_name)
            .field("enabled", &self.enabled)
            .finish()
    }
}

/// Cookie SameSite policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CookieSameSite {
    Lax,
    Strict,
    None,
}

impl Default for CookieSameSite {
    fn default() -> Self {
        Self::Lax
    }
}

/// Rate limiting configuration.
#[derive(Serialize, Clone)]
pub struct RateLimitConfig {
    pub auth_login_per_ip_per_min: u32,
    pub default_rps: f32,
    pub burst: u32,
}

impl fmt::Debug for RateLimitConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RateLimitConfig")
            .field("auth_login_per_ip_per_min", &self.auth_login_per_ip_per_min)
            .field("default_rps", &self.default_rps)
            .field("burst", &self.burst)
            .finish()
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            auth_login_per_ip_per_min: 10,
            default_rps: 50.0,
            burst: 100,
        }
    }
}

/// Session management configuration.
#[derive(Serialize, Clone)]
pub struct SessionConfig {
    pub ttl_seconds: u64,
    pub session_cookie_name: String,
    pub csrf_cookie_name: String,
}

impl fmt::Debug for SessionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionConfig")
            .field("ttl_seconds", &self.ttl_seconds)
            .field("session_cookie_name", &self.session_cookie_name)
            .field("csrf_cookie_name", &self.csrf_cookie_name)
            .finish()
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 1_209_600, // 14 days
            session_cookie_name: "SESSION_ID".into(),
            csrf_cookie_name: "CSRF-TOKEN".into(),
        }
    }
}

/// OAuth provider configuration.
#[derive(Serialize, Clone)]
pub struct OAuthConfig {
    pub redirect_base: Option<Url>,
    pub github: Option<OAuthProvider>,
}

impl fmt::Debug for OAuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthConfig")
            .field("redirect_base", &self.redirect_base)
            .field(
                "github_configured",
                &self
                    .github
                    .as_ref()
                    .map(|provider| provider.is_configured()),
            )
            .finish()
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            redirect_base: None,
            github: None,
        }
    }
}

/// OAuth client credentials.
#[derive(Serialize, Clone)]
pub struct OAuthProvider {
    pub client_id: SecretString,
    pub client_secret: SecretString,
}

impl OAuthProvider {
    fn empty() -> Self {
        Self {
            client_id: SecretString::new(String::new()),
            client_secret: SecretString::new(String::new()),
        }
    }

    fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }
}

impl fmt::Debug for OAuthProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthProvider")
            .field("client_id_set", &!self.client_id.is_empty())
            .field("client_secret_set", &!self.client_secret.is_empty())
            .finish()
    }
}

/// Database connectivity configuration.
#[derive(Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub statement_timeout_ms: u64,
    pub max_connections: u32,
    pub bootstrap_path: PathBuf,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &redact_url_credentials(&self.url))
            .field("statement_timeout_ms", &self.statement_timeout_ms)
            .field("max_connections", &self.max_connections)
            .field("bootstrap_path", &self.bootstrap_path)
            .finish()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://tinroof:rusty@localhost/rustygpt_dev".into(),
            statement_timeout_ms: 5_000,
            max_connections: 10,
            bootstrap_path: PathBuf::from("../scripts/pg"),
        }
    }
}

/// Server Sent Events configuration.
#[derive(Serialize, Clone)]
pub struct SseConfig {
    pub heartbeat_seconds: u64,
    pub channel_capacity: usize,
    pub id_prefix: String,
}

impl fmt::Debug for SseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SseConfig")
            .field("heartbeat_seconds", &self.heartbeat_seconds)
            .field("channel_capacity", &self.channel_capacity)
            .field("id_prefix", &self.id_prefix)
            .finish()
    }
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            heartbeat_seconds: 20,
            channel_capacity: 128,
            id_prefix: "evt_".into(),
        }
    }
}

/// Generic `.well-known` entry configuration.
#[derive(Serialize, Clone, Default)]
pub struct WellKnownConfig {
    pub entries: Vec<WellKnownEntry>,
}

impl WellKnownConfig {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for WellKnownConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WellKnownConfig")
            .field("entry_count", &self.entries.len())
            .finish()
    }
}

/// A single `.well-known` asset.
#[derive(Serialize, Clone)]
pub struct WellKnownEntry {
    pub path: String,
    pub content_type: String,
    pub body: String,
}

impl fmt::Debug for WellKnownEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WellKnownEntry")
            .field("path", &self.path)
            .field("content_type", &self.content_type)
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// Feature flags toggling optional subsystems.
#[derive(Serialize, Clone, Default)]
pub struct FeatureFlags {
    pub auth_v1: bool,
    pub well_known: bool,
    pub sse_v1: bool,
}

impl fmt::Debug for FeatureFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeatureFlags")
            .field("auth_v1", &self.auth_v1)
            .field("well_known", &self.well_known)
            .field("sse_v1", &self.sse_v1)
            .finish()
    }
}

/// CLI specific configuration.
#[derive(Serialize, Clone)]
pub struct CliConfig {
    pub session_store: PathBuf,
}

impl fmt::Debug for CliConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliConfig")
            .field("session_store", &self.session_store)
            .finish()
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            session_store: default_session_store_path(),
        }
    }
}

/// Web/static asset configuration.
#[derive(Serialize, Clone)]
pub struct WebConfig {
    pub static_dir: PathBuf,
    pub spa_index: PathBuf,
}

impl fmt::Debug for WebConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebConfig")
            .field("static_dir", &self.static_dir)
            .field("spa_index", &self.spa_index)
            .finish()
    }
}

/// Main configuration structure.
#[derive(Serialize, Clone)]
pub struct Config {
    pub profile: Profile,
    pub logging: LoggingConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub rate_limits: RateLimitConfig,
    pub session: SessionConfig,
    pub oauth: OAuthConfig,
    pub db: DatabaseConfig,
    pub sse: SseConfig,
    pub well_known: WellKnownConfig,
    pub features: FeatureFlags,
    pub cli: CliConfig,
    pub web: WebConfig,
    pub llm: LLMConfiguration,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("profile", &self.profile)
            .field("logging", &self.logging)
            .field("server", &self.server)
            .field("security", &self.security)
            .field("rate_limits", &self.rate_limits)
            .field("session", &self.session)
            .field("oauth", &self.oauth)
            .field("db", &self.db)
            .field("sse", &self.sse)
            .field("well_known", &self.well_known)
            .field("features", &self.features)
            .field("cli", &self.cli)
            .field("web", &self.web)
            .finish()
    }
}

impl Config {
    /// Helper retained for backwards compatibility. Equivalent to `default_for_profile(Profile::Dev)`.
    pub fn with_defaults() -> Self {
        Self::defaults(Profile::Dev)
    }

    /// Create a configuration instance populated with defaults for the provided profile.
    pub fn default_for_profile(profile: Profile) -> Self {
        Self::defaults(profile)
    }

    /// Load configuration with precedence: defaults < file < env < CLI overrides.
    pub fn load_config(
        config_path: Option<PathBuf>,
        port_override: Option<u16>,
    ) -> Result<Self, ConfigError> {
        let file_data = FileConfigData::load(config_path)?;

        let env_profile = env_value(&["profile"])
            .map(|value| Profile::from_str(&value))
            .transpose()?;

        let profile = env_profile
            .or(file_data.partial.profile)
            .unwrap_or(Profile::Dev);

        let mut config = Config::defaults(profile);
        let file_flags = config.apply_file_partial(&file_data.partial)?;
        let env_flags = config.apply_env_overrides()?;

        if let Some(port) = port_override {
            config.server.port = port;
        }

        if !(file_flags.public_base_url_set || env_flags.public_base_url_set) {
            config.server.public_base_url = Config::derive_public_base_url(
                config.profile,
                &config.server.host,
                config.server.port,
            )?;
        }

        if config.oauth.redirect_base.is_none()
            && (file_flags.redirect_base_set || env_flags.redirect_base_set)
        {
            // already set by user through file/env
        }

        let warnings = config.validate()?;
        for warning in warnings {
            warn!("{warning}");
        }

        Ok(config)
    }

    fn defaults(profile: Profile) -> Self {
        let host = profile.default_host();
        let port = profile.default_port();
        let public_base_url = Config::derive_public_base_url(profile, &host, port)
            .expect("default URL must be valid");

        Self {
            profile,
            logging: LoggingConfig {
                level: match profile {
                    Profile::Dev => "debug".into(),
                    Profile::Test => "info".into(),
                    Profile::Prod => "info".into(),
                },
                format: LogFormat::default(),
            },
            server: ServerConfig {
                host,
                port,
                public_base_url,
                cors: CorsConfig::defaults(profile),
                request_id_header: "x-request-id".into(),
            },
            security: SecurityConfig::defaults(profile),
            rate_limits: RateLimitConfig::default(),
            session: SessionConfig::default(),
            oauth: OAuthConfig::default(),
            db: DatabaseConfig::default(),
            sse: SseConfig::default(),
            well_known: WellKnownConfig::default(),
            features: FeatureFlags::default(),
            cli: CliConfig::default(),
            web: WebConfig {
                static_dir: profile.default_static_dir(),
                spa_index: profile.default_spa_index(),
            },
            llm: LLMConfiguration::default(),
        }
    }

    fn apply_file_partial(&mut self, partial: &FileConfig) -> Result<ApplyFlags, ConfigError> {
        let mut flags = ApplyFlags::default();

        if let Some(logging) = &partial.logging {
            if let Some(level) = &logging.level {
                self.logging.level = level.clone();
            }
            if let Some(format) = logging.format {
                self.logging.format = format;
            }
        }

        if let Some(server) = &partial.server {
            if let Some(host) = &server.host {
                self.server.host = host.clone();
            }
            if let Some(port) = server.port {
                self.server.port = port;
            }
            if let Some(public_base_url) = &server.public_base_url {
                self.server.public_base_url = parse_url("server.public_base_url", public_base_url)?;
                flags.public_base_url_set = true;
            }
            if let Some(cors) = &server.cors {
                if let Some(origins) = &cors.allowed_origins {
                    self.server.cors.allowed_origins = origins.clone();
                }
                if let Some(allow_credentials) = cors.allow_credentials {
                    self.server.cors.allow_credentials = allow_credentials;
                }
                if let Some(max_age) = cors.max_age_seconds {
                    self.server.cors.max_age_seconds = max_age;
                }
            }
            if let Some(header) = &server.request_id_header {
                self.server.request_id_header = header.clone();
            }
        }

        if let Some(security) = &partial.security {
            if let Some(hsts) = &security.hsts {
                if let Some(enabled) = hsts.enabled {
                    self.security.hsts.enabled = enabled;
                }
                if let Some(max_age) = hsts.max_age_seconds {
                    self.security.hsts.max_age_seconds = max_age;
                }
                if let Some(include) = hsts.include_subdomains {
                    self.security.hsts.include_subdomains = include;
                }
                if let Some(preload) = hsts.preload {
                    self.security.hsts.preload = preload;
                }
            }
            if let Some(cookie) = &security.cookie {
                if let Some(domain) = &cookie.domain {
                    self.security.cookie.domain = Some(domain.clone());
                }
                if let Some(secure) = cookie.secure {
                    self.security.cookie.secure = secure;
                }
                if let Some(same_site) = cookie.same_site {
                    self.security.cookie.same_site = same_site;
                }
            }
            if let Some(csrf) = &security.csrf {
                if let Some(cookie_name) = &csrf.cookie_name {
                    self.security.csrf.cookie_name = cookie_name.clone();
                }
                if let Some(header_name) = &csrf.header_name {
                    self.security.csrf.header_name = header_name.clone();
                }
                if let Some(enabled) = csrf.enabled {
                    self.security.csrf.enabled = enabled;
                }
            }
        }

        if let Some(rate_limits) = &partial.rate_limits {
            if let Some(value) = rate_limits.auth_login_per_ip_per_min {
                self.rate_limits.auth_login_per_ip_per_min = value;
            }
            if let Some(value) = rate_limits.default_rps {
                self.rate_limits.default_rps = value;
            }
            if let Some(value) = rate_limits.burst {
                self.rate_limits.burst = value;
            }
        }

        if let Some(session) = &partial.session {
            if let Some(ttl) = session.ttl_seconds {
                self.session.ttl_seconds = ttl;
            }
            if let Some(cookie_name) = &session.session_cookie_name {
                self.session.session_cookie_name = cookie_name.clone();
            }
            if let Some(csrf_cookie_name) = &session.csrf_cookie_name {
                self.session.csrf_cookie_name = csrf_cookie_name.clone();
            }
        }

        if let Some(oauth) = &partial.oauth {
            if let Some(redirect_base) = &oauth.redirect_base {
                self.oauth.redirect_base = Some(parse_url("oauth.redirect_base", redirect_base)?);
                flags.redirect_base_set = true;
            }
            if let Some(github) = &oauth.github {
                let mut provider = self
                    .oauth
                    .github
                    .clone()
                    .unwrap_or_else(OAuthProvider::empty);
                if let Some(client_id) = &github.client_id {
                    provider.client_id = SecretString::new(client_id.clone());
                }
                if let Some(client_secret) = &github.client_secret {
                    provider.client_secret = SecretString::new(client_secret.clone());
                }
                self.oauth.github = Some(provider);
            }
        }

        if let Some(database) = &partial.db {
            if let Some(url) = &database.url {
                self.db.url = url.clone();
            }
            if let Some(timeout) = database.statement_timeout_ms {
                self.db.statement_timeout_ms = timeout;
            }
            if let Some(max_connections) = database.max_connections {
                self.db.max_connections = max_connections;
            }
            if let Some(path) = &database.bootstrap_path {
                self.db.bootstrap_path = path.clone();
            }
        }

        if let Some(sse) = &partial.sse {
            if let Some(heartbeat) = sse.heartbeat_seconds {
                self.sse.heartbeat_seconds = heartbeat;
            }
            if let Some(capacity) = sse.channel_capacity {
                self.sse.channel_capacity = capacity as usize;
            }
            if let Some(prefix) = &sse.id_prefix {
                self.sse.id_prefix = prefix.clone();
            }
        }

        if let Some(well_known) = &partial.well_known {
            if let Some(entries) = &well_known.entries {
                self.well_known.entries = entries
                    .iter()
                    .map(|entry| WellKnownEntry {
                        path: entry.path.clone(),
                        content_type: entry.content_type.clone(),
                        body: entry.body.clone(),
                    })
                    .collect();
            }
        }

        if let Some(features) = &partial.features {
            if let Some(value) = features.auth_v1 {
                self.features.auth_v1 = value;
            }
            if let Some(value) = features.well_known {
                self.features.well_known = value;
            }
            if let Some(value) = features.sse_v1 {
                self.features.sse_v1 = value;
            }
        }

        if let Some(cli) = &partial.cli {
            if let Some(path) = &cli.session_store {
                self.cli.session_store = path.clone();
            }
        }

        if let Some(web) = &partial.web {
            if let Some(static_dir) = &web.static_dir {
                self.web.static_dir = static_dir.clone();
            }
            if let Some(spa_index) = &web.spa_index {
                self.web.spa_index = spa_index.clone();
            }
        }

        if let Some(llm) = &partial.llm {
            self.llm = llm.clone();
        }

        Ok(flags)
    }

    fn apply_env_overrides(&mut self) -> Result<EnvOverrideFlags, ConfigError> {
        let mut flags = EnvOverrideFlags::default();

        if let Some(level) = env_value(&["logging", "level"]) {
            self.logging.level = level;
        }
        if let Some(format) = env_value(&["logging", "format"]) {
            self.logging.format = LogFormat::from_str(&format)?;
        }

        if let Some(host) = env_value(&["server", "host"]) {
            self.server.host = host;
        }
        if let Some(port) = env_value_u16(&["server", "port"])? {
            self.server.port = port;
        }
        if let Some(base_url) = env_value(&["server", "public_base_url"]) {
            self.server.public_base_url = parse_url("server.public_base_url", &base_url)?;
            flags.public_base_url_set = true;
        }
        if let Some(origins) = env_value(&["server", "cors", "allowed_origins"]) {
            let parsed = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            self.server.cors.allowed_origins = parsed;
        }
        if let Some(allow_credentials) = env_value_bool(&["server", "cors", "allow_credentials"])? {
            self.server.cors.allow_credentials = allow_credentials;
        }
        if let Some(max_age) = env_value_u64(&["server", "cors", "max_age_seconds"])? {
            self.server.cors.max_age_seconds = max_age;
        }
        if let Some(header) = env_value(&["server", "request_id_header"]) {
            self.server.request_id_header = header;
        }

        if let Some(enabled) = env_value_bool(&["security", "hsts", "enabled"])? {
            self.security.hsts.enabled = enabled;
        }
        if let Some(max_age) = env_value_u64(&["security", "hsts", "max_age_seconds"])? {
            self.security.hsts.max_age_seconds = max_age;
        }
        if let Some(include_subdomains) =
            env_value_bool(&["security", "hsts", "include_subdomains"])?
        {
            self.security.hsts.include_subdomains = include_subdomains;
        }
        if let Some(preload) = env_value_bool(&["security", "hsts", "preload"])? {
            self.security.hsts.preload = preload;
        }
        if let Some(domain) = env_value(&["security", "cookie", "domain"]) {
            self.security.cookie.domain = if domain.is_empty() {
                None
            } else {
                Some(domain)
            };
        }
        if let Some(secure) = env_value_bool(&["security", "cookie", "secure"])? {
            self.security.cookie.secure = secure;
        }
        if let Some(same_site) = env_value(&["security", "cookie", "same_site"]) {
            self.security.cookie.same_site = CookieSameSite::from_str(&same_site)?;
        }
        if let Some(cookie_name) = env_value(&["security", "csrf", "cookie_name"]) {
            self.security.csrf.cookie_name = cookie_name;
        }
        if let Some(header_name) = env_value(&["security", "csrf", "header_name"]) {
            self.security.csrf.header_name = header_name;
        }
        if let Some(enabled) = env_value_bool(&["security", "csrf", "enabled"])? {
            self.security.csrf.enabled = enabled;
        }

        if let Some(auth_limit) = env_value_u32(&["rate_limits", "auth_login_per_ip_per_min"])? {
            self.rate_limits.auth_login_per_ip_per_min = auth_limit;
        }
        if let Some(default_rps) = env_value_f32(&["rate_limits", "default_rps"])? {
            self.rate_limits.default_rps = default_rps;
        }
        if let Some(burst) = env_value_u32(&["rate_limits", "burst"])? {
            self.rate_limits.burst = burst;
        }

        if let Some(ttl) = env_value_u64(&["session", "ttl_seconds"])? {
            self.session.ttl_seconds = ttl;
        }
        if let Some(cookie_name) = env_value(&["session", "session_cookie_name"]) {
            self.session.session_cookie_name = cookie_name;
        }
        if let Some(csrf_cookie_name) = env_value(&["session", "csrf_cookie_name"]) {
            self.session.csrf_cookie_name = csrf_cookie_name;
        }

        if let Some(redirect_base) = env_value(&["oauth", "redirect_base"]) {
            self.oauth.redirect_base = Some(parse_url("oauth.redirect_base", &redirect_base)?);
            flags.redirect_base_set = true;
        }
        if let Some(client_id) = env_value(&["oauth", "github", "client_id"]) {
            let mut provider = self
                .oauth
                .github
                .clone()
                .unwrap_or_else(OAuthProvider::empty);
            provider.client_id = SecretString::new(client_id);
            self.oauth.github = Some(provider);
        }
        if let Some(client_secret) = env_value(&["oauth", "github", "client_secret"]) {
            let mut provider = self
                .oauth
                .github
                .clone()
                .unwrap_or_else(OAuthProvider::empty);
            provider.client_secret = SecretString::new(client_secret);
            self.oauth.github = Some(provider);
        }

        if let Some(url) = env_value(&["db", "url"]) {
            self.db.url = url;
        }
        if let Some(timeout) = env_value_u64(&["db", "statement_timeout_ms"])? {
            self.db.statement_timeout_ms = timeout;
        }
        if let Some(max_connections) = env_value_u32(&["db", "max_connections"])? {
            self.db.max_connections = max_connections;
        }
        if let Some(path) = env_value(&["db", "bootstrap_path"]) {
            self.db.bootstrap_path = PathBuf::from(path);
        }

        if let Some(heartbeat) = env_value_u64(&["sse", "heartbeat_seconds"])? {
            self.sse.heartbeat_seconds = heartbeat;
        }
        if let Some(channel_capacity) = env_value_usize(&["sse", "channel_capacity"])? {
            self.sse.channel_capacity = channel_capacity;
        }
        if let Some(id_prefix) = env_value(&["sse", "id_prefix"]) {
            self.sse.id_prefix = id_prefix;
        }

        if let Some(enabled) = env_value_bool(&["features", "auth_v1"])? {
            self.features.auth_v1 = enabled;
        }
        if let Some(enabled) = env_value_bool(&["features", "well_known"])? {
            self.features.well_known = enabled;
        }
        if let Some(enabled) = env_value_bool(&["features", "sse_v1"])? {
            self.features.sse_v1 = enabled;
        }

        if let Some(session_store) = env_value(&["cli", "session_store"]) {
            self.cli.session_store = PathBuf::from(session_store);
        }

        if let Some(static_dir) = env_value(&["web", "static_dir"]) {
            self.web.static_dir = PathBuf::from(static_dir);
        }
        if let Some(spa_index) = env_value(&["web", "spa_index"]) {
            self.web.spa_index = PathBuf::from(spa_index);
        }

        Ok(flags)
    }

    fn validate(&mut self) -> Result<Vec<String>, ConfigError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.server.host.trim().is_empty() {
            errors.push("server.host must not be empty".into());
        }
        if self.server.port == 0 {
            errors.push("server.port must be greater than zero".into());
        }

        match self.server.public_base_url.scheme() {
            "http" | "https" => {
                if self.profile.is_prod() && self.server.public_base_url.scheme() != "https" {
                    errors.push("server.public_base_url must use https in production".into());
                }
            }
            other => errors.push(format!(
                "server.public_base_url scheme '{other}' is not supported (only http/https)"
            )),
        }

        if self.security.cookie.secure == false && self.profile.is_prod() {
            errors.push("security.cookie.secure must be true in production".into());
        }
        if self.security.cookie.secure == false && !self.profile.is_prod() {
            warnings.push(
                "security.cookie.secure is disabled; cookies may be exposed over HTTP".into(),
            );
        }

        if self.security.csrf.enabled && !self.features.auth_v1 {
            warnings.push("CSRF protection is enabled while auth feature is disabled; ensure configuration matches expectations".into());
        }

        if self.server.cors.allow_credentials
            && self
                .server
                .cors
                .allowed_origins
                .iter()
                .any(|item| item == "*")
        {
            errors.push(
                "server.cors.allowed_origins cannot contain '*' when allow_credentials is true"
                    .into(),
            );
        }

        if self.rate_limits.auth_login_per_ip_per_min == 0 {
            errors.push("rate_limits.auth_login_per_ip_per_min must be greater than zero".into());
        }
        if self.rate_limits.default_rps <= 0.0 {
            errors.push("rate_limits.default_rps must be positive".into());
        }
        if self.rate_limits.burst == 0 {
            errors.push("rate_limits.burst must be greater than zero".into());
        }

        if self.session.ttl_seconds == 0 {
            errors.push("session.ttl_seconds must be greater than zero".into());
        }

        if self.db.url.trim().is_empty() {
            errors.push("db.url must not be empty".into());
        } else if !self.db.url.starts_with("postgres://")
            && !self.db.url.starts_with("postgresql://")
        {
            errors.push("db.url must be a Postgres connection string (postgres://)".into());
        }

        if self.db.statement_timeout_ms == 0 {
            errors.push("db.statement_timeout_ms must be greater than zero".into());
        }

        if self.logging.level.trim().is_empty() {
            errors.push("logging.level must not be empty".into());
        } else if !is_valid_level(&self.logging.level) {
            errors.push(format!(
                "logging.level '{}' is not a known level (trace|debug|info|warn|error)",
                self.logging.level
            ));
        }

        if self.sse.channel_capacity == 0 {
            warnings.push(
                "SSE channel capacity is zero; increasing to at least 1 to avoid drop-all behaviour"
                    .into(),
            );
            self.sse.channel_capacity = 1;
        }

        if self.web.static_dir.to_string_lossy().is_empty() {
            errors.push("web.static_dir must not be empty".into());
        } else if !self.web.static_dir.exists() {
            warnings.push(format!(
                "web.static_dir '{}' does not exist; static assets may be unavailable",
                self.web.static_dir.display()
            ));
        }

        if self.web.spa_index.to_string_lossy().is_empty() {
            warnings.push("web.spa_index is empty; SPA fallback disabled".into());
        }

        if self.cli.session_store.to_string_lossy().is_empty() {
            warnings.push(
                "cli.session_store path is empty; CLI login cookie jar cannot be persisted".into(),
            );
        }

        if !self.db.bootstrap_path.exists() {
            warnings.push(format!(
                "db.bootstrap_path '{}' does not exist; database bootstrap may fail",
                self.db.bootstrap_path.display()
            ));
        }

        if self.features.auth_v1 {
            let mut missing = Vec::new();
            match &self.oauth.github {
                Some(provider) if provider.is_configured() => {}
                _ => missing.push("oauth.github"),
            }
            if self.oauth.redirect_base.is_none() {
                missing.push("oauth.redirect_base");
            }
            if !missing.is_empty() {
                warnings.push(format!(
                    "features.auth_v1 disabled: missing {}",
                    missing.join(", ")
                ));
                self.features.auth_v1 = false;
            }
        }

        if self.features.sse_v1 && self.sse.channel_capacity == 0 {
            warnings.push("features.sse_v1 disabled because sse.channel_capacity == 0".into());
            self.features.sse_v1 = false;
        }

        if self.features.sse_v1 && !self.features.auth_v1 {
            warnings.push("features.sse_v1 disabled: requires features.auth_v1".into());
            self.features.sse_v1 = false;
        }

        if self.features.well_known && self.well_known.is_empty() {
            warnings.push("features.well_known disabled: no well-known entries configured".into());
            self.features.well_known = false;
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            let summary = errors
                .iter()
                .map(|msg| format!("- {msg}"))
                .collect::<Vec<_>>()
                .join("\n");
            Err(ConfigError::Validation { errors, summary })
        }
    }

    fn derive_public_base_url(profile: Profile, host: &str, port: u16) -> Result<Url, ConfigError> {
        if host.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "server.host".into(),
                message: "host cannot be empty when constructing default base URL".into(),
            });
        }

        let scheme = profile.default_base_scheme();
        let omit_port =
            (scheme == "http" && port == 80) || (scheme == "https" && (port == 443 || port == 0));

        let candidate = if omit_port {
            format!("{scheme}://{host}")
        } else {
            format!("{scheme}://{host}:{port}")
        };

        Url::parse(&candidate).map_err(|err| ConfigError::InvalidValue {
            field: "server.public_base_url".into(),
            message: err.to_string(),
        })
    }
}

/// File configuration with optional fields.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    profile: Option<Profile>,
    #[serde(default)]
    logging: Option<LoggingPartial>,
    #[serde(default)]
    server: Option<ServerPartial>,
    #[serde(default)]
    security: Option<SecurityPartial>,
    #[serde(default)]
    rate_limits: Option<RateLimitPartial>,
    #[serde(default)]
    session: Option<SessionPartial>,
    #[serde(default)]
    oauth: Option<OAuthPartial>,
    #[serde(default)]
    db: Option<DatabasePartial>,
    #[serde(default)]
    sse: Option<SsePartial>,
    #[serde(default)]
    well_known: Option<WellKnownPartial>,
    #[serde(default)]
    features: Option<FeatureFlagsPartial>,
    #[serde(default)]
    cli: Option<CliPartial>,
    #[serde(default)]
    web: Option<WebPartial>,
    #[serde(default)]
    llm: Option<LLMConfiguration>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoggingPartial {
    pub level: Option<String>,
    pub format: Option<LogFormat>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerPartial {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub public_base_url: Option<String>,
    #[serde(default)]
    pub cors: Option<CorsPartial>,
    pub request_id_header: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorsPartial {
    pub allowed_origins: Option<Vec<String>>,
    pub allow_credentials: Option<bool>,
    pub max_age_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SecurityPartial {
    pub hsts: Option<HstsPartial>,
    pub cookie: Option<CookiePartial>,
    pub csrf: Option<CsrfPartial>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HstsPartial {
    pub enabled: Option<bool>,
    pub max_age_seconds: Option<u64>,
    pub include_subdomains: Option<bool>,
    pub preload: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CookiePartial {
    pub domain: Option<String>,
    pub secure: Option<bool>,
    pub same_site: Option<CookieSameSite>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CsrfPartial {
    pub cookie_name: Option<String>,
    pub header_name: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RateLimitPartial {
    pub auth_login_per_ip_per_min: Option<u32>,
    pub default_rps: Option<f32>,
    pub burst: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionPartial {
    pub ttl_seconds: Option<u64>,
    pub session_cookie_name: Option<String>,
    pub csrf_cookie_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct OAuthPartial {
    pub redirect_base: Option<String>,
    #[serde(default)]
    pub github: Option<OAuthProviderPartial>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct OAuthProviderPartial {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatabasePartial {
    pub url: Option<String>,
    pub statement_timeout_ms: Option<u64>,
    pub max_connections: Option<u32>,
    pub bootstrap_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SsePartial {
    pub heartbeat_seconds: Option<u64>,
    pub channel_capacity: Option<u64>,
    pub id_prefix: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WellKnownPartial {
    #[serde(default)]
    pub entries: Option<Vec<WellKnownEntryPartial>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WellKnownEntryPartial {
    pub path: String,
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureFlagsPartial {
    pub auth_v1: Option<bool>,
    pub well_known: Option<bool>,
    pub sse_v1: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CliPartial {
    pub session_store: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WebPartial {
    pub static_dir: Option<PathBuf>,
    pub spa_index: Option<PathBuf>,
}

#[derive(Default)]
struct ApplyFlags {
    public_base_url_set: bool,
    redirect_base_set: bool,
}

#[derive(Default)]
struct EnvOverrideFlags {
    public_base_url_set: bool,
    redirect_base_set: bool,
}

/// Configuration loading errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration file {path} could not be read: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported configuration format for {path}")]
    UnsupportedFormat { path: PathBuf },
    #[error("failed to parse configuration file {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("environment variable {key} is invalid: {message}")]
    InvalidEnv { key: String, message: String },
    #[error("invalid profile '{0}'")]
    InvalidProfile(String),
    #[error("invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
    #[error("configuration validation failed:\n{summary}")]
    Validation {
        errors: Vec<String>,
        summary: String,
    },
}

impl ConfigError {
    fn from_env_parse<T: fmt::Display>(key: &str, source: T) -> Self {
        ConfigError::InvalidEnv {
            key: key.to_string(),
            message: source.to_string(),
        }
    }
}

struct FileConfigData {
    _path: Option<PathBuf>,
    partial: FileConfig,
}

impl FileConfigData {
    fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        if let Some(path) = config_path {
            return Self::from_path(path);
        }

        for candidate in CONFIG_SEARCH_LOCATIONS {
            let path = Path::new(candidate);
            if path.exists() {
                return Self::from_path(path.to_path_buf());
            }
        }

        Ok(Self {
            _path: None,
            partial: FileConfig::default(),
        })
    }

    fn from_path(path: PathBuf) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(&path).map_err(|source| ConfigError::FileRead {
            path: path.clone(),
            source,
        })?;
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let partial = match ext.as_str() {
            "toml" => toml::from_str(&contents).map_err(|err| ConfigError::Parse {
                path: path.clone(),
                message: err.to_string(),
            })?,
            "yaml" | "yml" => serde_yml::from_str(&contents).map_err(|err| ConfigError::Parse {
                path: path.clone(),
                message: err.to_string(),
            })?,
            "json" => serde_json::from_str(&contents).map_err(|err| ConfigError::Parse {
                path: path.clone(),
                message: err.to_string(),
            })?,
            "" => {
                return Err(ConfigError::UnsupportedFormat { path });
            }
            _ => {
                return Err(ConfigError::UnsupportedFormat { path });
            }
        };

        Ok(Self {
            _path: Some(path),
            partial,
        })
    }
}

/// Secret string wrapper that redacts debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("***")
    }
}

impl FromStr for LogFormat {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "text" | "pretty" | "plain" => Ok(LogFormat::Text),
            "json" => Ok(LogFormat::Json),
            other => Err(ConfigError::InvalidValue {
                field: "logging.format".into(),
                message: format!("unknown log format '{other}'"),
            }),
        }
    }
}

impl FromStr for CookieSameSite {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "lax" => Ok(CookieSameSite::Lax),
            "strict" => Ok(CookieSameSite::Strict),
            "none" => Ok(CookieSameSite::None),
            other => Err(ConfigError::InvalidValue {
                field: "security.cookie.same_site".into(),
                message: format!("unknown SameSite value '{other}'"),
            }),
        }
    }
}

fn parse_url(field: &str, value: &str) -> Result<Url, ConfigError> {
    Url::parse(value).map_err(|err| ConfigError::InvalidValue {
        field: field.into(),
        message: err.to_string(),
    })
}

fn env_value(path: &[&str]) -> Option<String> {
    let upper_segments: Vec<String> = path
        .iter()
        .map(|segment| segment.replace('-', "_").to_ascii_uppercase())
        .collect();
    let underscore_key = format!("RUSTYGPT_{}", upper_segments.join("_"));
    let double_key = format!("RUSTYGPT__{}", upper_segments.join("__"));
    let dot_key = format!("RUSTYGPT.{}", path.join("."));

    for key in [underscore_key, double_key, dot_key] {
        if let Ok(value) = env::var(&key) {
            return Some(value);
        }
    }

    None
}

fn env_value_bool(path: &[&str]) -> Result<Option<bool>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(Some(true)),
            "false" | "0" | "no" | "off" => Ok(Some(false)),
            _ => Err(ConfigError::from_env_parse(&key, "expected boolean")),
        }
    } else {
        Ok(None)
    }
}

fn env_value_u16(path: &[&str]) -> Result<Option<u16>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        value
            .parse::<u16>()
            .map(Some)
            .map_err(|err| ConfigError::from_env_parse(&key, err))
    } else {
        Ok(None)
    }
}

fn env_value_u32(path: &[&str]) -> Result<Option<u32>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        value
            .parse::<u32>()
            .map(Some)
            .map_err(|err| ConfigError::from_env_parse(&key, err))
    } else {
        Ok(None)
    }
}

fn env_value_u64(path: &[&str]) -> Result<Option<u64>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        value
            .parse::<u64>()
            .map(Some)
            .map_err(|err| ConfigError::from_env_parse(&key, err))
    } else {
        Ok(None)
    }
}

fn env_value_usize(path: &[&str]) -> Result<Option<usize>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        value
            .parse::<usize>()
            .map(Some)
            .map_err(|err| ConfigError::from_env_parse(&key, err))
    } else {
        Ok(None)
    }
}

fn env_value_f32(path: &[&str]) -> Result<Option<f32>, ConfigError> {
    if let Some(value) = env_value(path) {
        let key = format!("RUSTYGPT_{}", path.join("_").to_ascii_uppercase());
        value
            .parse::<f32>()
            .map(Some)
            .map_err(|err| ConfigError::from_env_parse(&key, err))
    } else {
        Ok(None)
    }
}

fn is_valid_level(level: &str) -> bool {
    matches!(
        level.to_ascii_lowercase().as_str(),
        "trace" | "debug" | "info" | "warn" | "error"
    )
}

fn default_session_store_path() -> PathBuf {
    if let Some(base_dirs) = BaseDirs::new() {
        let mut path = base_dirs.config_dir().to_path_buf();
        path.push("rustygpt");
        path.push("session.json");
        path
    } else {
        PathBuf::from("./session.json")
    }
}

fn redact_url_credentials(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        if parsed.password().is_some() || parsed.username() != "" {
            let mut redacted = parsed.clone();
            redacted.set_username("****").ok();
            redacted.set_password(Some("****")).ok();
            return redacted.to_string();
        }
    }
    url.to_string()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn set_env_var(key: &str, value: &str) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env_var(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }

    #[test]
    fn load_defaults_dev_profile() {
        let config = Config::load_config(None, None).expect("defaults load");
        assert_eq!(config.profile, Profile::Dev);
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.features.auth_v1, false);
        assert!(!config.server.host.is_empty());
    }

    #[test]
    fn load_from_toml_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
profile = "test"

[server]
host = "0.0.0.0"
port = 9090
public_base_url = "http://localhost:9090"

[rate_limits]
auth_login_per_ip_per_min = 5
default_rps = 25.0
burst = 50

[features]
auth_v1 = false
well_known = false
sse_v1 = true
"#
        )
        .unwrap();

        let config =
            Config::load_config(Some(file.path().to_path_buf()), None).expect("load config");
        assert_eq!(config.profile, Profile::Test);
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.rate_limits.auth_login_per_ip_per_min, 5);
        assert!(config.features.sse_v1);
    }

    #[test]
    #[serial]
    fn env_overrides_take_precedence() {
        set_env_var("RUSTYGPT_SERVER_PORT", "5555");
        set_env_var("RUSTYGPT_SERVER__PUBLIC_BASE_URL", "http://localhost:5555");
        set_env_var("RUSTYGPT_FEATURES__AUTH_V1", "true");
        set_env_var("RUSTYGPT_OAUTH__GITHUB__CLIENT_ID", "abc");
        set_env_var("RUSTYGPT_OAUTH__GITHUB__CLIENT_SECRET", "def");
        set_env_var(
            "RUSTYGPT_OAUTH__REDIRECT_BASE",
            "http://localhost:5555/auth/callback",
        );

        let config = Config::load_config(None, None).expect("load config");
        assert_eq!(config.server.port, 5555);
        assert_eq!(
            config.server.public_base_url.as_str(),
            "http://localhost:5555/"
        );
        assert!(
            config.features.auth_v1,
            "auth feature should remain enabled with env overrides"
        );

        remove_env_var("RUSTYGPT_SERVER_PORT");
        remove_env_var("RUSTYGPT_SERVER__PUBLIC_BASE_URL");
        remove_env_var("RUSTYGPT_FEATURES__AUTH_V1");
        remove_env_var("RUSTYGPT_OAUTH__GITHUB__CLIENT_ID");
        remove_env_var("RUSTYGPT_OAUTH__GITHUB__CLIENT_SECRET");
        remove_env_var("RUSTYGPT_OAUTH__REDIRECT_BASE");
    }

    #[test]
    fn invalid_db_url_fails_validation() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[db]
url = "sqlite://memory"
"#
        )
        .unwrap();

        let err = Config::load_config(Some(file.path().to_path_buf()), None)
            .expect_err("validation should fail");

        match err {
            ConfigError::Validation { errors, .. } => {
                assert!(errors.iter().any(|msg| msg.contains("db.url")));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

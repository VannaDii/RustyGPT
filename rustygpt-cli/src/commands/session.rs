use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use clap::Args;
use directories::BaseDirs;
use reqwest::{
    Client, StatusCode,
    cookie::{CookieStore, Jar},
};
use rpassword::prompt_password;
use shared::{
    config::server::Config,
    models::{AuthenticatedUser, LoginRequest, LoginResponse, MeResponse, SessionSummary},
};
use url::Url;

#[derive(Args, Debug)]
pub struct LoginArgs {
    #[arg(long, short)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct MeArgs {
    #[arg(long, short)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct LogoutArgs {
    #[arg(long, short)]
    pub config: Option<PathBuf>,
}

pub async fn login(args: LoginArgs) -> Result<()> {
    let config = Config::load_config(args.config.clone(), None)?;
    let origin = config.server.public_base_url.clone();
    let jar_path = session_path();
    ensure_parent(&jar_path)?;

    let email = prompt("Email: ")?;
    let password = prompt_password("Password: ")?;
    if password.trim().is_empty() {
        bail!("password must not be empty");
    }

    let jar = Arc::new(Jar::default());
    let client = build_client(jar.clone())?;

    let login_url = origin
        .join("api/auth/login")
        .context("invalid login endpoint")?;
    let response = client
        .post(login_url)
        .json(&LoginRequest { email, password })
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("login failed with {}: {}", status, body);
    }

    let login: LoginResponse = response.json().await?;
    persist_cookie_jar(&jar, &origin, &jar_path)?;
    print_session_summary(&login.user, &login.session, &jar_path);
    Ok(())
}

pub async fn me(args: MeArgs) -> Result<()> {
    let config = Config::load_config(args.config.clone(), None)?;
    let origin = config.server.public_base_url.clone();
    let jar_path = session_path();

    let jar = load_cookie_jar(&origin, &jar_path)
        .with_context(|| "no active session found; run `rustygpt session login` first")?;
    let client = build_client(jar.clone())?;

    let me_url = origin
        .join("api/auth/me")
        .context("invalid profile endpoint")?;
    let response = client.get(me_url).send().await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        bail!("session expired. run `rustygpt session login` to sign in again");
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("refresh failed with {}: {}", status, body);
    }

    let profile: MeResponse = response.json().await?;
    persist_cookie_jar(&jar, &origin, &jar_path)?;
    print_session_summary(&profile.user, &profile.session, &jar_path);
    Ok(())
}

pub async fn logout(args: LogoutArgs) -> Result<()> {
    let config = Config::load_config(args.config.clone(), None)?;
    let origin = config.server.public_base_url.clone();
    let jar_path = session_path();

    match load_cookie_jar(&origin, &jar_path) {
        Ok(jar) => {
            let client = build_client(jar.clone())?;
            let logout_url = origin
                .join("api/auth/logout")
                .context("invalid logout endpoint")?;
            let response = client.post(logout_url).send().await?;
            if response.status() != StatusCode::UNAUTHORIZED && !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!("warning: logout request failed with {}: {}", status, body);
            }
        }
        Err(err) => {
            eprintln!("warning: {}", err);
        }
    }

    if jar_path.exists() {
        fs::remove_file(&jar_path)
            .with_context(|| format!("failed to remove session jar {}", jar_path.display()))?;
        println!("Removed session cookies at {}", jar_path.display());
    } else {
        println!("No session cookies found at {}", jar_path.display());
    }

    Ok(())
}

pub fn session_path() -> PathBuf {
    BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("rustygpt").join("session.cookies"))
        .unwrap_or_else(|| PathBuf::from("./session.cookies"))
}

fn prompt(message: &str) -> Result<String> {
    print!("{}", message);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        bail!("input must not be empty");
    }
    Ok(trimmed)
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create session directory {}", parent.display()))?;
    }
    Ok(())
}

pub fn build_client(jar: Arc<Jar>) -> Result<Client> {
    Client::builder()
        .cookie_provider(jar)
        .user_agent("rustygpt-cli")
        .build()
        .context("failed to build HTTP client")
}

pub fn load_cookie_jar(origin: &Url, path: &Path) -> Result<Arc<Jar>> {
    if !path.exists() {
        bail!("session cookie jar not found at {}", path.display());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read session jar {}", path.display()))?;
    let jar = Arc::new(Jar::default());
    for entry in contents.split(';') {
        let cookie = entry.trim();
        if !cookie.is_empty() {
            jar.add_cookie_str(cookie, origin);
        }
    }
    Ok(jar)
}

pub fn persist_cookie_jar(jar: &Arc<Jar>, origin: &Url, path: &Path) -> Result<()> {
    if let Some(header) = jar.cookies(origin) {
        fs::write(path, header.to_str()?.as_bytes())
            .with_context(|| format!("failed to write session jar at {}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .context("failed to set session jar permissions")?;
        }
        println!("Session cookies saved to {}", path.display());
    } else if path.exists() {
        fs::remove_file(path).ok();
    }
    Ok(())
}

pub fn csrf_token_from_jar(jar: &Arc<Jar>, origin: &Url) -> Option<String> {
    jar.cookies(origin)
        .and_then(|value| value.to_str().ok().map(|s| s.to_string()))
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|entry| {
                let trimmed = entry.trim();
                trimmed
                    .strip_prefix("CSRF-TOKEN=")
                    .map(|token| token.to_string())
            })
        })
}

fn print_session_summary(user: &AuthenticatedUser, session: &SessionSummary, jar_path: &Path) {
    println!("Logged in as {}", user.email);
    if let Some(display) = &user.display_name {
        println!("display name: {}", display);
    }
    println!(
        "roles: {}",
        user.roles
            .iter()
            .map(|role| role.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "session expires at: {} (absolute: {})",
        session.expires_at.0, session.absolute_expires_at.0
    );
    println!("cookies stored at {}", jar_path.display());
}

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use directories::BaseDirs;
use reqwest::{
    Client,
    cookie::{CookieStore, Jar},
};
use serde::Deserialize;
use std::sync::Arc;

use shared::config::server::Config;

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

#[derive(Deserialize)]
struct MeResponse {
    pub id: String,
    pub email: Option<String>,
    pub username: Option<String>,
}

pub async fn login(args: LoginArgs) -> Result<()> {
    let config = Config::load_config(args.config.clone(), None)?;
    let jar_path = session_path();
    if let Some(parent) = jar_path.parent() {
        std::fs::create_dir_all(parent).context("failed to create session directory")?;
    }

    let cookie_url = config.server.public_base_url.clone();
    let jar = Arc::new(Jar::default());
    let client = Client::builder()
        .cookie_provider(jar.clone())
        .build()
        .context("failed to build HTTP client")?;

    let init_url = format!("{}/api/auth/github/device", config.server.public_base_url);
    let response = client.post(init_url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("failed to initiate device login: {}", response.status());
    }

    println!("{}", response.text().await?);

    if let Some(cookies) = jar.cookies(&cookie_url) {
        std::fs::write(&jar_path, cookies.to_str()?.as_bytes())
            .with_context(|| format!("failed to write session jar at {}", jar_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&jar_path, std::fs::Permissions::from_mode(0o600))?;
        }
        println!("Session stored at {}", jar_path.display());
    }

    Ok(())
}

pub async fn me(args: MeArgs) -> Result<()> {
    let config = Config::load_config(args.config.clone(), None)?;
    let jar_path = session_path();
    let cookie_string = std::fs::read_to_string(&jar_path)
        .with_context(|| format!("failed to read session jar {}", jar_path.display()))?;

    let cookie_url = config.server.public_base_url.clone();
    let jar = Arc::new(Jar::default());
    for cookie in cookie_string.split(';') {
        jar.add_cookie_str(cookie.trim(), &cookie_url);
    }

    let client = Client::builder()
        .cookie_provider(jar)
        .build()
        .context("failed to build HTTP client")?;

    let me_url = format!("{}/api/auth/me", config.server.public_base_url);
    let response = client.get(me_url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("/api/auth/me failed: {}", response.status());
    }

    let body: MeResponse = response.json().await?;
    println!("id: {}", body.id);
    if let Some(email) = body.email {
        println!("email: {}", email);
    }
    if let Some(username) = body.username {
        println!("username: {}", username);
    }

    Ok(())
}

pub fn session_path() -> PathBuf {
    BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("rustygpt").join("session.json"))
        .unwrap_or_else(|| PathBuf::from("./session.json"))
}

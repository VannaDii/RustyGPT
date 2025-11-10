use std::{collections::HashMap, convert::TryFrom, fmt::Write as _, sync::Arc};

use anyhow::{Context, Result};
use clap::Args;
use futures_util::StreamExt;
use reqwest::{Client, cookie::Jar};
use serde_json::from_str;
use shared::models::{
    ConversationStreamEvent, MembershipChangeAction, MessageRole, ReplyMessageRequest,
    ThreadListResponse, ThreadTreeResponse, UnreadSummaryResponse,
};
use tokio::time::{Duration, sleep};
use url::Url;
use uuid::Uuid;

use super::session;

fn client_with_session(server: &str) -> Result<(Client, Arc<Jar>, Url)> {
    let server_url = Url::parse(server).context("invalid server URL")?;
    let jar_path = session::session_path();
    let jar = session::load_cookie_jar(&server_url, &jar_path).with_context(|| {
        format!(
            "no session cookie jar found at {}; run `rustygpt session login` first",
            jar_path.display()
        )
    })?;
    let client = session::build_client(jar.clone())?;
    Ok((client, jar, server_url))
}

#[derive(Args, Debug)]
#[command(about = "List conversation threads or view a specific thread")]
pub struct ChatArgs {
    /// Conversation identifier to operate on
    #[arg(long, alias = "conv")]
    pub conversation: Uuid,

    /// Optional thread root identifier to display
    #[arg(long)]
    pub root: Option<Uuid>,

    /// Maximum number of items to fetch (threads or messages)
    #[arg(long)]
    pub limit: Option<i32>,

    /// `RustyGPT` server base URL (default: <http://localhost:8080>)
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,
}

#[derive(Args, Debug)]
#[command(about = "Reply to an existing message")]
pub struct ReplyArgs {
    /// Parent message identifier to reply to
    #[arg(long)]
    pub parent: Uuid,

    /// Reply text content
    #[arg()]
    pub text: String,

    /// `RustyGPT` server base URL (default: <http://localhost:8080>)
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,
}

#[derive(Args, Debug)]
#[command(about = "Follow streaming updates for a thread")]
pub struct FollowArgs {
    /// Thread root identifier to follow
    #[arg(long)]
    pub root: Uuid,

    /// `RustyGPT` server base URL (default: <http://localhost:8080>)
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,
}

pub async fn handle_chat(args: ChatArgs) -> Result<()> {
    let (client, _jar, server_url) = client_with_session(&args.server)?;
    let api_base = server_url
        .join("api/")
        .context("invalid API base for chat operations")?;

    if let Some(root) = args.root {
        let tree = fetch_thread_tree(&client, &api_base, root, args.limit).await?;
        render_thread(&tree);
    } else {
        let threads = fetch_threads(&client, &api_base, args.conversation, args.limit).await?;
        let unread = match fetch_unread_summary(&client, &api_base, args.conversation).await {
            Ok(summary) => summary,
            Err(err) => {
                eprintln!(
                    "warning: failed to fetch unread summary for {}: {err}",
                    args.conversation
                );
                UnreadSummaryResponse {
                    threads: Vec::new(),
                }
            }
        };
        let unread_map: HashMap<Uuid, i64> = unread
            .threads
            .into_iter()
            .map(|entry| (entry.root_id, entry.unread))
            .collect();
        render_thread_list(&threads, &unread_map);
    }

    Ok(())
}

pub async fn handle_reply(args: ReplyArgs) -> Result<()> {
    let (client, jar, server_url) = client_with_session(&args.server)?;
    let api_base = server_url
        .join("api/")
        .context("invalid API base for reply")?;

    let payload = ReplyMessageRequest {
        content: args.text.clone(),
        role: Some(MessageRole::User),
    };

    let mut request = client
        .post(api_base.join(&format!("messages/{parent}/reply", parent = args.parent))?)
        .json(&payload);
    if let Some(csrf) = session::csrf_token_from_jar(&jar, &server_url) {
        request = request.header("X-CSRF-Token", csrf);
    }
    let response = request
        .send()
        .await
        .context("request failed")?
        .error_for_status()
        .context("reply rejected")?;

    let reply: shared::models::ReplyMessageResponse = response.json().await?;
    println!(
        "Reply created: message={} root={}",
        reply.message_id, reply.root_id
    );
    Ok(())
}

pub async fn handle_follow(args: FollowArgs) -> Result<()> {
    let (client, _jar, server_url) = client_with_session(&args.server)?;
    let api_base = server_url
        .join("api/")
        .context("invalid API base for stream")?;

    // Determine conversation identifier by loading the thread once.
    let tree = fetch_thread_tree(&client, &api_base, args.root, Some(10)).await?;
    let conversation_id = tree
        .messages
        .first()
        .map(|msg| msg.conversation_id)
        .ok_or_else(|| anyhow::anyhow!("thread has no messages yet"))?;

    println!(
        "Following thread {} in conversation {}... (press Ctrl+C to stop)",
        args.root, conversation_id
    );

    let stream_url = api_base
        .join(&format!("stream/conversations/{conversation_id}"))
        .context("invalid stream endpoint")?;
    let mut last_event_id: Option<String> = None;
    let mut last_timestamp: Option<i64> = None;

    loop {
        let mut url = stream_url.clone();
        if let Some(ts) = last_timestamp {
            url.query_pairs_mut().append_pair("since", &ts.to_string());
        }

        let mut request = client.get(url);
        if let Some(id) = &last_event_id {
            request = request.header("Last-Event-ID", id);
        }

        let response = match request.send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("[stream] request rejected: {err}");
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            },
            Err(err) => {
                eprintln!("[stream] connection failed: {err}");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut stream = response.bytes_stream();
        let mut event_name: Option<String> = None;
        let mut data_buffer = String::new();
        let mut current_event_id: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!("[stream] chunk error: {err}");
                    break;
                }
            };
            let text = String::from_utf8_lossy(&bytes);

            for line in text.split('\n') {
                let trimmed = line.trim_end_matches('\r');

                if let Some(value) = trimmed.strip_prefix("event:") {
                    event_name = Some(value.trim().to_string());
                } else if let Some(value) = trimmed.strip_prefix("data:") {
                    let payload = value.trim();
                    data_buffer.push_str(payload);
                } else if let Some(value) = trimmed.strip_prefix("id:") {
                    current_event_id = Some(value.trim().to_string());
                } else if trimmed.is_empty() {
                    if let Some(name) = &event_name
                        && !data_buffer.is_empty()
                        && data_buffer != "[DONE]"
                    {
                        handle_stream_event(name, &data_buffer, args.root, conversation_id)?;
                    }
                    if let Some(id_value) = current_event_id.take() {
                        if let Some(ts) = parse_event_timestamp(&id_value) {
                            last_timestamp = Some(ts);
                        }
                        last_event_id = Some(id_value);
                    }
                    event_name = None;
                    data_buffer.clear();
                }
            }
        }

        if last_event_id.is_none() {
            return Ok(());
        }

        sleep(Duration::from_secs(1)).await;
    }
}

fn handle_stream_event(
    event_name: &str,
    data: &str,
    root_filter: Uuid,
    conversation_filter: Uuid,
) -> Result<()> {
    if let Ok(event) = from_str::<ConversationStreamEvent>(data) {
        match event {
            ConversationStreamEvent::MessageDelta { payload } => {
                if payload.root_id == root_filter {
                    for choice in payload.choices {
                        if let Some(content) = choice.delta.content {
                            print!("{content}");
                        }
                    }
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            ConversationStreamEvent::MessageDone { payload } => {
                if payload.root_id == root_filter {
                    println!();
                    if let Some(reason) = payload.finish_reason {
                        println!("[stream finished: {reason}]");
                    }
                    if let Some(usage) = payload.usage {
                        let prompt = usage.prompt_tokens;
                        let completion = usage.completion_tokens;
                        let total = usage.total_tokens;
                        println!("[usage prompt={prompt} completion={completion} total={total}]");
                    }
                }
            }
            ConversationStreamEvent::ThreadActivity { payload } => {
                if payload.root_id == root_filter {
                    let timestamp = payload.last_activity_at.0.format("%Y-%m-%d %H:%M:%S");
                    println!("[thread activity at {timestamp}]");
                }
            }
            ConversationStreamEvent::TypingUpdate { payload } => {
                if payload.root_id == root_filter {
                    let expires = payload.expires_at.0.format("%Y-%m-%d %H:%M:%S");
                    println!(
                        "[typing update user={user} expires={expires}]",
                        user = payload.user_id
                    );
                }
            }
            ConversationStreamEvent::PresenceUpdate { payload } => {
                let last_seen = payload.last_seen_at.0.format("%Y-%m-%d %H:%M:%S");
                println!(
                    "[presence user={user} status={status:?} last_seen={last_seen}]",
                    user = payload.user_id,
                    status = payload.status
                );
            }
            ConversationStreamEvent::UnreadUpdate { payload } => {
                if payload.root_id == root_filter {
                    let unread = payload.unread;
                    println!("[unread count updated: {unread}]");
                }
            }
            ConversationStreamEvent::MembershipChanged { payload } => {
                if payload.conversation_id == conversation_filter {
                    let action = match payload.action {
                        MembershipChangeAction::Added => "joined",
                        MembershipChangeAction::Removed => "left",
                        MembershipChangeAction::RoleUpdated => "role updated",
                    };
                    let role = payload
                        .role
                        .map_or_else(|| "unknown".to_string(), |role| role.as_str().to_string());
                    println!(
                        "[membership] user {user} {action} (role {role})",
                        user = payload.user_id
                    );
                }
            }
            ConversationStreamEvent::Error { payload } => {
                eprintln!(
                    "[stream error {code}] {message}",
                    code = payload.code,
                    message = payload.message
                );
            }
            ConversationStreamEvent::ThreadNew { .. } => {}
        }
    } else {
        eprintln!("[unparsed {event_name}] {data}");
    }

    Ok(())
}

fn parse_event_timestamp(event_id: &str) -> Option<i64> {
    let parts: Vec<&str> = event_id.split(':').collect();
    if parts.len() == 4 {
        parts[3].parse::<i64>().ok()
    } else {
        None
    }
}

async fn fetch_threads(
    client: &Client,
    api_base: &Url,
    conversation: Uuid,
    limit: Option<i32>,
) -> Result<ThreadListResponse> {
    let endpoint = api_base
        .join(&format!("conversations/{conversation}/threads"))
        .context("invalid threads endpoint")?;
    let mut request = client.get(endpoint);
    if let Some(limit) = limit {
        request = request.query(&[("limit", limit)]);
    }

    let response = request
        .send()
        .await
        .context("failed to fetch threads")?
        .error_for_status()
        .context("thread listing rejected")?;

    Ok(response.json().await?)
}

async fn fetch_thread_tree(
    client: &Client,
    api_base: &Url,
    root: Uuid,
    limit: Option<i32>,
) -> Result<ThreadTreeResponse> {
    let endpoint = api_base
        .join(&format!("threads/{root}/tree"))
        .context("invalid thread endpoint")?;
    let mut request = client.get(endpoint);
    if let Some(limit) = limit {
        request = request.query(&[("limit", limit)]);
    }

    let response = request
        .send()
        .await
        .context("failed to fetch thread tree")?
        .error_for_status()
        .context("thread request rejected")?;

    Ok(response.json().await?)
}

async fn fetch_unread_summary(
    client: &Client,
    api_base: &Url,
    conversation: Uuid,
) -> Result<UnreadSummaryResponse> {
    let endpoint = api_base
        .join(&format!("conversations/{conversation}/unread"))
        .context("invalid unread endpoint")?;
    let response = client
        .get(endpoint)
        .send()
        .await
        .context("failed to fetch unread summary")?
        .error_for_status()
        .context("unread summary rejected")?;

    Ok(response.json().await?)
}

fn render_thread_list(list: &ThreadListResponse, unread: &HashMap<Uuid, i64>) {
    if list.threads.is_empty() {
        println!("No threads found.");
        return;
    }

    for summary in &list.threads {
        let unread_count = unread.get(&summary.root_id).copied().unwrap_or(0);
        println!(
            "- root={} messages={} participants={} unread={} last={}",
            summary.root_id,
            summary.message_count,
            summary.participant_count,
            unread_count,
            summary.last_activity_at.0.format("%Y-%m-%d %H:%M:%S"),
        );
        if !summary.root_excerpt.is_empty() {
            println!("  snippet: {}", summary.root_excerpt);
        }
        println!();
    }

    if let Some(after) = &list.next_after {
        println!(
            "(More threads available after {})",
            after.0.format("%Y-%m-%d %H:%M:%S")
        );
    }
}

fn render_thread(tree: &ThreadTreeResponse) {
    println!("Thread {}", tree.root_id);
    for message in &tree.messages {
        let mut line = String::new();
        let indent = "  ".repeat(usize::try_from(message.depth.saturating_sub(1)).unwrap_or(0));
        let _ = write!(
            &mut line,
            "{}[{}] {}",
            indent,
            message.created_at.0.format("%Y-%m-%d %H:%M:%S"),
            message.content
        );
        println!("{line}");
    }

    if let Some(cursor) = &tree.next_cursor {
        println!("(More messages after path {cursor})");
    }
}

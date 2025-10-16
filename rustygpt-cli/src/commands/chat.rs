use std::fmt::Write as _;

use anyhow::{Context, Result};
use clap::Args;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::from_str;
use shared::models::{
    ConversationStreamEvent, MessageRole, ReplyMessageRequest, ThreadListResponse,
    ThreadTreeResponse,
};
use uuid::Uuid;

const API_SUFFIX: &str = "/api";

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

    /// RustyGPT server base URL (default: http://localhost:8080)
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

    /// RustyGPT server base URL (default: http://localhost:8080)
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,
}

#[derive(Args, Debug)]
#[command(about = "Follow streaming updates for a thread")]
pub struct FollowArgs {
    /// Thread root identifier to follow
    #[arg(long)]
    pub root: Uuid,

    /// RustyGPT server base URL (default: http://localhost:8080)
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,
}

pub async fn handle_chat(args: ChatArgs) -> Result<()> {
    let client = Client::new();
    let api_base = api_base(&args.server);

    if let Some(root) = args.root {
        let tree = fetch_thread_tree(&client, &api_base, root, args.limit).await?;
        render_thread(&tree);
    } else {
        let threads = fetch_threads(&client, &api_base, args.conversation, args.limit).await?;
        render_thread_list(&threads);
    }

    Ok(())
}

pub async fn handle_reply(args: ReplyArgs) -> Result<()> {
    let client = Client::new();
    let api_base = api_base(&args.server);

    let payload = ReplyMessageRequest {
        content: args.text.clone(),
        role: Some(MessageRole::User),
    };

    let url = format!("{}/messages/{}/reply", api_base, args.parent);
    let response = client
        .post(url)
        .json(&payload)
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
    let client = Client::new();
    let api_base = api_base(&args.server);

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

    let url = format!("{}/stream/conversations/{}", api_base, conversation_id);
    let response = client
        .get(url)
        .send()
        .await
        .context("failed to connect to stream")?
        .error_for_status()
        .context("stream request rejected")?;

    let mut stream = response.bytes_stream();
    let mut event_name: Option<String> = None;
    let mut data_buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = String::from_utf8_lossy(&bytes);

        for line in text.split('\n') {
            let trimmed = line.trim_end_matches('\r');

            if trimmed.starts_with("event:") {
                event_name = Some(trimmed[6..].trim().to_string());
            } else if trimmed.starts_with("data:") {
                let payload = trimmed[5..].trim();
                data_buffer.push_str(payload);
            } else if trimmed.is_empty() {
                if let Some(name) = &event_name {
                    if !data_buffer.is_empty() {
                        handle_stream_event(name, &data_buffer, args.root).await?;
                    }
                }
                event_name = None;
                data_buffer.clear();
            }
        }
    }

    Ok(())
}

async fn handle_stream_event(event_name: &str, data: &str, root_filter: Uuid) -> Result<()> {
    if let Ok(event) = from_str::<ConversationStreamEvent>(data) {
        match event {
            ConversationStreamEvent::MessageDelta { payload } => {
                if payload.root_id == root_filter {
                    for choice in payload.choices {
                        if let Some(content) = choice.delta.content {
                            print!("{}", content);
                        }
                    }
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            ConversationStreamEvent::MessageDone { payload } => {
                if payload.root_id == root_filter {
                    println!();
                    if let Some(reason) = payload.finish_reason {
                        println!("[stream finished: {}]", reason);
                    }
                    if let Some(usage) = payload.usage {
                        println!(
                            "[usage prompt={} completion={} total={}]",
                            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                        );
                    }
                }
            }
            ConversationStreamEvent::ThreadActivity { payload } => {
                if payload.root_id == root_filter {
                    println!(
                        "[thread activity at {}]",
                        payload.last_activity_at.0.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
            ConversationStreamEvent::Error { payload } => {
                eprintln!("[stream error {}] {}", payload.code, payload.message);
            }
            _ => {}
        }
    } else {
        eprintln!("[unparsed {}] {}", event_name, data);
    }

    Ok(())
}

async fn fetch_threads(
    client: &Client,
    api_base: &str,
    conversation: Uuid,
    limit: Option<i32>,
) -> Result<ThreadListResponse> {
    let url = format!("{}/conversations/{}/threads", api_base, conversation);
    let mut request = client.get(url);
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
    api_base: &str,
    root: Uuid,
    limit: Option<i32>,
) -> Result<ThreadTreeResponse> {
    let url = format!("{}/threads/{}/tree", api_base, root);
    let mut request = client.get(url);
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

fn render_thread_list(list: &ThreadListResponse) {
    if list.threads.is_empty() {
        println!("No threads found.");
        return;
    }

    for summary in &list.threads {
        println!(
            "- root={} messages={} participants={} last={}",
            summary.root_id,
            summary.message_count,
            summary.participant_count,
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
        let indent = "  ".repeat(message.depth.saturating_sub(1) as usize);
        let _ = write!(
            &mut line,
            "{}[{}] {}",
            indent,
            message.created_at.0.format("%Y-%m-%d %H:%M:%S"),
            message.content
        );
        println!("{}", line);
    }

    if let Some(cursor) = &tree.next_cursor {
        println!("(More messages after path {})", cursor);
    }
}

fn api_base(server: &str) -> String {
    let mut base = server.trim_end_matches('/').to_string();
    base.push_str(API_SUFFIX);
    base
}

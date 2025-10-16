use reqwest::{Client, Error};
use shared::models::{
    PostRootMessageRequest, PostRootMessageResponse, ReplyMessageRequest, ReplyMessageResponse,
    ThreadListResponse, ThreadTreeResponse,
};
use shared::models::{SetupRequest, SetupResponse};
use uuid::Uuid;

/// Lightweight API client for RustyGPT web interactions.
#[derive(Clone, Debug)]
pub struct RustyGPTClient {
    base_url: String,
    client: Client,
}

impl RustyGPTClient {
    /// Create a new API client with the provided base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// Retrieve setup state.
    pub async fn get_setup(&self) -> Result<SetupResponse, Error> {
        let url = self.api_url("setup");
        self.client.get(url).send().await?.json().await
    }

    /// Submit setup configuration.
    pub async fn post_setup(&self, payload: &SetupRequest) -> Result<(), Error> {
        let url = self.api_url("setup");
        self.client.post(url).json(payload).send().await?;
        Ok(())
    }

    /// List threads for a conversation.
    pub async fn list_threads(
        &self,
        conversation_id: &Uuid,
        after: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ThreadListResponse, Error> {
        let url = self.api_url(&format!("conversations/{}/threads", conversation_id));
        let mut request = self.client.get(url);
        if let Some(after) = after {
            request = request.query(&[("after", after)]);
        }
        if let Some(limit) = limit {
            request = request.query(&[("limit", &limit)]);
        }
        request.send().await?.json().await
    }

    /// Fetch a depth-first slice of a thread.
    pub async fn get_thread_tree(
        &self,
        root_id: &Uuid,
        cursor_path: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ThreadTreeResponse, Error> {
        let url = self.api_url(&format!("threads/{}/tree", root_id));
        let mut request = self.client.get(url);
        if let Some(cursor) = cursor_path {
            request = request.query(&[("cursor_path", cursor)]);
        }
        if let Some(limit) = limit {
            request = request.query(&[("limit", &limit)]);
        }
        request.send().await?.json().await
    }

    /// Post a root message starting a new thread.
    pub async fn post_root_message(
        &self,
        conversation_id: &Uuid,
        request: &PostRootMessageRequest,
    ) -> Result<PostRootMessageResponse, Error> {
        let url = self.api_url(&format!("threads/{}/root", conversation_id));
        self.client
            .post(url)
            .json(request)
            .send()
            .await?
            .json()
            .await
    }

    /// Reply to an existing message.
    pub async fn reply_message(
        &self,
        parent_id: &Uuid,
        request: &ReplyMessageRequest,
    ) -> Result<ReplyMessageResponse, Error> {
        let url = self.api_url(&format!("messages/{}/reply", parent_id));
        self.client
            .post(url)
            .json(request)
            .send()
            .await?
            .json()
            .await
    }

    /// Helper to construct the SSE conversation stream URL.
    pub fn conversation_stream_url(&self, conversation_id: &Uuid) -> String {
        self.api_url(&format!("stream/conversations/{}", conversation_id))
    }
}

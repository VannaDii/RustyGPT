use once_cell::unsync::OnceCell;
use reqwest::{Client, Error, RequestBuilder, Response, StatusCode};
use shared::models::{
    LoginRequest, LoginResponse, MeResponse, PostRootMessageRequest, PostRootMessageResponse,
    ReplyMessageRequest, ReplyMessageResponse, ThreadListResponse, ThreadTreeResponse,
    UnreadSummaryResponse,
};
use shared::models::{SetupRequest, SetupResponse};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, Window};

const CSRF_COOKIE_NAME: &str = "CSRF-TOKEN";
const SESSION_ROTATED_HEADER: &str = "x-session-rotated";
const DEFAULT_BASE_URL: &str = "/api";

thread_local! {
    static SHARED_CLIENT: OnceCell<RustyGPTClient> = OnceCell::new();
}

/// Lightweight API client for RustyGPT web interactions.
#[derive(Clone, Debug)]
pub struct RustyGPTClient {
    base_url: String,
    client: Client,
    csrf_token: Arc<Mutex<Option<String>>>,
}

impl RustyGPTClient {
    /// Create a new API client with the provided base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            csrf_token: Arc::new(Mutex::new(None)),
        };

        if let Some(token) = read_cookie(CSRF_COOKIE_NAME) {
            client.set_csrf_token(Some(token));
        }

        client
    }

    pub fn shared() -> Self {
        SHARED_CLIENT.with(|cell| cell.get_or_init(|| Self::new(DEFAULT_BASE_URL)).clone())
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub fn set_csrf_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.csrf_token.lock() {
            *guard = token;
        }
    }

    pub fn current_csrf_token(&self) -> Option<String> {
        self.csrf_token
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().cloned())
    }

    fn apply_csrf(&self, request: RequestBuilder) -> RequestBuilder {
        if let Some(token) = self.current_csrf_token() {
            request.header("X-CSRF-Token", token)
        } else {
            request
        }
    }

    fn capture_rotation(&self, response: &reqwest::Response) {
        if response
            .headers()
            .get(SESSION_ROTATED_HEADER)
            .map(|value| value == "1")
            .unwrap_or(false)
        {
            if let Some(token) = read_cookie(CSRF_COOKIE_NAME) {
                self.set_csrf_token(Some(token));
            }
        }
    }

    fn should_refresh(response: &Response) -> bool {
        response.status() == StatusCode::UNAUTHORIZED
            && response.headers().contains_key("www-authenticate")
    }

    async fn send_with_refresh<F>(&self, build: F) -> Result<Response, Error>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut response = build().send().await?;
        if Self::should_refresh(&response) {
            drop(response);
            self.refresh_session().await?;
            response = build().send().await?;
        }
        Ok(response)
    }

    /// Retrieve setup state.
    pub async fn get_setup(&self) -> Result<SetupResponse, Error> {
        let url = self.api_url("setup");
        let response = self
            .send_with_refresh(move || self.client.get(url.clone()))
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Submit setup configuration.
    pub async fn post_setup(&self, payload: &SetupRequest) -> Result<(), Error> {
        let url = self.api_url("setup");
        let payload_ref = payload;
        let response = self
            .send_with_refresh(move || self.client.post(url.clone()).json(payload_ref))
            .await?;
        self.capture_rotation(&response);
        Ok(())
    }

    /// Authenticate with email/password credentials.
    pub async fn login(&self, payload: &LoginRequest) -> Result<LoginResponse, Error> {
        let url = self.api_url("auth/login");
        let response = self.client.post(url).json(payload).send().await?;
        self.capture_rotation(&response);
        let body: LoginResponse = response.json().await?;
        self.set_csrf_token(Some(body.csrf_token.clone()));
        Ok(body)
    }

    /// Refresh the current session cookie.
    pub async fn refresh_session(&self) -> Result<LoginResponse, Error> {
        let url = self.api_url("auth/refresh");
        let response = self.client.post(url).send().await?;
        self.capture_rotation(&response);
        let body: LoginResponse = response.json().await?;
        self.set_csrf_token(Some(body.csrf_token.clone()));
        Ok(body)
    }

    /// Retrieve the authenticated user profile without rotating the session.
    pub async fn get_profile(&self) -> Result<MeResponse, Error> {
        let url = self.api_url("auth/me");
        let response = self
            .send_with_refresh(move || self.client.get(url.clone()))
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Terminate the current session.
    pub async fn logout(&self) -> Result<(), Error> {
        let url = self.api_url("auth/logout");
        let response = self.apply_csrf(self.client.post(url)).send().await?;
        self.capture_rotation(&response);
        self.set_csrf_token(None);
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
        let after_param = after.map(|value| value.to_string());
        let limit_param = limit;
        let response = self
            .send_with_refresh(move || {
                let mut request = self.client.get(url.clone());
                if let Some(ref cursor) = after_param {
                    request = request.query(&[("after", cursor.as_str())]);
                }
                if let Some(limit) = limit_param {
                    request = request.query(&[("limit", &limit)]);
                }
                request
            })
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Fetch a depth-first slice of a thread.
    pub async fn get_thread_tree(
        &self,
        root_id: &Uuid,
        cursor_path: Option<&str>,
        limit: Option<i32>,
    ) -> Result<ThreadTreeResponse, Error> {
        let url = self.api_url(&format!("threads/{}/tree", root_id));
        let cursor_param = cursor_path.map(|value| value.to_string());
        let limit_param = limit;
        let response = self
            .send_with_refresh(move || {
                let mut request = self.client.get(url.clone());
                if let Some(ref cursor) = cursor_param {
                    request = request.query(&[("cursor_path", cursor.as_str())]);
                }
                if let Some(limit) = limit_param {
                    request = request.query(&[("limit", &limit)]);
                }
                request
            })
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Fetch unread summary for a conversation.
    pub async fn unread_summary(
        &self,
        conversation_id: &Uuid,
    ) -> Result<UnreadSummaryResponse, Error> {
        let url = self.api_url(&format!("conversations/{}/unread", conversation_id));
        let response = self
            .send_with_refresh(move || self.client.get(url.clone()))
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Post a root message starting a new thread.
    pub async fn post_root_message(
        &self,
        conversation_id: &Uuid,
        request: &PostRootMessageRequest,
    ) -> Result<PostRootMessageResponse, Error> {
        let url = self.api_url(&format!("threads/{}/root", conversation_id));
        let payload = request.clone();
        let response = self
            .send_with_refresh(move || {
                let builder = self.apply_csrf(self.client.post(url.clone()));
                builder.json(&payload)
            })
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Reply to an existing message.
    pub async fn reply_message(
        &self,
        parent_id: &Uuid,
        request: &ReplyMessageRequest,
    ) -> Result<ReplyMessageResponse, Error> {
        let url = self.api_url(&format!("messages/{}/reply", parent_id));
        let payload = request.clone();
        let response = self
            .send_with_refresh(move || {
                let builder = self.apply_csrf(self.client.post(url.clone()));
                builder.json(&payload)
            })
            .await?;
        self.capture_rotation(&response);
        response.json().await
    }

    /// Helper to construct the SSE conversation stream URL.
    pub fn conversation_stream_url(&self, conversation_id: &Uuid) -> String {
        self.api_url(&format!("stream/conversations/{}", conversation_id))
    }
}

fn read_cookie(name: &str) -> Option<String> {
    let window: Window = web_sys::window()?;
    let document = window.document()?;
    let html_doc: HtmlDocument = document.dyn_into().ok()?;
    let cookie_string = html_doc.cookie().ok()?;

    for pair in cookie_string.split(';') {
        let mut parts = pair.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name {
            return Some(value.to_string());
        }
    }
    None
}

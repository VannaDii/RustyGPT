#![allow(dead_code)]

use reqwest::{Client, Error};
use shared::models::{
    Conversation, SetupRequest, SetupResponse,
    conversation::{SendMessageRequest, SendMessageResponse},
    oauth::{OAuthInitResponse, OAuthRequest},
};
use uuid::Uuid;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Enum for supported OAuth providers.
pub enum OAuthProvider {
    GitHub,
    Apple,
}

impl OAuthProvider {
    fn as_str(&self) -> &'static str {
        match self {
            OAuthProvider::GitHub => "github",
            OAuthProvider::Apple => "apple",
        }
    }
}

/// API client for interacting with the RustyGPT backend.
pub struct RustyGPTClient {
    base_url: String,
    client: Client,
}

impl RustyGPTClient {
    /// Create a new RustyGPTClient.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    /// Get the setup status.
    pub async fn get_setup(&self) -> Result<SetupResponse, Error> {
        let url = format!("{}/setup", self.base_url);
        let response = self.client.get(&url).send().await?.json().await?;
        log(format!("Setup response: {:?}", response).as_str());
        Ok(response)
    }

    /// Post a setup request.
    pub async fn post_setup(&self, setup_request: SetupRequest) -> Result<(), Error> {
        let url = format!("{}/setup", self.base_url);
        self.client.post(&url).json(&setup_request).send().await?;
        Ok(())
    }

    /// Get all conversations.
    pub async fn get_conversations(&self) -> Result<Vec<Conversation>, Error> {
        let url = format!("{}/conversation", self.base_url);
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Send a message to a conversation.
    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        message_request: SendMessageRequest,
    ) -> Result<SendMessageResponse, Error> {
        let url = format!(
            "{}/conversation/{}/messages",
            self.base_url, conversation_id
        );
        let response = self
            .client
            .post(&url)
            .json(&message_request)
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
    }

    /// Initialize OAuth authentication.
    pub async fn init_oauth(&self, provider: OAuthProvider) -> Result<OAuthInitResponse, Error> {
        let url = format!("{}/oauth/{}", self.base_url, provider.as_str());
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Complete OAuth authentication.
    pub async fn complete_oauth(
        &self,
        provider: OAuthProvider,
        oauth_request: OAuthRequest,
    ) -> Result<(), Error> {
        let url = format!("{}/oauth/{}/manual", self.base_url, provider.as_str());
        self.client.post(&url).json(&oauth_request).send().await?;
        Ok(())
    }
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthProvider {
    /// GitHub OAuth provider
    GitHub,
    /// Apple OAuth provider
    Apple,
}

impl OAuthProvider {
    /// Returns the string representation of the OAuth provider.
    ///
    /// # Returns
    /// A string slice representing the provider name in lowercase.
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthProvider::GitHub => "github",
            OAuthProvider::Apple => "apple",
        }
    }
}

/// API client for interacting with the RustyGPT backend.
#[derive(Debug, Clone)]
pub struct RustyGPTClient {
    /// Base URL for the API server
    base_url: String,
    /// HTTP client for making requests
    client: Client,
}

impl RustyGPTClient {
    /// Create a new RustyGPTClient.
    ///
    /// # Arguments
    /// * `base_url` - The base URL for the API server
    ///
    /// # Returns
    /// A new [`RustyGPTClient`] instance.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    /// Get the base URL for this client.
    ///
    /// # Returns
    /// A reference to the base URL string.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the setup status.
    ///
    /// # Returns
    /// A [`Result`] containing the setup response or an error.
    pub async fn get_setup(&self) -> Result<SetupResponse, Error> {
        let url = format!("{}/setup", self.base_url);
        let response = self.client.get(&url).send().await?.json().await?;
        log(format!("Setup response: {:?}", response).as_str());
        Ok(response)
    }

    /// Post a setup request.
    ///
    /// # Arguments
    /// * `setup_request` - The setup request to send
    ///
    /// # Returns
    /// A [`Result`] indicating success or an error.
    pub async fn post_setup(&self, setup_request: SetupRequest) -> Result<(), Error> {
        let url = format!("{}/setup", self.base_url);
        self.client.post(&url).json(&setup_request).send().await?;
        Ok(())
    }

    /// Get all conversations.
    ///
    /// # Returns
    /// A [`Result`] containing a vector of conversations or an error.
    pub async fn get_conversations(&self) -> Result<Vec<Conversation>, Error> {
        let url = format!("{}/conversation", self.base_url);
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Send a message to a conversation.
    ///
    /// # Arguments
    /// * `conversation_id` - UUID of the target conversation
    /// * `message_request` - The message to send
    ///
    /// # Returns
    /// A [`Result`] containing the send message response or an error.
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
    ///
    /// # Arguments
    /// * `provider` - The OAuth provider to use
    ///
    /// # Returns
    /// A [`Result`] containing the OAuth initialization response or an error.
    pub async fn init_oauth(&self, provider: OAuthProvider) -> Result<OAuthInitResponse, Error> {
        let url = format!("{}/oauth/{}", self.base_url, provider.as_str());
        let response = self.client.get(&url).send().await?.json().await?;
        Ok(response)
    }

    /// Complete OAuth authentication.
    ///
    /// # Arguments
    /// * `provider` - The OAuth provider being used
    /// * `oauth_request` - The OAuth request data
    ///
    /// # Returns
    /// A [`Result`] indicating success or an error.
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

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    /// Test OAuthProvider string conversion
    #[wasm_bindgen_test]
    fn test_oauth_provider_as_str() {
        assert_eq!(OAuthProvider::GitHub.as_str(), "github");
        assert_eq!(OAuthProvider::Apple.as_str(), "apple");
    }

    /// Test OAuthProvider equality
    #[wasm_bindgen_test]
    fn test_oauth_provider_equality() {
        assert_eq!(OAuthProvider::GitHub, OAuthProvider::GitHub);
        assert_eq!(OAuthProvider::Apple, OAuthProvider::Apple);
        assert_ne!(OAuthProvider::GitHub, OAuthProvider::Apple);
    }

    /// Test OAuthProvider cloning
    #[wasm_bindgen_test]
    fn test_oauth_provider_clone() {
        let provider = OAuthProvider::GitHub;
        let cloned = provider.clone();
        assert_eq!(provider, cloned);
    }

    /// Test RustyGPTClient creation
    #[wasm_bindgen_test]
    fn test_rustygpt_client_new() {
        let client = RustyGPTClient::new("http://localhost:8080");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    /// Test RustyGPTClient with custom URL
    #[wasm_bindgen_test]
    fn test_rustygpt_client_custom_url() {
        let client = RustyGPTClient::new("https://api.rustygpt.com");
        assert_eq!(client.base_url(), "https://api.rustygpt.com");
    }

    /// Test RustyGPTClient with empty URL
    #[wasm_bindgen_test]
    fn test_rustygpt_client_empty_url() {
        let client = RustyGPTClient::new("");
        assert_eq!(client.base_url(), "");
    }

    /// Test RustyGPTClient URL formation for setup endpoints
    #[wasm_bindgen_test]
    fn test_setup_url_formation() {
        let client = RustyGPTClient::new("http://localhost:8080");

        // Test the URL formation logic used in get_setup
        let expected_url = format!("{}/setup", client.base_url());
        assert_eq!(expected_url, "http://localhost:8080/setup");
    }

    /// Test RustyGPTClient URL formation for conversation endpoints
    #[wasm_bindgen_test]
    fn test_conversation_url_formation() {
        let client = RustyGPTClient::new("http://localhost:8080");

        // Test the URL formation logic used in get_conversations
        let expected_url = format!("{}/conversation", client.base_url());
        assert_eq!(expected_url, "http://localhost:8080/conversation");
    }

    /// Test RustyGPTClient URL formation for message endpoints
    #[wasm_bindgen_test]
    fn test_message_url_formation() {
        let client = RustyGPTClient::new("http://localhost:8080");
        let conversation_id = Uuid::new_v4();

        // Test the URL formation logic used in send_message
        let expected_url = format!(
            "{}/conversation/{}/messages",
            client.base_url(),
            conversation_id
        );
        assert!(expected_url.contains("/conversation/"));
        assert!(expected_url.contains("/messages"));
        assert!(expected_url.contains(&conversation_id.to_string()));
    }

    /// Test RustyGPTClient URL formation for OAuth endpoints
    #[wasm_bindgen_test]
    fn test_oauth_url_formation() {
        let client = RustyGPTClient::new("http://localhost:8080");

        // Test the URL formation logic used in init_oauth
        let github_url = format!(
            "{}/oauth/{}",
            client.base_url(),
            OAuthProvider::GitHub.as_str()
        );
        let apple_url = format!(
            "{}/oauth/{}",
            client.base_url(),
            OAuthProvider::Apple.as_str()
        );

        assert_eq!(github_url, "http://localhost:8080/oauth/github");
        assert_eq!(apple_url, "http://localhost:8080/oauth/apple");

        // Test complete OAuth URL formation
        let complete_github_url = format!(
            "{}/oauth/{}/manual",
            client.base_url(),
            OAuthProvider::GitHub.as_str()
        );
        assert_eq!(
            complete_github_url,
            "http://localhost:8080/oauth/github/manual"
        );
    }

    /// Test RustyGPTClient cloning
    #[wasm_bindgen_test]
    fn test_rustygpt_client_clone() {
        let client = RustyGPTClient::new("http://test.com");
        let cloned = client.clone();
        assert_eq!(client.base_url(), cloned.base_url());
    }

    /// Test RustyGPTClient with various URL formats
    #[wasm_bindgen_test]
    fn test_rustygpt_client_url_formats() {
        let http_client = RustyGPTClient::new("http://example.com");
        let https_client = RustyGPTClient::new("https://example.com");
        let port_client = RustyGPTClient::new("http://localhost:3000");
        let path_client = RustyGPTClient::new("https://api.example.com/v1");

        assert_eq!(http_client.base_url(), "http://example.com");
        assert_eq!(https_client.base_url(), "https://example.com");
        assert_eq!(port_client.base_url(), "http://localhost:3000");
        assert_eq!(path_client.base_url(), "https://api.example.com/v1");
    }

    /// Test all OAuth provider variants
    #[wasm_bindgen_test]
    fn test_all_oauth_providers() {
        let providers = vec![OAuthProvider::GitHub, OAuthProvider::Apple];
        let expected_strings = vec!["github", "apple"];

        for (provider, expected) in providers.iter().zip(expected_strings.iter()) {
            assert_eq!(provider.as_str(), *expected);
        }
    }

    /// Test OAuth provider debug formatting
    #[wasm_bindgen_test]
    fn test_oauth_provider_debug() {
        let github = OAuthProvider::GitHub;
        let apple = OAuthProvider::Apple;

        let github_debug = format!("{:?}", github);
        let apple_debug = format!("{:?}", apple);

        assert!(github_debug.contains("GitHub"));
        assert!(apple_debug.contains("Apple"));
    }

    /// Test RustyGPTClient debug formatting
    #[wasm_bindgen_test]
    fn test_rustygpt_client_debug() {
        let client = RustyGPTClient::new("http://test.com");
        let debug_str = format!("{:?}", client);

        assert!(debug_str.contains("RustyGPTClient"));
        assert!(debug_str.contains("http://test.com"));
    }
}

pub mod apple_auth;
pub mod conversation;
pub mod copilot;
pub mod github_auth;
pub mod setup;
pub mod streaming;

#[cfg(test)]
mod conversation_test;

#[cfg(test)]
mod conversation_tests;

#[cfg(test)]
mod apple_auth_test;

#[cfg(test)]
mod apple_auth_tests;

#[cfg(test)]
mod github_auth_test;

#[cfg(test)]
mod github_auth_tests;

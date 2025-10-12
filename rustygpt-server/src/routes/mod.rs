pub mod auth;
pub mod copilot;
pub mod health;
pub mod openapi;
pub mod protected;
pub mod setup;
pub mod well_known;

#[cfg(test)]
mod auth_test;

#[cfg(test)]
mod openapi_tests;

#[cfg(test)]
mod protected_test;

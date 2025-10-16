//! Setup container module for first-time application configuration.
//!
//! This module implements the setup form that is displayed when the application
//! is first run and no users exist in the database. It collects the necessary
//! information to create the first admin user.

use super::setup_validation::{
    ValidationError, validate_confirm_password, validate_email, validate_password,
    validate_username,
};
use crate::api::RustyGPTClient;
use i18nrs::yew::use_translation;
use shared::models::SetupRequest;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, window};
use yew::{
    AttrValue, Html, TargetCast,
    events::{Event, FocusEvent, SubmitEvent},
    function_component, html, use_effect_with, use_node_ref, use_state,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Component for first-time setup of the application.
///
/// This component is shown when the system detects no users exist in the database,
/// prompting the creation of the first admin user. It presents a form collecting:
///
/// * Username (min 3 characters)
/// * Email address (validated for basic format)
/// * Password (min 8 characters)
/// * Password confirmation (must match)
///
/// After successful setup, it automatically reloads the application to enter the
/// main interface with the newly created user.
#[function_component(Setup)]
pub fn setup() -> Html {
    let (i18n, _) = use_translation();

    // Form inputs state
    let username = use_state(String::new);
    let email = use_state(String::new);
    let password = use_state(String::new);
    let confirm_password = use_state(String::new);

    // Form validation state
    let username_error = use_state(|| Option::<AttrValue>::None);
    let email_error = use_state(|| Option::<AttrValue>::None);
    let password_error = use_state(|| Option::<AttrValue>::None);
    let confirm_password_error = use_state(|| Option::<AttrValue>::None);

    // Form processing state
    let is_submitting = use_state(|| false);
    let form_error = use_state(|| Option::<AttrValue>::None);
    let setup_complete = use_state(|| false);

    // Input refs for focus management
    let username_ref = use_node_ref();
    let email_ref = use_node_ref();
    let password_ref = use_node_ref();
    let confirm_password_ref = use_node_ref();

    // Set up initial dark theme
    initialize_theme();

    // Handle successful setup completion
    handle_setup_completion(*setup_complete);

    // Form change handlers
    let on_username_change = {
        let username = username.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            username.set(input.value());
        }
    };

    let on_email_change = {
        let email = email.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            email.set(input.value());
        }
    };

    let on_password_change = {
        let password = password.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        }
    };

    let on_confirm_password_change = {
        let confirm_password = confirm_password.clone();
        move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            confirm_password.set(input.value());
        }
    };

    // Form validation handlers
    let on_username_blur = {
        let username = username.clone();
        let username_error = username_error.clone();
        let i18n = i18n.clone();

        move |_: FocusEvent| {
            let value = (*username).clone();
            match validate_username(&value) {
                Ok(_) => username_error.set(None),
                Err(ValidationError::Required) => {
                    username_error.set(Some(i18n.t("setup.errors.username_required").into()));
                }
                Err(ValidationError::UsernameTooShort) => {
                    username_error.set(Some(i18n.t("setup.errors.username_too_short").into()));
                }
                _ => {} // Other error types not applicable for username
            }
        }
    };

    let on_email_blur = {
        let email = email.clone();
        let email_error = email_error.clone();
        let i18n = i18n.clone();

        move |_: FocusEvent| {
            let value = (*email).clone();
            match validate_email(&value) {
                Ok(_) => email_error.set(None),
                Err(ValidationError::Required) => {
                    email_error.set(Some(i18n.t("setup.errors.email_required").into()));
                }
                Err(ValidationError::InvalidEmail) => {
                    email_error.set(Some(i18n.t("setup.errors.email_invalid").into()));
                }
                _ => {} // Other error types not applicable for email
            }
        }
    };

    let on_password_blur = {
        let password = password.clone();
        let password_error = password_error.clone();
        let i18n = i18n.clone();

        move |_: FocusEvent| {
            let value = (*password).clone();
            match validate_password(&value) {
                Ok(_) => password_error.set(None),
                Err(ValidationError::Required) => {
                    password_error.set(Some(i18n.t("setup.errors.password_required").into()));
                }
                Err(ValidationError::PasswordTooShort) => {
                    password_error.set(Some(i18n.t("setup.errors.password_too_short").into()));
                }
                _ => {} // Other error types not applicable for password
            }
        }
    };

    let on_confirm_password_blur = {
        let password = password.clone();
        let confirm_password = confirm_password.clone();
        let confirm_password_error = confirm_password_error.clone();
        let i18n = i18n.clone();

        move |_: FocusEvent| {
            let password_val = (*password).clone();
            let confirm_val = (*confirm_password).clone();

            match validate_confirm_password(&confirm_val, &password_val) {
                Ok(_) => confirm_password_error.set(None),
                Err(ValidationError::Required) => {
                    confirm_password_error.set(Some(
                        i18n.t("setup.errors.confirm_password_required").into(),
                    ));
                }
                Err(ValidationError::PasswordsDoNotMatch) => {
                    confirm_password_error
                        .set(Some(i18n.t("setup.errors.passwords_dont_match").into()));
                }
                _ => {} // Other error types not applicable for password confirmation
            }
        }
    };

    // Form submission handler
    let on_submit = {
        let username = username.clone();
        let email = email.clone();
        let password = password.clone();
        let confirm_password = confirm_password.clone();
        let username_error = username_error.clone();
        let email_error = email_error.clone();
        let password_error = password_error.clone();
        let confirm_password_error = confirm_password_error.clone();
        let is_submitting = is_submitting.clone();
        let form_error = form_error.clone();
        let setup_complete = setup_complete.clone();
        let i18n = i18n.clone();

        move |e: SubmitEvent| {
            e.prevent_default();

            // Don't submit if already submitting
            if *is_submitting {
                return;
            }

            // Reset errors
            form_error.set(None);

            // Validate all fields
            let mut has_errors = false;

            // Username validation
            match validate_username(&username) {
                Ok(_) => username_error.set(None),
                Err(ValidationError::Required) => {
                    username_error.set(Some(i18n.t("setup.errors.username_required").into()));
                    has_errors = true;
                }
                Err(ValidationError::UsernameTooShort) => {
                    username_error.set(Some(i18n.t("setup.errors.username_too_short").into()));
                    has_errors = true;
                }
                _ => {} // Other error types not applicable for username
            }

            // Email validation
            match validate_email(&email) {
                Ok(_) => email_error.set(None),
                Err(ValidationError::Required) => {
                    email_error.set(Some(i18n.t("setup.errors.email_required").into()));
                    has_errors = true;
                }
                Err(ValidationError::InvalidEmail) => {
                    email_error.set(Some(i18n.t("setup.errors.email_invalid").into()));
                    has_errors = true;
                }
                _ => {} // Other error types not applicable for email
            }

            // Password validation
            match validate_password(&password) {
                Ok(_) => password_error.set(None),
                Err(ValidationError::Required) => {
                    password_error.set(Some(i18n.t("setup.errors.password_required").into()));
                    has_errors = true;
                }
                Err(ValidationError::PasswordTooShort) => {
                    password_error.set(Some(i18n.t("setup.errors.password_too_short").into()));
                    has_errors = true;
                }
                _ => {} // Other error types not applicable for password
            }

            // Password confirmation validation
            match validate_confirm_password(&confirm_password, &password) {
                Ok(_) => confirm_password_error.set(None),
                Err(ValidationError::Required) => {
                    confirm_password_error.set(Some(
                        i18n.t("setup.errors.confirm_password_required").into(),
                    ));
                    has_errors = true;
                }
                Err(ValidationError::PasswordsDoNotMatch) => {
                    confirm_password_error
                        .set(Some(i18n.t("setup.errors.passwords_dont_match").into()));
                    has_errors = true;
                }
                _ => {} // Other error types not applicable for password confirmation
            }

            // Skip submission if there are validation errors
            if has_errors {
                return;
            }

            // Set submitting state and create the API request
            is_submitting.set(true);

            let username_value = (*username).clone();
            let email_value = (*email).clone();
            let password_value = (*password).clone();
            let form_error = form_error.clone();
            let is_submitting = is_submitting.clone();
            let setup_complete = setup_complete.clone();

            // Perform the API request asynchronously
            spawn_local(async move {
                let client = create_api_client();
                let setup_request = SetupRequest {
                    username: username_value,
                    email: email_value,
                    password: password_value,
                };

                match client.post_setup(&setup_request).await {
                    Ok(_) => {
                        setup_complete.set(true);
                    }
                    Err(err) => {
                        form_error.set(Some(format!("{}", err).into()));
                        is_submitting.set(false);
                    }
                }
            });
        }
    };

    html! {
      <div class="flex flex-col justify-center items-center min-h-screen p-6">
          <div class="flex flex-col items-center text-center gap-6 max-w-xl w-full">
              <span class="text-sm text-accent">{i18n.t("app.title")}</span>
              <h1 class="text-5xl font-bold">{i18n.t("setup.heading")}</h1>
              <p class="text-lg">
                  {i18n.t("setup.description")}
              </p>

              if *setup_complete {
                <div class="alert alert-success shadow-lg">
                  <div>
                    <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current flex-shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <span>{i18n.t("setup.success_message")}</span>
                  </div>
                </div>
              } else {
                <div class="card w-full bg-base-200 shadow-xl">
                  <div class="card-body">
                    if let Some(error) = &*form_error {
                      <div class="alert alert-error shadow-lg mb-4">
                        <div>
                          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current flex-shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
                          </svg>
                          <span>{error}</span>
                        </div>
                      </div>
                    }

                    <form onsubmit={on_submit}>
                      <div class="form-control">
                        <label class="label">
                          <span class="label-text">{i18n.t("setup.form.username_label")}</span>
                        </label>
                        <input
                          type="text"
                          placeholder={i18n.t("setup.form.username_placeholder")}
                          class={format!("input input-bordered w-full {}", if username_error.is_some() { "input-error" } else { "" })}
                          value={(*username).clone()}
                          onchange={on_username_change}
                          onblur={on_username_blur}
                          ref={username_ref}
                          disabled={*is_submitting}
                          data-testid="setup-username-input"
                        />
                        if let Some(error) = &*username_error {
                          <label class="label">
                            <span class="label-text-alt text-error">{error}</span>
                          </label>
                        }
                      </div>

                      <div class="form-control mt-2">
                        <label class="label">
                          <span class="label-text">{i18n.t("setup.form.email_label")}</span>
                        </label>
                        <input
                          type="email"
                          placeholder={i18n.t("setup.form.email_placeholder")}
                          class={format!("input input-bordered w-full {}", if email_error.is_some() { "input-error" } else { "" })}
                          value={(*email).clone()}
                          onchange={on_email_change}
                          onblur={on_email_blur}
                          ref={email_ref}
                          disabled={*is_submitting}
                          data-testid="setup-email-input"
                        />
                        if let Some(error) = &*email_error {
                          <label class="label">
                            <span class="label-text-alt text-error">{error}</span>
                          </label>
                        }
                      </div>

                      <div class="form-control mt-2">
                        <label class="label">
                          <span class="label-text">{i18n.t("setup.form.password_label")}</span>
                        </label>
                        <input
                          type="password"
                          placeholder={i18n.t("setup.form.password_placeholder")}
                          class={format!("input input-bordered w-full {}", if password_error.is_some() { "input-error" } else { "" })}
                          value={(*password).clone()}
                          onchange={on_password_change}
                          onblur={on_password_blur}
                          ref={password_ref}
                          disabled={*is_submitting}
                          data-testid="setup-password-input"
                        />
                        if let Some(error) = &*password_error {
                          <label class="label">
                            <span class="label-text-alt text-error">{error}</span>
                          </label>
                        }
                      </div>

                      <div class="form-control mt-2">
                        <label class="label">
                          <span class="label-text">{i18n.t("setup.form.confirm_password_label")}</span>
                        </label>
                        <input
                          type="password"
                          placeholder={i18n.t("setup.form.confirm_password_placeholder")}
                          class={format!("input input-bordered w-full {}", if confirm_password_error.is_some() { "input-error" } else { "" })}
                          value={(*confirm_password).clone()}
                          onchange={on_confirm_password_change}
                          onblur={on_confirm_password_blur}
                          ref={confirm_password_ref}
                          disabled={*is_submitting}
                          data-testid="setup-confirm-password-input"
                        />
                        if let Some(error) = &*confirm_password_error {
                          <label class="label">
                            <span class="label-text-alt text-error">{error}</span>
                          </label>
                        }
                      </div>

                      <div class="form-control mt-6">
                        <button
                          type="submit"
                          class="btn btn-primary"
                          disabled={*is_submitting}
                          data-testid="setup-submit-button"
                        >
                          if *is_submitting {
                            <span class="loading loading-spinner loading-sm mr-2"></span>
                          }
                          {i18n.t("setup.form.submit_button")}
                        </button>
                      </div>
                    </form>
                  </div>
                </div>
              }
          </div>
      </div>
    }
}

/// Creates an API client for interacting with the backend.
///
/// This function is extracted to make it easier to mock in tests.
///
/// # Returns
/// A [`RustyGPTClient`](crate::api::RustyGPTClient) instance configured to connect to the backend.
pub fn create_api_client() -> RustyGPTClient {
    RustyGPTClient::new("http://localhost:8080/api")
}

/// Sets the initial theme for the application.
///
/// Applies the 'dark' theme to the document's HTML element by setting the
/// 'data-theme' attribute.
fn initialize_theme() {
    use_effect_with((), |_| {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(html_element) = document.document_element() {
                    html_element
                        .set_attribute("data-theme", "dark")
                        .unwrap_or_default();
                }
            }
        }
        || {}
    });
}

/// Handles the redirection after successful setup completion.
///
/// Handles the redirection after successful setup completion.
///
/// # Arguments
/// * `is_complete` - Boolean indicating if setup has completed successfully
fn handle_setup_completion(is_complete: bool) {
    use_effect_with(is_complete, move |&is_complete| {
        if is_complete {
            // Wait 2 seconds before reloading the page to show the main app
            let window = web_sys::window().expect("no global window exists");
            let closure = Closure::once_into_js(move || {
                let window = web_sys::window().expect("no global window exists");
                let _ = window.location().reload();
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                2000,
            );
        }
        || {}
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_create_api_client() {
        let client = create_api_client();
        let stream_url = client.conversation_stream_url(&Uuid::nil());
        assert!(
            stream_url.contains("/api/stream/conversations/00000000-0000-0000-0000-000000000000")
        );
    }
}

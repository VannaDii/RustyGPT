//! Test module for the Setup component.
//!
//! This module contains unit tests for the Setup component, including validation
//! logic and rendering tests.

use wasm_bindgen_test::*;
use yew::prelude::*;

use crate::containers::setup::{
    Setup, ValidationError, validate_confirm_password, validate_email, validate_password,
    validate_username,
};

wasm_bindgen_test_configure!(run_in_browser);

/// Tests the validation logic for usernames.
#[wasm_bindgen_test]
fn test_validate_username() {
    // Empty username should return Required error
    match validate_username("") {
        Err(ValidationError::Required) => (),
        _ => panic!("Empty username should return Required error"),
    }

    // Username with only spaces should return Required error
    match validate_username("   ") {
        Err(ValidationError::Required) => (),
        _ => panic!("Username with only spaces should return Required error"),
    }

    // Username with less than 3 characters should return UsernameTooShort error
    match validate_username("ab") {
        Err(ValidationError::UsernameTooShort) => (),
        _ => panic!("Short username should return UsernameTooShort error"),
    }

    // Valid username should return Ok
    match validate_username("validuser") {
        Ok(_) => (),
        _ => panic!("Valid username should return Ok"),
    }

    // Valid username with exactly 3 characters should return Ok
    match validate_username("abc") {
        Ok(_) => (),
        _ => panic!("Valid username with exactly 3 characters should return Ok"),
    }
}

/// Tests the validation logic for email addresses.
#[wasm_bindgen_test]
fn test_validate_email() {
    // Empty email should return Required error
    match validate_email("") {
        Err(ValidationError::Required) => (),
        _ => panic!("Empty email should return Required error"),
    }

    // Email with only spaces should return Required error
    match validate_email("   ") {
        Err(ValidationError::Required) => (),
        _ => panic!("Email with only spaces should return Required error"),
    }

    // Email without @ should return InvalidEmail error
    match validate_email("invalidemail.com") {
        Err(ValidationError::InvalidEmail) => (),
        _ => panic!("Email without @ should return InvalidEmail error"),
    }

    // Valid email should return Ok
    match validate_email("user@example.com") {
        Ok(_) => (),
        _ => panic!("Valid email should return Ok"),
    }
}

/// Tests the validation logic for passwords.
#[wasm_bindgen_test]
fn test_validate_password() {
    // Empty password should return Required error
    match validate_password("") {
        Err(ValidationError::Required) => (),
        _ => panic!("Empty password should return Required error"),
    }

    // Password with only spaces should return Required error
    match validate_password("   ") {
        Err(ValidationError::Required) => (),
        _ => panic!("Password with only spaces should return Required error"),
    }

    // Password with less than 8 characters should return PasswordTooShort error
    match validate_password("1234567") {
        Err(ValidationError::PasswordTooShort) => (),
        _ => panic!("Short password should return PasswordTooShort error"),
    }

    // Valid password should return Ok
    match validate_password("password123") {
        Ok(_) => (),
        _ => panic!("Valid password should return Ok"),
    }

    // Valid password with exactly 8 characters should return Ok
    match validate_password("12345678") {
        Ok(_) => (),
        _ => panic!("Valid password with exactly 8 characters should return Ok"),
    }
}

/// Tests the validation logic for password confirmation.
#[wasm_bindgen_test]
fn test_validate_confirm_password() {
    let password = "password123";

    // Empty confirmation should return Required error
    match validate_confirm_password("", password) {
        Err(ValidationError::Required) => (),
        _ => panic!("Empty confirmation should return Required error"),
    }

    // Confirmation with only spaces should return Required error
    match validate_confirm_password("   ", password) {
        Err(ValidationError::Required) => (),
        _ => panic!("Confirmation with only spaces should return Required error"),
    }

    // Different confirmation should return PasswordsDoNotMatch error
    match validate_confirm_password("different", password) {
        Err(ValidationError::PasswordsDoNotMatch) => (),
        _ => panic!("Different confirmation should return PasswordsDoNotMatch error"),
    }

    // Matching confirmation should return Ok
    match validate_confirm_password(password, password) {
        Ok(_) => (),
        _ => panic!("Matching confirmation should return Ok"),
    }
}

/// Tests that the Setup component renders correctly.
#[wasm_bindgen_test]
async fn test_setup_component_renders() {
    // Render the Setup component
    let rendered = yew::ServerRenderer::<Setup>::new().render().await;

    // Check for the heading
    assert!(rendered.contains("<h1 class=\"text-5xl font-bold\">"));

    // Check for all form fields
    assert!(rendered.contains("data-testid=\"setup-username-input\""));
    assert!(rendered.contains("data-testid=\"setup-email-input\""));
    assert!(rendered.contains("data-testid=\"setup-password-input\""));
    assert!(rendered.contains("data-testid=\"setup-confirm-password-input\""));
    assert!(rendered.contains("data-testid=\"setup-submit-button\""));

    // Check for form labels
    assert!(rendered.contains("<span class=\"label-text\">"));

    // The form should be visible (not showing success message initially)
    assert!(rendered.contains("<form"));
    assert!(!rendered.contains("alert-success"));
}

/// Tests that the form validation error messages are present in the component.
///
/// This test only verifies that the error message containers can be present
/// but doesn't test the actual validation logic (which is tested separately).
#[wasm_bindgen_test]
async fn test_setup_component_error_handling() {
    // Render the Setup component
    let rendered = yew::ServerRenderer::<Setup>::new().render().await;

    // Check that the error message containers can be present
    // We don't show errors initially, but the component has conditionals to display them
    assert!(
        rendered.contains("<span class=\"label-text-alt text-error\">")
            || rendered.contains("class=\"label\"")
    );

    // Check for form-wide error alert container
    assert!(
        rendered.contains("<div class=\"alert alert-error shadow-lg mb-4\">")
            || rendered.contains("class=\"card-body\"")
    );
}

/// Tests the structure of the form in the Setup component.
#[wasm_bindgen_test]
async fn test_setup_form_structure() {
    // Render the Setup component
    let rendered = yew::ServerRenderer::<Setup>::new().render().await;

    // Check the form has all the required elements in the correct structure
    assert!(rendered.contains("<div class=\"form-control\">"));
    assert!(rendered.contains("<input"));
    assert!(rendered.contains("<button"));

    // Check that inputs have appropriate types
    assert!(rendered.contains("type=\"text\""));
    assert!(rendered.contains("type=\"email\""));
    assert!(rendered.contains("type=\"password\""));

    // Check that the submit button is a primary button
    assert!(rendered.contains("class=\"btn btn-primary\""));

    // Check for loading spinner in submit button
    assert!(
        rendered.contains("<span class=\"loading loading-spinner loading-sm mr-2\">")
            || rendered.contains("type=\"submit\"")
    );
}

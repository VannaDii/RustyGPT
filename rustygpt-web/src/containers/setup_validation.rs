//! Validation logic for the setup form.
//!
//! This module contains the validation functions used by the setup form,
//! extracted from the main setup component to enable easier testing.

/// Validation errors that can occur during form validation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValidationError {
    /// Field is required but empty
    Required,
    /// Username is too short (less than 3 characters)
    UsernameTooShort,
    /// Email address is invalid (missing @ symbol)
    InvalidEmail,
    /// Password is too short (less than 8 characters)
    PasswordTooShort,
    /// Password confirmation doesn't match password
    PasswordsDoNotMatch,
}

/// Validates a username.
///
/// # Arguments
/// * `username` - The username to validate
///
/// # Returns
/// `Ok(())` if the username is valid, otherwise a [`ValidationError`].
///
/// # Validation rules
/// - Username must not be empty
/// - Username must be at least 3 characters long
pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.trim().is_empty() {
        return Err(ValidationError::Required);
    }

    if username.trim().len() < 3 {
        return Err(ValidationError::UsernameTooShort);
    }

    Ok(())
}

/// Validates an email address.
///
/// # Arguments
/// * `email` - The email address to validate
///
/// # Returns
/// `Ok(())` if the email is valid, otherwise a [`ValidationError`].
///
/// # Validation rules
/// - Email must not be empty
/// - Email must contain an '@' symbol
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::Required);
    }

    if !trimmed.contains('@') {
        return Err(ValidationError::InvalidEmail);
    }

    Ok(())
}

/// Validates a password.
///
/// # Arguments
/// * `password` - The password to validate
///
/// # Returns
/// `Ok(())` if the password is valid, otherwise a [`ValidationError`].
///
/// # Validation rules
/// - Password must not be empty
/// - Password must be at least 8 characters long
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.trim().is_empty() {
        return Err(ValidationError::Required);
    }

    if password.len() < 8 {
        return Err(ValidationError::PasswordTooShort);
    }

    Ok(())
}

/// Validates that the password confirmation matches the password.
///
/// # Arguments
/// * `confirm_password` - The confirmation password
/// * `password` - The original password
///
/// # Returns
/// `Ok(())` if the confirmation is valid, otherwise a [`ValidationError`].
///
/// # Validation rules
/// - Confirmation must not be empty
/// - Confirmation must match the password
pub fn validate_confirm_password(
    confirm_password: &str,
    password: &str,
) -> Result<(), ValidationError> {
    if confirm_password.trim().is_empty() {
        return Err(ValidationError::Required);
    }

    if confirm_password != password {
        return Err(ValidationError::PasswordsDoNotMatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("testuser").is_ok());
        assert!(validate_username("a_very_long_username").is_ok());
        assert!(validate_username("abc").is_ok()); // Exactly 3 characters
    }

    #[test]
    fn test_validate_username_invalid() {
        // Empty username
        assert_eq!(validate_username(""), Err(ValidationError::Required));

        // Whitespace only
        assert_eq!(validate_username("   "), Err(ValidationError::Required));

        // Too short
        assert_eq!(
            validate_username("ab"),
            Err(ValidationError::UsernameTooShort)
        );
        assert_eq!(
            validate_username("a"),
            Err(ValidationError::UsernameTooShort)
        );
    }

    #[test]
    fn test_validate_username_edge_cases() {
        // Username with leading/trailing spaces (trimmed length matters)
        assert_eq!(
            validate_username("  ab  "),
            Err(ValidationError::UsernameTooShort)
        );

        // Username with special characters should be valid if long enough
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("user.name").is_ok());
    }

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user@domain.org").is_ok());
        assert!(validate_email("simple@test.co").is_ok());
        assert!(validate_email("user.name+tag@example.com").is_ok());
        assert!(validate_email("@").is_ok()); // Minimal valid case
    }

    #[test]
    fn test_validate_email_invalid() {
        // Empty email
        assert_eq!(validate_email(""), Err(ValidationError::Required));

        // Whitespace only
        assert_eq!(validate_email("   "), Err(ValidationError::Required));

        // Missing @ symbol
        assert_eq!(
            validate_email("userexample.com"),
            Err(ValidationError::InvalidEmail)
        );
        assert_eq!(
            validate_email("user.name"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn test_validate_email_edge_cases() {
        // Multiple @ symbols - our simple validation considers this valid
        assert!(validate_email("user@@example.com").is_ok());

        // Email with spaces (but contains @)
        assert!(validate_email("user @example.com").is_ok());

        // @ at different positions
        assert!(validate_email("@example.com").is_ok());
        assert!(validate_email("user@").is_ok());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("12345678").is_ok()); // Exactly 8 characters
        assert!(validate_password("a_very_secure_password").is_ok());
        assert!(validate_password("MySecureP@ssw0rd!").is_ok());
    }

    #[test]
    fn test_validate_password_invalid() {
        // Empty password
        assert_eq!(validate_password(""), Err(ValidationError::Required));

        // Whitespace only
        assert_eq!(validate_password("   "), Err(ValidationError::Required));

        // Too short
        assert_eq!(
            validate_password("1234567"),
            Err(ValidationError::PasswordTooShort)
        );
        assert_eq!(
            validate_password("short"),
            Err(ValidationError::PasswordTooShort)
        );
        assert_eq!(
            validate_password("a"),
            Err(ValidationError::PasswordTooShort)
        );
    }

    #[test]
    fn test_validate_password_edge_cases() {
        // Password with spaces should be valid if long enough
        assert!(validate_password("pass word 123").is_ok());

        // Password with special characters
        assert!(validate_password("p@ssw0rd!").is_ok());

        // Unicode characters
        assert!(validate_password("pässwörd123").is_ok());
    }

    #[test]
    fn test_validate_confirm_password_valid() {
        assert!(validate_confirm_password("password123", "password123").is_ok());
        assert!(validate_confirm_password("MySecureP@ssw0rd!", "MySecureP@ssw0rd!").is_ok());
        assert!(validate_confirm_password("pass word", "pass word").is_ok());
    }

    #[test]
    fn test_validate_confirm_password_invalid() {
        // Empty confirmation
        assert_eq!(
            validate_confirm_password("", "password123"),
            Err(ValidationError::Required)
        );

        // Whitespace only confirmation
        assert_eq!(
            validate_confirm_password("   ", "password123"),
            Err(ValidationError::Required)
        );

        // Non-matching passwords
        assert_eq!(
            validate_confirm_password("different", "password123"),
            Err(ValidationError::PasswordsDoNotMatch)
        );

        // Case sensitive comparison
        assert_eq!(
            validate_confirm_password("Password123", "password123"),
            Err(ValidationError::PasswordsDoNotMatch)
        );
    }

    #[test]
    fn test_validate_confirm_password_edge_cases() {
        // Both empty should still be an error (confirmation is required)
        assert_eq!(
            validate_confirm_password("", ""),
            Err(ValidationError::Required)
        );

        // Spaces matter in comparison
        assert_eq!(
            validate_confirm_password("password ", "password"),
            Err(ValidationError::PasswordsDoNotMatch)
        );

        // Special characters must match exactly
        assert_eq!(
            validate_confirm_password("p@ssw0rd", "p@ssword"),
            Err(ValidationError::PasswordsDoNotMatch)
        );
    }

    #[test]
    fn test_validation_error_variants() {
        // Test that we can create and match on all variants
        let errors = vec![
            ValidationError::Required,
            ValidationError::UsernameTooShort,
            ValidationError::InvalidEmail,
            ValidationError::PasswordTooShort,
            ValidationError::PasswordsDoNotMatch,
        ];

        assert_eq!(errors.len(), 5);

        // Verify all error variants can be matched and handled
        for error in errors {
            match error {
                ValidationError::Required => {
                    // Test that required error is handled
                }
                ValidationError::UsernameTooShort => {
                    // Test that username too short error is handled
                }
                ValidationError::InvalidEmail => {
                    // Test that invalid email error is handled
                }
                ValidationError::PasswordTooShort => {
                    // Test that password too short error is handled
                }
                ValidationError::PasswordsDoNotMatch => {
                    // Test that passwords don't match error is handled
                }
            }
        }
    }

    #[test]
    fn test_comprehensive_validation_workflow() {
        // Test a complete validation workflow with valid inputs
        let username = "testuser";
        let email = "test@example.com";
        let password = "password123";
        let confirm_password = "password123";

        assert!(validate_username(username).is_ok());
        assert!(validate_email(email).is_ok());
        assert!(validate_password(password).is_ok());
        assert!(validate_confirm_password(confirm_password, password).is_ok());
    }

    #[test]
    fn test_realistic_user_inputs() {
        // Test with realistic but edge case inputs
        assert!(validate_username("user_123").is_ok());
        assert!(validate_email("user.name+tag@example.com").is_ok());
        assert!(validate_password("MySecureP@ssw0rd!").is_ok());
        assert!(validate_confirm_password("MySecureP@ssw0rd!", "MySecureP@ssw0rd!").is_ok());

        // Test with minimal valid inputs
        assert!(validate_username("abc").is_ok());
        assert!(validate_email("a@b").is_ok());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_confirm_password("12345678", "12345678").is_ok());
    }

    #[test]
    fn test_validation_error_debug_and_clone() {
        let error = ValidationError::Required;
        let cloned = error.clone();
        assert_eq!(error, cloned);

        // Test Debug formatting
        let debug_str = format!("{:?}", ValidationError::UsernameTooShort);
        assert!(debug_str.contains("UsernameTooShort"));
    }
}

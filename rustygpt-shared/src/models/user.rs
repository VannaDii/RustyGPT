use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use utoipa::ToSchema;

use super::Timestamp;

/// Global role assignments for a user account.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Member,
    ReadOnly,
}

impl UserRole {
    /// Return the canonical string representation expected by persistence layers.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
            Self::ReadOnly => "read_only",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UserRole {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "read_only" => Ok(Self::ReadOnly),
            _ => Err("unknown user role"),
        }
    }
}

/// Represents a user in the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct User {
    /// Unique identifier for the user.
    pub id: uuid::Uuid,

    /// The user's username.
    pub username: String,

    /// The user's email address.
    pub email: String,

    /// When the user was created.
    pub created_at: Timestamp,
}

/// Request to create a new user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CreateUserRequest {
    /// The user's username.
    pub username: String,

    /// The user's email address.
    pub email: String,

    /// The user's password.
    pub password: String,
}

/// Request to authenticate a user with username or email
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AuthenticateRequest {
    /// The user's username or email address.
    pub username_or_email: String,

    /// The user's password.
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json;
    use uuid::Uuid;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            created_at: Timestamp(Utc::now()),
        };

        assert!(!user.id.is_nil(), "User ID should not be nil");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
    }

    #[test]
    fn test_user_equality() {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let user1 = User {
            id,
            username: "sameuser".to_string(),
            email: "same@example.com".to_string(),
            created_at: Timestamp(now),
        };

        let user2 = User {
            id,
            username: "sameuser".to_string(),
            email: "same@example.com".to_string(),
            created_at: Timestamp(now),
        };

        let user3 = User {
            id: Uuid::new_v4(), // Different ID
            username: "diffuser".to_string(),
            email: "same@example.com".to_string(),
            created_at: Timestamp(now),
        };

        assert_eq!(user1, user2, "Users with the same data should be equal");
        assert_ne!(
            user1, user3,
            "Users with different data should not be equal"
        );
    }

    #[test]
    fn test_user_serialization() {
        let id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
        let timestamp = Timestamp(Utc::now());

        let user = User {
            id,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            created_at: timestamp,
        };

        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, user);
        assert_eq!(deserialized.id, id);
        assert_eq!(deserialized.username, "testuser");
        assert_eq!(deserialized.email, "test@example.com");
    }

    #[test]
    fn test_create_user_request() {
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(request.username, "testuser");
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
    }

    #[test]
    fn test_authenticate_request() {
        let request = AuthenticateRequest {
            username_or_email: "testuser".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(request.username_or_email, "testuser");
        assert_eq!(request.password, "password123");

        // Test with email
        let request_email = AuthenticateRequest {
            username_or_email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(request_email.username_or_email, "test@example.com");
        assert_eq!(request_email.password, "password123");
    }

    #[test]
    fn user_role_roundtrip() {
        for (text, role) in [
            ("admin", UserRole::Admin),
            ("member", UserRole::Member),
            ("read_only", UserRole::ReadOnly),
        ] {
            assert_eq!(role.as_str(), text);
            assert_eq!(role.to_string(), text);
            assert_eq!(UserRole::from_str(text).unwrap(), role);
        }
    }

    #[test]
    fn user_role_invalid() {
        assert!(UserRole::from_str("guest").is_err());
    }
}

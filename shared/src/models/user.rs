use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents a user in the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct User {
    /// Unique identifier for the user.
    pub id: uuid::Uuid,

    /// The user's chosen display name.
    pub username: String,

    /// The user's email address.
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use uuid::Uuid;

    #[test]
    fn test_user_creation() {
        let user = User {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
        };

        assert!(!user.id.is_nil(), "User ID should not be nil");
        assert_eq!(user.username, "test_user");
        assert_eq!(user.email, "test@example.com");
    }

    #[test]
    fn test_user_equality() {
        let id = Uuid::new_v4();
        let user1 = User {
            id,
            username: "same_user".to_string(),
            email: "same@example.com".to_string(),
        };

        let user2 = User {
            id,
            username: "same_user".to_string(),
            email: "same@example.com".to_string(),
        };

        let user3 = User {
            id: Uuid::new_v4(), // Different ID
            username: "same_user".to_string(),
            email: "same@example.com".to_string(),
        };

        assert_eq!(user1, user2, "Users with the same ID should be equal");
        assert_ne!(user1, user3, "Users with different IDs should not be equal");
    }

    #[test]
    fn test_user_serialization() {
        let id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
        let user = User {
            id,
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
        };

        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, user);
        assert_eq!(deserialized.id, id);
        assert_eq!(deserialized.username, "test_user");
        assert_eq!(deserialized.email, "test@example.com");
    }

    #[test]
    fn test_user_with_empty_username() {
        let user = User {
            id: Uuid::new_v4(),
            username: "".to_string(),
            email: "test@example.com".to_string(),
        };

        assert_eq!(user.username, "");
        assert!(user.username.is_empty());
    }

    #[test]
    fn test_user_with_empty_email() {
        let user = User {
            id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "".to_string(),
        };

        assert_eq!(user.email, "");
        assert!(user.email.is_empty());
    }
}

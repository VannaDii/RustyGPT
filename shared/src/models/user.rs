use serde::{Deserialize, Serialize};

/// Represents a user in the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

        assert_eq!(user1, user2, "Users with the same ID should be equal");
    }
}

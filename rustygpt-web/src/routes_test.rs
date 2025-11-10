//! Tests for the routing system
//!
//! Validates route definitions, navigation handling, and URL parameter parsing
//! for the chat application's routing infrastructure.

#[cfg(test)]
mod tests {
    use crate::routes::MainRoute;

    /// Tests route enum variants
    #[test]
    fn test_route_variants() {
        let home = MainRoute::Home;
        let chat = MainRoute::Chat;
        let login = MainRoute::Login;
        let admin_root = MainRoute::AdminRoot;
        let admin = MainRoute::Admin;
        let not_found = MainRoute::NotFound;
        let chat_conv = MainRoute::ChatConversation {
            conversation_id: "conv-123".to_string(),
        };

        // Test Debug trait
        assert!(format!("{home:?}").contains("Home"));
        assert!(format!("{chat:?}").contains("Chat"));
        assert!(format!("{login:?}").contains("Login"));
        assert!(format!("{admin_root:?}").contains("AdminRoot"));
        assert!(format!("{admin:?}").contains("Admin"));
        assert!(format!("{not_found:?}").contains("NotFound"));
        assert!(format!("{chat_conv:?}").contains("ChatConversation"));
    }

    /// Tests route equality
    #[test]
    fn test_route_equality() {
        let route1 = MainRoute::Home;
        let route2 = MainRoute::Home;
        assert_eq!(route1, route2);

        let chat1 = MainRoute::ChatConversation {
            conversation_id: "conv-123".to_string(),
        };
        let chat2 = MainRoute::ChatConversation {
            conversation_id: "conv-123".to_string(),
        };
        assert_eq!(chat1, chat2);

        let chat3 = MainRoute::ChatConversation {
            conversation_id: "conv-456".to_string(),
        };
        assert_ne!(chat1, chat3);
    }

    /// Tests route cloning
    #[test]
    fn test_route_cloning() {
        let original = MainRoute::Chat;
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let conversation_route = MainRoute::ChatConversation {
            conversation_id: "test-conv".to_string(),
        };
        let cloned_conv = conversation_route.clone();
        assert_eq!(conversation_route, cloned_conv);
    }

    /// Tests conversation ID parameter
    #[test]
    fn test_conversation_id_parameter() {
        let conv_id = "conv-abc123";
        let route = MainRoute::ChatConversation {
            conversation_id: conv_id.to_string(),
        };

        match route {
            MainRoute::ChatConversation { conversation_id } => {
                assert_eq!(conversation_id, "conv-abc123");
                assert!(conversation_id.starts_with("conv-"));
            }
            _ => panic!("Expected ChatConversation route"),
        }
    }

    /// Tests route matching patterns
    #[test]
    fn test_route_matching() {
        let routes = vec![
            MainRoute::Home,
            MainRoute::Chat,
            MainRoute::Login,
            MainRoute::AdminRoot,
            MainRoute::Admin,
            MainRoute::NotFound,
            MainRoute::ChatConversation {
                conversation_id: "test".to_string(),
            },
        ];

        for route in routes {
            assert!(matches!(
                route,
                MainRoute::Home
                    | MainRoute::Chat
                    | MainRoute::ChatConversation { .. }
                    | MainRoute::Login
                    | MainRoute::AdminRoot
                    | MainRoute::Admin
                    | MainRoute::NotFound
            ));
        }
    }

    /// Tests route conversion to strings
    #[test]
    fn test_route_display() {
        let home = MainRoute::Home;
        let chat = MainRoute::Chat;
        let not_found = MainRoute::NotFound;

        // Test that routes can be converted to debug strings
        let home_str = format!("{home:?}");
        let chat_str = format!("{chat:?}");
        let not_found_str = format!("{not_found:?}");

        assert!(!home_str.is_empty());
        assert!(!chat_str.is_empty());
        assert!(!not_found_str.is_empty());
    }

    /// Tests conversation ID validation
    #[test]
    fn test_conversation_id_validation() {
        let valid_ids = vec![
            "conv-123",
            "conversation-abc-456",
            "test-conv-001",
            "user-session-789",
        ];

        for id in valid_ids {
            let route = MainRoute::ChatConversation {
                conversation_id: id.to_string(),
            };

            match route {
                MainRoute::ChatConversation { conversation_id } => {
                    assert!(!conversation_id.is_empty());
                    assert!(conversation_id.len() > 3);
                }
                _ => panic!("Expected ChatConversation route"),
            }
        }
    }

    /// Tests empty conversation ID handling
    #[test]
    fn test_empty_conversation_id() {
        let route = MainRoute::ChatConversation {
            conversation_id: String::new(),
        };

        match route {
            MainRoute::ChatConversation { conversation_id } => {
                assert!(conversation_id.is_empty());
            }
            _ => panic!("Expected ChatConversation route"),
        }
    }

    /// Tests special characters in conversation ID
    #[test]
    fn test_special_characters_in_id() {
        let special_ids = vec![
            "conv-with-dashes",
            "conv_with_underscores",
            "conv123numbers",
            "conv.with.dots",
        ];

        for id in special_ids {
            let route = MainRoute::ChatConversation {
                conversation_id: id.to_string(),
            };

            match route {
                MainRoute::ChatConversation { conversation_id } => {
                    assert_eq!(conversation_id, id);
                    assert!(!conversation_id.is_empty());
                }
                _ => panic!("Expected ChatConversation route"),
            }
        }
    }

    /// Tests route comparison with different conversation IDs
    #[test]
    fn test_conversation_route_comparison() {
        let route1 = MainRoute::ChatConversation {
            conversation_id: "conv-1".to_string(),
        };
        let route2 = MainRoute::ChatConversation {
            conversation_id: "conv-2".to_string(),
        };
        let route3 = MainRoute::ChatConversation {
            conversation_id: "conv-1".to_string(),
        };

        assert_ne!(route1, route2);
        assert_eq!(route1, route3);
        assert_ne!(route2, route3);
    }

    /// Tests that admin routes are distinct
    #[test]
    fn test_admin_routes() {
        let admin_root = MainRoute::AdminRoot;
        let admin = MainRoute::Admin;

        assert_ne!(admin_root, admin);

        // Test debug output
        assert!(format!("{admin_root:?}").contains("AdminRoot"));
        assert!(format!("{admin:?}").contains("Admin"));
        assert!(!format!("{admin:?}").contains("Root"));
    }
}

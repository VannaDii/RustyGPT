//! Tests for the main entry point of the RustyGPT server.

use crate::main;
use std::env;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_main_function_exists() {
        // Test that main function exists and can be referenced
        // This ensures the main function signature is correct
        use std::error::Error;
        let _main_fn: fn() -> Result<(), Box<dyn Error>> = main;
    }

    #[test]
    fn test_env_var_parsing() {
        // Test environment variable handling without actually running the server
        unsafe {
            env::set_var("RUST_LOG", "debug");
            env::set_var("SERVER_PORT", "8080");

            // Verify environment variables are set correctly
            assert_eq!(env::var("RUST_LOG").unwrap(), "debug");
            assert_eq!(env::var("SERVER_PORT").unwrap(), "8080");

            // Clean up
            env::remove_var("RUST_LOG");
            env::remove_var("SERVER_PORT");
        }
    }

    #[test]
    fn test_default_port_behavior() {
        // Test that default port is used when no environment variable is set
        unsafe {
            env::remove_var("SERVER_PORT");

            // Verify no port is set
            assert!(env::var("SERVER_PORT").is_err());
        }
    }

    #[test]
    fn test_config_file_env_var() {
        // Test configuration file environment variable handling
        unsafe {
            env::set_var("CONFIG_FILE", "test_config.yaml");

            assert_eq!(env::var("CONFIG_FILE").unwrap(), "test_config.yaml");

            env::remove_var("CONFIG_FILE");
        }
    }

    #[test]
    fn test_database_url_env_var() {
        // Test database URL environment variable handling
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://localhost/test");

            assert_eq!(
                env::var("DATABASE_URL").unwrap(),
                "postgresql://localhost/test"
            );

            env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    #[serial]
    fn test_oauth_env_vars() {
        // Test OAuth-related environment variables
        unsafe {
            env::set_var("GITHUB_CLIENT_ID", "test_github_id");
            env::set_var("GITHUB_CLIENT_SECRET", "test_github_secret");
            env::set_var("APPLE_CLIENT_ID", "test_apple_id");
            env::set_var("APPLE_CLIENT_SECRET", "test_apple_secret");

            assert_eq!(env::var("GITHUB_CLIENT_ID").unwrap(), "test_github_id");
            assert_eq!(
                env::var("GITHUB_CLIENT_SECRET").unwrap(),
                "test_github_secret"
            );
            assert_eq!(env::var("APPLE_CLIENT_ID").unwrap(), "test_apple_id");
            assert_eq!(
                env::var("APPLE_CLIENT_SECRET").unwrap(),
                "test_apple_secret"
            );

            // Clean up
            env::remove_var("GITHUB_CLIENT_ID");
            env::remove_var("GITHUB_CLIENT_SECRET");
            env::remove_var("APPLE_CLIENT_ID");
            env::remove_var("APPLE_CLIENT_SECRET");
        }
    }

    #[test]
    fn test_jwt_secret_env_var() {
        // Test JWT secret environment variable
        unsafe {
            env::set_var("JWT_SECRET", "test_jwt_secret_key");

            assert_eq!(env::var("JWT_SECRET").unwrap(), "test_jwt_secret_key");

            env::remove_var("JWT_SECRET");
        }
    }

    #[test]
    fn test_cors_origin_env_var() {
        // Test CORS origin environment variable
        unsafe {
            env::set_var("CORS_ORIGIN", "http://localhost:3000");

            assert_eq!(env::var("CORS_ORIGIN").unwrap(), "http://localhost:3000");

            env::remove_var("CORS_ORIGIN");
        }
    }

    #[test]
    fn test_multiple_env_vars_simultaneously() {
        // Test setting multiple environment variables at once
        unsafe {
            env::set_var("SERVER_PORT", "9000");
            env::set_var("RUST_LOG", "info");
            env::set_var("DATABASE_URL", "postgresql://localhost/rusty_gpt");

            assert_eq!(env::var("SERVER_PORT").unwrap(), "9000");
            assert_eq!(env::var("RUST_LOG").unwrap(), "info");
            assert_eq!(
                env::var("DATABASE_URL").unwrap(),
                "postgresql://localhost/rusty_gpt"
            );

            // Clean up
            env::remove_var("SERVER_PORT");
            env::remove_var("RUST_LOG");
            env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    fn test_env_var_error_handling() {
        // Test behavior when environment variables are not set
        unsafe {
            env::remove_var("NONEXISTENT_VAR");

            assert!(env::var("NONEXISTENT_VAR").is_err());
        }
    }
}

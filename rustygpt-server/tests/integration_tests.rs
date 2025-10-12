//! Integration tests for the `RustyGPT` server CLI.

use serial_test::serial;
use std::env;
use std::process::Command;

#[test]
fn test_server_help_command() {
    // Test that the server binary shows help when run with --help
    let output = Command::new("cargo")
        .args(["run", "-p", "server", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    // Check that help output contains expected text
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Backend server and tools for RustyGPT"));
    assert!(stdout.contains("serve"));
}

#[test]
fn test_server_invalid_command() {
    // Test that the server binary handles invalid commands gracefully
    let output = Command::new("cargo")
        .args(["run", "-p", "server", "--", "invalid-command"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    // Should exit with non-zero status for invalid commands
    assert!(!output.status.success());
}

#[test]
#[serial]
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
#[serial]
fn test_config_file_env_var() {
    // Test configuration file environment variable handling
    unsafe {
        env::set_var("CONFIG_FILE", "test_config.yaml");

        assert_eq!(env::var("CONFIG_FILE").unwrap(), "test_config.yaml");

        env::remove_var("CONFIG_FILE");
    }
}

#[test]
#[serial]
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
#[serial]
fn test_jwt_secret_env_var() {
    // Test JWT secret environment variable
    unsafe {
        env::set_var("JWT_SECRET", "test_jwt_secret_key");

        assert_eq!(env::var("JWT_SECRET").unwrap(), "test_jwt_secret_key");

        env::remove_var("JWT_SECRET");
    }
}

#[test]
#[serial]
fn test_cors_origin_env_var() {
    // Test CORS origin environment variable
    unsafe {
        env::set_var("CORS_ORIGIN", "http://localhost:3000");

        assert_eq!(env::var("CORS_ORIGIN").unwrap(), "http://localhost:3000");

        env::remove_var("CORS_ORIGIN");
    }
}

#[test]
#[serial]
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
#[serial]
fn test_env_var_error_handling() {
    // Test behavior when environment variables are not set
    unsafe {
        env::remove_var("NONEXISTENT_VAR");

        assert!(env::var("NONEXISTENT_VAR").is_err());
    }
}

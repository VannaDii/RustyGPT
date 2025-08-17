//! Integration tests for the CLI chat command.

use assert_cmd::Command;
use predicates;

#[tokio::test]
async fn test_chat_command_help() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains(
            "Start an interactive chat session with the AI",
        ))
        .stdout(predicates::str::contains("--model"))
        .stdout(predicates::str::contains("--max-tokens"))
        .stdout(predicates::str::contains("--temperature"))
        .stdout(predicates::str::contains("--system"));
}

#[tokio::test]
async fn test_chat_command_with_nonexistent_model() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--model")
        .arg("/nonexistent/model.gguf")
        .timeout(std::time::Duration::from_secs(5))
        .write_stdin("");

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("Failed to load LLM model"));
}

#[tokio::test]
async fn test_chat_command_invalid_temperature() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--temperature")
        .arg("1.5")
        .timeout(std::time::Duration::from_secs(5));

    cmd.assert().failure().stderr(predicates::str::contains(
        "Temperature must be between 0.0 and 1.0",
    ));
}

#[tokio::test]
async fn test_chat_command_invalid_max_tokens() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--max-tokens")
        .arg("0")
        .timeout(std::time::Duration::from_secs(5));

    cmd.assert().failure().stderr(predicates::str::contains(
        "Max tokens must be greater than 0",
    ));
}

#[tokio::test]
async fn test_chat_command_with_mock_model() {
    // Test that the chat command properly handles model loading errors
    // (The mock implementation has realistic hardware validation)
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--system")
        .arg("You are a test assistant.")
        .timeout(std::time::Duration::from_secs(5))
        .write_stdin("exit\n");

    cmd.assert()
        .failure() // Expect failure due to missing model
        .stdout(predicates::str::contains("🚀 Starting RustyGPT Chat..."))
        .stderr(predicates::str::contains("Failed to load LLM model"));
}

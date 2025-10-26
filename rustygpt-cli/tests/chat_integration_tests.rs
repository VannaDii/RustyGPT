//! Integration tests for the CLI chat command.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

#[tokio::test]
async fn test_chat_command_help() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains(
            "Start an interactive chat session with the AI",
        ))
        .stdout(predicates::str::contains("--conversation"))
        .stdout(predicates::str::contains("--root"))
        .stdout(predicates::str::contains("--limit"))
        .stdout(predicates::str::contains("--server"));
}

#[tokio::test]
async fn test_chat_command_requires_conversation() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat").timeout(std::time::Duration::from_secs(5));

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains(
            "the following required arguments were not provided",
        ))
        .stderr(predicates::str::contains("--conversation <CONVERSATION>"));
}

#[tokio::test]
async fn test_chat_command_invalid_conversation_uuid() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--conversation")
        .arg("not-a-uuid")
        .timeout(std::time::Duration::from_secs(5));

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"))
        .stderr(predicates::str::contains("--conversation <CONVERSATION>"));
}

#[tokio::test]
async fn test_chat_command_connection_failure() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("chat")
        .arg("--conversation")
        .arg("00000000-0000-0000-0000-000000000001")
        .arg("--limit")
        .arg("5")
        .timeout(std::time::Duration::from_secs(10));

    cmd.assert().failure().stderr(
        predicates::str::contains("failed to fetch threads")
            .or(predicates::str::contains("no session cookie jar found")),
    );
}

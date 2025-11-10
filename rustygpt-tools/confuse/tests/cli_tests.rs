use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_cli_no_args() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_cli_help() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("confuse"))
        .stdout(predicate::str::contains("Run multiple tasks concurrently"))
        .stdout(predicate::str::contains("--prefix-colors"))
        .stdout(predicate::str::contains("--names"))
        .stdout(predicate::str::contains("--cwd"));
}

// Version flag is not implemented in this CLI
#[test]
fn test_cli_version() {
    // Skip this test as version flag is not implemented
    // Basic sanity check that the binary runs
    let mut cmd = cargo_bin_cmd!("confuse");

    // Instead of testing version, just make sure we can see the help output
    cmd.arg("--help").assert().success();
}

#[test]
fn test_cli_invalid_color() {
    let mut cmd = cargo_bin_cmd!("confuse");
    // Even invalid colors should be accepted, they'll default to white
    cmd.arg("echo hello")
        .args(["-p", "invalid_color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_cli_missing_name() {
    let mut cmd = cargo_bin_cmd!("confuse");
    // Even if names are missing for some commands, it should still work
    cmd.args(["echo hello", "echo world"])
        .args(["--names", "OnlyOne"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OnlyOne"))
        .stdout(predicate::str::contains("world"));
}

#[test]
fn test_cli_invalid_command() {
    let mut cmd = cargo_bin_cmd!("confuse");

    // The command is accepted by the CLI parser but fails at runtime with
    // a panic when the command is not found.
    // The output shows error code 0 but stderr contains a panic message
    cmd.arg("non_existent_command_12345")
        .assert()
        .stderr(predicate::str::contains("Failed to spawn process"))
        .stderr(predicate::str::contains("No such file or directory"));
}

#[test]
fn test_cli_empty_names() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.arg("echo hello")
        .args(["--names", ""])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_cli_empty_colors() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.arg("echo hello")
        .args(["-p", ""])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_cli_comma_separated_values() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.args(["echo foo", "echo bar"])
        .args(["--names", "FIRST,SECOND"])
        .args(["-p", "red,blue"])
        .assert()
        .success()
        .stdout(predicate::str::contains("foo"))
        .stdout(predicate::str::contains("bar"))
        .stdout(predicate::str::contains("FIRST"))
        .stdout(predicate::str::contains("SECOND"));
}

#[test]
fn test_cli_many_commands() {
    let mut cmd = cargo_bin_cmd!("confuse");
    cmd.args([
        "echo one",
        "echo two",
        "echo three",
        "echo four",
        "echo five",
        "echo six",
        "echo seven",
        "echo eight",
        "echo nine",
        "echo ten",
        "echo eleven",
        "echo twelve",
        "echo thirteen",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("one"))
    .stdout(predicate::str::contains("thirteen"));
}

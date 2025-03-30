use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

// Helper function to create a simple test script
fn create_test_script(dir: &Path, name: &str, content: &str) -> std::io::Result<String> {
    let file_path = dir.join(name);
    let mut file = File::create(&file_path)?;
    writeln!(file, "#!/bin/sh")?;
    writeln!(file, "{}", content)?;

    // Make the script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&file_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&file_path, perms)?;
    }

    Ok(file_path.to_string_lossy().to_string())
}

#[test]
fn test_execute_single_command() {
    let temp_dir = tempdir().unwrap();
    let script_path =
        create_test_script(temp_dir.path(), "echo_test.sh", "echo \"Hello, ConFuse!\"").unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(&script_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, ConFuse!"));
}

#[test]
fn test_multiple_commands() {
    let temp_dir = tempdir().unwrap();
    let script1 = create_test_script(
        temp_dir.path(),
        "count_to_3.sh",
        "for i in 1 2 3; do echo \"Counting: $i\"; sleep 0.1; done",
    )
    .unwrap();

    let script2 = create_test_script(
        temp_dir.path(),
        "letters.sh",
        "for letter in A B C; do echo \"Letter: $letter\"; sleep 0.1; done",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(&script1)
        .arg(&script2)
        .assert()
        .success()
        .stdout(predicate::str::contains("Counting: 1"))
        .stdout(predicate::str::contains("Counting: 2"))
        .stdout(predicate::str::contains("Counting: 3"))
        .stdout(predicate::str::contains("Letter: A"))
        .stdout(predicate::str::contains("Letter: B"))
        .stdout(predicate::str::contains("Letter: C"));
}

#[test]
fn test_with_custom_names() {
    let temp_dir = tempdir().unwrap();
    let script1 = create_test_script(
        temp_dir.path(),
        "script1.sh",
        "echo \"Output from script 1\"",
    )
    .unwrap();

    let script2 = create_test_script(
        temp_dir.path(),
        "script2.sh",
        "echo \"Output from script 2\"",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(&script1)
        .arg(&script2)
        .args(["--names", "FIRST,SECOND"])
        .assert()
        .success()
        .stdout(predicate::str::contains("["))
        .stdout(predicate::str::contains("FIRST"))
        .stdout(predicate::str::contains("SECOND"))
        .stdout(predicate::str::contains("Output from script 1"))
        .stdout(predicate::str::contains("Output from script 2"));
}

#[test]
fn test_with_custom_colors() {
    let temp_dir = tempdir().unwrap();
    let script =
        create_test_script(temp_dir.path(), "test_script.sh", "echo \"Testing colors\"").unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(&script)
        .args(["-p", "red"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Testing colors"));
}

#[test]
fn test_working_directory_option() {
    let temp_dir = tempdir().unwrap();
    let script = create_test_script(temp_dir.path(), "pwd_test.sh", "pwd").unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(&script)
        .args(["-d", temp_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(temp_dir.path().to_str().unwrap()));
}

#[test]
fn test_command_with_inline_name() {
    let temp_dir = tempdir().unwrap();
    let script =
        create_test_script(temp_dir.path(), "test.sh", "echo \"Testing inline name\"").unwrap();

    let cmd_str = format!("CustomName:{}", script);

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(cmd_str)
        .assert()
        .success()
        .stdout(predicate::str::contains("CustomName"))
        .stdout(predicate::str::contains("Testing inline name"));
}

#[test]
fn test_command_with_working_dir_in_string() {
    let temp_dir = tempdir().unwrap();
    let script = create_test_script(temp_dir.path(), "pwd_test.sh", "pwd").unwrap();

    // Get just the filename without the path
    let script_name = Path::new(&script).file_name().unwrap().to_str().unwrap();

    // Create command with inline working directory
    // Format: "TaskName@/path/to/dir:./script.sh"
    let cmd_str = format!(
        "TaskWithPath@{}:./{}",
        temp_dir.path().to_str().unwrap(),
        script_name
    );

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    cmd.arg(cmd_str)
        .assert()
        .success()
        .stdout(predicate::str::contains("TaskWithPath"))
        .stdout(predicate::str::contains(temp_dir.path().to_str().unwrap()));
}

#[test]
fn test_error_handling() {
    let mut cmd = Command::cargo_bin("confuse").unwrap();

    // The command succeeds initially because the CLI parser accepts it,
    // but it fails at runtime with a panic when trying to execute the non-existent command.
    cmd.arg("non_existent_command")
        .assert()
        .stderr(predicate::str::contains("Failed to spawn process"))
        .stderr(predicate::str::contains("No such file or directory"));
}

#[test]
fn test_duplicate_task_names() {
    let temp_dir = tempdir().unwrap();
    let script = create_test_script(temp_dir.path(), "echo.sh", "echo \"Hello\"").unwrap();

    let mut cmd = Command::cargo_bin("confuse").unwrap();
    // Use the same name for both tasks, should get disambiguated
    cmd.arg(&script)
        .arg(&script)
        .args(["--names", "SAME,SAME"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SAME"))
        .stdout(predicate::str::contains("SAME#1"));
}

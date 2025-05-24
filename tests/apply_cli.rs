use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::io::Write;
use serde_json::json;

/// Dry-run of `og apply`: starting from empty JSON, adding a new markdown task
#[test]
fn apply_dry_run_empty_json_add() {
    let mut cmd = Command::cargo_bin("og").unwrap();
    let json_file = NamedTempFile::new().unwrap();
    cmd.arg("apply")
        .arg("--from").arg("markdown")
        .arg("--target-json").arg(json_file.path())
        .arg("--dry-run")
        .write_stdin("- [ ] NewTask\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run summary:"))
        .stdout(predicate::str::contains("Added tasks:"))
        .stdout(predicate::str::contains("NewTask"));
}

/// Actual run of `og apply`: updates the JSON file and outputs the final markdown
#[test]
fn apply_update_json_file_and_output_markdown() {
    let mut cmd = Command::cargo_bin("og").unwrap();
    let mut json_file = NamedTempFile::new().unwrap();
    // Prepare existing JSON with a single task
    let existing = json!({
        "id": 1,
        "name": "OldTask",
        "status": "NONE",
        "notes": null,
        "created": "2024-01-01",
        "updated": null,
        "due": null,
        "completed": null,
        "tags": [],
        "subtasks": [],
        "priority": "N",
        "display_order": 1,
        "project": null,
        "contexts": null,
        "extra": null,
        "repeat": null
    });
    writeln!(json_file, "{}", existing.to_string()).unwrap();

    // Apply markdown change (rename task)
    cmd.arg("apply")
        .arg("--from").arg("markdown")
        .arg("--target-json").arg(json_file.path())
        .write_stdin("- [ ] NewName\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("NewName"));

    // JSON file should be updated
    let contents = std::fs::read_to_string(json_file.path()).unwrap();
    assert!(contents.contains("\"name\":\"NewName\""));
}
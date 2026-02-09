use std::fs;
use std::process::Command;

use conescope_git::repository::GitRepo;
use conescope_git::status::{FileStatus, StageStatus};
use tempfile::TempDir;

fn init_repo() -> (TempDir, GitRepo) {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    fs::write(dir.path().join("README.md"), "# test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let repo = GitRepo::open(dir.path()).unwrap();
    (dir, repo)
}

#[test]
fn status_untracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("new.txt"), "hello").unwrap();
    let entries = repo.status().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "new.txt");
    assert_eq!(entries[0].status, FileStatus::Untracked);
    assert_eq!(entries[0].stage, StageStatus::Unstaged);
}

#[test]
fn stage_and_unstage() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("new.txt"), "hello").unwrap();

    repo.stage(&["new.txt"]).unwrap();
    let entries = repo.status().unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e.path == "new.txt" && e.stage == StageStatus::Staged)
    );

    repo.unstage(&["new.txt"]).unwrap();
    let entries = repo.status().unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e.path == "new.txt" && e.stage == StageStatus::Unstaged)
    );
}

#[test]
fn diff_modified_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("README.md"), "# changed\nnew line").unwrap();
    let hunks = repo.diff_file("README.md", false).unwrap();
    assert!(!hunks.is_empty());
    assert!(
        hunks[0]
            .lines
            .iter()
            .any(|l| l.origin == conescope_git::diff::LineOrigin::Addition)
    );
}

#[test]
fn discard_tracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("README.md"), "modified").unwrap();
    assert_ne!(
        fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "# test"
    );

    repo.discard(&["README.md"]).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "# test"
    );
}

#[test]
fn discard_untracked_file() {
    let (dir, repo) = init_repo();
    fs::write(dir.path().join("junk.txt"), "delete me").unwrap();
    assert!(dir.path().join("junk.txt").exists());

    repo.discard(&["junk.txt"]).unwrap();
    assert!(!dir.path().join("junk.txt").exists());
}

#[test]
fn head_branch() {
    let (_dir, repo) = init_repo();
    let branch = repo.head_branch();
    assert!(branch.is_some());
}

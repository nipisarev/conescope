use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: Option<String>,
}

#[derive(Debug)]
pub struct GitCli {
    binary: PathBuf,
    work_dir: PathBuf,
}

impl GitCli {
    pub fn new(work_dir: &Path) -> Result<Self> {
        let binary = which::which("git").context("git binary not found in PATH")?;
        Ok(Self {
            binary,
            work_dir: work_dir.to_owned(),
        })
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.binary)
            .args(args)
            .current_dir(&self.work_dir)
            .output()
            .with_context(|| format!("failed to run git {}", args.join(" ")))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git {} failed: {}", args.join(" "), stderr.trim());
        }
    }

    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["add", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }

    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["reset", "HEAD", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }

    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        let mut args = vec!["checkout", "--"];
        args.extend(paths);
        self.run(&args)?;
        Ok(())
    }

    pub fn worktree_add(&self, path: &str, branch: &str) -> Result<()> {
        self.run(&["worktree", "add", "-b", branch, path, "HEAD"])?;
        Ok(())
    }

    pub fn worktree_remove(&self, path: &str, force: bool) -> Result<()> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(path);
        self.run(&args)?;
        Ok(())
    }

    pub fn worktree_list(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self.run(&["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        let mut current_path: Option<String> = None;
        let mut current_branch: Option<String> = None;

        for line in output.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if let Some(prev_path) = current_path.take() {
                    worktrees.push(WorktreeInfo {
                        path: prev_path,
                        branch: current_branch.take(),
                    });
                }
                current_path = Some(path.to_owned());
                current_branch = None;
            } else if let Some(branch_ref) = line.strip_prefix("branch ") {
                current_branch = branch_ref
                    .strip_prefix("refs/heads/")
                    .map(ToOwned::to_owned)
                    .or_else(|| Some(branch_ref.to_owned()));
            }
        }

        if let Some(path) = current_path {
            worktrees.push(WorktreeInfo {
                path,
                branch: current_branch,
            });
        }

        Ok(worktrees)
    }

    pub fn worktree_prune(&self) -> Result<()> {
        self.run(&["worktree", "prune"])?;
        Ok(())
    }
}

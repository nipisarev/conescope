use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

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
}

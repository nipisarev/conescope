use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{DiffFormat, DiffOptions, Repository, StatusOptions};

use crate::cli::GitCli;
use crate::diff::{DiffHunk, DiffLine, LineOrigin};
use crate::status::{FileStatus, GitFileEntry, StageStatus};

pub struct GitRepo {
    repo: Repository,
    cli: GitCli,
    work_dir: PathBuf,
}

impl std::fmt::Debug for GitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepo")
            .field("work_dir", &self.work_dir)
            .finish_non_exhaustive()
    }
}

impl GitRepo {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path)
            .with_context(|| format!("no git repo found at {}", path.display()))?;
        let work_dir = repo
            .workdir()
            .context("bare repositories not supported")?
            .to_owned();
        let cli = GitCli::new(&work_dir)?;
        Ok(Self {
            repo,
            cli,
            work_dir,
        })
    }

    #[must_use]
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    #[must_use]
    pub fn head_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|r| r.shorthand().map(str::to_owned))
    }

    pub fn status(&self) -> Result<Vec<GitFileEntry>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let Some(path) = entry.path() else {
                continue;
            };
            let s = entry.status();

            if s.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED,
            ) {
                let status = if s.contains(git2::Status::INDEX_NEW) {
                    FileStatus::Added
                } else if s.contains(git2::Status::INDEX_DELETED) {
                    FileStatus::Deleted
                } else if s.contains(git2::Status::INDEX_RENAMED) {
                    FileStatus::Renamed
                } else {
                    FileStatus::Modified
                };
                entries.push(GitFileEntry {
                    path: path.to_owned(),
                    status,
                    stage: StageStatus::Staged,
                });
            }

            if s.intersects(
                git2::Status::WT_MODIFIED | git2::Status::WT_DELETED | git2::Status::WT_NEW,
            ) {
                let status = if s.contains(git2::Status::WT_NEW) {
                    FileStatus::Untracked
                } else if s.contains(git2::Status::WT_DELETED) {
                    FileStatus::Deleted
                } else {
                    FileStatus::Modified
                };
                entries.push(GitFileEntry {
                    path: path.to_owned(),
                    status,
                    stage: StageStatus::Unstaged,
                });
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    pub fn diff_file(&self, path: &str, staged: bool) -> Result<Vec<DiffHunk>> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        let diff = if staged {
            let head_tree = self.repo.head()?.peel_to_tree()?;
            self.repo
                .diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        } else {
            self.repo.diff_index_to_workdir(None, Some(&mut opts))?
        };

        let mut hunks: Vec<DiffHunk> = Vec::new();

        diff.print(DiffFormat::Patch, |_delta, hunk, line| {
            let origin = match line.origin() {
                '+' => LineOrigin::Addition,
                '-' => LineOrigin::Deletion,
                ' ' => LineOrigin::Context,
                _ => {
                    // File header or hunk header line — ensure hunk exists
                    if let Some(hunk_header) = hunk {
                        let header = String::from_utf8_lossy(hunk_header.header())
                            .trim()
                            .to_owned();
                        if hunks.last().is_none_or(|h| h.header != header) {
                            hunks.push(DiffHunk {
                                header,
                                lines: Vec::new(),
                            });
                        }
                    }
                    return true;
                }
            };

            // Ensure a hunk exists for content lines
            if let Some(hunk_header) = hunk {
                let header = String::from_utf8_lossy(hunk_header.header())
                    .trim()
                    .to_owned();
                if hunks.last().is_none_or(|h| h.header != header) {
                    hunks.push(DiffHunk {
                        header,
                        lines: Vec::new(),
                    });
                }
            }

            let diff_line = DiffLine {
                origin,
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
                content: String::from_utf8_lossy(line.content()).to_string(),
            };

            if let Some(last_hunk) = hunks.last_mut() {
                last_hunk.lines.push(diff_line);
            }

            true
        })?;

        Ok(hunks)
    }

    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        self.cli.stage(paths)
    }

    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        self.cli.unstage(paths)
    }

    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        let statuses = self.status()?;
        let untracked: std::collections::HashSet<&str> = statuses
            .iter()
            .filter(|e| e.status == FileStatus::Untracked)
            .map(|e| e.path.as_str())
            .collect();

        let tracked: Vec<&str> = paths
            .iter()
            .copied()
            .filter(|p| !untracked.contains(p))
            .collect();
        let to_delete: Vec<&str> = paths
            .iter()
            .copied()
            .filter(|p| untracked.contains(p))
            .collect();

        if !tracked.is_empty() {
            self.cli.discard(&tracked)?;
        }
        for path in to_delete {
            let full = self.work_dir.join(path);
            let _ = std::fs::remove_file(full);
        }
        Ok(())
    }
}

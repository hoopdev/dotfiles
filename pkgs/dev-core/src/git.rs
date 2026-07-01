//! git2-based git operations for dev task harvest and diff.
//! Feature-gated: only compiled when `features = ["git"]`.

use std::path::{Path, PathBuf};

#[cfg(feature = "config")]
use serde_json::{json, Value};

/// Open a git repository, searching parent dirs if needed.
pub fn repo_open(path: &Path) -> Result<git2::Repository, git2::Error> {
    git2::Repository::discover(path)
}

/// Result of a diff operation.
pub struct DiffResult {
    /// Paths of changed files (relative to repo root).
    pub files: Vec<String>,
    /// Total insertions across all changed files.
    pub insertions: usize,
    /// Total deletions across all changed files.
    pub deletions: usize,
}

/// Diff HEAD..workdir for a repo at `repo_path`.
/// Returns the list of changed files and stats.
pub fn diff_head_to_workdir(repo_path: &Path) -> Result<DiffResult, git2::Error> {
    let repo = repo_open(repo_path)?;
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts.include_untracked(true);

    let diff = if let Ok(head) = repo.head() {
        if let Ok(tree) = head.peel_to_tree() {
            repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut diff_opts))?
        } else {
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
        }
    } else {
        repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
    };

    let stats = diff.stats()?;
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(p) = delta.new_file().path() {
                files.push(p.to_string_lossy().into_owned());
            } else if let Some(p) = delta.old_file().path() {
                files.push(p.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    )?;
    files.sort();
    files.dedup();

    Ok(DiffResult {
        files,
        insertions: stats.insertions(),
        deletions: stats.deletions(),
    })
}

/// Diff between two branches/refs (e.g., "main" and "task/feat-x").
/// Returns changed files and stats.
pub fn diff_refs(repo_path: &Path, base: &str, head: &str) -> Result<DiffResult, git2::Error> {
    let repo = repo_open(repo_path)?;
    let base_obj = repo.revparse_single(base)?;
    let head_obj = repo.revparse_single(head)?;
    let base_commit = base_obj.peel_to_commit()?;
    let head_commit = head_obj.peel_to_commit()?;
    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    let stats = diff.stats()?;
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(p) = delta.new_file().path() {
                files.push(p.to_string_lossy().into_owned());
            }
            true
        },
        None,
        None,
        None,
    )?;
    files.sort();
    files.dedup();

    Ok(DiffResult {
        files,
        insertions: stats.insertions(),
        deletions: stats.deletions(),
    })
}

/// Worktree info.
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
}

/// List all worktrees of a repository.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>, git2::Error> {
    let repo = repo_open(repo_path)?;
    let names = repo.worktrees()?;
    let mut result = Vec::new();
    for name in names.iter().flatten() {
        if let Ok(wt) = repo.find_worktree(name) {
            let path = wt.path().to_path_buf();
            // Try to find the branch by reading HEAD in worktree path
            let branch = read_worktree_branch(&path);
            result.push(WorktreeInfo {
                name: name.to_string(),
                path,
                branch,
            });
        }
    }
    Ok(result)
}

fn read_worktree_branch(wt_path: &Path) -> Option<String> {
    // Worktree HEAD file contains "ref: refs/heads/<branch>" or a sha
    let gitdir = wt_path.join(".git");
    let head_path = if gitdir.is_file() {
        // Worktrees use a .git file pointing to the real .git/worktrees/<name>/
        let content = std::fs::read_to_string(&gitdir).ok()?;
        let gitdir_path = content.strip_prefix("gitdir: ")?.trim();
        PathBuf::from(gitdir_path).join("HEAD")
    } else {
        gitdir.join("HEAD")
    };
    let content = std::fs::read_to_string(&head_path).ok()?;
    let content = content.trim();
    content
        .strip_prefix("ref: refs/heads/")
        .map(|branch| branch.to_string())
}

/// Get the current branch name at `repo_path`.
pub fn current_branch(repo_path: &Path) -> Result<Option<String>, git2::Error> {
    let repo = repo_open(repo_path)?;
    if repo.head_detached().unwrap_or(true) {
        return Ok(None);
    }
    let head = repo.head()?;
    Ok(head.shorthand().map(|s| s.to_string()))
}

/// Get the short SHA of HEAD.
pub fn head_sha(repo_path: &Path) -> Result<String, git2::Error> {
    let repo = repo_open(repo_path)?;
    let head = repo.head()?;
    let oid = head.peel_to_commit()?.id();
    Ok(format!("{:.8}", oid))
}

// ── status (branch / head oneline / dirty count) ────────────────────────────

/// A one-glance git summary — the bash `dev git status` fields.
pub struct GitStatus {
    pub branch: String,
    /// `<short-sha> <summary>` (git's `log --oneline -1`).
    pub head: String,
    /// Number of `git status --short` entries.
    pub changes: i64,
}

fn head_oneline(repo: &git2::Repository) -> Option<String> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let short = commit.as_object().short_id().ok()?;
    let short = short.as_str().unwrap_or("").to_string();
    let summary = commit.summary().unwrap_or("");
    Some(format!("{short} {summary}").trim_end().to_string())
}

fn count_changes(repo: &git2::Repository) -> Result<i64, git2::Error> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(false);
    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses
        .iter()
        .filter(|e| e.status() != git2::Status::CURRENT)
        .count() as i64)
}

/// Local git summary via git2 (branch, head oneline, dirty count).
pub fn status_local(repo_path: &Path) -> Result<GitStatus, git2::Error> {
    let repo = repo_open(repo_path)?;
    let branch = if repo.head_detached().unwrap_or(false) {
        String::new()
    } else {
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .unwrap_or_default()
    };
    Ok(GitStatus {
        branch,
        head: head_oneline(&repo).unwrap_or_default(),
        changes: count_changes(&repo).unwrap_or(0),
    })
}

/// Remote git summary over SSH (bash/zsh remotes; pwsh/nu handled by the
/// windows module). Mirrors the bash `_dev_remote_git_summary` unix path.
#[cfg(feature = "config")]
pub fn status_remote(env: &crate::config::Env, rp: &str) -> Option<GitStatus> {
    #[cfg(feature = "windows")]
    if env.is_windows() {
        return crate::windows::git_summary(env, rp).map(|(branch, head, changes)| GitStatus {
            branch,
            head,
            changes,
        });
    }
    let script = "echo \"B:$(git branch --show-current 2>/dev/null)\"; \
                  echo \"H:$(git log --oneline -1 2>/dev/null)\"; \
                  echo \"C:$(git status --short 2>/dev/null | wc -l | tr -d ' ')\"";
    let out = crate::ssh::exec_stdout(env, rp, script)?;
    let mut st = GitStatus {
        branch: String::new(),
        head: String::new(),
        changes: 0,
    };
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("B:") {
            st.branch = b.to_string();
        } else if let Some(h) = line.strip_prefix("H:") {
            st.head = h.to_string();
        } else if let Some(c) = line.strip_prefix("C:") {
            st.changes = c.trim().parse().unwrap_or(0);
        }
    }
    Some(st)
}

/// `{target,kind,ok,branch,head,changes}` for one target — the
/// `dev git status --json` row schema (kind is `local`|`remote`|`env`).
#[cfg(feature = "config")]
pub fn status_for_target(cfg: &crate::config::Config, name: &str) -> Value {
    use crate::config::Target;
    match cfg.resolve(name) {
        Some(Target::Local { path, .. }) => match status_local(Path::new(&path)) {
            Ok(st) => json!({"target": name, "kind": "local", "ok": true,
                             "branch": st.branch, "head": st.head, "changes": st.changes}),
            Err(_) => json!({"target": name, "kind": "local", "ok": true,
                             "branch": Value::Null, "head": Value::Null, "changes": Value::Null}),
        },
        Some(Target::Remote { env, path, .. }) => {
            match cfg.env(&env).and_then(|e| status_remote(e, &path)) {
                Some(st) => json!({"target": name, "kind": "remote", "ok": true,
                                   "branch": st.branch, "head": st.head, "changes": st.changes}),
                None => json!({"target": name, "kind": "remote", "ok": false,
                               "branch": Value::Null, "head": Value::Null, "changes": Value::Null}),
            }
        }
        _ => json!({"target": name, "kind": "env", "ok": false,
                    "branch": Value::Null, "head": Value::Null, "changes": Value::Null}),
    }
}

/// Parallel git status across many targets, preserving input order.
#[cfg(feature = "config")]
pub fn status_all(cfg: &crate::config::Config, names: &[String]) -> Vec<Value> {
    std::thread::scope(|s| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| s.spawn(move || status_for_target(cfg, name)))
            .collect();
        // A panicking worker (git2 / json) must not take down the whole fleet
        // status — degrade to an error row, preserving input order.
        handles
            .into_iter()
            .zip(names)
            .map(|(h, name)| {
                h.join().unwrap_or_else(|_| {
                    json!({"target": name, "kind": "unknown", "ok": false, "error": "panicked",
                           "branch": Value::Null, "head": Value::Null, "changes": Value::Null})
                })
            })
            .collect()
    })
}

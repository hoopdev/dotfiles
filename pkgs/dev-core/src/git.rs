//! git2-based git operations for dev task harvest and diff.
//! Feature-gated: only compiled when `features = ["git"]`.

use std::path::{Path, PathBuf};

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
        None, None, None,
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
pub fn diff_refs(
    repo_path: &Path,
    base: &str,
    head: &str,
) -> Result<DiffResult, git2::Error> {
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
        None, None, None,
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
    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        Some(branch.to_string())
    } else {
        None
    }
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

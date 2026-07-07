---
name: update-docs
description: Synchronize project documentation (README, guides, API pages, notebooks) with verified code behavior.
argument-hint: "[scope: all | docs | readme | notebooks | api]"
---

# Update Documentation

Synchronize project documentation with the current codebase state. `$ARGUMENTS` selects the scope (default: `all`). Scope-to-target mappings and the doc-file inventory are project-specific — see the Project Notes section if present.

## Context

- Current branch: !`git branch --show-current`
- Recent changes: !`git diff HEAD~5 --stat`

## Workflow

1. **Identify what changed.** Inspect the current diff and affected source, and cross-reference with the documentation that might be affected. Do not assume the last five commits define the user's task.
2. **Read each affected document** before editing it, to understand its current state.
3. **Verify code references.** Check import paths, class names, function signatures, CLI commands, config keys, defaults, units, and outputs directly against source and tests. Do not fabricate API details.
4. **Update examples.** Code snippets must be runnable against the current API and use actual import paths.
5. **Preserve style and content.** Match each document's tone, formatting, and heading structure; keep intentionally retained historical or benchmark context. Update rather than rewrite.
6. **Cross-check consistency.** The README must match actual capabilities, and doc references (e.g. in `CLAUDE.md`) must point to existing files.
7. **Report changes.** Summarize what was updated, verification performed, and anything skipped with the reason.

## Multi-Language Documentation

When docs are maintained in multiple languages (e.g. `docs/en/` and `docs/jp/`):

- File names and directory structure must be identical across languages.
- Keep equations, units, sign conventions, parameter names, technical terms, code blocks, links, and file paths identical across languages; translate only prose.
- Preserve heading structure so each translation stays a faithful mirror of its source page. Add the mirror when a source page is added; remove it when the source page is deleted.

## Notebooks

When the project has tutorial or example notebooks:

- Use structured notebook editing. Preserve unrelated outputs and metadata.
- Execute behavior-changing notebooks when code behavior changed; report any notebooks not executed with the reason.

## Markdown Compatibility

Documents should remain readable both on GitHub and on the project's docs site (if it has one):

- Use relative links and fenced code blocks with language tags.
- Keep GitHub alerts as a contiguous blockquote with `[!NOTE]`, `[!TIP]`,
  `[!IMPORTANT]`, `[!WARNING]`, or `[!CAUTION]` alone on the first line.
- Do not wrap a Markdown document in a raw HTML `<div>`.
- Use `$...$` for inline math and `$$...$$` or a `math` fence for display math.
- Use `aligned`, not `align`, for multi-line equations rendered on GitHub.
- Avoid site-generator-only syntax (e.g. MkDocs `!!!` admonitions) when the same files are read on GitHub.
- Keep Mermaid blocks, tables, footnotes, and existing HTML only where all rendering targets handle them acceptably.

## Guidelines

- Keep docs concise — match the existing terse style.
- No emoji unless the document already uses them.

## Verification

- Build the docs site if the project has one.
- Check every edited relative link and any changed anchor.
- Search for removed names, old paths, and stale commands.

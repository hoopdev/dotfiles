//! Task lifecycle service — the pure decision logic behind `dev task`.
//!
//! Phase 1 of `docs/dev-cli-tui-refactor.md` pulls the task lifecycle rules out
//! of the `dev-cli` command layer so they live in `dev-core` and can be unit
//! tested without a terminal or an on-disk task store. `dev task …` stays a thin
//! adapter: it parses args, calls these helpers plus the store/agent/git APIs,
//! and formats human/JSON output.
//!
//! Gated on `config` to mirror [`crate::agent`] — the CLI enables that feature
//! and is the only consumer.

use serde_json::Value;

// ── review recommendation parsing ─────────────────────────────────────────────

/// Recommendation parsed from a review agent's output. Precedence matters: a
/// failed run wins, then `needs_fix`, then `reject`, then a *positive* mergeable
/// signal (guarded so "not mergeable" / "not yet mergeable" never read as
/// mergeable), else `unknown`.
pub fn review_recommendation(output: &str, ok: bool) -> &'static str {
    if !ok {
        return "failed";
    }

    let lower = output.to_lowercase();
    if lower.contains("needs_fix") || lower.contains("needs fix") || lower.contains("needs-fix") {
        return "needs_fix";
    }
    if lower.contains("reject") || lower.contains("rejected") {
        return "reject";
    }

    let mergeable_negated = [
        "not mergeable",
        "isn't mergeable",
        "is not mergeable",
        "not yet mergeable",
        "not ready to merge",
        "should not merge",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if !mergeable_negated
        && (lower.contains("recommendation: mergeable")
            || lower.contains("recommendation\": \"mergeable")
            || lower.contains("→ mergeable")
            || lower.contains("no findings")
            || lower.contains("no issues found"))
    {
        return "mergeable";
    }

    "unknown"
}

// ── dispatch / plan / fix prompt builders ─────────────────────────────────────

/// Prompt for the planning agent (`dev task plan`): read the shared context,
/// raise blocking questions when ambiguous, otherwise write the plan.
pub fn plan_prompt(task_id: &str, project_id: &str) -> String {
    format!(
        "You are planning dev task {task_id} for project {project_id}.\n\n\
         Rules:\n\
         - Do not edit files.\n\
         - Read the shared task context: dev task context {task_id} --markdown\n\
         - If behavior, scope, compatibility, API, UX, migration, release, or validation is ambiguous, run:\n\
             dev task ask {task_id} \"<question>\" --category <category> --severity blocking\n\
           and stop.\n\
         - If there are no blocking questions, write the plan:\n\
             dev task write-plan {task_id}\n\
         - The plan must include: understanding, proposed behavior, files to touch, \
         files not to touch, implementation steps, validation, risks, rollback.\n\
         - Do not implement until the task is approved."
    )
}

/// Prompt for the implementation agent (`dev task dispatch`): implement only the
/// approved plan, raise blocking questions if it is insufficient, write a handoff.
pub fn dispatch_prompt(task_id: &str, project_id: &str) -> String {
    format!(
        "You are implementing approved dev task {task_id} for project {project_id}.\n\n\
         Rules:\n\
         - Read `dev task context {task_id} --markdown`.\n\
         - Implement only the approved plan.\n\
         - Do not broaden scope.\n\
         - If the approved plan is insufficient, run:\n\
             dev task ask {task_id} \"<question>\" --category <category> --severity blocking\n\
           and stop.\n\
         - Run the declared validation commands when feasible.\n\
         - At the end, write a handoff:\n\
             dev task write-handoff {task_id}\n\
           Include: changed files, tests run, results, risks, follow-up."
    )
}

/// Prompt for the fix agent (`dev task fix`): fix only the reported issues from
/// the last review, then write a new handoff.
pub fn fix_prompt(task_id: &str, project_id: &str) -> String {
    format!(
        "You are fixing dev task {task_id} (phase: needs_fix) for project {project_id}.\n\n\
         Rules:\n\
         - Read `dev task context {task_id} --markdown` for the approved plan.\n\
         - Read `dev task handoff {task_id} --markdown` for the last handoff and review feedback.\n\
         - Fix only the reported issues. Do not change unrelated code.\n\
         - Run declared validation commands.\n\
         - At the end, write a new handoff: dev task write-handoff {task_id}"
    )
}

// ── phase transitions ─────────────────────────────────────────────────────────

/// Phase after a plan is written: parked in `needs_spec` while blocking
/// questions remain, else `planned`.
pub fn phase_after_plan(blocking_open: usize) -> &'static str {
    if blocking_open > 0 {
        "needs_spec"
    } else {
        "planned"
    }
}

/// Phase after a handoff is written: parked in `needs_spec` while blocking
/// questions remain, else `review`.
pub fn phase_after_handoff(blocking_open: usize) -> &'static str {
    if blocking_open > 0 {
        "needs_spec"
    } else {
        "review"
    }
}

/// Phase after answering a question: when the last blocking question is resolved
/// and the task was parked in `needs_spec`, it returns to `planning`. Otherwise
/// the phase is unchanged (`None`).
pub fn phase_after_answer(remaining_blocking: usize, current_phase: &str) -> Option<&'static str> {
    if remaining_blocking == 0 && current_phase == "needs_spec" {
        Some("planning")
    } else {
        None
    }
}

/// Phase after a review, given the parsed recommendation and the current phase.
/// Only `reject` / `needs_fix` / `mergeable` move the task; anything else keeps
/// it where it is.
pub fn phase_after_review<'a>(recommendation: &str, current_phase: &'a str) -> &'a str {
    match recommendation {
        "reject" => "rejected",
        "needs_fix" => "needs_fix",
        "mergeable" => "mergeable",
        _ => current_phase,
    }
}

/// Aggregate validation status from pass/fail counts: `passed` when nothing
/// failed, `failed` when nothing passed, else `partial`.
pub fn test_status(passed: usize, failed: usize) -> &'static str {
    if failed == 0 {
        "passed"
    } else if passed == 0 {
        "failed"
    } else {
        "partial"
    }
}

// ── task summary update ───────────────────────────────────────────────────────

/// Set `task["summary"][key] = value`, creating `summary` if it is absent or not
/// an object. Pure transform on the parsed `task.json`; the caller owns the
/// read/write of the file.
pub fn set_summary_field(task: &mut Value, key: &str, value: Value) {
    if !task.get("summary").is_some_and(|s| s.is_object()) {
        task["summary"] = serde_json::json!({});
    }
    task["summary"][key] = value;
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_recommendation_negation_precedence() {
        // Positive mergeable signals.
        assert_eq!(
            review_recommendation("Recommendation: mergeable", true),
            "mergeable"
        );
        assert_eq!(review_recommendation("No findings.", true), "mergeable");
        assert_eq!(review_recommendation("no issues found", true), "mergeable");

        // Negated mergeable must NOT read as mergeable.
        assert_eq!(
            review_recommendation("recommendation: mergeable? no — not mergeable", true),
            "unknown"
        );
        assert_eq!(
            review_recommendation("this is not yet mergeable", true),
            "unknown"
        );

        // needs_fix / reject take precedence, even over a mergeable mention.
        assert_eq!(
            review_recommendation("mergeable once needs_fix items are addressed", true),
            "needs_fix"
        );
        assert_eq!(review_recommendation("I reject this change", true), "reject");

        // A failed run is always "failed", regardless of text.
        assert_eq!(
            review_recommendation("recommendation: mergeable", false),
            "failed"
        );

        // Neutral text is unknown.
        assert_eq!(review_recommendation("looks interesting", true), "unknown");
    }

    #[test]
    fn phase_transitions() {
        // draft → planning → needs_spec → planned → approved → implementing →
        // review → mergeable: the helpers cover the branching points.
        assert_eq!(phase_after_plan(0), "planned");
        assert_eq!(phase_after_plan(2), "needs_spec");

        assert_eq!(phase_after_handoff(0), "review");
        assert_eq!(phase_after_handoff(1), "needs_spec");

        assert_eq!(phase_after_answer(0, "needs_spec"), Some("planning"));
        assert_eq!(phase_after_answer(1, "needs_spec"), None);
        assert_eq!(phase_after_answer(0, "planning"), None);

        assert_eq!(phase_after_review("reject", "review"), "rejected");
        assert_eq!(phase_after_review("needs_fix", "review"), "needs_fix");
        assert_eq!(phase_after_review("mergeable", "review"), "mergeable");
        assert_eq!(phase_after_review("unknown", "review"), "review");
    }

    #[test]
    fn test_status_aggregation() {
        assert_eq!(test_status(3, 0), "passed");
        assert_eq!(test_status(0, 2), "failed");
        assert_eq!(test_status(2, 1), "partial");
    }

    #[test]
    fn set_summary_field_creates_and_preserves() {
        let mut v = serde_json::json!({ "id": "T-1" });
        set_summary_field(&mut v, "test_status", serde_json::json!("passed"));
        assert_eq!(v["summary"]["test_status"], serde_json::json!("passed"));
        // A second field is added without dropping the first.
        set_summary_field(&mut v, "review_status", serde_json::json!("mergeable"));
        assert_eq!(v["summary"]["test_status"], serde_json::json!("passed"));
        assert_eq!(v["summary"]["review_status"], serde_json::json!("mergeable"));
    }

    /// Review/test artifact ids are allocated in the *task-local* directory
    /// (`reviews/` and `test-results/` under the task dir), not a project root.
    #[test]
    fn artifact_ids_are_task_local() {
        let base = std::env::temp_dir().join(format!("dev-ts-artifact-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let reviews = base.join("reviews");
        let results = base.join("test-results");

        let rid = crate::store::next_review_id_in(&reviews).unwrap();
        let vid = crate::store::next_test_run_id_in(&results).unwrap();
        assert!(rid.starts_with("R-"), "review id: {rid}");
        assert!(vid.starts_with("V-"), "test id: {vid}");
        // The helpers created the dirs we asked for (task-local), nothing else.
        assert!(reviews.is_dir());
        assert!(results.is_dir());

        // Dropping an artifact with that id and asking again increments within
        // the same directory.
        std::fs::write(reviews.join(format!("{rid}.md")), "x").unwrap();
        let rid2 = crate::store::next_review_id_in(&reviews).unwrap();
        assert_ne!(rid, rid2);

        let _ = std::fs::remove_dir_all(&base);
    }
}

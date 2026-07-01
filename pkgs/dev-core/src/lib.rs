#[cfg(feature = "config")]
pub mod agent;
#[cfg(feature = "config")]
pub mod config;
#[cfg(feature = "git")]
pub mod git;
#[cfg(feature = "config")]
pub mod notify;
#[cfg(feature = "config")]
pub mod ssh;
pub mod statusline;
pub mod store;
pub mod task;
#[cfg(feature = "windows")]
pub mod windows;

pub use task::{
    dev_store_path, load_board_snapshot, load_dev_tasks, load_task_detail, BoardSnapshot,
    DevQuestion, DevTask, QuestionOption, TaskDetail,
};

pub use store::{
    blocking_questions_open, event_append, find_project_dir_for_question,
    find_project_dir_for_task, find_task_dir, handoff_write, next_question_id, next_review_id,
    next_review_id_in, next_task_id, next_test_run_id, next_test_run_id_in, now_iso, plan_approve,
    plan_write, question_answer, question_new, task_new, task_phase_set, task_update_field,
};

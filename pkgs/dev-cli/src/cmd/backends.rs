//! `dev backends` — the agent backend registry (from dev-core::agent).

use dev_core::agent;

pub fn backends(json_out: bool) {
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&agent::registry_json()).unwrap()
        );
        return;
    }
    println!(
        "{:<10} {:<12} {:<7} {:<12} ATTACH",
        "NAME", "DISPATCH", "REVIEW", "PS_DETECT"
    );
    for b in agent::BACKENDS {
        println!(
            "{:<10} {:<12} {:<7} {:<12} {:?}",
            b.name, b.dispatchable, b.review, b.ps_detect, b.attach
        );
    }
}

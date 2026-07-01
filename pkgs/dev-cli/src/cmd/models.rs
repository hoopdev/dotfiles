//! `dev models [backend]` — the models each backend offers, from the single
//! dev-core registry (the picker and dispatch validation read the same list).

use dev_core::agent;

pub fn models(backend: Option<String>, json_out: bool) {
    let name = backend.as_deref().unwrap_or("claude");
    let Some(spec) = agent::backend(name) else {
        eprintln!("dev models: unknown backend '{name}'");
        std::process::exit(1);
    };
    if json_out {
        let arr: Vec<_> = spec
            .models
            .iter()
            .map(|m| serde_json::json!({ "label": m.label, "id": m.id }))
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap());
        return;
    }
    for m in spec.models {
        if m.id.is_empty() || m.label == m.id {
            println!("{}", m.label);
        } else {
            println!("{:<28} {}", m.label, m.id);
        }
    }
}

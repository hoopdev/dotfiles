//! `dev notify` — send a Telegram message (dev-core::notify).

pub fn notify(msg: Vec<String>) {
    let m = msg.join(" ");
    if m.trim().is_empty() {
        eprintln!("Usage: dev notify <message...>");
        std::process::exit(1);
    }
    if !dev_core::notify::send(&m) {
        eprintln!(
            "dev notify: not sent (TELEGRAM_BOT_TOKEN/TELEGRAM_CHAT_ID unset or request failed)"
        );
        std::process::exit(1);
    }
}

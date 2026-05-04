use std::sync::OnceLock;

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var("LLNZY_TRACE_EXTERNAL_INPUT")
            .map(|value| !matches!(value.as_str(), "" | "0" | "false" | "FALSE" | "off" | "OFF"))
            .unwrap_or(false)
    })
}

pub fn trace(source: &str, detail: impl FnOnce() -> String) {
    if enabled() {
        eprintln!("[llnzy external-input] {source}: {}", detail());
    }
}

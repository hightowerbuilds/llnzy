use std::sync::OnceLock;

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var("LLNZY_TRACE_EXTERNAL_INPUT")
            .map(|value| flag_value_enabled(&value))
            .unwrap_or(false)
    })
}

pub fn trace(source: &str, detail: impl FnOnce() -> String) {
    if enabled() {
        eprintln!("[llnzy external-input] {source}: {}", detail());
    }
}

fn flag_value_enabled(value: &str) -> bool {
    !matches!(value, "" | "0" | "false" | "FALSE" | "off" | "OFF")
}

#[cfg(test)]
mod tests {
    use super::flag_value_enabled;

    #[test]
    fn trace_flag_accepts_common_disabled_values() {
        for value in ["", "0", "false", "FALSE", "off", "OFF"] {
            assert!(!flag_value_enabled(value));
        }
    }

    #[test]
    fn trace_flag_treats_other_values_as_enabled() {
        for value in ["1", "true", "TRUE", "on", "debug"] {
            assert!(flag_value_enabled(value));
        }
    }
}

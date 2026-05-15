use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::error_log::ErrorLog;

pub const DIAGNOSTICS_REPORT_FILENAME: &str = "diagnostics-report.txt";

pub fn diagnostics_dir() -> PathBuf {
    crate::platform::paths::development_paths().logs_dir
}

pub fn diagnostics_path(filename: impl AsRef<Path>) -> PathBuf {
    diagnostics_dir().join(filename)
}

pub fn write_diagnostic(
    filename: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    let path = diagnostics_path(filename);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

pub fn export_diagnostics_report(log: Option<&ErrorLog>) -> std::io::Result<PathBuf> {
    let path = diagnostics_path(DIAGNOSTICS_REPORT_FILENAME);
    write_diagnostics_report_to(&path, log)?;
    Ok(path)
}

pub fn write_diagnostics_report_to(
    path: impl AsRef<Path>,
    log: Option<&ErrorLog>,
) -> std::io::Result<()> {
    let path = path.as_ref();
    let report = render_diagnostics_report(log);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, report)
}

pub fn render_diagnostics_report(log: Option<&ErrorLog>) -> String {
    let paths = crate::platform::paths::development_paths();
    let mut report = String::new();

    let _ = writeln!(report, "LLNZY Diagnostics Report");
    let _ = writeln!(report, "version: {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(report, "platform: {:?}", crate::platform::current_family());
    let _ = writeln!(report, "config_dir: {}", paths.config_dir.display());
    let _ = writeln!(report, "data_dir: {}", paths.data_dir.display());
    let _ = writeln!(report, "logs_dir: {}", paths.logs_dir.display());
    let _ = writeln!(
        report,
        "crash_reports_dir: {}",
        paths.crash_reports_dir.display()
    );

    match log {
        Some(log) => {
            let (errors, warnings) = log.counts();
            let _ = writeln!(report, "runtime_errors: {errors}");
            let _ = writeln!(report, "runtime_warnings: {warnings}");
            let _ = writeln!(report);
            let _ = writeln!(report, "Recent Runtime Log");
            for entry in log.recent(50) {
                let location = entry
                    .module
                    .as_deref()
                    .map(|m| format!(" {m}"))
                    .unwrap_or_default();
                let source_hint = match (entry.file.as_deref(), entry.line) {
                    (Some(file), Some(line)) => format!(" ({file}:{line})"),
                    (Some(file), None) => format!(" ({file})"),
                    _ => String::new(),
                };
                let _ = writeln!(
                    report,
                    "{} [{}]{}{} {}",
                    entry.timestamp_label(),
                    entry.level.label(),
                    location,
                    source_hint,
                    entry.message
                );
            }
        }
        None => {
            let _ = writeln!(report, "runtime_log: unavailable");
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_path_uses_logs_directory() {
        let path = diagnostics_path("crash.log");
        assert!(path.ends_with(Path::new("llnzy").join("logs").join("crash.log")));
    }

    #[test]
    fn diagnostics_report_includes_context_and_recent_log_entries() {
        let log = ErrorLog::new();
        log.warn("LSP server unavailable: rust-analyzer");
        log.error("Terminal session restart failed");

        let report = render_diagnostics_report(Some(&log));

        assert!(report.contains("LLNZY Diagnostics Report"));
        assert!(report.contains(concat!("version: ", env!("CARGO_PKG_VERSION"))));
        assert!(report.contains("config_dir:"));
        assert!(report.contains("runtime_errors: 1"));
        assert!(report.contains("runtime_warnings: 1"));
        assert!(report.contains("[WARN] LSP server unavailable: rust-analyzer"));
        assert!(report.contains("[ERR ] Terminal session restart failed"));
    }

    #[test]
    fn diagnostics_report_handles_missing_runtime_log() {
        let report = render_diagnostics_report(None);

        assert!(report.contains("runtime_log: unavailable"));
    }

    #[test]
    fn diagnostics_report_writer_creates_parent_directory() {
        let root =
            std::env::temp_dir().join(format!("llnzy-diagnostics-report-{}", std::process::id()));
        let path = root.join("nested").join(DIAGNOSTICS_REPORT_FILENAME);

        write_diagnostics_report_to(&path, None).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("LLNZY Diagnostics Report"));

        let _ = std::fs::remove_dir_all(root);
    }
}

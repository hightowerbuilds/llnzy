# LLNZY Operations And Diagnostics

LLNZY is local-first, so operational quality is mostly about preserving user
work, producing useful local diagnostics, and keeping release checks repeatable.

## Diagnostics Locations

Runtime diagnostics use the platform path set in `src/platform/paths.rs`.

On macOS, the active app paths live under the LLNZY config/data directories:

- logs: `~/Library/Application Support/llnzy/logs`
- crash reports: `~/Library/Application Support/llnzy/crash-reports`
- config: `~/Library/Application Support/llnzy/config.toml`

The app writes `crash.log` through `src/diagnostics.rs` when the top-level panic
hook receives an unrecoverable failure. The in-app diagnostics panel shows
recent runtime warnings and errors.

## Diagnostics Report

`src/diagnostics.rs` now provides a backend report export:

- `render_diagnostics_report(log)` renders version, platform, app path context,
  and recent runtime log entries.
- `export_diagnostics_report(log)` writes `diagnostics-report.txt` into the app
  logs directory.

This report may contain local paths and runtime error text. Treat it as
potentially sensitive user data and only share it intentionally.

## Release Readiness

Before a release candidate:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --release --test performance_budgets -- --ignored --nocapture
./bundle.sh --release
```

Dependency advisory/license checks remain a release-readiness requirement. Use
`cargo audit` and `cargo deny` when available, and document any accepted risk in
the release notes or pull request.

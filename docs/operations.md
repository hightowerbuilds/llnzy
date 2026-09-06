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

## Data From Removed Surfaces

The Stacker (prompt queue) and Sketch (drawing canvas) surfaces were removed in
September 2026. The app no longer reads or writes their data, and it does not
delete it either. If you used an earlier build, these may still exist under the
config directory (`~/Library/Application Support/llnzy/`, or `llnzy-dev/` when
running with `LLNZY_PROFILE=dev`):

- `stacker.json`, `stacker_queue.json`, `stacker.json.migrated`: legacy JSON
  queue and state files.
- `prompts/inbox/`, `prompts/saved/`, `prompts/archive/`: the portable markdown
  prompt library with frontmatter. Readable by any editor; safe to keep, move,
  or delete.
- `sketches/` (including `scratch.json`): sketch documents.

Saved themes may still contain `apply_to_stacker` and `apply_to_sketch` keys and
workspace recovery snapshots may still reference `Stacker`/`Sketch` tabs; both
are ignored on load.

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

Current dependency-audit snapshot:

- `cargo audit` completes successfully with warnings allowed.
- Reported warnings are transitive unmaintained/yanked crates through `gpui`,
  `image`, `notify`, and `portable-pty`: `async-std`, `core2`, `instant`,
  `paste`, `rustls-pemfile`, and `serial`.
- No direct replacement work was completed in this roadmap pass. Treat these as
  release-review items before broader distribution.
- `cargo deny` is not installed in the current local toolchain; install it
  before enforcing license/duplicate-dependency policy.

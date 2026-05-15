# LLNZY Performance Budgets

Performance-sensitive work should keep the editor, syntax parser, terminal, and
search paths inside explicit budgets. These checks are intentionally separate
from the normal unit-test gate because desktop timing is noisier than pure
logic.

## Release Budget Command

```sh
cargo test --release --test performance_budgets -- --ignored --nocapture
```

Current hard budgets:

| Budget | Limit | Coverage |
|---|---:|---|
| Editor large insert | 500ms | Large rope insertion plus repeated end-of-buffer edits |
| Rust syntax parse | 1s | Tree-sitter parse over generated Rust functions |
| Terminal throughput | 500ms | ANSI-colored PTY-style output into the terminal emulator |

Initial local release baseline:

| Budget | Observed |
|---|---:|
| Editor large insert | 13.45ms |
| Rust syntax parse | 71.91ms |
| Terminal throughput | 36.25ms |

Treat failures as regressions unless the machine is under obvious load. If a
budget needs to change, record the reason in the change that updates it.

## Policy

- Normal correctness gates remain `cargo fmt --check`, clippy with warnings
  denied, and the full test suite.
- Release-readiness checks should include the release budget command above and
  `./bundle.sh --release`.
- GPU/effects frame timing remains a manual visual smoke item until there is a
  stable harness for GPUI frame measurement.
- New large-file, parser, terminal, search, or LSP work should state whether it
  affects one of these budgets.

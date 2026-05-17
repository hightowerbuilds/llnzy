# LLNZY Code Quality Checklist

Use this before claiming code-quality cleanup is complete.

## Rust Gates

- Run `cargo check --all-targets --all-features`.
- Run `cargo clippy --all-targets --all-features`.
- Treat new warnings as work items unless they are explicitly documented with `#[expect(..., reason = "...")]`.

## Skyscraper File Check

Warn or fail when a Rust source file in active code exceeds 1,000 lines:

```sh
find src tests -name '*.rs' -print0 | xargs -0 wc -l | awk '$2 != "total" && $1 > 1000 { print; failed = 1 } END { exit failed }'
```

Regenerate the roadmap skyscraper list:

```sh
find src tests -name '*.rs' -print0 | xargs -0 wc -l | sort -nr | awk '$2 != "total" && $1 >= 1000 { printf "| %s | `%s` |\n", $1, $2 }'
```

## Unused Dependency Check

- Run `cargo machete` for direct dependency usage.
- When nightly is acceptable, run `cargo +nightly udeps --all-targets --all-features`.
- Verify any reported dependency against feature-gated code before removing it.

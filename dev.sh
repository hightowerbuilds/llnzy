#!/bin/sh
# Launch a development instance of llnzy with isolated config/data/cache
# directories (the `llnzy-dev` app dir instead of `llnzy`) so it never
# touches the daily-driver install. Extra arguments pass through to
# `cargo run`, e.g. `./dev.sh --release`.
set -e
cd "$(dirname "$0")"
exec env LLNZY_PROFILE="${LLNZY_PROFILE:-dev}" cargo run "$@"

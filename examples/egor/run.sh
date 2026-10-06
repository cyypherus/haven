#!/bin/sh
set -eu
cd "$(dirname "$0")"
exec cargo run --manifest-path "$PWD/Cargo.toml" --target-dir "$PWD/../../target" "$@"

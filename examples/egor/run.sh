#!/bin/sh
set -eu
cd "$(dirname "$0")"
engine="$PWD/target/egor"
if [ ! -d "$engine" ]; then
    mkdir -p target
    checkout=$(mktemp -d "$PWD/target/egor.XXXXXX")
    git -C "$checkout" init
    git -C "$checkout" fetch --depth 1 https://github.com/wick3dr0se/egor f474834ad059a87866347d630d2c99375f88f588
    git -C "$checkout" checkout --detach FETCH_HEAD
    git -C "$checkout" apply "$PWD/wgpu-29.patch"
    mv "$checkout" "$engine"
fi
exec cargo run --locked --manifest-path "$PWD/Cargo.toml" --target-dir "$PWD/../../target" \
    --config "patch.crates-io.egor_render.path=\"$engine/crates/egor_render\"" "$@"

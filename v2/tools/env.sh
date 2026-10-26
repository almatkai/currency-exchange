#!/bin/sh
# Source from v2/ (all mutable build/test state stays inside this clone).
export RUSTUP_HOME=/Users/almat/.rustup
export HOME="$PWD/.qa/home"
export CARGO_HOME="$PWD/.qa/cargo-home"
export CARGO_TARGET_DIR="$PWD/target"
export TMPDIR="$PWD/.qa/tmp"
export npm_config_cache="$PWD/.qa/npm-cache"
mkdir -p "$HOME" "$CARGO_HOME" "$TMPDIR"

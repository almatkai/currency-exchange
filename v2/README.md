# Currency Exchange — Rust V2

Standalone external executable for `com.tezbar.native.currency-exchange`, schema 3,
protocol 1. One view command, `index` / Currency Exchange. Exact replacement IDs:
`com.tezbar.currency-exchange` and `raycast.currency-exchange`.

This is an **unpublished development port**. `extension.json` has `artifacts: {}`.
No install, release URL, catalog registration or full-host acceptance is claimed.
The host prerequisites in [integration.md](docs/integration.md) are **unapplied**;
the pinned host needs them before this command's actions can be used safely.

The product requires Rust 1.85 to build, uses a locked immutable HTTPS Git SDK,
and runs without Node, Bun, a JavaScript engine, Tauri or a host path dependency.
Only development oracles use TypeScript/React. Source, SDK, host and original PNG
hashes are in [PROVENANCE.json](PROVENANCE.json).

Type `10`, `1+2`, `5.5 in EUR`, or `100 in JPY at 2023/1/15`. Select the base
currency, pin currencies, and use Set Currency Date. Provider and required API Key
preferences match V1. A settings change requires closing and reopening the command
because SDK session preferences are immutable. Errors retain an editable query;
Retry Exchange makes one deliberate GET. Copy writes plain numeric text, keeps
the view open and issues a fresh action ID after success. Unknown clipboard
acknowledgement offers an explicit retry of the exact prepared bytes.

Private storage is `<dataDir>/currency-exchange/storage.json`. Its atomic updates
preserve unknown root keys and serialize concurrent sessions. It contains the
base currency, ordered pins and up to 32 rate entries scoped by provider, SHA256
key identity and historical civil date. It contains no API key or request URL.
Current rates expire at the provider's next-update timestamp, using rounded Unix
seconds; historical entries do not expire. Invalid cache entries are refetched.
Malformed root JSON remains untouched and displays a repair/retry message.
No profile, legacy V1 store or settings file is imported.

See [behavior.md](docs/behavior.md) for exact parsing/numeric rules and deliberate
validation differences; [QA.md](docs/QA.md) records evidence and limitations.

Build and test from this directory:

```sh
. ./tools/env.sh
cargo test --locked --all-targets
cargo +1.85.0 test --locked --all-targets
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/env.sh` confines mutable Cargo/HOME/temp/npm state to `.qa/` and uses the
existing local Rust toolchains. Adjust the read-only `RUSTUP_HOME` on another
machine. Do not run development tools against a real Tezbar profile.

For reproducible source/renderer QA, install the frozen V1 package-lock into the
owned `.qa/v1/` copy with `npm ci --ignore-scripts`; install `qa/package-lock.json`
into `.qa/renderer/` the same way. Run `tools/source-oracle.cjs` with
`TZ=Asia/Almaty`, then `tools/prepare-host.py`, `tools/renderer-qa.cjs`, and
`tools/schema-qa.cjs`. The latter harnesses read only bundled pinned fixtures;
they do not read the current host checkout. The source VM's network is a synthetic
fixture; Rust transport tests separately exercise actual reqwest over owned TLS.

`tools/package.py` creates an ignored ARM64 development ZIP with a closed inventory
and extracts it under `.qa/`. `tools/process-qa.py` exercises the extracted binary's
real JSONL pipes with OS-denied network and an owned synthetic store. Publication
and native platform acceptance remain parent-owned. `tools/fresh-build.py` uses
an unused tag for independent empty Cargo/home/target state; pass another simple
tag when repeating it. Existing directories are never cleared.

# Verification record

Source HEAD `8759bec78fa594cc3c56e9c3746bfd31e78d0c69`; branch
`feat/native-v2-rust`; staged index empty. Only `v2/` and one root README discovery
link are allowed changes. `tools/audit.py` verifies every original tracked file,
the exact README append, both PNGs, fixture hashes and a complete Git-visible
`INDEX.sha256` (excluding itself and all ignored build/test state).

## Executed checks

* Stable Rust 1.98 and Rust 1.85: 22 tests each, 12 adapter/transport/SDK tests,
  3 native boundary tests and 7 oracle suites. Formatting and all-target strict
  Clippy pass on both compilers. macOS ARM64 release executable builds locked.
* Whole untouched TSX and helper execute in a closed VM with all 22 loaded source
  and locked library modules, including actual node-fetch Response/body decoding.
  Synthetic time, LocalStorage, HTTP responses, hooks/navigation and clipboard
  captures are provided. Native file/OS/process/network/browser/settings/dialog
  effects are denied; explicit denied-boundary probes run. Console arguments are
  discarded and counted, so source logging cannot disclose fixture credentials.
* Differential evidence: **3,394** parsing/property vectors (including 1,070
  parent-added legacy slash-date/century cases and 160
  fractional powers and JS-specific whitespace), **3,703** en-US numeric format
  vectors across every **161** source currencies, **48** ordinary order/filter/
  pin cases, **4** non-finite/missing-base cases. Whole-TSX flows cover rendering,
  base selection, pinning, date submit/pop/empty selection, provider mapping,
  current cache expiry and indefinite historical cache. All compared values/copy
  strings match; validation exclusions are enumerated in behavior.md.
* **32** assertions execute actual pinned List, Form, Surface and RuntimeView with
  React 19.0.0. Four unpatched failures are reproduced, then fixed by the unapplied
  patch. Tests invoke real search/dropdown/submit/action handlers, cover A/B/A,
  action-response ownership, pending field protection, plaintext malicious input
  without active HTML, EmptyView feedback and original PNG payload identity.
* Actual reqwest production transport code uses owned localhost TLS test roots
  through a `cfg(test)` boundary. Both provider request/auth/history shapes are
  asserted. HTTP 401/429/302/500, invalid JSON, known provider errors, invalid base,
  non-finite numeric data, untrusted TLS, declared and streamed >1 MiB bodies are
  rejected safely. No production origin override exists. Actual delayed TLS A/B/A
  replies prove only the last response writes rates/cache/UI. No product API was
  contacted during QA.
* Pinned SDK JSONL pipes exercise current/stale actions, repeated deliberate Copy
  with a new ID, strict ACK negatives, unknown ACK exact retry, cancellation and
  late ACK, search supersession, pending dropdown tokens, refresh/load-more, EOF,
  128-session saturation and cleanup. 140 opens canceled after real allocation/
  TLS dispatch prove authoritative cancellation pruning; another 140 immediate
  canceled opens test the reader/handler boundary. No background polling task is
  introduced. Concurrent fresh storage updates, duplicate-pin removal, preserved
  unknown keys, cache bounds, malformed cache recovery and root corruption are
  tested in owned directories.
* Full pinned schema validates source and development archive manifests with
  **17** rejection cases. Development archive has **325** inventoried non-manifest
  files, no duplicate/traversal members, verified content hashes and executable
  permission. It is extracted before real process testing.
* Real extracted ARM64 executable runs JSONL with **25** assertions and OS-denied
  network, an owned synthetic store, capturing host clipboard and no GUI. Actual
  command/search/copy/unknown-ACK retry/pin/base/date/cancel/dispose/EOF flows run.
* Frozen V1 package-lock is installed only into `.qa/v1/`; all **15** disposable
  original tests and original typecheck pass. Root package/lock/source/assets/
  license/metadata are byte-audited, not reformatted or migrated.
* `tools/fresh-build.py` copies final source outside the working source directory
  (still inside this clone), verifies hashes, and builds release with an independent
  initially empty Cargo home/target/home. The SDK must resolve via immutable HTTPS
  Git to `e955911f0bf5b558f9d3eb5183df0adc95c22af2` inside that new Cargo home. The
  independently built executable receives the same real-process smoke.

Detailed machine-readable results and logs are under ignored `.qa/`; source oracle
vectors, pinned fixtures, license notices, locks and reproducible harnesses are
Git-visible. `tests/source-oracle.json` is generated from actual source execution,
not hand-authored expected Rust output. `integration/host-fixtures.json` binds the
host evidence to its exact pin. `PROVENANCE.json` binds source/SDK/assets.

## Coverage limits

This is a process/transport/renderer boundary verification, not real OS GUI or a
full host installation. The host patch is not applied or compiled as a whole.
Its Rust tests compile selected exact functions/selection blocks with package/OS
mocks. React renderer tests do not claim desktop accessibility/layout acceptance.
Pinned host path discovery also used `git ls-tree` at the exact pin, a procedural
deviation from the git-show-only instruction; host file contents used `git show`.
No actual clipboard, dialog, foreground app, user profile/keychain, real key, live
rate request, install, public catalog, release, push or deploy is exercised.

Windows GNU check was attempted but cannot finish because the existing machine
lacks `x86_64-w64-mingw32-gcc`; link/build also encounters missing
`x86_64-w64-mingw32-dlltool`. No tools were installed. Windows MSVC target/native
execution is **not run**. macOS native GUI is **not run**. Both manifest platform
verification rows remain `not-run`; process tests do not promote those rows.

Parsing and numeric evidence are exact on these tested compilers/platform and
pinned en-US ICU data. It does not assert universal OS Intl output for arbitrary
locales, or mathematical equivalence of every possible f64 exponent across every
CPU/libm implementation. Input/resource and provider validation exclusions have
explicit outcomes; an arbitrary native error does not count as an oracle pass.
The rate-date subtitle is UTC metadata instead of OS-localized text. Malformed
root storage requires private-store repair and manual retry, without automatic
loss of original bytes. Clipboard checkpointing is session-local, not crash
persistence or an exactly-once guarantee.

## XERJ

Canonical prefix `ax-c201a25adef3`, state only `.qa/xerj/`. Initial index completed
with generation 1, 5/5 code files indexed; exit 3 means skipped non-code/junk
assets, not a claim that every file was searchable. Discovery searched this exact
prefix before reading definitions. Full source reads, `git ls-files`, exact `rg`
and SHA256 audits provide exhaustive verification. Final update aborted with frozen-dataset/schema rejection for the new
`v2/DEPENDENCIES.json` shape (exit 1). The final index is **incomplete**; no alternate
prefix, schema mutation or global server restart was attempted. Final coverage
uses complete Git-visible files, exact `rg`/full-file reads and SHA256 inventory. Build outputs, Cargo/npm homes and test stores remain
ignored under this one clone.

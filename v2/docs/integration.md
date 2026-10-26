# Unapplied host prerequisites

`integration/host-prerequisites.patch` targets **only** Tezbar
`ea100cee3f69711de12485ba061950c5fc65258f`. Nothing was applied to the host checkout.
Host file contents came from `git show` at that exact pin. Procedural exception:
path discovery also used read-only `git ls-tree` of the same pinned object, despite
the requested git-show-only restriction. Its concurrently dirty working files
were neither read, built, checked out, stashed nor modified. The transcript and
pinned fixtures preserve this evidence; no global or unknown data was removed. Full original
fixtures and SHA256s are bundled in `integration/fixtures/host/` and
`integration/host-fixtures.json`.

The patch has seven file targets, all generic:

* Registry: active verified version is the resource root for icons/extensionPath;
  canonical package path remains the preference owner. Existing preference
  retention, unknown keys and command scopes remain independent of assets.
* Native routing: replacement aliases must exactly match a verified installed
  manifest, have a Rust command on this platform, and resolve unambiguously. This
  permits both declared own and Raycast aliases without slug guessing or a catalog
  name branch. Canonical damaged packages retain their explicit failure path.
* List: honor controlled dropdown values, use a real large select for 161 entries,
  send native server searches immediately, notify Surface before draft changes,
  honor server empty feedback and render the original EmptyView PNG.
* Surface: inject current view/search/from tokens for all action paths, including
  row Enter, dropdown, action overlay, submit and Back. Block old actions as soon
  as a draft or pending dropdown changes. Form submissions briefly freeze their
  exact submitted field snapshot; later fresh views issue fresh action IDs.
* Form: the pin already supports DatePicker as an HTML date input and full submit
  values. Only pending-field protection/draft notification is added; no fictitious
  date API or extension-specific DatePicker callback is introduced.
* RuntimeView: all search/action/refresh/load-more results are fenced against
  draft/session ownership; unchanged actions keep the current view. An A/B/A text
  cycle cannot make an older response current merely because its text matches.
* Shared type: include the SDK's actual `unchanged` invoke-action result.

Without the patch the source List dropdown chooses its first entry instead of the
stored selected base, stale Copy remains callable before the debounce, and a late
action response overwrites a newer query. The extension rejects absent/stale
renderer tokens with a no-effect unchanged response, rather than authorizing an
old conversion. This is a host prerequisite, not a claim of GUI compatibility.

Tests execute actual pinned List/Form/Surface/RuntimeView code with React 19.0.0,
before and after the patch; actual event handlers are invoked. They reproduce four
unpatched failures, exercise draft/response fencing, DatePicker values, pending
fields, malicious plaintext and original PNG payload identity. Surrounding app
services/OS are mocked or denied. No screenshots or live GUI acceptance is claimed.

`integration/native-extracted.rs` carries exact complete selected registry
functions and the actual native canonical-selection block, with narrow test
wrappers. Standalone tests use mocked discovery/package verification and owned
files. They verify aliases, uncovered/lookalike/ambiguous/unverified negatives,
active-version icons, canonical preferences and the actual clipboard producer
acknowledgement. This is **not** compilation of the entire native host or an
end-to-end installer test. Parent must review/build/integrate the full patch in
its chosen host tree before release.

The actual selected worker producer's `Action.CopyToClipboard` writes clipboard
content without hiding the window or pasting. Its extracted handler is executed
in QA with a capturing clipboard. The Rust adapter uses only
`host.clipboard.write {text}`. The actual native producer returns `{ok:true}`;
the adapter accepts only closed `{ok:true,message?:string}`. Null, booleans,
contradictory/unknown keys and malformed messages produce an explicit unknown-ACK
retry. No window visibility service, OS paste or browser call is requested.

Action consumption and prepared clipboard bytes live outside the SDK's staged
transaction clone. Cancelled writes retain an unknown checkpoint; late ACKs have
no authority. Retry uses those exact bytes without fetching/parsing/reclocking.
A successful deliberate copy renders another fresh Copy ID; the old ID stays
inert. This is session-local fencing, not an exactly-once or crash-durable
clipboard guarantee. Registry capacity is 128, with cancellation pruning on entry
and Drop-owned SDK cancellation, without a polling task.

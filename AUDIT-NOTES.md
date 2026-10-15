# Repository inspection notes

This is a bounded inspection of the fork based on `ddd6cf8`, not a statement
that every path is safe. See [TESTING.md](TESTING.md) for reproducible local checks
and the distinction between offline fixtures and live integrations.

## Follow-up work identified by the repository-wide inspection

These are inherited concerns, not regressions attributed to the RuStore status
fix. Confirm against the current source before starting a follow-up change.

| Area | Remaining concern | Consequence / recommended next step |
|---|---|---|
| Other source exit statuses | APKPure/F-Droid/Google Play still have paths which only print an item error instead of aggregating failure to the CLI | Do not treat exit 0 as an artifact-validity proof across all sources; apply the explicit batch result contract source by source |
| APKPure/F-Droid/Google downloader dependency | Legacy callers still write directly to final names and may retry against partial files; the vendored patch now fixes short writes, missing flush and missing SHA stream initialization | A failed transfer can still leave a partial file; use temporary output, no-clobber publication, cleanup and integrity validation before web integration |
| Input and output names | Legacy sources construct filenames from app/version/ABI values without a unified safe-component policy | Validate before requests and publication; test traversal, symlinks and races, including Windows separately |
| APKPure metadata parsing | Broad version/URL regular expressions can cross record boundaries and lack a documented destination policy | Replace with fixture-backed structured parsing; never silently download another version under a pinned name |
| F-Droid cache and network boundaries | Several expected network/cache/decode errors still panic or exit deep in helpers; the cache's verification/fingerprint policy needs separate review | Add bounded HTTP/cache tests and structured errors; do not weaken signature verification to make tests pass |
| Shared input | CSV is split by commas, not a fully quoted CSV dialect; a missing version column can silently become latest | Define the input contract and reject incomplete records rather than silently changing requested version |
| Google credentials | Interactive token input uses ordinary terminal echo; some explicit INI errors are ignored by dispatch | Add no-echo/fail-closed secret input and distinguish missing default config from malformed explicit config; no real credentials in tests |
| HTTP availability | Several legacy clients lack explicit application timeouts/body caps | Add bounded per-stage request policies; a loopback fixture does not prove the public service cannot stall |
| Generated help | `build.rs` writes tracked `USAGE` in the source checkout | Move generation into `OUT_DIR` and provide an explicit checked documentation update command; CI currently checks drift only |
| Release reproducibility | The remote script still clones upstream EFForg instead of a supplied immutable fork commit, disables SSH host-key checking and bootstraps mutable artifacts | The error-stop regression is fixed separately, but this script is not a verified release pipeline for this fork; pin inputs and verify artifact hashes before use |
| Dependencies | `bincode 2.0.1` via gpapi has a RustSec unmaintained warning, not a demonstrated vulnerability | Coordinate serializer migration with upstream; do not invent a patched version or break cache compatibility |

Some findings come from static inspection and explicitly labelled semantic
reproductions, not a reproduced end-to-end exploit. Trusted local inputs,
custom repository metadata and hostile network responses are different threat
models; do not report them as equivalent. Signature verification alone does not
establish publisher trust, and a skipped existing file has not been reverified.

## Review boundaries

- Tests cover actual local Rust helpers, bounded HTTP fixtures and child CLI
  behavior. A passing test suite is not a full security approval.
- Google authentication, remote store compatibility, cross-platform releases and
  web APK Downloader jobs require separate checks.
- No accepted Terms of Service, public credentials, service restarts or production
  binary replacement are part of running these tests.

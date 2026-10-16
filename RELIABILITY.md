# Download and CLI reliability contracts

## Status and output

- APKPure, F-Droid and Google Play download/list/token callers report failure
  through the real CLI exit status, not only printed diagnostics. An unsupported
  Google Play version listing returns nonzero; it is not a successful empty list.
- Mixed batches retain completed outputs and process remaining valid inputs, but
  any failed/invalid item makes the overall command fail. Consumers must check
  exit status **and** validate the selected completed artifact; neither alone
  certifies application authenticity or installability.
- Errors must not dump credentials, signed URLs, server response bodies or
  unvalidated input into diagnostics. Requesting a new AAS token is deliberately
  an output-producing operation: protect its stdout as a secret.

## Files and deadlines

The shared downloader uses a private same-filesystem temporary file, complete
writes and flush, then no-clobber publication. Its F-Droid checksum API verifies
SHA256 **before** the final filename becomes visible. Truncation, empty bodies,
errors and cancellation do not leave the partial under the final name. Existing
regular outputs are not overwritten; existing symlinks/directories are errors.

Google Play uses a separate private directory for each attempt, retaining gpapi's
plain/split/extra-file layout. Failed or timed-out attempts are discarded rather
than exposing partial files. This is caller-policy coverage, **not** a live
certification of Google authentication or gpapi payloads.

Network policies bound metadata requests and download work. A slow connection
can now return a timeout instead of waiting indefinitely. Intentional batch sleep,
several finite retries and many apps can still make a whole batch take longer
than one request deadline. SIGKILL, machine failure, disk failure and filesystem
semantics are outside normal async-cancellation guarantees. No fsync/crash-durable
transaction or multi-artifact atomicity is claimed. OUTPATH and its ancestors
must be trusted and not concurrently replaced by other users; pathname-based
publication is not a dirfd-based defense against a hostile local filesystem owner.

## CSV and credentials

CSV is parsed as CSV, not split on commas. Quoted commas, escaped quotes and
multiline quoted fields are supported; malformed quoting is rejected. Selected
app/version columns must be present on every nonblank row. An explicitly empty
version cell means `latest`; a missing cell is an error. A blank app, empty batch,
control characters in a selected value and inputs over 8 MiB are rejected.

An explicitly selected Google Play INI is mandatory and must parse even when
CLI credentials are complete. An absent default INI is optional; an existing
unreadable/invalid default is an error. Configuration errors never print file
contents. Missing credentials are not read from a pipe as an empty token. The
interactive token prompt disables echo and restores terminal settings on a
normal return, EOF or ordinary Ctrl-C. The local `third-party/rpassword`
Unix patch restores termios before raising SIGINT; retain its original Apache-2.0
license and patch provenance. This is not a SIGKILL/crash guarantee. Prefer a protected INI or the no-echo prompt;
credential arguments can still be exposed through shell history/process listings.
OAuth token retrieval requires email and cannot be combined with version listing.

## Releases and boundaries

See [HACKING.md](HACKING.md) for the exact-source native-Linux remote-build
contract. The script is no longer a provisioning/cross-compilation/publishing
shortcut: it uses one exact local committed source archive, strict pinned SSH
host keys and verified private artifact collection. Real remote builds and
other targets require independent validation.

The Git checkout must include `third-party/tokio-dl-stream-to-disk` (MIT) and
`third-party/rpassword` (Apache-2.0), their original licenses and patch provenance. A normal crates.io consumer does **not** inherit this root
`[patch.crates-io]`; registry packaging/publication is not supported by this change.

The web APK Downloader has a separate staged candidate and separate tests. No
Rust build or local commit replaces its active service or binaries. Store API
compatibility, live Google authentication, Docker, other OS targets and source
trust/authenticity remain distinct integration/security gates.

# Local downstream patch

Original: `tokio-dl-stream-to-disk` **1.0.2**, EFForg, crates.io distribution.
Author: **William Budington <bill@eff.org>**. Original MIT license and EFF
copyright are retained verbatim in LICENSE. Cargo.toml comes from the published
Cargo.toml.orig, with the additional `tempfile = "3"` dependency; src/error.rs
is unchanged. The Cargo patch applies to apkeep and its transitive gpapi user
without editing the registry cache. Existing public methods and the
`sha256sum` feature are retained.

## Transfer contract

- `new`, consuming `get`, `length`, `download`, and
  `download_and_return_sha256sum` remain available. Hash downloads initialize
  the HTTP stream lazily, including retries after a failed transfer.
- A filename must be a nonempty single filename: `.`, `..`, either separator
  (`/` or `\`), and control characters are rejected before network access.
- An existing **regular file** produces `FileExists` without a request from
  the download methods. This is a skip, not revalidation of its contents.
  Symlinks (including broken ones), directories, and other nonregular entries
  fail, but never as `FileExists`. `get` remains a header-only preflight and
  does not skip an existing destination file.
- Transfers use a private `.apkeep-*` NamedTempFile in the destination directory.
  The owner stays in the transfer future, so failures, elapsed deadlines, and
  dropped/cancelled futures remove the temporary name. No failure path removes
  a preexisting final output.
- The shared loop uses `write_all`, rejects empty and length-mismatched bodies,
  awaits `flush`, then publishes through `persist_noclobber`. Publication must
  not replace a file, symlink, or directory created during the transfer.
- With `sha256sum`, the additional method
  `download_and_verify_sha256(&mut self, expected: &[u8], cb: &Option<Box<dyn Fn(u64) -> ()>>) -> Result<Vec<u8>, TDSTDError>`
  validates the 32-byte digest length **before any request** and compares the
  downloaded digest **before publication**. A mismatch leaves no final file.
  The legacy hash-returning method does not claim remote checksum verification.
- Production HTTP policy: 15-second connect deadline (including TLS handshake),
  30-second per-read deadline (including response headers), and 600-second
  whole-request deadline. The latter spans `get` and response body consumption;
  an additional 600-second future deadline covers the local transfer loop.
  Tests change only a private typed policy. No environment variable can disable
  or shorten production timeouts.

## Verification and boundaries

Parent `tests/downloader_dependency.rs` exercises the actual patched dependency:
complete immediate-read bytes, SHA256 success/mismatch, malformed digest and
filename rejection before requests, regular/symlink/directory outputs,
truncation/empty responses, publication races, concurrent same-path transfers,
and cancellation after data has reached the private temporary file.

Vendor `src/tests.rs` uses loopback HTTP/TLS stalls and a private short-deadline
policy to exercise stalled headers/body, TLS connect timeouts, trickling-body
whole deadlines, preflight deadline continuity, and cleanup when the outer
future timeout drops a local stream without relying on a reqwest timer.

This does not add fsync/power-loss durability, APK/ZIP validation, a URL allowlist,
or a guarantee against an adversary who can rename the destination directory
itself. Callbacks are synchronous user code and must not block the executor.
Replace/remove this patch only with equivalent tested upstream behavior, not
by silently dropping the Cargo patch stanza.

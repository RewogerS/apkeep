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
- Every artifact has an inclusive **2 GiB (2,147,483,648 bytes)** ceiling,
  whether or not SHA256 support is enabled. `with_max_bytes(u64)` lowers this
  ceiling; values above 2 GiB are clamped and zero rejects all nonempty bodies.
  Oversized Content-Length is rejected from headers, before creating a
  tempfile. Lowering the cap after `get` is checked again before creating it.
  Before **every** `write_all`, `checked_add` checks cumulative bytes against
  both the ceiling and any advertised length. Unknown-length/chunked bodies
  cannot bypass the cap; a crossing read is discarded without a write.
  Over-budget, truncated and hash-invalid responses publish nothing and remove
  their private temporary name. This is a per-artifact limit, **not an aggregate
  filesystem quota** across concurrent jobs, retries or retained artifacts.
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

## Public-network client contract

- `pub fn public_http_client_builder() -> reqwest::ClientBuilder` and
  `pub fn validate_public_url(url: &reqwest::Url) -> Result<(), std::io::Error>`
  are shared with the parent providers (the same reqwest 0.13 type).
- The pair is mandatory: validate **every initial URL** before sending it with
  the builder. Validation rejects userinfo, non-HTTP(S), and nonpublic IP
  literals. reqwest skips its DNS resolver for literal IPs, and its builder has
  no initial-request validation hook; using the builder alone is insufficient.
  `AsyncDownload::new` performs this validation automatically.
- IPv4 special/private/loopback/link-local/shared/documentation/benchmark/
  multicast/reserved blocks are rejected. IPv6 permits global unicast only,
  excluding special-purpose/documentation/6to4 blocks; mapped/compatible IPv4,
  NAT64, local, multicast and reserved IPv6 are rejected conservatively.
- DNS resolution has a 15-second deadline. The complete answer is checked;
  **one nonpublic address rejects the entire answer**. The connector receives
  those exact checked socket addresses, without a validation-then-relookup gap.
- Every HTTP(S) redirect target is validated, including literal IPs and
  userinfo, with a maximum of ten followed hops. Redirect DNS uses the same
  resolver. reqwest stops unsupported-scheme redirects without making a target
  request but can return the original 3xx as `Ok`; the downloader therefore
  also requires a 2xx response before accepting headers/body. Direct callers
  must not mistake `error_for_status()` alone for a 2xx check.
- `.no_proxy()` disables environment and system proxies. Production callers
  must not replace the resolver/redirect/proxy policy or use DNS overrides.
- `AsyncDownload::with_client(self, client: reqwest::Client) -> Self` explicitly
  replaces the default client/policy for trusted dependency injection. Local
  fixtures provide unguarded clients this way. There is **no environment flag**
  or `cfg(test)` exception disabling production networking enforcement. The
  byte ceiling applies even with an injected client. This API is not a sandbox
  against trusted Rust callers deliberately choosing an unguarded client.

## Verification and boundaries

Parent `tests/downloader_dependency.rs` exercises the actual patched dependency:
complete immediate-read bytes, SHA256 success/mismatch, malformed digest and
filename rejection before requests, regular/symlink/directory outputs,
truncation/empty responses, publication races, concurrent same-path transfers,
and cancellation after data has reached the private temporary file. Byte-budget
regressions cover chunked/unknown-length streams without EOF in all three
download modes, observe four actual tempfile bytes before a crossing read,
assert only four bytes ever reach `write_all` under an eight-byte cap, verify
cleanup, inclusive boundaries, unchanged hashes, and post-preflight cap changes.

Vendor `src/tests.rs` uses loopback HTTP/TLS stalls and a private short-deadline
policy to exercise stalled headers/body, TLS connect timeouts, trickling-body
whole deadlines, preflight deadline continuity, and cleanup when the outer
future timeout drops a local stream without relying on a reqwest timer. It also
checks default oversized headers before the first write, cumulative/single-read
overruns and advertised-size overruns using the exact shared transfer loop,
production loopback rejection, mixed DNS answers and lookup count/deadlines,
mapped IPv6, redirect targets and hop count, and ignored environment proxies
in an isolated subprocess. Public address examples are inspected as values;
the suite makes no public exploit requests or live-store requests.

This does not add fsync/power-loss durability, APK/ZIP validation, a host allowlist,
or a guarantee against an adversary who can rename the destination directory
itself. Callbacks are synchronous user code and must not block the executor.
Replace/remove this patch only with equivalent tested upstream behavior, not
by silently dropping the Cargo patch stanza.

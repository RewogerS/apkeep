# Local downstream patch

Original: `tokio-dl-stream-to-disk` **1.0.2**, EFForg, crates.io distribution.
Author: **William Budington <bill@eff.org>**. Original MIT license and EFF
copyright are retained verbatim in LICENSE. Cargo.toml comes from the published
Cargo.toml.orig; src/error.rs is unchanged. The Cargo patch applies to apkeep and
its transitive gpapi user without editing the registry cache or changing API.

Only implementation deltas to src/lib.rs:

1. Both stream loops use `write_all` rather than ignoring short-write counts.
2. Both loops await `flush` before success/hash return. Dropping a Tokio File does
   not establish visibility of in-flight writes to an immediate reader.
3. `download_and_return_sha256sum` lazily initializes the HTTP stream like
   `download` does, instead of unwrapping None on a new/retry instance.

Regression tests are in the parent `tests/downloader_dependency.rs` and the
F-Droid entry-index loopback test. They test the actual patched dependency.
The patch does not add fsync/power-loss durability, atomic publication, cleanup
on failed transfers, URL policy or timeouts. Legacy callers still need those
separate controls. Replace/remove this patch only with equivalent tested
upstream behavior, not by silently dropping the Cargo patch stanza.

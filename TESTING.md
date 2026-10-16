# Testing apkeep

## Scope

Tests are local and deterministic unless explicitly marked as manual live checks.
`cargo test` never signs into Google Play, accepts Terms of Service, buys apps, or
contacts the public stores. Loopback HTTP tests exercise real HTTP clients against
bounded local servers. CLI tests spawn the real compiled executable with isolated
output/config directories and child-only environment changes.

```sh
cargo test --locked --all-targets
cargo test --locked -p tokio-dl-stream-to-disk --lib
cargo build --locked
APKEEP_TEST_BINARY=target/debug/apkeep python3 -B -m unittest discover -s tests
cargo clippy --locked --all-targets
cargo doc --locked --no-deps
bash -n build-remote.sh
```

Add `--offline` only if dependency sources are already cached. Run in a dedicated
worktree and use a distinct `CARGO_TARGET_DIR` for baseline, mutation and candidate
builds: sharing a target directory between different snapshots can silently reuse
the wrong executable. The lockfile is part of the input, not regenerated for tests.

The GitHub Tests workflow runs tests, clippy, documentation and release-script
syntax checks, then verifies that the generated `USAGE` file remains unchanged.
Clippy warnings inherited from upstream are reported, not claimed absent. New
warnings must be reviewed against a clean baseline. Do not execute
`build-remote.sh` against a real host as a test. Its replacement is an exact-source
native build collector, not a provisioning or publishing script. The Python
release tests mock every transport/toolchain action; no SSH is performed.

## Test strategy

- Check failure **status** as well as diagnostic text, files and batch semantics.
- Bound subprocesses and HTTP fixtures; a stalled test must fail, not hang CI.
- Verify output file contents and no-clobber behavior; mere file existence is not
  proof that a download finished or that an APK is valid.
- Keep fixture endpoints private to tests; do not add production endpoint
  overrides to make testing convenient.
- Never put real tokens, passwords, signed download URLs or user configurations
  in fixtures, snapshots or logs.
- Bug fixes need a regression which fails on the original behavior and passes
  after the fix. Characterization tests of existing behavior are identified as
  such, not called RED/GREEN evidence.

## Reliability follow-up

See [RELIABILITY.md](RELIABILITY.md) for batch status, atomic publication, deadlines,
CSV and credential contracts. Google Play caller policy is exercised through the
same bounded backend adapter as the real gpapi caller, with no Google login. The
POSIX terminal test uses a real PTY and a deliberately invalid synthetic token;
it asserts no-echo, terminal restoration and refusal before authentication.

## Shared downloader dependency

The root Cargo patch uses the MIT-preserving local copy documented in
`third-party/tokio-dl-stream-to-disk/PATCHES.md`. Its direct integration tests
assert immediate byte visibility (not eventual reads), SHA256 stream initialization,
existing-file preservation and truncated-body failures. Keep this patch during
source copies/builds; the root lockfile alone is not sufficient.

## Coverage

With `cargo-llvm-cov` and the matching `llvm-tools-preview` component installed:

```sh
cargo llvm-cov --locked --all-targets --json --output-path coverage.json
```

Put coverage artifacts outside the tracked source tree (or in an ignored output
path). Source-based coverage can include `#[cfg(test)]` modules and child-process
execution; report production-source and test-file numbers separately. A high
line percentage is not evidence for remote API compatibility or cryptographic
trust. No claimed percentage should be inferred from the number of test cases.

## Manual validation boundaries

Authenticated Google login/download, live signed F-Droid indexes, APKPure service
availability/anti-bot behavior, release cross-compilation, Docker publishing and
remote build hosts require separate environments/credentials. Offline fixtures do
not certify those integrations. A live APK check should include fresh destination,
ZIP CRC, manifest/package/version, signature verification and API size/hash where
available, and identify the exact executable/source being exercised.

The web APK Downloader is a separate layer: its scan/job handling and deployment
must be tested separately. Passing Rust tests does not deploy or fix that service.

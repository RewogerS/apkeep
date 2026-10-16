To download from the F-Droid open source repository:

```shell
apkeep -a org.mozilla.fennec_fdroid -d f-droid .
```

More advanced options for this download source can be passed with the `-o` option.  For instance, to use an F-Droid mirror:

```shell
apkeep -a org.mozilla.fennec_fdroid -d f-droid -o repo=https://cloudflare.f-droid.org/repo .
```

In addition to specifying a mirror, a wholly separate F-Droid repo can be specified along with its fingerprint:

```shell
apkeep -a org.torproject.android -d f-droid -o repo=https://guardianproject.info/fdroid/repo?fingerprint=B7C2EEFD8DAC7806AF67DFCD92EB18126BC08312A7F2D6F3862E46013C7A6135 .
```

If a repo only supports the old (v1) package index, the newer [entry point specification](https://f-droid.org/docs/All_our_APIs/#the-repo-index) can be disabled by use of the `use_entry` option:

```shell
apkeep -a org.torproject.android -d f-droid -o repo=https://guardianproject.info/fdroid/repo?fingerprint=B7C2EEFD8DAC7806AF67DFCD92EB18126BC08312A7F2D6F3862E46013C7A6135,use_entry=false .
```

## Reliability and verification policy

- Downloads and version listings succeed only when **every** requested app succeeds. An empty request, missing app/version/ABI, malformed metadata, HTTP error, timeout, filesystem error or checksum mismatch is a failure. Valid jobs still run when another job fails; the CLI reports an unsuccessful overall result and diagnostics on stderr.
- APKs and the index referenced by a verified entry are checked against their expected SHA256 **before** atomic, no-clobber publication. Failed/truncated transfers and checksum mismatches do not leave a final APK/index file. An existing regular APK counts as a successful skip only if its SHA256 matches; mismatching files are preserved and reported as failures. Symlinks, dangling symlinks, directories and other nonregular destinations are errors, never successful skips.
- Package IDs, requested versions and ABI values must be nonempty ASCII path components using letters, digits, `.`, `_`, `-` or `+` (not `.` or `..`, at most 200 bytes per component). Generated APK filenames must fit within 255 bytes. Output directory components must be existing real directories: symlinks and parent (`..`) traversal are rejected.
- Entry/index and APK repository filenames must contain only literal repository-relative components using ASCII letters, digits, `.`, `_` or `-`. One leading `/` is normalized; empty, `.` and `..` components, encoded separators, backslashes, query/fragment syntax and full URLs are rejected. Safe nested paths remain supported. APK candidates must carry a hex SHA256 representing exactly 32 bytes; malformed or unsafe candidates are ignored, making an unresolved request fail.
- GETs have a 15-second connect deadline and a 120-second overall F-Droid transfer deadline (per APK attempt, up to three attempts). Metadata downloads and archive extraction are bounded to 128 MiB; archives are limited to 4096 entries. Unsafe archive entries and extraction errors fail closed.
- Conditional GET caching stores **signed archives**, not trusted unsigned JSON. Every cached archive is reverified against the current signature/fingerprint policy, including after HTTP 304; the old JSON/etag cache is not reused. Missing ETags cause a fresh download, not a panic. Corrupt or nonregular cache entries produce an error. No network, cache or parsing error silently disables verification.

The explicit `verify-index=false` debugging option below is still supported; it disables repository authentication and the entry's index hash check, **not** APK hash checking. It is never selected automatically after a failure.

A special option can also be used to skip verification of the repository index.  *Only use for debugging purposes*:

```shell
apkeep -a org.torproject.android -d f-droid -o repo=https://guardianproject.info/fdroid/repo,verify-index=false .
```

It is also possible to download a specific architecture variant of an app with the `arch=` option:

```shell
apkeep -a org.videloan.vlc@3.5.4 -d f-droid -o arch=arm64-v8a .
```

To list what versions are available, use `-l`:

```shell
apkeep -l -a org.mozilla.fennec_fdroid -d f-droid
```

To output the list in json, use the `output_format=json` option:

```shell
apkeep -l -a org.mozilla.fennec_fdroid -d f-droid -o output_format=json
```

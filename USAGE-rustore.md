# Downloading from RuStore

Select RuStore with `-d ru-store`. Only the latest available version is supported;
`package@version` requests are skipped with a diagnostic and make the download
command fail. Listing versions with
`-l` explains this limitation rather than returning historical versions.

```shell
apkeep -a com.vk.calls -d ru-store .
apkeep -l -a com.vk.calls -d ru-store
```

## Device and client options

Use the existing `-o key=value,key=value` syntax. Option names are case-sensitive
and use snake_case. Quote the entire argument in a shell, especially when values
contain semicolons or spaces:

```shell
apkeep -a com.vk.calls -d ru-store \
  -o 'arch=arm64-v8a;armeabi-v7a,sdk_version=34,screen_density=480,locales=ru_RU;en_US,mobile_services=GMS;HMS,without_splits=true' .

apkeep -a com.vk.calls -d ru-store \
  -o 'rustore_version=11500,user_agent=RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)' .
```

| Option | Default | Accepted value / request field |
| --- | --- | --- |
| `arch` | `arm64-v8a` | Non-empty semicolon-separated ABI tokens; `supportedAbis` |
| `sdk_version` | `30` | Positive decimal `u32` (1 through 4294967295); `sdkVersion` |
| `screen_density` | `420` | Positive decimal `u32` (1 through 4294967295); `screenDensity` |
| `locales` | `en_US` | Non-empty semicolon-separated locale tokens; `supportedLocales` |
| `mobile_services` | `GMS;HMS` | Non-empty semicolon-separated service tokens; `mobileServices` |
| `without_splits` | `true` | Exactly `true` or `false`; `withoutSplits` (see below) |
| `rustore_version` | `11500` | Positive decimal digits, not all zero; required `ruStoreVerCode` HTTP header |
| `user_agent` | See below | Non-empty valid HTTP header value; `User-Agent` |

The default user agent is unchanged:

```text
RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)
```

List tokens may contain ASCII letters, digits, `_`, `-`, and `.`; empty elements,
whitespace and control characters are rejected. Numbers cannot have signs,
whitespace, or fractional parts. Device settings do not automatically rewrite the
user agent. The server may not provide a build matching every device configuration.

The shared `-o` parser uses commas to separate options; a comma cannot be included
inside a value, even when shell-quoted. Unknown options follow the existing shared
parser behavior (they are ignored); use the names above exactly. There are no
options to change the API endpoints. `firstInstall=true` and
`signatureFingerprint=null` remain fixed.

## RuStore version precedence

The `ruStoreVerCode` header is always sent on lookup and download-link requests.
Its value is selected in this order:

1. Explicit `-o rustore_version=...`.
2. The `APKEEP_RUSTORE_VER` environment variable, when present.
3. The default `11500`.

```shell
APKEEP_RUSTORE_VER=11500 apkeep -a com.vk.calls -d ru-store .
# The explicit option wins, even if the environment value is invalid:
APKEEP_RUSTORE_VER=invalid apkeep -a com.vk.calls -d ru-store \
  -o 'rustore_version=11500' .
```

An invalid selected explicit or environment value is an error, not a request to
fall back to another source. All supported options are validated before any
RuStore network requests. Diagnostics name the invalid option/source without
printing its value. HTTP errors, network/read errors, malformed responses,
application-not-found results, and API rejections are reported separately, without
printing response bodies or signed download URLs.

## Exit status and output safety

RuStore download commands exit with status `0` only when every requested app
was downloaded successfully or its existing regular output file was skipped.
Invalid selected configuration/environment, an empty app list, unsupported
version pins, invalid package names, lookup/link/API/HTTP errors, and failed or
empty downloads produce a nonzero status. Batch processing still attempts every
item and keeps successful downloads even when other items fail. Diagnostics go
to stderr; progress and successful-download/skip messages go to stdout. Invalid
option values, untrusted version strings, response bodies and signed download
URLs are not printed.

For downloads, `-r/--parallel` must be greater than zero (for every source), and
is rejected before starting downloads or Google authentication. RuStore also
validates this in its download helper. `-l` remains an offline explanation of the
latest-only limitation; it does not validate unused download options.

Package names must consist of dot-separated nonempty ASCII identifiers, each
starting with a letter or `_`, followed by letters, digits or `_`. Path separators,
query/fragment syntax and control characters are rejected before RuStore network
requests. Output is named `<package>.apk` inside OUTPATH. Existing symlinks and
other nonregular entries are errors, not successful skips. New downloads use a
private temporary file, are flushed before no-clobber publication, and are cleaned
up after failed transfers; a failed retry cannot turn a partial file into success.
There are at most three transfer attempts.

### Fixed network and resource limits

RuStore uses a 15-second connection timeout and a 30-second idle-read timeout
(including waiting for response headers). Each metadata request has a 60-second
total deadline and a 4 MiB body limit. Metadata is read incrementally: oversized
`Content-Length` is rejected before reading the body, and responses without a
length (including chunked responses) are capped while streaming, before JSON parsing.
Lookup and download-link requests are not retried automatically.

Each APK transfer attempt has a 15-minute total deadline and a 2 GiB byte limit.
The total deadline does not reset when more data arrives; the idle-read timeout
bounds a stalled transfer even after the first chunk. Oversized declared lengths
are rejected before creating a download file, streamed bytes are checked before
writing, and a declared `Content-Length` must match the completed transfer before
publication. Timeouts, truncated responses and size-limit failures return failure
and remove temporary files on every attempt. Diagnostic messages omit URLs,
signed query strings, server bodies and raw transport errors.

These limits are fixed production policy, not additional `-o` options or
environment variables. The existing API endpoints, device/header precedence,
latest-only and split-APK rules are unchanged.

Existing regular files are preserved **without revalidation**. A successful HTTP
transfer is not an APK signature, ZIP/manifest, or API-checksum verification; use a
fresh output directory and independently verify downloaded APKs when required.

## Split APK limitation

Keep `without_splits=true` for a standalone APK. Setting
`-o 'without_splits=false'` changes the API request, but does **not** add split APK
installation, merging, or XAPK output support. If RuStore returns more than one
download URL, apkeep rejects the entire result with a "split APKs are not
supported" diagnostic; it never downloads only the first member and presents it
as a complete app. This safeguard also applies when `without_splits=true`, in
case the server returns multiple URLs despite that preference. A response with
exactly one valid HTTP(S) URL continues through the existing APK downloader.

# Downloading from Huawei AppGallery

Select this source explicitly; the default download source remains Google Play.

```shell
# Compatible default service country: IE
apkeep -a com.huawei.appmarket -d huawei-app-gallery .

# Explicitly request the CN catalogue with the otherwise unchanged client profile
apkeep -a com.icbc -d huawei-app-gallery -o region=CN .

# One package per line, same configured region for the entire batch
apkeep -c apps.csv -d huawei-app-gallery -o region=CN -r 2 .
```

## Region and availability

The only Huawei-specific option is `region=<two ASCII letters>`. Lowercase letters are
normalized to uppercase. The default is `IE`, matching the original implementation.
Empty, malformed, repeated, or unknown options are errors. This is a service-country
selector, not a change to language, device, firmware, client version, or network location.
There is **no automatic fallback** to another country.

A successful API response with an empty list means **unavailable in the configured
region/client profile**. It does not establish that the package is absent everywhere.
For example, a package may be offered in CN but not IE. A nonempty list with no unique
matching package is instead an invalid API response; another package is never downloaded.

The update-check API only offers the latest APK for this particular client profile.
This is not a guarantee of the globally newest public release. Historical downloads
(`-a package@version`, or a nonempty CSV version field selected with `-v`) are unsupported
and fail. For compatibility, `-l` with valid package IDs remains an informational
success explaining the limitation, without making network requests; it is not
an availability query or a returned version list.

## Integrity and output safety

Only complete single-APK metadata with numeric `isDiff=0`, a positive numeric size, and
a 64-character hexadecimal SHA256 is accepted. If `fullSize` is present, it must equal
`size`. Delta responses, split markers, missing/unknown required fields, ambiguous package
matches, and unsafe URLs are rejected **before** contacting the APK download server.
A supplied `fullDownUrl` is preferred to `downurl`; it must itself be valid. Only absolute
HTTP(S) URLs with a host and no credentials or fragment are accepted initially.
The production HTTP client also rejects private/reserved addresses and URL
credentials on every redirect; redirect chains are bounded.

The download is streamed to a randomly named temporary file in OUTPATH. The API size
and SHA256 must both match before the file is atomically published as `<package>.apk`,
without overwriting any existing path. Normal I/O, transfer, timeout, integrity, and
publication errors remove the temporary file. A forcibly killed process or machine crash
can leave an unadvertised temporary file; no incomplete `<package>.apk` is published.

An existing regular `<package>.apk` is left unchanged but **rejected as unverified**,
not reported as a successful skip. This deliberately changes the old skip behavior;
use a fresh output directory for a newly API-checked download. Existing symlinks
and nonregular output paths are errors. Package IDs are restricted to dot-separated ASCII
identifier segments and at most 240 bytes; slashes, empty segments and path traversal
are invalid. These checks protect filenames; they are not a complete Android manifest
validator.

SHA256 proves agreement with the API's metadata, not publisher identity or APK signing
certificate trust. The CLI does not inspect ZIP CRC, the Android manifest's package
or versionCode, APK signing certificates, or installability. A server can provide
matching size/SHA256 for non-APK bytes. These publisher/manifest checks belong to a
separate APK validation layer; they are not implemented or certified here.

## Diagnostics, bounds, and batch status

Huawei failures are printed directly to stderr, including when redirected or not attached
to a terminal. HTTP errors, API errors, malformed responses, unavailable-in-region results,
transport errors, timeouts, and integrity failures have distinct diagnostics. Raw response
bodies and signed download URLs are not printed. A batch processes all items and returns
nonzero if **any** item failed; successful downloads are retained. Parallelism must be at
least one and an empty download list fails.

The HTTP client bounds connections to 10 seconds, individual stalled reads to 30 seconds,
and each complete request/response to 600 seconds. The API JSON body is capped at 1 MiB;
the APK is bounded by the advertised size and an absolute 2 GiB limit. Declared
API sizes above that limit fail before the artifact request; streamed bytes are
checked against the accepted API size before writing. There are no automatic retries or cross-region
requests. `-s` adds the requested delay before each batch item; it is separate from network
timeouts. Use conservative batch sizes and parallelism to avoid overloading the service.


Production metadata and artifact requests enforce the shared public-network-only
policy (including DNS and redirects): no URL credentials, private, loopback,
link-local or reserved addresses; environment proxies are not used. There is no
CLI/environment bypass. See [RELIABILITY.md](RELIABILITY.md#public-http-boundary).
OUTPATH and its ancestors must be trusted and not concurrently replaced by other
users: path-based no-clobber publication is not a dirfd/openat defense against a
hostile local directory owner.

## Offline regression tests

```shell
cargo test --locked --offline --all-targets
```

Tests use real loopback TCP/HTTP servers and local files, not the public service. The
endpoint injection and subprocess diagnostic fixture are private test seams; there is no
Huawei endpoint override in the production CLI or environment.

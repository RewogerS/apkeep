use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use futures_util::StreamExt;
use indicatif::MultiProgress;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration as TokioDuration};

use crate::util::progress_bar::progress_wrapper;

#[cfg(test)]
#[path = "rustore_tests.rs"]
mod tests;

#[derive(Clone)]
struct RustoreOptions {
    headers: HeaderMap,
    device: Value,
}

impl RustoreOptions {
    fn parse(options: &HashMap<&str, &str>, env_version: Option<&str>) -> Result<Self, String> {
        let list = |key, default| {
            let values = options
                .get(key)
                .copied()
                .unwrap_or(default)
                .split(';')
                .collect::<Vec<_>>();
            if values.iter().any(|value| {
                value.is_empty()
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            }) {
                return Err(format!("Invalid RuStore option {key}: expected non-empty semicolon-separated tokens (letters, digits, _, -, .)"));
            }
            Ok(values)
        };
        let number = |key, default| {
            let value = options.get(key).copied().unwrap_or(default);
            value
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0 && value.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| format!("Invalid RuStore option {key}: expected a positive u32"))
        };
        let without_splits = options
            .get("without_splits")
            .copied()
            .unwrap_or("true")
            .parse::<bool>()
            .map_err(|_| {
                "Invalid RuStore option without_splits: expected true or false".to_string()
            })?;
        let mut headers = HeaderMap::new();
        let user_agent = options
            .get("user_agent")
            .copied()
            .unwrap_or("RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)");
        let version = options
            .get("rustore_version")
            .copied()
            .or(env_version)
            .unwrap_or("11500");
        if version.is_empty()
            || !version.bytes().all(|b| b.is_ascii_digit())
            || !version.bytes().any(|b| b != b'0')
        {
            let source = if options.contains_key("rustore_version") {
                "option rustore_version"
            } else {
                "APKEEP_RUSTORE_VER"
            };
            return Err(format!(
                "Invalid RuStore {source}: expected a positive decimal version code"
            ));
        }
        if user_agent.is_empty() {
            return Err(
                "Invalid RuStore option user_agent: expected a non-empty HTTP header".to_string(),
            );
        }
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(user_agent).map_err(|_| {
                "Invalid RuStore option user_agent: expected a valid HTTP header".to_string()
            })?,
        );
        headers.insert("ruStoreVerCode", HeaderValue::from_str(version)
            .map_err(|_| "Invalid RuStore rustore_version / APKEEP_RUSTORE_VER: expected a valid HTTP header".to_string())?);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        Ok(Self {
            headers,
            device: json!({
                "firstInstall": true,
                "mobileServices": list("mobile_services", "GMS;HMS")?,
                "supportedAbis": list("arch", "arm64-v8a")?,
                "screenDensity": number("screen_density", "420")?,
                "supportedLocales": list("locales", "en_US")?,
                "sdkVersion": number("sdk_version", "30")?,
                "withoutSplits": without_splits,
                "signatureFingerprint": null
            }),
        })
    }

    fn download_request_body(&self, app_id: u64) -> String {
        let mut body = self.device.clone();
        body["appId"] = json!(app_id);
        body.to_string()
    }
}

// Private transport seam for loopback tests; endpoints are not user-configurable.
async fn get_app_id_at(
    http_client: &reqwest::Client,
    headers: &HeaderMap,
    package_name: &str,
    endpoint: &str,
) -> Result<u64, String> {
    let lookup_url = format!("{}?packageNames={}", endpoint, package_name);

    let response = request_json(http_client.get(&lookup_url).headers(headers.clone())).await?;
    let apps = response
        .as_array()
        .ok_or_else(|| "RuStore malformed response: expected an app array".to_string())?;
    let app = apps
        .first()
        .ok_or_else(|| "RuStore app not found".to_string())?;
    app.get("id")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "RuStore malformed response: missing or invalid app id".to_string())
}

async fn get_download_url_at(
    http_client: &reqwest::Client,
    config: &RustoreOptions,
    app_id: u64,
    download_url: &str,
) -> Result<String, String> {
    let response = request_json(
        http_client
            .post(download_url)
            .headers(config.headers.clone())
            .body(config.download_request_body(app_id)),
    )
    .await?;
    let code = response
        .get("code")
        .and_then(Value::as_str)
        .filter(|code| !code.is_empty())
        .ok_or_else(|| "RuStore malformed response: missing or invalid result code".to_string())?;
    if code != "OK" {
        return Err("RuStore API rejected download request".to_string());
    }
    let urls = response
        .get("body")
        .and_then(|body| body.get("downloadUrls"))
        .and_then(Value::as_array)
        .ok_or_else(|| "RuStore malformed response: missing or invalid downloadUrls".to_string())?;
    // Never label the first member of a split set as a complete application.
    // Check even when withoutSplits=true, in case the server ignores that preference.
    if urls.len() > 1 {
        return Err("RuStore split APKs are not supported (multiple download URLs); use without_splits=true".to_string());
    }
    let url = urls
        .first()
        .and_then(|entry| entry.get("url"))
        .and_then(Value::as_str)
        .ok_or_else(|| "RuStore malformed response: missing download URL".to_string())?;
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| "RuStore malformed response: invalid download URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") || !parsed.has_host() {
        return Err("RuStore malformed response: expected an HTTP(S) download URL".to_string());
    }
    Ok(url.to_string())
}

// Fixed production limits; private typed injection keeps loopback tests fast without
// adding environment knobs or configurable public endpoints.
#[derive(Clone, Copy)]
struct NetworkPolicy {
    connect: TokioDuration,
    metadata_total: TokioDuration,
    metadata_max_bytes: usize,
    apk_total: TokioDuration,
    idle_read: TokioDuration,
    apk_max_bytes: u64,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            connect: TokioDuration::from_secs(15),
            metadata_total: TokioDuration::from_secs(60),
            metadata_max_bytes: 4 * 1024 * 1024,
            apk_total: TokioDuration::from_secs(15 * 60),
            idle_read: TokioDuration::from_secs(30),
            apk_max_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

async fn request_json(request: reqwest::RequestBuilder) -> Result<Value, String> {
    request_json_with_policy(request, NetworkPolicy::default()).await
}

fn network_client(
    policy: NetworkPolicy,
    total: TokioDuration,
) -> Result<reqwest::Client, &'static str> {
    reqwest::Client::builder()
        .connect_timeout(policy.connect)
        .read_timeout(policy.idle_read)
        .timeout(total)
        .build()
        .map_err(|_| "RuStore could not initialize HTTP client")
}

async fn request_json_with_policy(
    request: reqwest::RequestBuilder,
    policy: NetworkPolicy,
) -> Result<Value, String> {
    // An outer deadline preserves any shorter timeout on an injected client.
    // RequestBuilder::timeout would replace that deadline rather than cap it.
    tokio::time::timeout(policy.metadata_total, async {
        // Never format reqwest errors: URLs and signed query strings are untrusted.
        let mut response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                "RuStore network error: metadata request timed out".to_string()
            } else {
                "RuStore network error while sending request".to_string()
            }
        })?;
        if !response.status().is_success() {
            return Err(format!("RuStore HTTP {}", response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > policy.metadata_max_bytes as u64)
        {
            return Err("RuStore metadata size limit exceeded".to_string());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            if error.is_timeout() {
                "RuStore network error: metadata request timed out".to_string()
            } else {
                "RuStore network error while reading response".to_string()
            }
        })? {
            if chunk.len() > policy.metadata_max_bytes.saturating_sub(body.len()) {
                return Err("RuStore metadata size limit exceeded".to_string());
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body)
            .map_err(|_| "RuStore malformed response: invalid JSON".to_string())
    })
    .await
    .unwrap_or_else(|_| Err("RuStore network error: metadata request timed out".to_string()))
}

pub async fn download_apps(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    options: HashMap<&str, &str>,
) -> bool {
    download_apps_at(
        apps,
        parallel,
        sleep_duration,
        outpath,
        options,
        (
            "https://backapi.rustore.ru/applicationData/store-app",
            "https://backapi.rustore.ru/applicationData/v2/download-link",
        ),
    )
    .await
}

fn valid_package_name(package: &str) -> bool {
    package.split('.').all(|segment| {
        let mut bytes = segment.bytes();
        matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    })
}

// Private transport seam only: production endpoints are fixed above.
async fn download_apps_at(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    options: HashMap<&str, &str>,
    endpoints: (&str, &str),
) -> bool {
    if parallel == 0 {
        eprintln!("RuStore parallel must be greater than zero");
        return false;
    }
    if apps.is_empty() {
        eprintln!("No RuStore apps to download");
        return false;
    }
    let env_version = std::env::var("APKEEP_RUSTORE_VER");
    if !options.contains_key("rustore_version")
        && matches!(env_version, Err(std::env::VarError::NotUnicode(_)))
    {
        eprintln!("Invalid RuStore APKEEP_RUSTORE_VER: expected a positive decimal version code");
        return false;
    }
    let config = match RustoreOptions::parse(&options, env_version.as_deref().ok()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return false;
        }
    };
    let policy = NetworkPolicy::default();
    let http_client = match network_client(policy, policy.metadata_total) {
        Ok(client) => Rc::new(client),
        Err(error) => {
            eprintln!("{error}");
            return false;
        }
    };

    let mp = Rc::new(MultiProgress::new());
    let results = futures_util::stream::iter(apps.into_iter().map(|app| {
        let (app_id, app_version) = app;
        let http_client = Rc::clone(&http_client);
        let config = config.clone();
        let mp = Rc::clone(&mp);
        let mp_log = Rc::clone(&mp);
        async move {
            if !valid_package_name(&app_id) {
                mp_log.suspend(|| {
                    eprintln!(
                        "Invalid RuStore package name: expected dot-separated ASCII identifiers"
                    )
                });
                return false;
            }
            if app_version.is_none() {
                mp_log.suspend(|| println!("Downloading {}...", app_id));
                if sleep_duration > 0 {
                    sleep(TokioDuration::from_millis(sleep_duration)).await;
                }

                match get_app_id_at(&http_client, &config.headers, &app_id, endpoints.0).await {
                    Ok(rustore_app_id) => {
                        match get_download_url_at(
                            &http_client,
                            &config,
                            rustore_app_id,
                            endpoints.1,
                        )
                        .await
                        {
                            Ok(download_url) => {
                                download_apk(download_url, app_id.to_string(), outpath, mp).await
                            }
                            Err(error) => {
                                mp_log.suspend(|| {
                                    eprintln!(
                                        "Could not get download URL for {}: {}. Skipping...",
                                        app_id, error
                                    )
                                });
                                false
                            }
                        }
                    }
                    Err(error) => {
                        mp_log.suspend(|| {
                            eprintln!("Could not look up {}: {}. Skipping...", app_id, error)
                        });
                        false
                    }
                }
            } else {
                mp_log.suspend(|| {
                    eprintln!(
                        "Specific versions can not be downloaded from RuStore ({}). Skipping...",
                        app_id
                    )
                });
                false
            }
        }
    }))
    .buffer_unordered(parallel)
    .collect::<Vec<bool>>()
    .await;
    results.into_iter().all(|success| success)
}

async fn download_apk(
    download_url: String,
    app_string: String,
    outpath: &Path,
    mp: Rc<MultiProgress>,
) -> bool {
    download_apk_with_policy(
        download_url,
        app_string,
        outpath,
        mp,
        NetworkPolicy::default(),
    )
    .await
}

async fn download_apk_with_policy(
    download_url: String,
    app_string: String,
    outpath: &Path,
    mp: Rc<MultiProgress>,
    policy: NetworkPolicy,
) -> bool {
    use tokio::io::AsyncWriteExt;

    if !valid_package_name(&app_string) {
        mp.suspend(|| {
            eprintln!("Invalid RuStore package name: expected dot-separated ASCII identifiers")
        });
        return false;
    }
    let fname = format!("{}.apk", app_string);
    let destination = outpath.join(&fname);
    match std::fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_file() => {
            mp.suspend(|| println!("File already exists for {}. Skipping...", app_string));
            return true;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => {
            mp.suspend(|| {
                eprintln!(
                    "Unsafe or inaccessible RuStore output file for {}. Skipping...",
                    app_string
                )
            });
            return false;
        }
    }
    let client = match network_client(policy, policy.apk_total) {
        Ok(client) => client,
        Err(error) => {
            mp.suspend(|| eprintln!("{error}"));
            return false;
        }
    };
    for attempt in 1..=3 {
        let result = tokio::time::timeout(policy.apk_total, async {
            let mut response = client
                .get(&download_url)
                .timeout(policy.apk_total)
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(|error| {
                    if error.is_timeout() {
                        "RuStore APK request timed out"
                    } else {
                        "Invalid response"
                    }
                })?;
            let expected_length = response.content_length();
            if expected_length.is_some_and(|length| length > policy.apk_max_bytes) {
                return Err("RuStore APK size limit exceeded");
            }
            let temporary = tempfile::Builder::new()
                .prefix(".apkeep-rustore-")
                .tempfile_in(outpath)
                .map_err(|_| "Could not create download file")?;
            let mut file = tokio::fs::File::from_std(
                temporary
                    .reopen()
                    .map_err(|_| "Could not open download file")?,
            );
            let cb = expected_length
                .map(|length| progress_wrapper(Rc::clone(&mp))(fname.clone(), length));
            let mut downloaded = 0u64;
            while let Some(chunk) = response.chunk().await.map_err(|error| {
                if error.is_timeout() {
                    "RuStore APK request timed out"
                } else {
                    "Could not read download"
                }
            })? {
                let next_length = downloaded
                    .checked_add(chunk.len() as u64)
                    .filter(|length| *length <= policy.apk_max_bytes)
                    .ok_or("RuStore APK size limit exceeded")?;
                if expected_length.is_some_and(|length| next_length > length) {
                    return Err("RuStore APK Content-Length mismatch");
                }
                file.write_all(&chunk)
                    .await
                    .map_err(|_| "Could not write download file")?;
                downloaded = next_length;
                if let Some(cb) = &cb {
                    cb(downloaded);
                }
            }
            if downloaded == 0 {
                return Err("Empty download response");
            }
            if expected_length.is_some_and(|length| downloaded != length) {
                return Err("RuStore APK Content-Length mismatch");
            }
            file.flush()
                .await
                .map_err(|_| "Could not flush download file")?;
            drop(file);
            temporary
                .persist_noclobber(&destination)
                .map_err(|_| "Could not publish download file")?;
            Ok::<(), &str>(())
        })
        .await
        .unwrap_or(Err("RuStore APK request timed out"));
        match result {
            Ok(()) => {
                mp.suspend(|| println!("{} downloaded successfully!", app_string));
                return true;
            }
            Err(error) => {
                mp.suspend(|| eprintln!("{} for {} (attempt {}).", error, app_string, attempt))
            }
        }
    }
    false
}

pub async fn list_versions(apps: Vec<(String, Option<String>)>) {
    for app in apps {
        let (app_id, _) = app;
        println!("Versions available for {} on RuStore:", app_id);
        println!("| RuStore does not make old versions of apps available.");
    }
}

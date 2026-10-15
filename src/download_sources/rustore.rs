use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use futures_util::StreamExt;
use indicatif::MultiProgress;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration as TokioDuration};
use tokio_dl_stream_to_disk::{error::ErrorKind as TDSTDErrorKind, AsyncDownload};

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

async fn get_app_id(
    http_client: &reqwest::Client,
    headers: &HeaderMap,
    package_name: &str,
) -> Result<u64, String> {
    get_app_id_at(
        http_client,
        headers,
        package_name,
        "https://backapi.rustore.ru/applicationData/store-app",
    )
    .await
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

async fn get_download_url(
    http_client: &reqwest::Client,
    config: &RustoreOptions,
    app_id: u64,
) -> Result<String, String> {
    get_download_url_at(
        http_client,
        config,
        app_id,
        "https://backapi.rustore.ru/applicationData/v2/download-link",
    )
    .await
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

async fn request_json(request: reqwest::RequestBuilder) -> Result<Value, String> {
    // Do not format reqwest errors: they can contain URLs and signed query strings.
    let response = request
        .send()
        .await
        .map_err(|_| "RuStore network error while sending request".to_string())?;
    if !response.status().is_success() {
        return Err(format!("RuStore HTTP {}", response.status().as_u16()));
    }
    let body = response
        .text()
        .await
        .map_err(|_| "RuStore network error while reading response".to_string())?;
    serde_json::from_str(&body).map_err(|_| "RuStore malformed response: invalid JSON".to_string())
}

pub async fn download_apps(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    options: HashMap<&str, &str>,
) {
    let env_version = std::env::var("APKEEP_RUSTORE_VER");
    if !options.contains_key("rustore_version")
        && matches!(env_version, Err(std::env::VarError::NotUnicode(_)))
    {
        eprintln!("Invalid RuStore APKEEP_RUSTORE_VER: expected a positive decimal version code");
        return;
    }
    let config = match RustoreOptions::parse(&options, env_version.as_deref().ok()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    let http_client = Rc::new(reqwest::Client::new());

    let mp = Rc::new(MultiProgress::new());
    futures_util::stream::iter(apps.into_iter().map(|app| {
        let (app_id, app_version) = app;
        let http_client = Rc::clone(&http_client);
        let config = config.clone();
        let mp = Rc::clone(&mp);
        let mp_log = Rc::clone(&mp);
        async move {
            if app_version.is_none() {
                mp_log.suspend(|| println!("Downloading {}...", app_id));
                if sleep_duration > 0 {
                    sleep(TokioDuration::from_millis(sleep_duration)).await;
                }

                match get_app_id(&http_client, &config.headers, &app_id).await {
                    Ok(rustore_app_id) => {
                        match get_download_url(&http_client, &config, rustore_app_id).await {
                            Ok(download_url) => {
                                download_apk(download_url, app_id.to_string(), outpath, mp).await;
                            }
                            Err(error) => {
                                mp_log.suspend(|| {
                                    eprintln!(
                                        "Could not get download URL for {}: {}. Skipping...",
                                        app_id, error
                                    )
                                });
                            }
                        }
                    }
                    Err(error) => {
                        mp_log.suspend(|| {
                            eprintln!("Could not look up {}: {}. Skipping...", app_id, error)
                        });
                    }
                }
            } else {
                mp_log.suspend(|| {
                    eprintln!(
                        "Specific versions can not be downloaded from RuStore ({}@{}). Skipping...",
                        app_id,
                        app_version.unwrap()
                    )
                });
            }
        }
    }))
    .buffer_unordered(parallel)
    .collect::<Vec<()>>()
    .await;
}

async fn download_apk(
    download_url: String,
    app_string: String,
    outpath: &Path,
    mp: Rc<MultiProgress>,
) {
    let mp_log = Rc::clone(&mp);
    let mp = Rc::clone(&mp);
    let fname = format!("{}.apk", app_string);

    match AsyncDownload::new(&download_url, Path::new(outpath), &fname)
        .get()
        .await
    {
        Ok(mut dl) => {
            let length = dl.length();
            let cb = length.map(|length| progress_wrapper(mp)(fname.clone(), length));

            match dl.download(&cb).await {
                Ok(_) => mp_log.suspend(|| println!("{} downloaded successfully!", app_string)),
                Err(err) if matches!(err.kind(), TDSTDErrorKind::FileExists) => {
                    mp_log
                        .println(format!(
                            "File already exists for {}. Skipping...",
                            app_string
                        ))
                        .unwrap();
                }
                Err(err) if matches!(err.kind(), TDSTDErrorKind::PermissionDenied) => {
                    mp_log
                        .println(format!(
                            "Permission denied when attempting to write file for {}. Skipping...",
                            app_string
                        ))
                        .unwrap();
                }
                Err(_) => {
                    mp_log
                        .println(format!(
                            "An error has occurred attempting to download {}.  Retry #1...",
                            app_string
                        ))
                        .unwrap();
                    match AsyncDownload::new(&download_url, Path::new(outpath), &fname)
                        .download(&cb)
                        .await
                    {
                        Ok(_) => {
                            mp_log.suspend(|| println!("{} downloaded successfully!", app_string))
                        }
                        Err(_) => {
                            mp_log
                                .println(format!(
                                    "An error has occurred attempting to download {}.  Retry #2...",
                                    app_string
                                ))
                                .unwrap();
                            match AsyncDownload::new(&download_url, Path::new(outpath), &fname)
                                .download(&cb)
                                .await
                            {
                                Ok(_) => mp_log.suspend(|| {
                                    println!("{} downloaded successfully!", app_string)
                                }),
                                Err(_) => {
                                    mp_log.println(format!("An error has occurred attempting to download {}. Skipping...", app_string)).unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            mp_log
                .println(format!("Invalid response for {}. Skipping...", app_string))
                .unwrap();
        }
    }
}

pub async fn list_versions(apps: Vec<(String, Option<String>)>) {
    for app in apps {
        let (app_id, _) = app;
        println!("Versions available for {} on RuStore:", app_id);
        println!("| RuStore does not make old versions of apps available.");
    }
}

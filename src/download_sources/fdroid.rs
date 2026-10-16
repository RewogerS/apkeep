use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};

use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use base64::{engine::general_purpose as b64_general_purpose, Engine as _};
use cryptographic_message_syntax::{SignedData, SignerInfo};
use futures_util::StreamExt;
use indicatif::MultiProgress;
use regex::Regex;
use ring::digest::{Context, SHA256};
use serde_json::{json, Value};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use simple_error::SimpleError;
use tempfile::{tempdir, TempDir};
use tokio::time::{sleep, Duration};
use tokio_dl_stream_to_disk::AsyncDownload;
use x509_certificate::certificate::CapturedX509Certificate;

use crate::config;
use crate::consts;
use crate::util::{progress_bar::progress_wrapper, OutputFormat};
#[path = "fdroid/error.rs"]
mod error;
use error::Error as FDroidError;

#[cfg(test)]
#[path = "fdroid_tests.rs"]
mod tests;

async fn retrieve_index(
    options: &HashMap<&str, &str>,
    mp: Rc<MultiProgress>,
    output_format: OutputFormat,
) -> Result<Value, Box<dyn Error>> {
    let config_dir = config::config_dir()
        .map_err(|_| SimpleError::new("F-Droid cache directory unavailable."))?;
    retrieve_index_using_cache(options, mp, output_format, &config_dir).await
}

async fn retrieve_index_using_cache(
    options: &HashMap<&str, &str>,
    mp: Rc<MultiProgress>,
    output_format: OutputFormat,
    config_dir: &Path,
) -> Result<Value, Box<dyn Error>> {
    // Reverify the signed archive on every cache hit. Never reuse the old
    // unsigned JSON cache, shared with verify-index=false and different pins.
    let temp_dir = tempdir()?;
    let (repo, fingerprint, use_entry, verify_index) = repository_options(options)?;
    validate_output_directory(config_dir)?;
    let cache_dir = config_dir.join("fdroid-signed-archives");
    fs::create_dir_all(&cache_dir)?;
    validate_output_directory(&cache_dir)?;
    let cache_name = hex::encode(Sha256::digest(format!("{repo}:{use_entry}")));
    let cache = cache_dir.join(cache_name);
    let etag = download_repository_archive(&temp_dir, &repo, use_entry, Some(&cache)).await?;
    let files = extract_archive(&temp_dir, &temp_dir.path().join("jar.zip"))?;
    let json = verify_and_return_json(
        &temp_dir,
        &files,
        &fingerprint,
        verify_index,
        use_entry,
        Rc::clone(&mp),
    )?;
    let index = if use_entry {
        verify_and_return_index_from_entry(&temp_dir, &repo, &json, verify_index, mp, output_format)
            .await?
    } else {
        json
    };
    let parsed = serde_json::from_str(&index)?;
    // Cache only after the full verification/JSON chain has succeeded.
    if let Some(etag) = etag {
        write_cache_file(
            &cache.with_extension("jar"),
            &fs::read(temp_dir.path().join("jar.zip"))?,
        )?;
        write_cache_file(&cache.with_extension("etag"), etag.as_bytes())?;
    }
    Ok(parsed)
}

fn repository_options(
    options: &HashMap<&str, &str>,
) -> Result<(String, Vec<u8>, bool, bool), Box<dyn Error>> {
    let mut repo = consts::FDROID_REPO.to_owned();
    let mut fingerprint = consts::FDROID_INDEX_FINGERPRINT.to_vec();
    if let Some(value) = options.get("repo") {
        if let Some((address, pin)) = value.split_once("?fingerprint=") {
            repo = address.to_owned();
            fingerprint = hex::decode(pin)?;
            if fingerprint.len() != 32 {
                return Err(Box::new(SimpleError::new(
                    "F-Droid fingerprint must contain exactly 32 bytes.",
                )));
            }
        } else {
            repo = value.to_string();
        }
    }
    let url = reqwest::Url::parse(&repo)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Box::new(SimpleError::new(
            "Invalid F-Droid repository URL.",
        )));
    }
    let use_entry = !options
        .get("use_entry")
        .is_some_and(|v| *v == "0" || v.eq_ignore_ascii_case("false"));
    let verify_index = options.get("verify-index") != Some(&"false");
    Ok((repo, fingerprint, use_entry, verify_index))
}

// Allowlist, not URL redaction: redirects and third-party error chains may
// contain signed URLs, bodies or credentials. Never Display/Debug/source an
// untrusted error at a user boundary, even if its outer message seems benign.
fn safe_error_summary(err: &(dyn Error + 'static)) -> String {
    if let Some(err) = err.downcast_ref::<reqwest::Error>() {
        if let Some(status) = err.status() {
            return format!("HTTP status {}", status.as_u16());
        }
        return if err.is_timeout() {
            "network request timed out"
        } else if err.is_connect() {
            "network connection failed"
        } else if err.is_body() || err.is_decode() {
            "network response transfer failed"
        } else {
            "network request failed"
        }
        .to_owned();
    }
    if err.is::<tokio::time::error::Elapsed>() {
        "operation timed out"
    } else if err.is::<io::Error>() {
        "filesystem or transfer I/O failed"
    } else if err.is::<serde_json::Error>() {
        "invalid repository JSON"
    } else if err.is::<zip::result::ZipError>() {
        "invalid repository archive"
    } else {
        "repository data validation or verification failed"
    }
    .to_owned()
}

fn print_error(err_msg: &str, output_format: OutputFormat) {
    eprintln!("{}", err_msg);
    if output_format.is_json() {
        println!("{}", json!({"error": err_msg}));
    }
}

pub async fn download_apps(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    options: HashMap<&str, &str>,
) -> bool {
    let (apps, all_valid) = valid_requests(apps);
    if options
        .get("arch")
        .is_some_and(|value| !safe_component(value))
    {
        eprintln!("Invalid F-Droid architecture.");
        return false;
    }
    if apps.is_empty() {
        eprintln!("No F-Droid applications requested.");
        return false;
    }
    if let Err(err) = validate_output_directory(outpath) {
        eprintln!("F-Droid output path: {}", safe_error_summary(err.as_ref()));
        return false;
    }
    let mp = Rc::new(MultiProgress::new());
    let index = match retrieve_index(&options, Rc::clone(&mp), OutputFormat::Plaintext).await {
        Ok(index) => index,
        Err(err) => {
            eprintln!(
                "Could not retrieve verified F-Droid package index: {}",
                safe_error_summary(err.as_ref())
            );
            return false;
        }
    };
    let downloaded = download_from_index(
        index,
        apps,
        parallel,
        sleep_duration,
        outpath,
        options.get("arch").map(|value| value.to_string()),
        mp,
    )
    .await;
    downloaded && all_valid
}

fn valid_requests(apps: Vec<(String, Option<String>)>) -> (Vec<(String, Option<String>)>, bool) {
    let mut all_valid = true;
    let apps = apps
        .into_iter()
        .filter(|(id, version)| {
            let valid = safe_component(id) && version.as_deref().is_none_or(safe_component);
            if !valid {
                eprintln!("Invalid F-Droid package/version.");
                all_valid = false;
            }
            valid
        })
        .collect();
    (apps, all_valid)
}

async fn download_from_index(
    index: Value,
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    arch: Option<String>,
    mp: Rc<MultiProgress>,
) -> bool {
    let requested = apps.len();
    if requested == 0 {
        eprintln!("No F-Droid applications requested.");
        return false;
    }
    if let Err(err) = validate_output_directory(outpath) {
        eprintln!("F-Droid output path: {}", safe_error_summary(err.as_ref()));
        return false;
    }
    let (jobs, repo) =
        match parse_json_for_download_information(index, apps, arch.clone(), Rc::clone(&mp)) {
            Ok(info) => info,
            Err(err) => {
                eprintln!(
                    "Could not parse F-Droid package index: {}",
                    safe_error_summary(&err)
                );
                return false;
            }
        };
    let complete_selection = jobs.len() == requested;
    if !complete_selection {
        eprintln!("Some requested F-Droid applications or variants are unavailable/invalid.");
    }
    let outcomes = futures_util::stream::iter(jobs.into_iter().map(|(id, version, name, hash)| {
        let repo = repo.clone();
        let arch = arch.clone();
        let mp = Rc::clone(&mp);
        async move {
            let mut label = id;
            if let Some(version) = version {
                label.push('@');
                label.push_str(&version);
            }
            if let Some(arch) = arch {
                label.push('@');
                label.push_str(&arch);
            }
            let filename = format!("{label}.apk");
            if filename.len() > 255 {
                eprintln!("F-Droid output filename too long: {label}");
                return false;
            }
            let target = outpath.join(&filename);
            match fs::symlink_metadata(&target) {
                Ok(metadata) => {
                    // Existing regular files are successful skips only when the
                    // complete expected digest matches. Symlinks are never skips.
                    let ok = if metadata.file_type().is_file() {
                        match file_matches_sha256(&target, &hash) {
                            Ok(matches) => matches,
                            Err(err) => {
                                eprintln!(
                                    "Could not verify existing {}: {}",
                                    target.display(),
                                    safe_error_summary(&err)
                                );
                                false
                            }
                        }
                    } else {
                        false
                    };
                    if !ok {
                        eprintln!(
                            "Unsafe or mismatching existing F-Droid destination: {}",
                            target.display()
                        );
                    }
                    return ok;
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => {
                    eprintln!(
                        "Could not inspect {}: {}",
                        target.display(),
                        safe_error_summary(&err)
                    );
                    return false;
                }
            }
            if sleep_duration > 0 {
                sleep(Duration::from_millis(sleep_duration)).await;
            }
            let url = format!(
                "{}/{}",
                repo.trim_end_matches('/'),
                name.trim_start_matches('/')
            );
            for attempt in 1..=3 {
                let transfer = async {
                    let mut dl = AsyncDownload::new(&url, outpath, &filename).get().await?;
                    let cb = dl
                        .length()
                        .map(|length| progress_wrapper(Rc::clone(&mp))(filename.clone(), length));
                    dl.download_and_verify_sha256(&hash, &cb)
                        .await
                        .map_err(|_| SimpleError::new("APK SHA256-verified transfer failed."))?;
                    Ok::<(), Box<dyn Error>>(())
                };
                match tokio::time::timeout(REQUEST_TIMEOUT, transfer).await {
                    Ok(Ok(())) => {
                        println!("{label} downloaded successfully!");
                        return true;
                    }
                    Ok(Err(err)) => eprintln!(
                        "F-Droid {label}, attempt {attempt}/3: {}",
                        safe_error_summary(err.as_ref())
                    ),
                    Err(_) => eprintln!("F-Droid {label}, attempt {attempt}/3 timed out."),
                }
            }
            false
        }
    }))
    .buffer_unordered(parallel.max(1))
    .collect::<Vec<bool>>()
    .await;
    complete_selection && !outcomes.is_empty() && outcomes.into_iter().all(|ok| ok)
}

fn file_matches_sha256(path: &Path, expected: &[u8]) -> Result<bool, io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(hasher.finalize().as_slice() == expected)
}

type DownloadInformation = (Vec<(String, Option<String>, String, Vec<u8>)>, String);
/// This currently works for `index-v1.json` as well as an index with version `20002`.  It is
/// flexible enough to parse either, and may work on future index versions as well.  Since `sha256`
/// digests are checked before proceeding, I don't foresee this having an insecure failure mode, so
/// checking the index version and making the parsing overly brittle has no substantive advantage.
fn parse_json_for_download_information(
    index: Value,
    apps: Vec<(String, Option<String>)>,
    app_arch: Option<String>,
    _mp_log: Rc<MultiProgress>,
) -> Result<DownloadInformation, FDroidError> {
    let index_map = index.as_object().ok_or(FDroidError::Dummy)?;
    let repo_address = index_map
        .get("repo")
        .ok_or(FDroidError::Dummy)?
        .get("address")
        .ok_or(FDroidError::Dummy)?
        .as_str()
        .ok_or(FDroidError::Dummy)?;

    let packages = index_map
        .get("packages")
        .ok_or(FDroidError::Dummy)?
        .as_object()
        .ok_or(FDroidError::Dummy)?;

    let fdroid_apps: Vec<(String, Option<String>, String, Vec<u8>)> = apps
        .into_iter()
        .map(|app| {
            let (app_id, app_version) = app;
            if !safe_component(&app_id)
                || app_version
                    .as_deref()
                    .is_some_and(|value| !safe_component(value))
                || app_arch
                    .as_deref()
                    .is_some_and(|value| !safe_component(value))
            {
                eprintln!("Unsafe F-Droid package/version/architecture.");
                return None;
            }
            match packages.get(&app_id) {
                Some(Value::Array(app_array)) => {
                    for single_app in app_array {
                        if let Value::Object(fdroid_app) = single_app {
                            if let Some(Value::String(version_name)) = fdroid_app.get("versionName")
                            {
                                if app_version
                                    .as_ref()
                                    .is_none_or(|requested| version_name == requested)
                                {
                                    if let (
                                        Some(Value::String(filename)),
                                        Some(Value::String(hash)),
                                    ) = (fdroid_app.get("apkName"), fdroid_app.get("hash"))
                                    {
                                        if let Ok(hash) = hex::decode(hash.to_string()) {
                                            if !safe_repo_name(
                                                filename.strip_prefix('/').unwrap_or(filename),
                                            ) || hash.len() != 32
                                            {
                                                continue;
                                            }
                                            if let Some(arch) = &app_arch {
                                                if let Some(Value::Array(nativecode_array)) =
                                                    fdroid_app.get("nativecode")
                                                {
                                                    if nativecode_array.iter().any(|value| {
                                                        if let Value::String(value_str) = value {
                                                            value_str == arch
                                                        } else {
                                                            false
                                                        }
                                                    }) {
                                                        return Some((
                                                            app_id,
                                                            app_version,
                                                            filename.to_string(),
                                                            hash,
                                                        ));
                                                    }
                                                }
                                            } else {
                                                return Some((
                                                    app_id,
                                                    app_version,
                                                    filename.to_string(),
                                                    hash,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let arch_str = app_arch
                        .as_ref()
                        .map_or("".to_string(), |x| format!(" {}", x));
                    eprintln!(
                        "Could not find version {}{} of {}.",
                        app_version.as_deref().unwrap_or("latest"),
                        arch_str,
                        app_id
                    );
                    return None;
                }
                Some(Value::Object(app_object)) => {
                    if let Some(Value::Object(versions)) = app_object.get("versions") {
                        let mut latest_version = 0;
                        let mut filename = String::new();
                        let mut hash = Vec::new();
                        for (_, version_value) in versions {
                            if let Value::Object(version) = version_value {
                                if let (Some(Value::Object(manifest)), Some(Value::Object(file))) =
                                    (version.get("manifest"), version.get("file"))
                                {
                                    if let Some(arch) = &app_arch {
                                        if !manifest
                                            .get("nativecode")
                                            .and_then(Value::as_array)
                                            .is_some_and(|abis| {
                                                abis.iter()
                                                    .any(|abi| abi.as_str() == Some(arch.as_str()))
                                            })
                                        {
                                            continue;
                                        }
                                    }
                                    if let (
                                        Some(Value::String(name)),
                                        Some(Value::String(sha256)),
                                    ) = (file.get("name"), file.get("sha256"))
                                    {
                                        let sha256 = match hex::decode(sha256) {
                                            Ok(hash)
                                                if hash.len() == 32
                                                    && safe_repo_name(
                                                        name.strip_prefix('/').unwrap_or(name),
                                                    ) =>
                                            {
                                                hash
                                            }
                                            _ => continue,
                                        };
                                        if app_version.is_some() {
                                            if let Some(Value::String(version_name)) =
                                                manifest.get("versionName")
                                            {
                                                if app_version.as_ref() == Some(version_name) {
                                                    return Some((
                                                        app_id,
                                                        app_version,
                                                        name.to_string(),
                                                        sha256,
                                                    ));
                                                }
                                            }
                                        } else {
                                            if let Some(Value::Number(version_code_number)) =
                                                manifest.get("versionCode")
                                            {
                                                if let Some(version_code) =
                                                    version_code_number.as_u64()
                                                {
                                                    if version_code > latest_version {
                                                        latest_version = version_code;
                                                        filename = name.to_string();
                                                        hash = sha256;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if app_version.is_none() && !filename.is_empty() {
                            return Some((app_id, app_version, filename, hash));
                        }
                    }
                }
                _ => eprintln!("Could not find {} in package list.", app_id),
            }
            None
        })
        .flatten()
        .collect();

    Ok((fdroid_apps, repo_address.to_string()))
}

pub async fn list_versions(
    apps: Vec<(String, Option<String>)>,
    options: HashMap<&str, &str>,
) -> bool {
    let (apps, all_valid) = valid_requests(apps);
    let output_format = if options
        .get("output_format")
        .is_some_and(|v| v.eq_ignore_ascii_case("json"))
    {
        OutputFormat::Json
    } else {
        OutputFormat::Plaintext
    };
    if apps.is_empty() {
        print_error("No F-Droid applications requested.", output_format);
        return false;
    }
    let index = match retrieve_index(
        &options,
        Rc::new(MultiProgress::new()),
        output_format.clone(),
    )
    .await
    {
        Ok(index) => index,
        Err(err) => {
            print_error(
                &format!(
                    "Could not retrieve F-Droid package index: {}",
                    safe_error_summary(err.as_ref())
                ),
                output_format,
            );
            return false;
        }
    };
    match parse_json_display_versions(index, apps, output_format.clone()) {
        Ok(()) => all_valid,
        Err(err) => {
            // The parser has already emitted its single JSON result, including
            // per-app errors. A second JSON object would corrupt stdout.
            eprintln!(
                "Could not list all F-Droid versions: {}",
                safe_error_summary(&err)
            );
            false
        }
    }
}

/// The comments for `parse_json_for_download_information` apply here, too.
fn parse_json_display_versions(
    index: Value,
    apps: Vec<(String, Option<String>)>,
    output_format: OutputFormat,
) -> Result<(), FDroidError> {
    let packages = match index.get("packages").and_then(Value::as_object) {
        Some(packages) => packages,
        None => {
            print_error("Invalid F-Droid package index.", output_format);
            return Err(FDroidError::Dummy);
        }
    };
    let mut all_ok = !apps.is_empty();
    let mut json_root = HashMap::new();
    for (app_id, version) in apps {
        let mut versions = HashSet::new();
        let valid = safe_component(&app_id) && version.as_deref().is_none_or(safe_component);
        if valid {
            match packages.get(&app_id) {
                Some(Value::Array(entries)) => {
                    for entry in entries {
                        if let Some(name) = entry.get("versionName").and_then(Value::as_str) {
                            if !name.is_empty() {
                                versions.insert(name.to_owned());
                            }
                        }
                    }
                }
                Some(Value::Object(package)) => {
                    if let Some(entries) = package.get("versions").and_then(Value::as_object) {
                        for entry in entries.values() {
                            if let Some(name) = entry
                                .get("manifest")
                                .and_then(|m| m.get("versionName"))
                                .and_then(Value::as_str)
                            {
                                if !name.is_empty() {
                                    versions.insert(name.to_owned());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if versions.is_empty() {
            all_ok = false;
            eprintln!("No F-Droid versions available for {app_id} (invalid, missing or malformed package).");
            json_root.insert(
                app_id,
                json!({"error": "No versions available or invalid request."}),
            );
            continue;
        }
        let mut versions: Vec<_> = versions.into_iter().collect();
        versions.sort();
        if output_format.is_plaintext() {
            println!(
                "Versions available for {app_id} on F-Droid:\n| {}",
                versions
                    .iter()
                    .map(|version| {
                        version
                            .chars()
                            .map(|ch| {
                                if ch.is_control() {
                                    ch.escape_default().to_string()
                                } else {
                                    ch.to_string()
                                }
                            })
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        json_root.insert(app_id, json!({"available_versions": versions.into_iter().map(|v| json!({"version": v})).collect::<Vec<_>>()}));
    }
    if output_format.is_json() {
        println!("{}", json!({"source": "F-Droid", "apps": json_root}));
    }
    if all_ok {
        Ok(())
    } else {
        Err(FDroidError::Dummy)
    }
}

fn verify_and_return_json(
    dir: &TempDir,
    files: &[String],
    fingerprint: &[u8],
    verify_index: bool,
    use_entry: bool,
    mp: Rc<MultiProgress>,
) -> Result<String, Box<dyn Error>> {
    let re = Regex::new(consts::FDROID_SIGNATURE_BLOCK_FILE_REGEX)?;
    let cert_file = {
        let mut cert_files = vec![];
        for file in files {
            if re.is_match(file) {
                cert_files.push(file.clone());
            }
        }
        if cert_files.is_empty() {
            return Err(Box::new(SimpleError::new(
                "Found no certificate file for F-Droid repository.",
            )));
        }
        if cert_files.len() > 1 {
            return Err(Box::new(SimpleError::new(
                "Found multiple certificate files for F-Droid repository.",
            )));
        }
        dir.path().join(cert_files[0].clone())
    };
    let signed_file = {
        let mut signed_file = cert_file.clone();
        signed_file.set_extension("SF");
        signed_file
    };

    let signed_content = fs::read(signed_file)?;

    if verify_index {
        mp.println("Verifying...")?;
        let signed_data = get_signed_data_from_cert_file(cert_file)?;
        let signer_info = signed_data
            .signers()
            .next()
            .ok_or_else(|| SimpleError::new("No F-Droid signer."))?;
        signer_info.verify_signature_with_signed_data_and_content(&signed_data, &signed_content)?;
        let cert = signed_data
            .certificates()
            .next()
            .ok_or_else(|| SimpleError::new("No F-Droid certificate."))?;
        let mut context = Context::new(&SHA256);
        context.update(&cert.encode_ber()?);
        let cert_fingerprint = context.finish();
        if cert_fingerprint.as_ref() != fingerprint {
            return Err(Box::new(SimpleError::new("Fingerprint of the key contained in the F-Droid repository does not match the expected fingerprint.")));
        };
    }

    let signed_file_string = std::str::from_utf8(&signed_content)?;
    let manifest_file = dir.path().join("META-INF").join("MANIFEST.MF");
    let manifest_file_data = fs::read(manifest_file)?;
    if verify_index {
        let (signed_file_regex, sha_algorithm_name) = if use_entry {
            (
                Regex::new(r"\r\nSHA-256-Digest-Manifest: (.*)\r\n")?,
                "sha256sum",
            )
        } else {
            (
                Regex::new(r"\r\nSHA1-Digest-Manifest: (.*)\r\n")?,
                "sha1sum",
            )
        };
        let signed_file_manifest_shasum = b64_general_purpose::STANDARD.decode(
            match signed_file_regex.captures(signed_file_string) {
                Some(caps) if caps.len() >= 2 => caps.get(1).ok_or(FDroidError::Dummy)?.as_str(),
                _ => {
                    return Err(Box::new(SimpleError::new(format!(
                        "Could not retrieve the manifest {} from the signed file.",
                        sha_algorithm_name
                    ))));
                }
            },
        )?;
        let actual_manifest_shasum = if use_entry {
            let mut hasher = Sha256::new();
            hasher.update(manifest_file_data.clone());
            Vec::from(hasher.finalize().as_slice())
        } else {
            let mut hasher = Sha1::new();
            hasher.update(manifest_file_data.clone());
            Vec::from(hasher.finalize().as_slice())
        };
        if signed_file_manifest_shasum != actual_manifest_shasum[..] {
            return Err(Box::new(SimpleError::new(format!(
                "The manifest {} from the signed file does not match the actual manifest {}.",
                sha_algorithm_name, sha_algorithm_name
            ))));
        }
    }

    let manifest_file_string = std::str::from_utf8(&manifest_file_data)?;
    let json_file = if use_entry {
        dir.path().join("entry.json")
    } else {
        dir.path().join("index-v1.json")
    };
    let json_file_data = fs::read(json_file)?;
    if verify_index {
        let (manifest_file_regex, file_algo) = if use_entry {
            (
                Regex::new(r"\r\nName: entry\.json\r\nSHA-256-Digest: (.*)\r\n")?,
                "entry sha256sum",
            )
        } else {
            (
                Regex::new(r"\r\nName: index-v1\.json\r\nSHA1-Digest: (.*)\r\n")?,
                "index sha1sum",
            )
        };
        let manifest_file_shasum = b64_general_purpose::STANDARD.decode(
            match manifest_file_regex.captures(manifest_file_string) {
                Some(caps) if caps.len() >= 2 => caps.get(1).ok_or(FDroidError::Dummy)?.as_str(),
                _ => {
                    return Err(Box::new(SimpleError::new(format!(
                        "Could not retrieve the {} from the manifest file.",
                        file_algo
                    ))));
                }
            },
        )?;
        let actual_shasum = if use_entry {
            let mut hasher = Sha256::new();
            hasher.update(json_file_data.clone());
            Vec::from(hasher.finalize().as_slice())
        } else {
            let mut hasher = Sha1::new();
            hasher.update(json_file_data.clone());
            Vec::from(hasher.finalize().as_slice())
        };
        if manifest_file_shasum != actual_shasum[..] {
            return Err(Box::new(SimpleError::new(format!(
                "The {} from the manifest file does not match the actual {}.",
                file_algo, file_algo
            ))));
        }
    }

    Ok(String::from(std::str::from_utf8(&json_file_data)?))
}

async fn verify_and_return_index_from_entry(
    dir: &TempDir,
    repo: &str,
    json: &str,
    verify_index: bool,
    _mp: Rc<MultiProgress>,
    _output_format: OutputFormat,
) -> Result<String, Box<dyn Error>> {
    let entry: Value = serde_json::from_str(json)?;
    let index = entry.get("index").ok_or(FDroidError::Dummy)?;
    let name = index
        .get("name")
        .and_then(Value::as_str)
        .ok_or(FDroidError::Dummy)?;
    let index_name = name.strip_prefix('/').unwrap_or(name);
    if !safe_repo_name(index_name) {
        return Err(Box::new(SimpleError::new(
            "Invalid F-Droid entry index name.",
        )));
    }
    let hash = index
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or(FDroidError::Dummy)?;
    let hash = if verify_index {
        match hex::decode(hash) {
            Ok(hash) if hash.len() == 32 => Some(hash),
            _ => {
                return Err(Box::new(SimpleError::new(
                    "Invalid F-Droid entry index sha256: expected 32 bytes of hex.",
                )))
            }
        }
    } else {
        None
    };
    let index_url = format!("{}/{}", repo.trim_end_matches('/'), index_name);
    // The vendor API checks the digest before atomically publishing the file.
    // Never use the legacy download-then-hash API for authenticated content.
    download_metadata(
        &index_url,
        dir.path(),
        index_name,
        hash.as_deref(),
        MAX_METADATA_BYTES,
        REQUEST_TIMEOUT,
    )
    .await?;
    read_bounded_string(&dir.path().join(index_name), MAX_METADATA_BYTES)
}

const MAX_METADATA_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 200
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
}

fn safe_repo_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && value
            .split('/')
            .all(|part| safe_component(part) && !part.contains('+'))
}

fn read_bounded_string(path: &Path, limit: u64) -> Result<String, Box<dyn Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err(Box::new(SimpleError::new(
            "Invalid or oversized F-Droid metadata file.",
        )));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Box::new(SimpleError::new(
            "F-Droid metadata exceeds size limit.",
        )));
    }
    Ok(String::from_utf8(bytes)?)
}

async fn download_metadata(
    url: &str,
    dir: &Path,
    name: &str,
    expected: Option<&[u8]>,
    limit: u64,
    deadline: Duration,
) -> Result<(), Box<dyn Error>> {
    if !safe_repo_name(name) {
        return Err(Box::new(SimpleError::new("Invalid F-Droid metadata path.")));
    }
    let target = dir.join(name);
    let parent = target.parent().ok_or(FDroidError::Dummy)?;
    fs::create_dir_all(parent)?;
    validate_output_directory(parent)?;
    let leaf = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FDroidError::Dummy)?;
    let exceeded = Rc::new(std::cell::Cell::new(false));
    let notify = Rc::new(tokio::sync::Notify::new());
    let signal = Rc::clone(&notify);
    let flag = Rc::clone(&exceeded);
    let cb: Option<Box<dyn Fn(u64)>> = Some(Box::new(move |count| {
        if count > limit {
            flag.set(true);
            signal.notify_one();
        }
    }));
    let download = async {
        let mut dl = AsyncDownload::new(url, parent, leaf).get().await?;
        if dl.length().is_some_and(|length| length > limit) {
            return Err::<(), Box<dyn Error>>(Box::new(SimpleError::new(
                "F-Droid metadata exceeds size limit.",
            )));
        }
        if let Some(expected) = expected {
            dl.download_and_verify_sha256(expected, &cb)
                .await
                .map_err(|_| SimpleError::new("Index SHA256-verified transfer failed."))?;
        } else {
            dl.download(&cb)
                .await
                .map_err(|_| SimpleError::new("Index transfer failed."))?;
        }
        Ok(())
    };
    let result = tokio::time::timeout(deadline, async {
        tokio::select! {
            biased;
            _ = notify.notified() => Err(Box::new(SimpleError::new("F-Droid metadata exceeds size limit.")) as Box<dyn Error>),
            result = download => result,
        }
    }).await?;
    if exceeded.get() {
        // The callback can fire on the last chunk. Even if the download future
        // wins that poll, an oversized metadata file is never used.
        if fs::symlink_metadata(&target).is_ok() {
            fs::remove_file(&target)?;
        }
        return Err(Box::new(SimpleError::new(
            "F-Droid metadata exceeds size limit.",
        )));
    }
    result
}

fn validate_output_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir | std::path::Component::Prefix(_) => {
                return Err(Box::new(SimpleError::new(
                    "Unsafe F-Droid output directory.",
                )));
            }
            std::path::Component::CurDir => current.push("."),
            _ => current.push(component.as_os_str()),
        }
        if !fs::symlink_metadata(&current)?.file_type().is_dir() {
            return Err(Box::new(SimpleError::new(
                "F-Droid output directory must contain only real directories (no symlinks).",
            )));
        }
    }
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(Box::new(SimpleError::new(
            "F-Droid output path is not a directory.",
        )));
    }
    Ok(())
}

fn get_signed_data_from_cert_file(
    signature_block_file: PathBuf,
) -> Result<SignedData, Box<dyn Error>> {
    let bytes = fs::read(signature_block_file)?;
    match SignedData::parse_ber(&bytes) {
        Ok(signed_data) => {
            let certificates: Vec<&CapturedX509Certificate> = signed_data.certificates().collect();
            if certificates.len() > 1 {
                return Err(Box::new(SimpleError::new(
                    "Too many certificates provided.",
                )));
            }
            if certificates.is_empty() {
                return Err(Box::new(SimpleError::new("No certificate provided.")));
            }
            let signatories: Vec<&SignerInfo> = signed_data.signers().collect();
            if signatories.len() > 1 {
                return Err(Box::new(SimpleError::new("Too many signatories provided.")));
            }
            if signatories.is_empty() {
                return Err(Box::new(SimpleError::new("No signatories provided.")));
            }
            Ok(signed_data)
        }
        Err(err) => Err(Box::new(err)),
    }
}

async fn download_repository_archive(
    dir: &TempDir,
    repo: &str,
    use_entry: bool,
    cache: Option<&Path>,
) -> Result<Option<String>, Box<dyn Error>> {
    let url = format!(
        "{}/{}",
        repo.trim_end_matches('/'),
        if use_entry {
            "entry.jar"
        } else {
            "index-v1.jar"
        }
    );
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()?;
    let mut request = client.get(&url);
    if let Some(cache) = cache {
        let jar = cache.with_extension("jar");
        let etag = cache.with_extension("etag");
        let jar_exists = regular_cache_exists(&jar)?;
        let etag_exists = regular_cache_exists(&etag)?;
        if jar_exists && etag_exists {
            let tag = read_bounded_string(&etag, 8192)?;
            request = request.header(reqwest::header::IF_NONE_MATCH, tag);
        }
    }
    let response = request.send().await?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        let cache = cache
            .ok_or_else(|| SimpleError::new("Unexpected F-Droid 304 without signed cache."))?;
        let cached = cache.with_extension("jar");
        if !regular_cache_exists(&cached)? || fs::metadata(&cached)?.len() > MAX_METADATA_BYTES {
            return Err(Box::new(SimpleError::new("Invalid F-Droid signed cache.")));
        }
        fs::copy(&cached, dir.path().join("jar.zip"))?;
        return Ok(None);
    }
    let response = response.error_for_status()?;
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .map(|tag| tag.to_str().map(str::to_owned))
        .transpose()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_METADATA_BYTES)
    {
        return Err(Box::new(SimpleError::new(
            "F-Droid repository archive exceeds size limit.",
        )));
    }
    let jar = dir.path().join("jar.zip");
    let mut output = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&jar)
        .await?;
    let mut stream = response.bytes_stream();
    let mut received = 0u64;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        received += chunk.len() as u64;
        if received > MAX_METADATA_BYTES {
            return Err(Box::new(SimpleError::new(
                "F-Droid repository archive exceeds size limit.",
            )));
        }
        output.write_all(&chunk).await?;
    }
    output.flush().await?;
    drop(output);
    Ok(etag)
}

fn regular_cache_exists(path: &Path) -> Result<bool, Box<dyn Error>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(Box::new(SimpleError::new(
            "Unsafe nonregular F-Droid cache file.",
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(Box::new(err)),
    }
}

fn write_cache_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    regular_cache_exists(path)?;
    let parent = path.parent().ok_or(FDroidError::Dummy)?;
    validate_output_directory(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.persist(path)?;
    Ok(())
}

fn extract_archive(dir: &TempDir, jar: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut archive = zip::ZipArchive::new(File::open(jar)?)?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(Box::new(SimpleError::new(
            "Too many F-Droid archive entries.",
        )));
    }
    let mut files = Vec::new();
    let mut extracted = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name()?.trim_end_matches('/').to_string();
        if !safe_repo_name(&name)
            || file.enclosed_name().is_none()
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(Box::new(SimpleError::new("Unsafe F-Droid archive entry.")));
        }
        let path = dir.path().join(&name);
        if file.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if file.size() > MAX_METADATA_BYTES - extracted {
            return Err(Box::new(SimpleError::new(
                "F-Droid extracted metadata exceeds size limit.",
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            validate_output_directory(parent)?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let length = io::copy(
            &mut (&mut file).take(MAX_METADATA_BYTES - extracted + 1),
            &mut output,
        )?;
        extracted += length;
        if extracted > MAX_METADATA_BYTES {
            return Err(Box::new(SimpleError::new(
                "F-Droid extracted metadata exceeds size limit.",
            )));
        }
        output.flush()?;
        files.push(name);
    }
    Ok(files)
}

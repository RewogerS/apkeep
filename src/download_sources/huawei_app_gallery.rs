//! Huawei AppGallery support for the latest APK offered to the configured client profile.

pub fn region(options: Option<&str>) -> Result<String, &'static str> {
    match options {
        None => Ok("IE".to_owned()),
        Some(options) => match options.strip_prefix("region=") {
            Some(value) if value.len() == 2 && value.bytes().all(|c| c.is_ascii_alphabetic()) => {
                Ok(value.to_ascii_uppercase())
            }
            _ => Err("invalid options; expected region=<two ASCII letters>"),
        },
    }
}

fn client_api_body(app: &str, region: &str) -> String {
    // Retain the original client profile; only the explicitly selected country changes.
    let profile = [
        ("agVersion", "12.0.1"),
        ("brand", "Android"),
        ("buildNumber", "QQ2A.200405.005.2020.04.07.17"),
        ("density", "420"),
        (
            "deviceSpecParams",
            r#"{"abis":"arm64-v8a,armeabi-v7a,armeabi","deviceFeatures":"U,P,B,0c,e,0J,p,a,b,04,m,android.hardware.wifi.rtt,com.google.hardware.camera.easel,com.google.android.feature.PIXEL_2017_EXPERIENCE,08,03,C,S,0G,q,L,2,6,Y,Z,0M,android.hardware.vr.high_performance,f,1,07,8,9,android.hardware.sensor.hifi_sensors,O,H,com.google.android.feature.TURBO_PRELOAD,android.hardware.vr.headtracking,W,x,G,o,06,0N,com.google.android.feature.PIXEL_EXPERIENCE,3,R,d,Q,n,android.hardware.telephony.carrierlock,y,T,i,r,u,com.google.android.feature.WELLBEING,l,4,0Q,N,M,01,09,V,7,5,0H,g,s,c,0l,t,0L,0W,0X,k,00,com.google.android.feature.GOOGLE_EXPERIENCE,android.hardware.sensor.assist,android.hardware.audio.pro,K,E,02,I,J,j,D,h,android.hardware.wifi.aware,05,X,v","dpi":420,"preferLan":"en"}"#,
        ),
        ("emuiApiLevel", "0"),
        ("firmwareVersion", "10"),
        ("getSafeGame", "1"),
        ("gmsSupport", "0"),
        ("hardwareType", "0"),
        ("harmonyApiLevel", "0"),
        ("harmonyDeviceType", ""),
        ("installCheck", "0"),
        ("isFullUpgrade", "0"),
        ("isUpdateSdk", "1"),
        ("locale", "en_US"),
        ("magicApiLevel", "0"),
        ("magicVer", ""),
        ("manufacturer", "Google"),
        ("mapleVer", "0"),
        ("method", "client.updateCheck"),
        ("odm", "0"),
        ("packageName", "com.huawei.appmarket"),
        ("phoneType", "Pixel 2"),
        ("resolution", "1080_1794"),
        ("sdkVersion", "4.0.1.300"),
        ("serviceType", "0"),
        ("supportMaple", "0"),
        ("ts", "1649970862661"),
        ("ver", "1.2"),
        ("version", "12.0.1.301"),
        ("versionCode", "120001301"),
    ];
    let info = serde_json::json!({"params": [{"isPre": 0, "maple": 0,
        "oldVersion": "1.0", "package": app, "pkgMode": 0, "shellApkVer": 0,
        "targetSdkVersion": 19, "versionCode": 1}]})
    .to_string();
    // reqwest's form feature is not enabled; Url uses the same form encoder.
    let mut url = reqwest::Url::parse("http://localhost/").expect("static URL");
    url.query_pairs_mut()
        .extend_pairs(profile)
        .append_pair("pkgInfo", &info)
        .append_pair("serviceCountry", region);
    url.query().expect("encoded form").to_owned()
}

#[derive(Debug)]
struct Apk {
    url: reqwest::Url,
    size: u64,
    sha256: Vec<u8>,
}

fn parse_apk(body: &[u8], package: &str) -> Result<Apk, &'static str> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "malformed API JSON")?;
    match value["rtnCode"].as_i64() {
        Some(0) => {}
        Some(_) => return Err("API returned nonzero rtnCode"),
        None => return Err("malformed API rtnCode"),
    }
    let list = value["list"].as_array().ok_or("malformed API list")?;
    if list.is_empty() {
        return Err("unavailable in configured region/profile");
    }
    let mut matching = list
        .iter()
        .filter(|item| item["package"].as_str() == Some(package));
    let item = matching.next().ok_or("missing matching package")?;
    if matching.next().is_some() {
        return Err("ambiguous matching package");
    }
    if item["isDiff"].as_u64() != Some(0)
        || item.get("splitInfo").is_some()
        || item.get("splits").is_some()
        || item.get("splitApks").is_some()
        || item.get("isSplit").is_some_and(|v| v.as_u64() != Some(0))
    {
        return Err("unsupported delta/split or unknown APK schema");
    }
    let size = item["size"]
        .as_u64()
        .filter(|size| *size > 0)
        .ok_or("invalid APK size")?;
    if item
        .get("fullSize")
        .is_some_and(|v| v.as_u64() != Some(size))
    {
        return Err("inconsistent full APK size");
    }
    let sha256 = hex::decode(item["sha256"].as_str().ok_or("missing APK SHA256")?)
        .map_err(|_| "invalid APK SHA256")?;
    if sha256.len() != 32 {
        return Err("invalid APK SHA256");
    }
    let raw_url = match item.get("fullDownUrl") {
        Some(value) => value.as_str().ok_or("invalid full APK URL")?,
        None => item["downurl"].as_str().ok_or("missing APK URL")?,
    };
    Ok(Apk {
        url: apk_url(raw_url)?,
        size,
        sha256,
    })
}

fn apk_url(raw: &str) -> Result<reqwest::Url, &'static str> {
    let authority = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))
        .ok_or("invalid APK URL scheme")?
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if authority.is_empty()
        || authority.contains('@')
        || raw
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err("invalid APK URL");
    }
    let url = reqwest::Url::parse(raw).map_err(|_| "invalid APK URL")?;
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid APK URL");
    }
    if let Some(domain) = url.domain() {
        let domain = domain.strip_suffix('.').unwrap_or(domain);
        if domain.len() > 253
            || !domain.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
        {
            return Err("invalid APK URL host");
        }
    }
    Ok(url)
}

async fn download_one(
    client: &reqwest::Client,
    endpoint: &str,
    package: &str,
    version: Option<&str>,
    region: &str,
    outpath: &std::path::Path,
) -> Result<(), String> {
    use std::io::Write;
    if !valid_package(package) {
        return Err("invalid package ID".to_owned());
    }
    if version.is_some() {
        return Err("historical versions are unsupported".to_owned());
    }
    match std::fs::symlink_metadata(outpath.join(format!("{package}.apk"))) {
        Ok(metadata) if metadata.file_type().is_file() => return Ok(()),
        Ok(_) => return Err("output exists but is not a regular file".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("cannot inspect existing output".to_owned()),
    }
    let mut response = client
        .post(endpoint)
        .header(
            reqwest::header::USER_AGENT,
            "UpdateSDK##4.0.1.300##Android##Pixel 2##com.huawei.appmarket##12.0.1.301",
        )
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(client_api_body(package, region))
        .send()
        .await
        .map_err(|error| network_error("API", &error))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("API HTTP status {}", response.status().as_u16()));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| network_error("API body", &error))?
    {
        if body.len() + chunk.len() > 1024 * 1024 {
            return Err("API response too large".to_owned());
        }
        body.extend_from_slice(&chunk);
    }
    let apk = parse_apk(&body, package)?;
    let mut response = client
        .get(apk.url)
        .send()
        .await
        .map_err(|error| network_error("APK", &error))?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("APK HTTP status {}", response.status().as_u16()));
    }
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut file =
        tempfile::NamedTempFile::new_in(outpath).map_err(|_| "cannot create temporary APK")?;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| network_error("APK body", &error))?
    {
        size = size
            .checked_add(chunk.len() as u64)
            .ok_or("APK size overflow")?;
        if size > apk.size {
            return Err("APK size exceeds API metadata".to_owned());
        }
        hash.update(&chunk);
        file.write_all(&chunk)
            .map_err(|_| "cannot write temporary APK")?;
    }
    if size != apk.size {
        return Err("APK size does not match API metadata".to_owned());
    }
    if hash.finalize().as_slice() != apk.sha256 {
        return Err("APK SHA256 does not match API metadata".to_owned());
    }
    file.as_file()
        .sync_all()
        .map_err(|_| "cannot sync temporary APK")?;
    file.persist_noclobber(outpath.join(format!("{package}.apk")))
        .map_err(|_| "cannot publish APK without overwriting existing output")?;
    Ok(())
}

fn network_error(stage: &str, error: &reqwest::Error) -> String {
    // Never format reqwest errors: they can contain signed URLs and response bodies.
    format!(
        "{stage} {}",
        if error.is_timeout() {
            "timeout"
        } else {
            "transport error"
        }
    )
}

fn valid_package(package: &str) -> bool {
    package.len() <= 240
        && package.contains('.')
        && package.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
                && bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
}

fn http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || apk_url(attempt.url().as_str()).is_err() {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
}

pub async fn download_apps(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &std::path::Path,
    region: &str,
) -> bool {
    let client = match http_client_builder().build() {
        Ok(client) => client,
        Err(_) => {
            eprintln!("Huawei AppGallery: cannot initialize HTTP client");
            return false;
        }
    };
    download_batch(
        &client,
        "https://store-dre.hispace.dbankcloud.com/hwmarket/api/clientApi",
        apps,
        parallel,
        sleep_duration,
        outpath,
        region,
    )
    .await
}

async fn download_batch(
    client: &reqwest::Client,
    endpoint: &str,
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &std::path::Path,
    region: &str,
) -> bool {
    use futures_util::StreamExt;
    if parallel == 0 || apps.is_empty() {
        eprintln!("Huawei AppGallery: nonempty app list and parallel >= 1 required");
        return false;
    }
    let results =
        futures_util::stream::iter(apps.into_iter().map(|(package, version)| async move {
            if sleep_duration > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(sleep_duration)).await;
            }
            let result = download_one(
                client,
                endpoint,
                &package,
                version.as_deref(),
                region,
                outpath,
            )
            .await;
            let label = if valid_package(&package) {
                &package
            } else {
                "<invalid package>"
            };
            match result {
                Ok(()) => {
                    println!(
                        "Huawei AppGallery: {label}: complete or existing file (region {region})"
                    );
                    true
                }
                Err(error) => {
                    eprintln!("Huawei AppGallery: {label} (region {region}): {error}");
                    false
                }
            }
        }))
        .buffer_unordered(parallel)
        .collect::<Vec<_>>()
        .await;
    results.into_iter().all(|success| success)
}

pub fn list_versions(apps: Vec<(String, Option<String>)>) -> bool {
    let mut success = !apps.is_empty();
    if !success {
        eprintln!("Huawei AppGallery: nonempty app list required");
    }
    for (app, _) in apps {
        if valid_package(&app) {
            println!("Huawei AppGallery does not expose historical versions ({app}).");
        } else {
            eprintln!("Huawei AppGallery: invalid package ID");
            success = false;
        }
    }
    success
}

#[cfg(test)]
#[path = "huawei_app_gallery_tests.rs"]
mod download_tests;

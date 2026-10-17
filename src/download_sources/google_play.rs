use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use futures_util::StreamExt;
use gpapi::error::{Error as GpError, ErrorKind as GpapiErrorKind};
use gpapi::Gpapi;
use indicatif::MultiProgress;
use tokio::time::{sleep, Duration};

use crate::util::progress_bar::progress_wrapper;

#[cfg(test)]
#[path = "google_play_tests.rs"]
mod tests;

#[derive(Clone, Copy, Default)]
struct DownloadSettings {
    split_apk: bool,
    include_dex_metadata: bool,
    include_additional_files: bool,
}

#[derive(Clone, Copy)]
struct Limits {
    action: Duration,
    download: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            action: Duration::from_secs(120),
            download: Duration::from_secs(600),
        }
    }
}

// The caller policy is shared by the real gpapi adapter and offline tests.
// In particular, tests never need Google credentials or a network connection.
trait PlayBackend {
    async fn login(&mut self) -> Result<(), GpError>;
    async fn accept_tos(&mut self) -> Result<(), GpError>;
    async fn request_token(&mut self, oauth: &str) -> Result<Option<String>, GpError>;
    async fn download(
        &self,
        app: &str,
        out: &Path,
        settings: DownloadSettings,
        progress: Rc<MultiProgress>,
    ) -> Result<(), GpError>;
}

impl PlayBackend for Gpapi {
    async fn login(&mut self) -> Result<(), GpError> {
        Gpapi::login(self).await
    }
    async fn accept_tos(&mut self) -> Result<(), GpError> {
        Gpapi::accept_tos(self)
            .await
            .map(|_| ())
            .map_err(GpError::from)
    }
    async fn request_token(&mut self, oauth: &str) -> Result<Option<String>, GpError> {
        self.request_aas_token(oauth).await?;
        Ok(self.get_aas_token().map(str::to_owned))
    }
    async fn download(
        &self,
        app: &str,
        out: &Path,
        settings: DownloadSettings,
        progress: Rc<MultiProgress>,
    ) -> Result<(), GpError> {
        Gpapi::download(
            self,
            app,
            None,
            settings.split_apk,
            settings.include_dex_metadata,
            settings.include_additional_files,
            out,
            Some(&progress_wrapper(progress)),
        )
        .await
        .map(|_| ())
    }
}

thread_local! {
    // Only synchronous gpapi construction is covered, never network tasks.
    static IN_CONSTRUCTOR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn acquire_device_properties_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A checked regular path can be replaced with a FIFO before open.
        // O_NONBLOCK prevents that acquisition from waiting for a writer; the
        // descriptor check below rejects it before any read. Regular-file reads
        // keep their normal semantics. Use the platform value, not Linux bits.
        options.custom_flags(libc::O_NONBLOCK);
    }
    // On non-Unix targets the portable path precheck still rejects known
    // nonregular input, but does not close the metadata/open substitution race.
    let input = options.open(path)?;
    if !input.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid Google Play device configuration",
        ));
    }
    Ok(input)
}

fn create_api<'a>(
    email: &str,
    options: &mut HashMap<&'a str, &'a str>,
) -> Result<Gpapi, &'static str> {
    use std::io::{Read, Write};
    const ERROR: &str = "Invalid Google Play device configuration";
    let device = options.remove("device").unwrap_or("px_9a");
    // gpapi 6.1.0 constructors unwrap device lookup, INI reads/parsing and typed
    // fields (src/lib.rs:160-223; snippets/device_properties.rs). They have no
    // fallible public equivalent. Snapshot bounded custom input before invoking
    // that API; never let its default panic hook print untrusted parser errors.
    let snapshot = match options.remove("device_properties_file") {
        None => None,
        Some(path) => {
            // Reject known nonregular paths before opening them: opening a FIFO
            // with no writer can otherwise block before descriptor validation.
            if !std::fs::metadata(path).map_err(|_| ERROR)?.is_file() {
                return Err(ERROR);
            }
            let input = acquire_device_properties_file(Path::new(path)).map_err(|_| ERROR)?;
            let mut bytes = Vec::new();
            input
                .take(256 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ERROR)?;
            if bytes.len() > 256 * 1024 {
                return Err(ERROR);
            }
            let mut file = tempfile::NamedTempFile::new().map_err(|_| ERROR)?;
            file.write_all(&bytes).map_err(|_| ERROR)?;
            Some(file)
        }
    };
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_CONSTRUCTOR.with(|flag| flag.get()) {
                previous(info);
            }
        }));
    });
    struct ConstructorGuard;
    impl Drop for ConstructorGuard {
        fn drop(&mut self) {
            IN_CONSTRUCTOR.with(|flag| flag.set(false));
        }
    }
    let mut api = {
        IN_CONSTRUCTOR.with(|flag| flag.set(true));
        let _guard = ConstructorGuard;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match snapshot.as_ref() {
            None => Gpapi::new(device, email),
            Some(file) => Gpapi::from_device_properties_file(
                device,
                email,
                file.path().to_str().expect("temporary UTF-8 path"),
            ),
        }))
        .map_err(|_| ERROR)?
    };
    if let Some(locale) = options.remove("locale") {
        api.set_locale(locale);
    }
    if let Some(timezone) = options.remove("timezone") {
        api.set_timezone(timezone);
    }
    Ok(api)
}

pub async fn download_apps(
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    email: &str,
    aas_token: Option<&str>,
    auth_token: Option<&str>,
    outpath: &Path,
    accept_tos: bool,
    mut options: HashMap<&str, &str>,
) -> bool {
    if aas_token.filter(|s| !s.trim().is_empty()).is_none()
        && auth_token.filter(|s| !s.trim().is_empty()).is_none()
    {
        eprintln!("Either AAS token or AUTH token must be provided.");
        return false;
    }
    let enabled =
        |value: Option<&str>| value.is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let settings = DownloadSettings {
        split_apk: enabled(options.remove("split_apk")),
        include_additional_files: enabled(options.remove("include_additional_files")),
        include_dex_metadata: enabled(options.remove("include_dex_metadata")),
    };
    let mut api = match create_api(email, &mut options) {
        Ok(api) => api,
        Err(error) => {
            eprintln!("{error}");
            return false;
        }
    };
    if let Some(aas) = aas_token.filter(|s| !s.trim().is_empty()) {
        api.set_aas_token(aas);
    } else if let Some(auth) = auth_token {
        api.set_auth_token(auth);
    }
    download_with_backend(
        &mut api,
        apps,
        parallel,
        sleep_duration,
        outpath,
        accept_tos,
        settings,
        Limits::default(),
    )
    .await
}

async fn download_with_backend<B: PlayBackend>(
    backend: &mut B,
    apps: Vec<(String, Option<String>)>,
    parallel: usize,
    sleep_duration: u64,
    outpath: &Path,
    accept_tos: bool,
    settings: DownloadSettings,
    limits: Limits,
) -> bool {
    let mut invalid = apps.is_empty();
    let apps: Vec<_> = apps
        .into_iter()
        .filter(|(app, version)| {
            if !valid_app_id(app) || version.is_some() {
                eprintln!("Invalid Google Play app ID or unsupported specific version.");
                invalid = true;
                false
            } else {
                true
            }
        })
        .collect();
    if apps.is_empty() || !outpath.is_dir() {
        return false;
    }
    match bounded(limits.action, backend.login()).await {
        Ok(()) => (),
        Err(err) if matches!(err.kind(), GpapiErrorKind::TermsOfService) => {
            if !accept_tos {
                eprintln!("Read https://play.google.com/about/play-terms/index.html and use --accept-tos only if you accept Google Play's terms.");
                return false;
            }
            if bounded(limits.action, backend.accept_tos()).await.is_err() {
                eprintln!("Could not accept Google Play Terms of Service.");
                return false;
            }
        }
        Err(_) => {
            eprintln!(
                "Could not log in to Google Play. Check your credentials and try again later."
            );
            return false;
        }
    }
    let backend = &*backend;
    let progress = Rc::new(MultiProgress::new());
    let results = futures_util::stream::iter(apps.into_iter().map(|(app, _)| {
        let progress = progress.clone();
        async move {
            if sleep_duration > 0 { sleep(Duration::from_millis(sleep_duration)).await; }
            for attempt in 0..3 {
                // A fresh private directory per attempt: gpapi may leave a base APK,
                // splits or OBB files behind when any sibling download fails.
                let mut builder = tempfile::Builder::new();
                builder.prefix(".apkeep-google-");
                #[cfg(unix)] {
                    use std::os::unix::fs::PermissionsExt;
                    builder.permissions(std::fs::Permissions::from_mode(0o700));
                }
                let stage = match builder.tempdir_in(outpath) {
                    Ok(stage) => stage,
                    Err(_) => { eprintln!("Cannot create Google Play staging directory."); return false; }
                };
                match bounded(limits.download, backend.download(&app, stage.path(), settings, progress.clone())).await {
                    Ok(()) => {
                        if publish_download(stage.path(), outpath, &app).is_err() {
                            eprintln!("Cannot publish Google Play download; destination may already exist.");
                            return false;
                        }
                        println!("{app} downloaded successfully!");
                        return true;
                    }
                    Err(err) if matches!(err.kind(), GpapiErrorKind::FileExists | GpapiErrorKind::DirectoryExists | GpapiErrorKind::InvalidApp | GpapiErrorKind::PermissionDenied | GpapiErrorKind::DirectoryMissing | GpapiErrorKind::Authentication | GpapiErrorKind::LoginRequired | GpapiErrorKind::TermsOfService) => break,
                    Err(_) if attempt < 2 => eprintln!("Google Play download failed; retrying."),
                    Err(_) => break,
                }
            }
            eprintln!("Google Play download failed.");
            false
        }
    })).buffer_unordered(parallel.max(1)).collect::<Vec<bool>>().await;
    !invalid && results.into_iter().all(|result| result)
}

fn valid_app_id(app: &str) -> bool {
    app.len() <= 255
        && app.contains('.')
        && app.split('.').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphabetic()
                && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
}

async fn bounded<T>(
    limit: Duration,
    future: impl std::future::Future<Output = Result<T, GpError>>,
) -> Result<T, GpError> {
    tokio::time::timeout(limit, future)
        .await
        .map_err(|_| GpError::from("Google Play action timed out"))?
}

fn publish_download(stage: &Path, out: &Path, app: &str) -> std::io::Result<()> {
    publish_download_with_link(stage, out, app, |from, to| std::fs::hard_link(from, to))
}

fn publish_download_with_link(
    stage: &Path,
    out: &Path,
    app: &str,
    mut link: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let invalid = || {
        Error::new(
            ErrorKind::InvalidData,
            "Invalid Google Play download artifacts",
        )
    };
    let apk = format!("{app}.apk");
    if !valid_app_id(app) {
        return Err(invalid());
    }
    let entries = std::fs::read_dir(stage)?.collect::<Result<Vec<_>, _>>()?;
    if entries.len() != 1 {
        return Err(invalid());
    }
    let entry = &entries[0];
    let metadata = std::fs::symlink_metadata(entry.path())?;
    if entry.file_name() == std::ffi::OsStr::new(app) && metadata.is_dir() {
        let base = std::fs::symlink_metadata(entry.path().join(&apk))?;
        if !base.is_file() || base.len() == 0 {
            return Err(invalid());
        }
        for child in std::fs::read_dir(entry.path())? {
            let child = child?;
            let metadata = std::fs::symlink_metadata(child.path())?;
            let name = child.file_name();
            let name = name.to_str().ok_or_else(invalid)?;
            let safe_name = name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'));
            let known_artifact = (name.starts_with(&format!("{app}.")) && name.ends_with(".apk"))
                || name == format!("{app}.dm")
                || ((name.starts_with("main.") || name.starts_with("patch."))
                    && name.ends_with(&format!(".{app}.obb")));
            if !metadata.is_file() || metadata.len() == 0 || !safe_name || !known_artifact {
                return Err(invalid());
            }
        }
        let destination = out.join(app);
        // Exclusive reservation rejects existing directories, files and symlinks.
        // No API call takes place after publication starts. On a local I/O error,
        // roll back only links created by this invocation, never existing data.
        std::fs::create_dir(&destination)?;
        let mut published = Vec::new();
        let result = (|| {
            for entry in std::fs::read_dir(stage.join(app))? {
                let entry = entry?;
                let target = destination.join(entry.file_name());
                link(&entry.path(), &target)?;
                published.push(target);
            }
            Ok(())
        })();
        if result.is_err() {
            for path in published {
                let _ = std::fs::remove_file(path);
            }
            let _ = std::fs::remove_dir(destination);
        }
        result
    } else if entry.file_name() == std::ffi::OsStr::new(&apk)
        && metadata.is_file()
        && metadata.len() > 0
    {
        // hard_link is an atomic no-clobber publication on the same filesystem.
        link(&stage.join(&apk), &out.join(&apk))
    } else {
        Err(invalid())
    }
}

pub async fn request_aas_token(
    email: &str,
    oauth_token: &str,
    mut options: HashMap<&str, &str>,
) -> bool {
    let mut api = match create_api(email, &mut options) {
        Ok(api) => api,
        Err(error) => {
            eprintln!("{error}");
            return false;
        }
    };
    request_token_with_backend(
        &mut api,
        oauth_token,
        Limits::default(),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
    .await
}

async fn request_token_with_backend<B: PlayBackend>(
    backend: &mut B,
    oauth_token: &str,
    limits: Limits,
    stdout: &mut impl std::io::Write,
    stderr: &mut impl std::io::Write,
) -> bool {
    match bounded(limits.action, backend.request_token(oauth_token)).await {
        Ok(Some(token)) if !token.trim().is_empty() && !token.chars().any(char::is_control) => {
            writeln!(stdout, "{token}").is_ok()
        }
        _ => {
            let _ = writeln!(
                stderr,
                "Could not retrieve an AAS token. Provide a new OAuth token and try again."
            );
            false
        }
    }
}

pub fn list_versions(_apps: Vec<(String, Option<String>)>) -> bool {
    eprintln!("Google Play does not make old versions of apps available.");
    false
}

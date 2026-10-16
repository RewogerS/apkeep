//! A micro-library for downloading from a URL and streaming it directly to the disk
//!
//! # Getting Started
//!
//! ```no_run
//! use std::path::Path;
//! use tokio_dl_stream_to_disk::AsyncDownload;
//!
//! #[tokio::main]
//! async fn main() {
//!     if AsyncDownload::new("https://bit.ly/3yWXSOW", Path::new("/tmp"), "5mb_test.bin").download(&None).await.is_ok() {
//!         println!("File downloaded successfully!");
//!     }
//! }
//! ```

pub mod error;

use std::error::Error;
use std::io::{Error as IOError, ErrorKind};
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::stream::Stream;
use futures_util::StreamExt;
#[cfg(feature = "sha256sum")]
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::io::StreamReader;

use crate::error::{Error as TDSTDError, ErrorKind as TDSTDErrorKind};

type S = dyn Stream<Item = Result<Bytes, IOError>> + Unpin;

#[derive(Clone, Copy)]
struct Timeouts {
    connect: std::time::Duration,
    read: std::time::Duration,
    whole: std::time::Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: std::time::Duration::from_secs(15),
            read: std::time::Duration::from_secs(30),
            whole: std::time::Duration::from_secs(600),
        }
    }
}

#[cfg(test)]
mod tests;

/// Streams a response to a private temporary file, publishing only on success.
pub struct AsyncDownload {
    url: String,
    dst_path: PathBuf,
    fname: String,
    length: Option<u64>,
    response_stream: Option<Box<S>>,
    timeouts: Timeouts,
}

impl AsyncDownload {
    /// Construct a downloader with a destination directory and single filename.
    pub fn new(url: &str, dst_path: &Path, fname: &str) -> Self {
        Self {
            url: url.into(),
            dst_path: dst_path.into(),
            fname: fname.into(),
            length: None,
            response_stream: None,
            timeouts: Timeouts::default(),
        }
    }

    /// The advertised response length, available after `get` or `download`.
    pub fn length(&self) -> Option<u64> {
        self.length
    }

    /// Fetch response headers without consuming its body.
    pub async fn get(mut self) -> Result<AsyncDownload, Box<dyn Error>> {
        self.get_non_consumable().await?;
        Ok(self)
    }

    async fn get_non_consumable(&mut self) -> Result<(), Box<dyn Error>> {
        self.validate_destination()
            .map_err(|_| IOError::new(ErrorKind::InvalidInput, "invalid download destination"))?;
        let client = reqwest::Client::builder()
            .connect_timeout(self.timeouts.connect)
            .read_timeout(self.timeouts.read)
            .timeout(self.timeouts.whole)
            .build()?;
        let response = client.get(&self.url).send().await?.error_for_status()?;
        self.length = response.content_length();
        self.response_stream =
            Some(Box::new(response.bytes_stream().map(|result| {
                result.map_err(|e| IOError::new(ErrorKind::Other, e))
            })));
        Ok(())
    }

    fn validate_destination(&self) -> Result<(), TDSTDError> {
        if self.fname.is_empty()
            || matches!(self.fname.as_str(), "." | "..")
            || self
                .fname
                .chars()
                .any(|c| c == '/' || c == '\\' || c.is_control())
        {
            return Err(
                IOError::new(ErrorKind::InvalidInput, "expected a single safe filename").into(),
            );
        }
        if !self.dst_path.is_dir() {
            return Err(TDSTDError::new(TDSTDErrorKind::DirectoryMissing));
        }
        Ok(())
    }

    fn check_output(&self) -> Result<PathBuf, TDSTDError> {
        self.validate_destination()?;
        let path = self.dst_path.join(&self.fname);
        Self::check_absent(&path)?;
        Ok(path)
    }

    fn check_absent(path: &Path) -> Result<(), TDSTDError> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_file() => {
                Err(TDSTDError::new(TDSTDErrorKind::FileExists))
            }
            Ok(_) => {
                Err(IOError::new(ErrorKind::InvalidInput, "output is not a regular file").into())
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Download to disk, optionally reporting the number of bytes written.
    pub async fn download(
        &mut self,
        cb: &Option<Box<dyn Fn(u64) -> ()>>,
    ) -> Result<(), TDSTDError> {
        self.transfer(None, cb).await.map(|_| ())
    }

    /// Download and return the SHA256 digest of the published bytes.
    #[cfg(feature = "sha256sum")]
    pub async fn download_and_return_sha256sum(
        &mut self,
        cb: &Option<Box<dyn Fn(u64) -> ()>>,
    ) -> Result<Vec<u8>, TDSTDError> {
        self.transfer(None, cb).await
    }

    /// Download and verify the SHA256 digest before publishing the output.
    /// `expected` must contain exactly 32 digest bytes (not hex text).
    #[cfg(feature = "sha256sum")]
    pub async fn download_and_verify_sha256(
        &mut self,
        expected: &[u8],
        cb: &Option<Box<dyn Fn(u64) -> ()>>,
    ) -> Result<Vec<u8>, TDSTDError> {
        if expected.len() != 32 {
            return Err(
                IOError::new(ErrorKind::InvalidInput, "SHA256 digest must be 32 bytes").into(),
            );
        }
        self.transfer(Some(expected), cb).await
    }

    async fn transfer(
        &mut self,
        expected: Option<&[u8]>,
        cb: &Option<Box<dyn Fn(u64) -> ()>>,
    ) -> Result<Vec<u8>, TDSTDError> {
        tokio::time::timeout(self.timeouts.whole, self.transfer_inner(expected, cb))
            .await
            .map_err(|_| {
                TDSTDError::from(IOError::new(
                    ErrorKind::TimedOut,
                    "download deadline exceeded",
                ))
            })?
    }

    async fn transfer_inner(
        &mut self,
        _expected: Option<&[u8]>,
        cb: &Option<Box<dyn Fn(u64) -> ()>>,
    ) -> Result<Vec<u8>, TDSTDError> {
        let path = self.check_output()?;
        if self.response_stream.is_none() {
            self.get_non_consumable()
                .await
                .map_err(|_| TDSTDError::new(TDSTDErrorKind::InvalidResponse))?;
        }
        // Keep the owner in this future: error, timeout, panic or cancellation
        // unlinks its private path. Never use the final path for in-flight data.
        let temp = tempfile::Builder::new()
            .prefix(".apkeep-")
            .tempfile_in(&self.dst_path)?;
        let mut dest = tokio::fs::File::from_std(temp.reopen()?);
        let mut reader = StreamReader::new(self.response_stream.take().unwrap());
        let mut buf = [0; 8192];
        let mut total = 0u64;
        #[cfg(feature = "sha256sum")]
        let mut hasher = Sha256::new();
        loop {
            let count = reader.read(&mut buf).await?;
            if count == 0 {
                break;
            }
            dest.write_all(&buf[..count]).await?;
            total += count as u64;
            #[cfg(feature = "sha256sum")]
            hasher.update(&buf[..count]);
            if let Some(cb) = cb {
                cb(total);
            }
        }
        if total == 0 || self.length.is_some_and(|expected| expected != total) {
            return Err(TDSTDError::new(TDSTDErrorKind::InvalidResponse));
        }
        dest.flush().await?;
        #[cfg(feature = "sha256sum")]
        let digest = hasher.finalize().to_vec();
        #[cfg(feature = "sha256sum")]
        if _expected.is_some_and(|expected| expected != digest.as_slice()) {
            return Err(TDSTDError::new(TDSTDErrorKind::InvalidResponse));
        }
        drop(dest);
        // persist_noclobber atomically creates the destination name without
        // replacing any entry that arrived after the initial existence check.
        if let Err(e) = temp.persist_noclobber(&path) {
            Self::check_absent(&path)?;
            return Err(e.error.into());
        }
        #[cfg(feature = "sha256sum")]
        return Ok(digest);
        #[cfg(not(feature = "sha256sum"))]
        Ok(Vec::new())
    }
}

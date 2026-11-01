//! Copying between any two sides — local to remote, remote to local, and
//! remote to remote through this machine — as one stream per file, with the
//! progress the UI polls kept in atomics.

use std::{
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, UNIX_EPOCH},
};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    fs::{Kind, local_join, remote_join},
    remote::{Remote, sftp_error},
};

const CHUNK: usize = 256 * 1024;

/// One side of a copy.
#[derive(Clone)]
pub enum Side {
    Local,
    Remote(Remote),
}

impl Side {
    pub fn join(&self, dir: &str, name: &str) -> String {
        match self {
            Side::Local => local_join(dir, name),
            Side::Remote(_) => remote_join(dir, name),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Side::Remote(_))
    }
}

#[derive(Default)]
pub struct Progress {
    pub bytes_total: AtomicU64,
    pub bytes_done: AtomicU64,
    pub files_total: AtomicU64,
    pub files_done: AtomicU64,
    pub cancel: AtomicBool,
    pub current: Mutex<String>,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        let total = self.bytes_total.load(Ordering::Relaxed);
        if total == 0 {
            let files = self.files_total.load(Ordering::Relaxed);
            if files == 0 {
                return 0.;
            }
            return self.files_done.load(Ordering::Relaxed) as f32 / files as f32;
        }
        self.bytes_done.load(Ordering::Relaxed) as f32 / total as f32
    }

    pub fn current(&self) -> String {
        self.current
            .lock()
            .map(|name| name.clone())
            .unwrap_or_default()
    }
}

/// What to copy: top-level items from one folder into another.
pub struct Job {
    pub from: Side,
    pub from_dir: String,
    pub items: Vec<(String, Kind)>,
    pub to: Side,
    pub to_dir: String,
}

enum Step {
    Mkdir(String),
    Copy {
        source: String,
        target: String,
        size: u64,
        modified: Option<i64>,
    },
}

pub async fn run(job: Job, progress: Arc<Progress>) -> Result<(), String> {
    let steps = plan(&job).await?;
    progress.files_total.store(
        steps
            .iter()
            .filter(|step| matches!(step, Step::Copy { .. }))
            .count() as u64,
        Ordering::Relaxed,
    );
    progress.bytes_total.store(
        steps
            .iter()
            .map(|step| match step {
                Step::Copy { size, .. } => *size,
                Step::Mkdir(_) => 0,
            })
            .sum(),
        Ordering::Relaxed,
    );
    for step in steps {
        if progress.cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
        match step {
            Step::Mkdir(path) => make_dir(&job.to, &path).await?,
            Step::Copy {
                source,
                target,
                modified,
                ..
            } => {
                if let Ok(mut current) = progress.current.lock() {
                    *current = crate::fs::file_name(&source);
                }
                copy_file(&job.from, &source, &job.to, &target, &progress).await?;
                set_modified(&job.to, &target, modified).await;
                progress.files_done.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    Ok(())
}

/// Walk the sources into folders to make and files to copy, parents first.
async fn plan(job: &Job) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    let mut stack: Vec<(String, String, bool)> = job
        .items
        .iter()
        .rev()
        .map(|(name, kind)| {
            (
                job.from.join(&job.from_dir, name),
                job.to.join(&job.to_dir, name),
                // A link to a folder is copied as the folder it shows.
                kind.is_dir(),
            )
        })
        .collect();
    while let Some((source, target, is_dir)) = stack.pop() {
        if !is_dir {
            let (size, modified) = stat(&job.from, &source).await?;
            steps.push(Step::Copy {
                source,
                target,
                size,
                modified,
            });
            continue;
        }
        steps.push(Step::Mkdir(target.clone()));
        let children = match &job.from {
            Side::Local => crate::fs::list_local(&source)?,
            Side::Remote(remote) => remote.list(&source).await?,
        };
        for child in children.into_iter().rev() {
            // A link inside a tree is not followed: it could lead back up it.
            let is_dir = child.kind == Kind::Dir;
            if child.kind == Kind::LinkDir {
                continue;
            }
            stack.push((
                job.from.join(&source, &child.name),
                job.to.join(&target, &child.name),
                is_dir,
            ));
        }
    }
    Ok(steps)
}

async fn stat(side: &Side, path: &str) -> Result<(u64, Option<i64>), String> {
    match side {
        Side::Local => {
            let meta = tokio::fs::metadata(path)
                .await
                .map_err(|e| format!("{path}: {e}"))?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_secs() as i64);
            Ok((meta.len(), modified))
        }
        Side::Remote(remote) => {
            let meta = remote.sftp.metadata(path).await.map_err(sftp_error)?;
            Ok((meta.size.unwrap_or(0), meta.mtime.map(i64::from)))
        }
    }
}

async fn make_dir(side: &Side, path: &str) -> Result<(), String> {
    match side {
        Side::Local => match tokio::fs::create_dir(path).await {
            Ok(()) => Ok(()),
            Err(_) if Path::new(path).is_dir() => Ok(()),
            Err(error) => Err(format!("{path}: {error}")),
        },
        Side::Remote(remote) => match remote.sftp.create_dir(path).await {
            Ok(()) => Ok(()),
            Err(error) => match remote.sftp.metadata(path).await {
                Ok(meta) if meta.is_dir() => Ok(()),
                _ => Err(format!("{path}: {}", sftp_error(error))),
            },
        },
    }
}

type Reader = Pin<Box<dyn AsyncRead + Send>>;
type Writer = Pin<Box<dyn AsyncWrite + Send>>;

async fn open_read(side: &Side, path: &str) -> Result<Reader, String> {
    Ok(match side {
        Side::Local => Box::pin(
            tokio::fs::File::open(path)
                .await
                .map_err(|error| format!("{path}: {error}"))?,
        ),
        Side::Remote(remote) => Box::pin(
            remote
                .sftp
                .open(path)
                .await
                .map_err(|error| format!("{path}: {}", sftp_error(error)))?,
        ),
    })
}

async fn open_write(side: &Side, path: &str) -> Result<Writer, String> {
    Ok(match side {
        Side::Local => Box::pin(
            tokio::fs::File::create(path)
                .await
                .map_err(|error| format!("{path}: {error}"))?,
        ),
        Side::Remote(remote) => Box::pin(
            remote
                .sftp
                .create(path)
                .await
                .map_err(|error| format!("{path}: {}", sftp_error(error)))?,
        ),
    })
}

async fn copy_file(
    from: &Side,
    source: &str,
    to: &Side,
    target: &str,
    progress: &Progress,
) -> Result<(), String> {
    let mut reader = open_read(from, source).await?;
    let mut writer = open_write(to, target).await?;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        if progress.cancel.load(Ordering::Relaxed) {
            let _ = writer.shutdown().await;
            return Err("Cancelled.".into());
        }
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| format!("Reading {source}: {error}"))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|error| format!("Writing {target}: {error}"))?;
        progress
            .bytes_done
            .fetch_add(read as u64, Ordering::Relaxed);
    }
    writer
        .flush()
        .await
        .map_err(|error| format!("Writing {target}: {error}"))?;
    writer
        .shutdown()
        .await
        .map_err(|error| format!("Closing {target}: {error}"))
}

/// Best effort: a copy keeps its modification time where the side allows.
async fn set_modified(side: &Side, path: &str, modified: Option<i64>) {
    let Some(modified) = modified.filter(|&m| m >= 0) else {
        return;
    };
    match side {
        Side::Local => {
            let path = path.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                let file = std::fs::File::options().write(true).open(path)?;
                file.set_modified(UNIX_EPOCH + Duration::from_secs(modified as u64))
            })
            .await;
        }
        Side::Remote(remote) => {
            let attributes = russh_sftp::protocol::FileAttributes {
                size: None,
                uid: None,
                user: None,
                gid: None,
                group: None,
                permissions: None,
                atime: Some(modified as u32),
                mtime: Some(modified as u32),
            };
            let _ = remote.sftp.set_metadata(path, attributes).await;
        }
    }
}

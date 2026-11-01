//! An SFTP session over SSH, with russh.
//!
//! Everything here runs on one process-wide tokio runtime ([`runtime`]); the
//! UI spawns onto it and awaits the `JoinHandle`, which is executor-agnostic,
//! so no tokio future ever reaches GPUI's executor.

use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};

use russh::{
    client::{self, AuthResult, Handle, KeyboardInteractiveAuthResponse},
    keys::{
        HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate, agent::AgentIdentity,
        check_known_hosts_path, known_hosts::learn_known_hosts_path, load_secret_key,
    },
};
use russh_sftp::client::SftpSession;
use tokio::sync::{mpsc, oneshot};

use crate::{
    fs::{Entry, Kind, remote_join},
    store::Site,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const KEEPALIVE: Duration = Duration::from_secs(30);
/// An SFTP request that has had no answer this long is given up on. Generous:
/// a listing of a huge directory on a slow link is one request.
const REQUEST_TIMEOUT_SECS: u64 = 60;

pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("filejam-io")
            .enable_all()
            .build()
            .expect("the I/O runtime must start")
    })
}

/// The server offered a key nobody has seen for this host: the UI asks.
pub struct HostKeyQuestion {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    pub reply: oneshot::Sender<bool>,
}

pub type Asker = mpsc::UnboundedSender<HostKeyQuestion>;

#[derive(Debug)]
pub struct SshError(pub String);

impl From<russh::Error> for SshError {
    fn from(error: russh::Error) -> Self {
        SshError(error.to_string())
    }
}

pub struct Client {
    host: String,
    port: u16,
    ask: Asker,
}

impl client::Handler for Client {
    type Error = SshError;

    /// `~/.ssh/known_hosts`, the same file `ssh` reads. A key it already has
    /// passes, a changed one is refused outright, and a new one is the
    /// user's to trust — and then written there, as `ssh` would.
    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key = server_key.public_key();
        let file = known_hosts_file();
        match check_known_hosts_path(&self.host, self.port, &key, &file) {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(russh::keys::Error::KeyChanged { line }) => {
                return Err(SshError(format!(
                    "The host key for {} has changed since it was last trusted \
                     ({}, line {line}). This can mean someone is \
                     intercepting the connection. Remove that line only if you \
                     know why the key changed.",
                    self.host,
                    file.display()
                )));
            }
            // No known_hosts yet, or one that cannot be read: the key is new.
            Err(_) => {}
        }
        let (reply, answer) = oneshot::channel();
        let question = HostKeyQuestion {
            host: self.host.clone(),
            port: self.port,
            algorithm: key.algorithm().as_str().to_string(),
            fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
            reply,
        };
        if self.ask.send(question).is_err() {
            return Ok(false);
        }
        let trusted = answer.await.unwrap_or(false);
        if trusted {
            // Trusted for this session even if it cannot be remembered.
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = learn_known_hosts_path(&self.host, self.port, &key, &file);
        }
        Ok(trusted)
    }
}

/// `~/.ssh/known_hosts`, the file `ssh` reads, unless `FILEJAM_KNOWN_HOSTS`
/// names another.
fn known_hosts_file() -> PathBuf {
    std::env::var_os("FILEJAM_KNOWN_HOSTS")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::fs::home().join(".ssh/known_hosts"))
}

#[derive(Default, Clone)]
pub struct Credentials {
    pub password: Option<String>,
    pub passphrase: Option<String>,
}

/// A live session. Cheap to clone: every clone is the same connection.
#[derive(Clone)]
pub struct Remote {
    pub sftp: Arc<SftpSession>,
    handle: Arc<Handle<Client>>,
    /// Where the login lands, resolved by the server.
    pub home: String,
}

impl Remote {
    pub async fn connect(site: Site, credentials: Credentials, ask: Asker) -> Result<Self, String> {
        let config = Arc::new(client::Config {
            keepalive_interval: Some(KEEPALIVE),
            inactivity_timeout: None,
            ..Default::default()
        });
        let handler = Client {
            host: site.host.clone(),
            port: site.port,
            ask,
        };
        let mut handle = tokio::time::timeout(
            CONNECT_TIMEOUT,
            client::connect(config, (site.host.as_str(), site.port), handler),
        )
        .await
        .map_err(|_| {
            format!(
                "{} did not answer within {}s.",
                site.host,
                CONNECT_TIMEOUT.as_secs()
            )
        })?
        .map_err(|SshError(error)| connect_error(&site, error))?;

        authenticate(&mut handle, &site, &credentials).await?;

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|error| format!("Could not open a session: {error}"))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| format!("The server refused SFTP: {error}"))?;
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| format!("SFTP did not start: {error}"))?;
        sftp.set_timeout(REQUEST_TIMEOUT_SECS);
        let home = sftp.canonicalize(".").await.unwrap_or_else(|_| "/".into());
        Ok(Self {
            sftp: Arc::new(sftp),
            handle: Arc::new(handle),
            home,
        })
    }

    pub async fn disconnect(&self) {
        let _ = self.sftp.close().await;
        let _ = self
            .handle
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    pub async fn list(&self, path: &str) -> Result<Vec<Entry>, String> {
        let read = self.sftp.read_dir(path).await.map_err(sftp_error)?;
        let mut entries = Vec::new();
        for item in read {
            let name = item.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let attrs = item.metadata();
            let kind = if attrs.is_symlink() {
                // Followed for what it opens as; a dangling link stays a link.
                match self.sftp.metadata(remote_join(path, &name)).await {
                    Ok(target) if target.is_dir() => Kind::LinkDir,
                    _ => Kind::Link,
                }
            } else if attrs.is_dir() {
                Kind::Dir
            } else {
                Kind::File
            };
            entries.push(Entry {
                name,
                kind,
                size: if kind.is_dir() {
                    0
                } else {
                    attrs.size.unwrap_or(0)
                },
                modified: attrs.mtime.map(i64::from),
                mode: attrs.permissions.map(|mode| mode & 0o7777),
            });
        }
        Ok(entries)
    }

    /// Resolve a typed path (`~`, `..`, a link) to the one the server means.
    pub async fn resolve(&self, path: &str) -> Result<String, String> {
        let path = if path == "~" {
            self.home.clone()
        } else if let Some(rest) = path.strip_prefix("~/") {
            remote_join(&self.home, rest)
        } else {
            path.to_string()
        };
        let resolved = self.sftp.canonicalize(path).await.map_err(sftp_error)?;
        let meta = self
            .sftp
            .metadata(resolved.clone())
            .await
            .map_err(sftp_error)?;
        if !meta.is_dir() {
            return Err(format!("{resolved} is not a folder."));
        }
        Ok(resolved)
    }

    pub async fn mkdir(&self, path: &str) -> Result<(), String> {
        self.sftp.create_dir(path).await.map_err(sftp_error)
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        if self.sftp.try_exists(to).await.unwrap_or(false) {
            return Err(format!("“{}” already exists.", crate::fs::file_name(to)));
        }
        self.sftp.rename(from, to).await.map_err(sftp_error)
    }

    /// A file, a link, or a whole tree. Links are removed, never followed.
    pub async fn delete(&self, path: &str) -> Result<(), String> {
        let meta = self.sftp.symlink_metadata(path).await.map_err(sftp_error)?;
        if !meta.is_dir() || meta.is_symlink() {
            return self.sftp.remove_file(path).await.map_err(sftp_error);
        }
        let mut stack = vec![(path.to_string(), false)];
        while let Some((dir, emptied)) = stack.pop() {
            if emptied {
                self.sftp.remove_dir(dir).await.map_err(sftp_error)?;
                continue;
            }
            stack.push((dir.clone(), true));
            for item in self.sftp.read_dir(dir.clone()).await.map_err(sftp_error)? {
                let name = item.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                let child = remote_join(&dir, &name);
                let attrs = item.metadata();
                if attrs.is_dir() && !attrs.is_symlink() {
                    stack.push((child, false));
                } else {
                    self.sftp.remove_file(child).await.map_err(sftp_error)?;
                }
            }
        }
        Ok(())
    }
}

pub fn sftp_error(error: russh_sftp::client::error::Error) -> String {
    match error {
        russh_sftp::client::error::Error::Status(status) => {
            let message = status.error_message.trim();
            if message.is_empty() {
                format!("{:?}", status.status_code)
            } else {
                message.to_string()
            }
        }
        other => other.to_string(),
    }
}

fn connect_error(site: &Site, error: String) -> String {
    if error.contains("host key") || error.contains("intercepting") {
        return error;
    }
    if error.contains("UnknownKey") || error.contains("Unknown server key") {
        return format!("The host key for {} was not trusted.", site.host);
    }
    format!("Could not connect to {}: {error}", site.address())
}

fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => crate::fs::home().join(rest),
        None => PathBuf::from(path),
    }
}

/// The keys `ssh` would offer when nothing names one.
fn default_keys() -> Vec<PathBuf> {
    let ssh = crate::fs::home().join(".ssh");
    ["id_ed25519", "id_ecdsa", "id_rsa"]
        .into_iter()
        .map(|name| ssh.join(name))
        .filter(|path| path.exists())
        .collect()
}

/// Agent, then keys, then password (as itself and as keyboard-interactive,
/// which is how many servers ask for one). The first that works wins; the
/// error names what was tried.
async fn authenticate(
    handle: &mut Handle<Client>,
    site: &Site,
    credentials: &Credentials,
) -> Result<(), String> {
    let user = if site.user.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "root".into())
    } else {
        site.user.clone()
    };
    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();
    let mut tried = Vec::new();
    let mut notes = Vec::new();

    #[cfg(unix)]
    if site.use_agent {
        match russh::keys::agent::client::AgentClient::connect_env().await {
            Ok(mut agent) => {
                tried.push("SSH agent");
                let identities = agent.request_identities().await.unwrap_or_default();
                for identity in identities {
                    let AgentIdentity::PublicKey { key, .. } = identity else {
                        continue;
                    };
                    let hash = matches!(key.algorithm(), russh::keys::Algorithm::Rsa { .. })
                        .then_some(rsa_hash)
                        .flatten();
                    if let Ok(result) = handle
                        .authenticate_publickey_with(user.clone(), key, hash, &mut agent)
                        .await
                        && result.success()
                    {
                        return Ok(());
                    }
                }
            }
            Err(_) => notes.push("no SSH agent is running".to_string()),
        }
    }

    let keys = match site
        .key_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        Some(path) => vec![expand_home(path.trim())],
        None => default_keys(),
    };
    for path in keys {
        let key = match load_secret_key(&path, credentials.passphrase.as_deref()) {
            Ok(key) => key,
            Err(error) => {
                notes.push(format!("{} could not be read: {error}", path.display()));
                continue;
            }
        };
        tried.push("key file");
        let hash = matches!(key.algorithm(), russh::keys::Algorithm::Rsa { .. })
            .then_some(rsa_hash)
            .flatten();
        let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
        match handle.authenticate_publickey(user.clone(), key).await {
            Ok(result) if result.success() => return Ok(()),
            Ok(_) => {}
            Err(error) => notes.push(error.to_string()),
        }
    }

    if let Some(password) = credentials.password.as_deref().filter(|p| !p.is_empty()) {
        tried.push("password");
        match handle.authenticate_password(user.clone(), password).await {
            Ok(AuthResult::Success) => return Ok(()),
            Ok(AuthResult::Failure { .. }) => {}
            Err(error) => notes.push(error.to_string()),
        }
        if keyboard_interactive(handle, &user, password).await {
            return Ok(());
        }
    }

    let mut message = format!("{} was not accepted for {user}", site.host);
    if tried.is_empty() {
        message = format!(
            "Nothing to sign in to {} with: no agent identity, no key file, no password.",
            site.host
        );
    } else {
        message.push_str(&format!(" (tried {}).", tried.join(", ")));
    }
    if !notes.is_empty() {
        message.push(' ');
        message.push_str(&notes.join("; "));
    }
    Err(message)
}

async fn keyboard_interactive(handle: &mut Handle<Client>, user: &str, password: &str) -> bool {
    let Ok(mut response) = handle
        .authenticate_keyboard_interactive_start(user.to_string(), None)
        .await
    else {
        return false;
    };
    for _ in 0..3 {
        match response {
            KeyboardInteractiveAuthResponse::Success => return true,
            KeyboardInteractiveAuthResponse::Failure { .. } => return false,
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = prompts.iter().map(|_| password.to_string()).collect();
                match handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                {
                    Ok(next) => response = next,
                    Err(_) => return false,
                }
            }
        }
    }
    false
}

/// Against a real server, skipped unless one is named:
/// `FILEJAM_TEST_SFTP=host:port:user:password cargo test -- --ignored`.
/// `docker run --rm -p 2222:22 atmoz/sftp:alpine tester:secret:::upload`
/// is one, with `localhost:2222:tester:secret`.
#[cfg(test)]
mod live {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use crate::transfer::{self, Job, Progress, Side};

    fn server() -> Option<(Site, String)> {
        let spec = std::env::var("FILEJAM_TEST_SFTP").ok()?;
        let mut parts = spec.splitn(4, ':');
        let site = Site {
            id: "live".into(),
            name: "live".into(),
            host: parts.next()?.into(),
            port: parts.next()?.parse().ok()?,
            user: parts.next()?.into(),
            use_agent: false,
            key_path: Some("/nonexistent".into()),
            ..Default::default()
        };
        Some((site, parts.next()?.into()))
    }

    async fn connect(
        site: &Site,
        password: &str,
        asked: Arc<AtomicUsize>,
    ) -> Result<Remote, String> {
        let (ask, mut questions) = mpsc::unbounded_channel::<HostKeyQuestion>();
        tokio::spawn(async move {
            while let Some(question) = questions.recv().await {
                asked.fetch_add(1, Ordering::SeqCst);
                assert!(question.fingerprint.starts_with("SHA256:"));
                let _ = question.reply.send(true);
            }
        });
        let credentials = Credentials {
            password: Some(password.into()),
            passphrase: None,
        };
        Remote::connect(site.clone(), credentials, ask).await
    }

    #[test]
    #[ignore]
    fn a_tree_goes_up_comes_back_and_is_deleted() {
        let Some((site, password)) = server() else {
            eprintln!("FILEJAM_TEST_SFTP not set; skipped");
            return;
        };
        let scratch = std::env::temp_dir().join(format!("filejam-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("up/tree/nested")).unwrap();
        std::fs::create_dir_all(scratch.join("down")).unwrap();
        std::fs::write(scratch.join("up/tree/a.txt"), "alpha").unwrap();
        let big: Vec<u8> = (0..3_000_000u32).map(|n| (n % 251) as u8).collect();
        std::fs::write(scratch.join("up/tree/nested/big.bin"), &big).unwrap();
        // SAFETY: the only test that reads or writes this variable.
        unsafe { std::env::set_var("FILEJAM_KNOWN_HOSTS", scratch.join("known_hosts")) };

        runtime().block_on(async {
            let asked = Arc::new(AtomicUsize::new(0));
            let remote = connect(&site, &password, asked.clone()).await.unwrap();
            assert_eq!(
                asked.load(Ordering::SeqCst),
                1,
                "a new host is asked about once"
            );
            let base = crate::fs::remote_join(&remote.home, "upload");

            let _ = remote.delete(&crate::fs::remote_join(&base, "tree")).await;
            let progress = Arc::new(Progress::default());
            transfer::run(
                Job {
                    from: Side::Local,
                    from_dir: scratch.join("up").to_string_lossy().into_owned(),
                    items: vec![("tree".into(), crate::fs::Kind::Dir)],
                    to: Side::Remote(remote.clone()),
                    to_dir: base.clone(),
                },
                progress.clone(),
            )
            .await
            .unwrap();
            assert_eq!(progress.files_done.load(Ordering::Relaxed), 2);
            assert_eq!(progress.bytes_done.load(Ordering::Relaxed), 3_000_005);

            let listed = remote
                .list(&crate::fs::remote_join(&base, "tree"))
                .await
                .unwrap();
            let mut names: Vec<_> = listed.iter().map(|e| (e.name.clone(), e.kind)).collect();
            names.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(
                names,
                vec![
                    ("a.txt".into(), crate::fs::Kind::File),
                    ("nested".into(), crate::fs::Kind::Dir)
                ]
            );

            transfer::run(
                Job {
                    from: Side::Remote(remote.clone()),
                    from_dir: base.clone(),
                    items: vec![("tree".into(), crate::fs::Kind::Dir)],
                    to: Side::Local,
                    to_dir: scratch.join("down").to_string_lossy().into_owned(),
                },
                Arc::new(Progress::default()),
            )
            .await
            .unwrap();
            assert_eq!(
                std::fs::read(scratch.join("down/tree/nested/big.bin")).unwrap(),
                big
            );
            assert_eq!(
                std::fs::read_to_string(scratch.join("down/tree/a.txt")).unwrap(),
                "alpha"
            );

            let tree = crate::fs::remote_join(&base, "tree");
            let renamed = crate::fs::remote_join(&base, "tree2");
            remote.rename(&tree, &renamed).await.unwrap();
            remote.delete(&renamed).await.unwrap();
            assert!(
                remote
                    .list(&base)
                    .await
                    .unwrap()
                    .iter()
                    .all(|e| e.name != "tree2")
            );
            assert_eq!(remote.resolve("~").await.unwrap(), remote.home);
            remote.disconnect().await;

            // Trusted once, known after: no second question.
            let again = connect(&site, &password, asked.clone()).await.unwrap();
            assert_eq!(asked.load(Ordering::SeqCst), 1);
            again.disconnect().await;

            let wrong = connect(&site, "wrong-password", asked.clone()).await;
            assert!(wrong.is_err(), "a wrong password is refused");
        });
        let _ = std::fs::remove_dir_all(&scratch);
    }
}

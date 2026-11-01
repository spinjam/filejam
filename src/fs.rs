//! What a pane lists, whichever side it is on, and the local side's own
//! operations. Paths are strings on both sides: a remote path is POSIX
//! whatever the client runs on, and a local path is the platform's own.

use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    /// A link whose target is a directory: it opens like one.
    LinkDir,
    /// A link to a file, or one that leads nowhere.
    Link,
}

impl Kind {
    pub fn is_dir(self) -> bool {
        matches!(self, Kind::Dir | Kind::LinkDir)
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    /// Unix seconds.
    pub modified: Option<i64>,
    /// Permission bits, when the side reports them.
    pub mode: Option<u32>,
}

impl Entry {
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    /// What the Kind column says.
    pub fn kind_label(&self) -> String {
        match self.kind {
            Kind::Dir => "Folder".into(),
            Kind::LinkDir => "Alias (folder)".into(),
            Kind::Link => "Alias".into(),
            Kind::File => match extension(&self.name) {
                Some(extension) => format!("{} file", extension.to_uppercase()),
                None => "Document".into(),
            },
        }
    }
}

pub fn extension(name: &str) -> Option<&str> {
    let (stem, extension) = name.rsplit_once('.')?;
    (!stem.is_empty() && !extension.is_empty()).then_some(extension)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
    Modified,
    Kind,
}

/// Folders first, then by the key; names compare the way Finder compares
/// them, case-blind and with digit runs read as numbers.
pub fn compare(a: &Entry, b: &Entry, key: SortKey, ascending: bool) -> Ordering {
    match (a.kind.is_dir(), b.kind.is_dir()) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }
    let by_key = match key {
        SortKey::Name => Ordering::Equal,
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::Modified => a.modified.cmp(&b.modified),
        SortKey::Kind => a.kind_label().cmp(&b.kind_label()),
    };
    let ordering = by_key.then_with(|| natural(&a.name, &b.name));
    if ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

/// Case-blind comparison with runs of digits compared by value: `file2`
/// before `file10`.
pub fn natural(a: &str, b: &str) -> Ordering {
    let mut a = a.chars().peekable();
    let mut b = b.chars().peekable();
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut left = String::new();
                while let Some(&c) = a.peek().filter(|c| c.is_ascii_digit()) {
                    left.push(c);
                    a.next();
                }
                let mut right = String::new();
                while let Some(&c) = b.peek().filter(|c| c.is_ascii_digit()) {
                    right.push(c);
                    b.next();
                }
                let left_trimmed = left.trim_start_matches('0');
                let right_trimmed = right.trim_start_matches('0');
                let ordering = left_trimmed
                    .len()
                    .cmp(&right_trimmed.len())
                    .then_with(|| left_trimmed.cmp(right_trimmed));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(x), Some(y)) => {
                let ordering = x.to_lowercase().cmp(y.to_lowercase());
                if ordering != Ordering::Equal {
                    return ordering;
                }
                a.next();
                b.next();
            }
        }
    }
}

// --- remote (POSIX) paths ----------------------------------------------------

pub fn remote_join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

pub fn remote_parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.rfind('/') {
        Some(0) => Some("/".into()),
        Some(index) => Some(trimmed[..index].into()),
        None => None,
    }
}

// --- local ---------------------------------------------------------------------

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn local_parent(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
}

pub fn local_join(dir: &str, name: &str) -> String {
    Path::new(dir).join(name).to_string_lossy().into_owned()
}

/// Every entry of a local directory. A link is followed for its kind and
/// size, and kept as a link if it leads nowhere.
pub fn list_local(path: &str) -> Result<Vec<Entry>, String> {
    let read = std::fs::read_dir(path).map_err(|error| format!("{path}: {error}"))?;
    let mut entries = Vec::new();
    for item in read.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        let Ok(own) = item
            .metadata()
            .or_else(|_| std::fs::symlink_metadata(item.path()))
        else {
            continue;
        };
        let link = item.file_type().map(|t| t.is_symlink()).unwrap_or(false);
        let target = if link {
            std::fs::metadata(item.path()).ok()
        } else {
            None
        };
        let meta = target.as_ref().unwrap_or(&own);
        let kind = match (link, meta.is_dir()) {
            (false, true) => Kind::Dir,
            (false, false) => Kind::File,
            (true, true) => Kind::LinkDir,
            (true, false) => Kind::Link,
        };
        entries.push(Entry {
            name,
            kind,
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified: meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_secs() as i64),
            mode: mode_of(meta),
        });
    }
    Ok(entries)
}

#[cfg(unix)]
fn mode_of(meta: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(meta.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn mode_of(_: &std::fs::Metadata) -> Option<u32> {
    None
}

pub fn local_mkdir(path: &str) -> Result<(), String> {
    std::fs::create_dir(path).map_err(|error| error.to_string())
}

pub fn local_rename(from: &str, to: &str) -> Result<(), String> {
    if Path::new(to).exists() {
        return Err(format!("“{}” already exists.", file_name(to)));
    }
    std::fs::rename(from, to).map_err(|error| error.to_string())
}

pub fn local_delete(path: &str) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .map_err(|error| error.to_string())
}

pub fn file_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string()
}

/// A path's segments for the breadcrumb: (label, full path) from the root.
pub fn crumbs(path: &str, remote: bool) -> Vec<(String, String)> {
    if remote {
        let mut out = vec![("/".to_string(), "/".to_string())];
        let mut current = String::new();
        for part in path.split('/').filter(|part| !part.is_empty()) {
            current = format!("{current}/{part}");
            out.push((part.to_string(), current.clone()));
        }
        out
    } else {
        let mut out = Vec::new();
        let mut current = PathBuf::new();
        for component in Path::new(path).components() {
            current.push(component);
            let label = match component {
                std::path::Component::RootDir => "/".to_string(),
                other => other.as_os_str().to_string_lossy().into_owned(),
            };
            out.push((label, current.to_string_lossy().into_owned()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: Kind, size: u64) -> Entry {
        Entry {
            name: name.into(),
            kind,
            size,
            modified: None,
            mode: None,
        }
    }

    #[test]
    fn names_sort_with_numbers_read_as_numbers() {
        assert_eq!(natural("file2", "file10"), Ordering::Less);
        assert_eq!(natural("File", "file"), Ordering::Equal);
        assert_eq!(natural("a", "B"), Ordering::Less);
    }

    #[test]
    fn folders_come_first_whichever_way_the_list_runs() {
        let dir = entry("zeta", Kind::Dir, 0);
        let file = entry("alpha", Kind::File, 10);
        assert_eq!(compare(&dir, &file, SortKey::Name, true), Ordering::Less);
        assert_eq!(compare(&dir, &file, SortKey::Name, false), Ordering::Less);
    }

    #[test]
    fn remote_paths_walk_up_to_the_root() {
        assert_eq!(remote_parent("/home/user").as_deref(), Some("/home"));
        assert_eq!(remote_parent("/home").as_deref(), Some("/"));
        assert_eq!(remote_parent("/"), None);
        assert_eq!(remote_join("/", "etc"), "/etc");
        assert_eq!(remote_join("/etc", "ssh"), "/etc/ssh");
    }

    #[test]
    fn crumbs_name_every_step_from_the_root() {
        let crumbs = crumbs("/var/www", true);
        assert_eq!(
            crumbs,
            vec![
                ("/".into(), "/".into()),
                ("var".into(), "/var".into()),
                ("www".into(), "/var/www".into())
            ]
        );
    }
}

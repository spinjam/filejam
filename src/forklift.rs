//! Reading ForkLift's favorites, to import them as sites and favorites.
//!
//! ForkLift 4 keeps them in a Core Data store in its group container: one
//! row per favorite, each an `NSKeyedArchiver` property list. ForkLift 3
//! kept a JSON file. Passwords are in ForkLift's keychain items and are not
//! read.

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use plist::{Dictionary, Value};

/// A server favorite, as ForkLift describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub path: Option<String>,
    /// A filejam site colour slug.
    pub color: Option<&'static str>,
}

#[derive(Debug, Default)]
pub struct Favorites {
    pub servers: Vec<Server>,
    /// Local folders, `~` expanded.
    pub folders: Vec<String>,
    /// Favorites of a kind filejam cannot open: FTP, S3, WebDAV, SMB…
    pub unsupported: usize,
}

/// Where ForkLift keeps its favorites, newest version first.
pub fn sources() -> Vec<PathBuf> {
    let home = crate::fs::home();
    vec![
        home.join("Library/Group Containers/J3CP9BBBN6.com.binarynights.ForkLift/Favorites.sqlite"),
        home.join("Library/Application Support/ForkLift/Favorites/Favorites.json"),
    ]
}

/// The newest ForkLift store there is: `Ok(None)` when ForkLift has never
/// been used here, an error when one exists but cannot be read.
pub fn find() -> Result<Option<PathBuf>, String> {
    for path in sources() {
        match std::fs::metadata(&path) {
            Ok(_) => return Ok(Some(path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(unreadable(&path, &error)),
        }
    }
    Ok(None)
}

fn unreadable(path: &Path, error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(1) {
        format!(
            "macOS did not let filejam read ForkLift's favorites ({}). Allow it when asked, \
             or choose the file yourself.",
            path.display()
        )
    } else {
        format!("{}: {error}", path.display())
    }
}

pub fn read(path: &Path) -> Result<Favorites, String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => read_json(path),
        _ => read_store(path),
    }
}

// --- ForkLift 4 ----------------------------------------------------------------

/// The store is live — ForkLift may be running and syncing it — so it is
/// copied, with its write-ahead log, and the copy is what gets opened.
fn read_store(path: &Path) -> Result<Favorites, String> {
    let scratch = std::env::temp_dir().join(format!("filejam-forklift-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|error| error.to_string())?;
    let result = (|| {
        let copy = scratch.join("Favorites.sqlite");
        std::fs::copy(path, &copy).map_err(|error| unreadable(path, &error))?;
        for suffix in ["-wal", "-shm"] {
            let side = PathBuf::from(format!("{}{suffix}", path.display()));
            if side.exists() {
                let to = PathBuf::from(format!("{}{suffix}", copy.display()));
                std::fs::copy(&side, &to).map_err(|error| unreadable(&side, &error))?;
            }
        }
        let db = rusqlite::Connection::open(&copy).map_err(|error| error.to_string())?;
        let mut query = db
            .prepare("SELECT ZDATA FROM ZFAVORITE WHERE ZDATA IS NOT NULL")
            .map_err(|_| "This is not a ForkLift favorites file.".to_string())?;
        let blobs = query
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        let records: Vec<Dictionary> = blobs.iter().filter_map(|blob| unarchive(blob)).collect();
        Ok(from_records(&records))
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

/// Turn the records into favorites, in ForkLift's own order: by group, then
/// by position in it.
fn from_records(records: &[Dictionary]) -> Favorites {
    let text = |record: &Dictionary, key: &str| {
        record
            .get(key)
            .and_then(Value::as_string)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let number = |record: &Dictionary, key: &str| {
        record.get(key).and_then(|value| {
            value
                .as_signed_integer()
                .or_else(|| value.as_unsigned_integer().map(|n| n as i64))
        })
    };
    let group_order = |id: Option<String>| {
        records
            .iter()
            .find(|record| text(record, "id") == id)
            .and_then(|record| number(record, "sortIndex"))
            .unwrap_or(i64::MAX)
    };
    let mut ordered: Vec<&Dictionary> = records.iter().collect();
    ordered.sort_by_key(|record| {
        (
            group_order(text(record, "parentID")),
            number(record, "sortIndex").unwrap_or(i64::MAX),
        )
    });

    let mut favorites = Favorites::default();
    for record in ordered {
        match text(record, "type").as_deref() {
            Some("sftp") => {
                let Some(host) = text(record, "host") else {
                    continue;
                };
                let (host, port_in_host) = split_port(&host);
                favorites.servers.push(Server {
                    name: text(record, "name").unwrap_or_else(|| host.clone()),
                    host,
                    port: number(record, "port")
                        .and_then(|port| u16::try_from(port).ok())
                        .filter(|port| *port > 0)
                        .or(port_in_host),
                    user: text(record, "username").unwrap_or_default(),
                    path: text(record, "path"),
                    color: number(record, "colorLabel").and_then(color),
                });
            }
            Some("local") => {
                if let Some(path) = text(record, "path") {
                    favorites.folders.push(expand(&path));
                }
            }
            Some("group" | "tags" | "tagGroup") | None => {}
            Some(_) => favorites.unsupported += 1,
        }
    }
    favorites
}

/// Undo `NSKeyedArchiver`: follow the object references from the root and
/// rebuild plain dictionaries, arrays and strings.
fn unarchive(blob: &[u8]) -> Option<Dictionary> {
    let archive = Value::from_reader(Cursor::new(blob)).ok()?;
    let archive = archive.as_dictionary()?;
    let objects = archive.get("$objects")?.as_array()?;
    let root = archive.get("$top")?.as_dictionary()?.get("root")?;
    resolve(root, objects, 0)?.into_dictionary()
}

fn resolve(value: &Value, objects: &[Value], depth: usize) -> Option<Value> {
    if depth > 64 {
        return None;
    }
    let value = match value {
        Value::Uid(uid) => objects.get(uid.get() as usize)?,
        other => other,
    };
    match value {
        Value::String(s) if s == "$null" => None,
        Value::Dictionary(dict) => {
            if let (Some(Value::Array(keys)), Some(Value::Array(values))) =
                (dict.get("NS.keys"), dict.get("NS.objects"))
            {
                let mut out = Dictionary::new();
                for (key, value) in keys.iter().zip(values) {
                    if let (Some(Value::String(key)), Some(value)) = (
                        resolve(key, objects, depth + 1),
                        resolve(value, objects, depth + 1),
                    ) {
                        out.insert(key, value);
                    }
                }
                Some(Value::Dictionary(out))
            } else if let Some(Value::Array(values)) = dict.get("NS.objects") {
                Some(Value::Array(
                    values
                        .iter()
                        .filter_map(|value| resolve(value, objects, depth + 1))
                        .collect(),
                ))
            } else if let Some(string) = dict.get("NS.string") {
                resolve(string, objects, depth + 1)
            } else {
                let mut out = Dictionary::new();
                for (key, value) in dict {
                    if key != "$class"
                        && let Some(value) = resolve(value, objects, depth + 1)
                    {
                        out.insert(key.clone(), value);
                    }
                }
                Some(Value::Dictionary(out))
            }
        }
        other => Some(other.clone()),
    }
}

// --- ForkLift 3 ----------------------------------------------------------------

fn read_json(path: &Path) -> Result<Favorites, String> {
    let text = std::fs::read_to_string(path).map_err(|error| unreadable(path, &error))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "This is not a ForkLift favorites file.")?;
    let mut favorites = Favorites::default();
    if let Some(items) = json.get("favorites").and_then(|v| v.as_array()) {
        walk_json(items, &mut favorites);
    }
    Ok(favorites)
}

fn walk_json(items: &[serde_json::Value], favorites: &mut Favorites) {
    for item in items {
        let attributes = item.get("attributes");
        let text = |key: &str| {
            attributes
                .and_then(|a| a.get(key))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        match item.get("type").and_then(|v| v.as_str()) {
            Some("Group") => {}
            Some("SFTP") => {
                if let Some(server) = text("Server") {
                    let (host, port_in_host) = split_port(&server);
                    let port = attributes
                        .and_then(|a| a.get("Port"))
                        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
                        .and_then(|port| u16::try_from(port).ok())
                        .filter(|port| *port > 0)
                        .or(port_in_host);
                    favorites.servers.push(Server {
                        name: text("Name").unwrap_or_else(|| host.clone()),
                        host,
                        port,
                        user: text("Username").unwrap_or_default(),
                        path: text("Path").or_else(|| text("RemotePath")),
                        color: None,
                    });
                }
            }
            Some("Local") => {
                if let Some(path) = text("Path") {
                    favorites.folders.push(expand(&path));
                }
            }
            _ => favorites.unsupported += 1,
        }
        if let Some(children) = item.get("childItems").and_then(|v| v.as_array()) {
            walk_json(children, favorites);
        }
    }
}

// --- shared ----------------------------------------------------------------------

/// `host:2222` written into the host field.
fn split_port(host: &str) -> (String, Option<u16>) {
    if let Some((name, port)) = host.rsplit_once(':')
        && !name.contains(':')
        && let Ok(port) = port.parse::<u16>()
    {
        return (name.to_string(), Some(port));
    }
    (host.to_string(), None)
}

fn expand(path: &str) -> String {
    match path.strip_prefix('~') {
        Some(rest) => format!("{}{rest}", crate::fs::home().to_string_lossy()),
        None => path.to_string(),
    }
}

/// ForkLift numbers its labels the way Finder does.
fn color(label: i64) -> Option<&'static str> {
    Some(match label {
        1 => "gray",
        2 => "green",
        3 => "purple",
        4 => "blue",
        5 => "yellow",
        6 => "red",
        7 => "orange",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pairs: &[(&str, Value)]) -> Dictionary {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    fn s(text: &str) -> Value {
        Value::String(text.into())
    }

    #[test]
    fn records_become_servers_and_folders_in_forklift_order() {
        let records = vec![
            record(&[("type", s("group")), ("id", s("g2")), ("sortIndex", 2.into())]),
            record(&[("type", s("group")), ("id", s("g1")), ("sortIndex", 1.into())]),
            record(&[
                ("type", s("sftp")),
                ("parentID", s("g2")),
                ("sortIndex", 0.into()),
                ("name", s("Later")),
                ("host", s("b.example.com:2222")),
                ("username", s("deploy")),
            ]),
            record(&[
                ("type", s("sftp")),
                ("parentID", s("g1")),
                ("sortIndex", 0.into()),
                ("name", s("First")),
                ("host", s("a.example.com")),
                ("username", s("root")),
                ("path", s("/var/www")),
                ("colorLabel", 6.into()),
            ]),
            record(&[("type", s("local")), ("parentID", s("g1")), ("path", s("/srv"))]),
            record(&[("type", s("ftp")), ("host", s("ftp.example.com"))]),
            record(&[("type", s("tags")), ("name", s("Red"))]),
        ];
        let favorites = from_records(&records);
        assert_eq!(
            favorites.servers,
            vec![
                Server {
                    name: "First".into(),
                    host: "a.example.com".into(),
                    port: None,
                    user: "root".into(),
                    path: Some("/var/www".into()),
                    color: Some("red"),
                },
                Server {
                    name: "Later".into(),
                    host: "b.example.com".into(),
                    port: Some(2222),
                    user: "deploy".into(),
                    path: None,
                    color: None,
                },
            ]
        );
        assert_eq!(favorites.folders, vec!["/srv".to_string()]);
        assert_eq!(favorites.unsupported, 1);
    }

    #[test]
    fn a_keyed_archive_unpacks_to_a_plain_dictionary() {
        // What NSKeyedArchiver writes for {"name": "Web", "port": 22}.
        let uid = |n| Value::Uid(plist::Uid::new(n));
        let objects = vec![
            s("$null"),
            record(&[
                ("NS.keys", Value::Array(vec![uid(2), uid(3)])),
                ("NS.objects", Value::Array(vec![uid(4), 22.into()])),
                ("$class", uid(5)),
            ])
            .into(),
            s("name"),
            s("port"),
            s("Web"),
            record(&[("$classname", s("NSDictionary"))]).into(),
        ];
        let archive: Value = record(&[
            ("$archiver", s("NSKeyedArchiver")),
            ("$top", record(&[("root", uid(1))]).into()),
            ("$objects", Value::Array(objects)),
        ])
        .into();
        let mut blob = Vec::new();
        archive.to_writer_binary(&mut blob).unwrap();
        let unpacked = unarchive(&blob).unwrap();
        assert_eq!(unpacked.get("name").and_then(Value::as_string), Some("Web"));
        assert_eq!(unpacked.get("port").and_then(Value::as_signed_integer), Some(22));
    }

    /// `FILEJAM_FORKLIFT=/path/to/Favorites.sqlite cargo test -- --ignored`
    /// reads a real store and prints what it holds, without hosts or users.
    #[test]
    #[ignore]
    fn reads_a_real_forklift_store() {
        let Some(path) = std::env::var_os("FILEJAM_FORKLIFT") else {
            return;
        };
        let favorites = read(Path::new(&path)).unwrap();
        for server in &favorites.servers {
            println!(
                "{} · port {:?} · path {} · colour {:?}",
                server.name,
                server.port,
                if server.path.is_some() { "yes" } else { "no" },
                server.color
            );
        }
        println!(
            "{} servers, {} folders, {} unsupported",
            favorites.servers.len(),
            favorites.folders.len(),
            favorites.unsupported
        );
        assert!(!favorites.servers.is_empty());
    }

    #[test]
    fn a_port_written_into_the_host_is_split_off() {
        assert_eq!(split_port("example.com:2222"), ("example.com".into(), Some(2222)));
        assert_eq!(split_port("example.com"), ("example.com".into(), None));
        assert_eq!(split_port("fe80::1"), ("fe80::1".into(), None));
    }
}

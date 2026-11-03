//! What survives a restart: the saved sites and a few settings in one TOML
//! file, and every secret in the platform keychain — never in the file.

use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    /// Where the remote pane opens; the login directory when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_path: Option<String>,
    /// Where the other pane goes when the site connects, if anywhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    /// A private key to offer. When unset, the usual ~/.ssh keys are tried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(default = "yes")]
    pub use_agent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn default_port() -> u16 {
    22
}

fn yes() -> bool {
    true
}

impl Site {
    /// `user@host:port`, the line under a site's name.
    pub fn address(&self) -> String {
        let user = if self.user.is_empty() {
            String::new()
        } else {
            format!("{}@", self.user)
        };
        if self.port == 22 {
            format!("{user}{}", self.host)
        } else {
            format!("{user}{}:{}", self.host, self.port)
        }
    }
}

/// Where the window was left, in logical pixels. When maximized, the place it
/// goes back to.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct WindowPlace {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub maximized: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default)]
    pub show_hidden: bool,
    /// Whether each pane, left and right, shows its folder tree.
    #[serde(default)]
    pub folder_tree: [bool; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowPlace>,
    /// The text editor Open in Editor uses: an app's name on macOS, a
    /// command elsewhere. The system's own when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
}

/// A folder pinned to the sidebar, on this computer or on a saved site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Favorite {
    pub path: String,
    /// The site it lives on, by id; this computer when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Stored {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub sites: Vec<Site>,
    #[serde(default)]
    pub favorites: Vec<Favorite>,
}

/// macOS keeps Application Support, Linux the XDG config directory, Windows
/// the roaming AppData.
fn directory() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library/Application Support/filejam"))
    }
    #[cfg(target_os = "windows")]
    {
        Some(PathBuf::from(std::env::var_os("APPDATA")?).join("filejam"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        match std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
            Some(config) => Some(PathBuf::from(config).join("filejam")),
            None => Some(PathBuf::from(std::env::var_os("HOME")?).join(".config/filejam")),
        }
    }
}

fn file() -> Option<PathBuf> {
    directory().map(|directory| directory.join("sites.toml"))
}

pub fn load() -> Stored {
    file()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

/// Written beside and renamed over, so a crash mid-write cannot leave half a
/// file where the sites were.
pub fn save(stored: &Stored) -> Result<(), String> {
    let path = file().ok_or("No home directory to save into.")?;
    let directory = path.parent().ok_or("No directory to save into.")?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let text = toml::to_string_pretty(stored).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text).map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error: io::Error| error.to_string())
}

/// A slug from the name, unique among the ids already taken.
pub fn site_id(name: &str, taken: &[String]) -> String {
    let slug = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if slug.is_empty() {
        "site".to_string()
    } else {
        slug
    };
    let mut candidate = base.clone();
    let mut suffix = 2;
    while taken.iter().any(|id| id == &candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}

#[derive(Clone, Copy)]
pub enum Secret {
    Password,
    Passphrase,
}

impl Secret {
    fn account(self, site_id: &str) -> String {
        match self {
            Secret::Password => format!("{site_id}:password"),
            Secret::Passphrase => format!("{site_id}:passphrase"),
        }
    }
}

const KEYCHAIN_SERVICE: &str = if cfg!(debug_assertions) {
    "filejam-dev"
} else {
    "filejam"
};

fn entry(site_id: &str, secret: Secret) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, &secret.account(site_id))
        .map_err(|error| format!("Could not reach the keychain: {error}"))
}

/// `Ok(None)` only for a keychain that holds nothing for this site.
pub fn secret(site_id: &str, secret: Secret) -> Result<Option<String>, String> {
    match entry(site_id, secret)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read the keychain: {error}")),
    }
}

/// An empty value removes the secret rather than saving a blank one.
pub fn set_secret(site_id: &str, secret: Secret, value: &str) -> Result<(), String> {
    let entry = entry(site_id, secret)?;
    if value.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("Could not update the keychain: {error}")),
        }
    } else {
        entry
            .set_password(value)
            .map_err(|error| format!("Could not save to the keychain: {error}"))
    }
}

pub fn forget_secrets(site_id: &str) {
    for secret in [Secret::Password, Secret::Passphrase] {
        let _ = set_secret(site_id, secret, "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_slugs_and_never_collide() {
        assert_eq!(site_id("My Server!", &[]), "my-server");
        assert_eq!(site_id("web", &["web".into()]), "web-2");
        assert_eq!(site_id("???", &[]), "site");
    }

    #[test]
    fn a_site_round_trips_through_toml() {
        let stored = Stored {
            settings: Settings {
                theme: Some("Filejam Dark".into()),
                show_hidden: true,
                folder_tree: [true, false],
                window: Some(WindowPlace {
                    x: 40.,
                    y: 60.5,
                    width: 1440.,
                    height: 900.,
                    maximized: true,
                }),
                editor: Some("Visual Studio Code".into()),
            },
            sites: vec![Site {
                id: "web".into(),
                name: "Web".into(),
                host: "example.com".into(),
                port: 2222,
                user: "deploy".into(),
                remote_path: Some("/var/www".into()),
                local_path: None,
                key_path: None,
                use_agent: true,
                color: Some("blue".into()),
            }],
            favorites: vec![
                Favorite {
                    path: "/Users/me/Projects".into(),
                    site: None,
                },
                Favorite {
                    path: "/var/www/html".into(),
                    site: Some("web".into()),
                },
            ],
        };
        let text = toml::to_string_pretty(&stored).unwrap();
        assert_eq!(toml::from_str::<Stored>(&text).unwrap(), stored);
    }

    #[test]
    fn addresses_leave_out_the_default_port() {
        let mut site = Site {
            host: "h".into(),
            port: 22,
            user: "u".into(),
            ..Default::default()
        };
        assert_eq!(site.address(), "u@h");
        site.port = 2200;
        assert_eq!(site.address(), "u@h:2200");
    }
}

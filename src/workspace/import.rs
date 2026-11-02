//! File > Import from ForkLift…: read ForkLift's favorites off the UI
//! thread, show what would be added, add it on confirmation.

#[cfg(target_os = "macos")]
use std::path::PathBuf;

use gpui::Context;
#[cfg(target_os = "macos")]
use gpui::PathPromptOptions;

use crate::{
    fs,
    store::{self, Favorite, Site},
};

use super::{Modal, Workspace};

/// What an import adds, worked out against the sites already saved.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub struct ImportPlan {
    pub sites: Vec<Site>,
    pub folders: Vec<String>,
    /// Servers already saved here, by host, user and port.
    pub existing: usize,
    pub unsupported: usize,
}

impl Workspace {
    pub fn import_forklift(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        self.read_forklift(None, cx);
        #[cfg(not(target_os = "macos"))]
        {
            self.set_status("ForkLift is a macOS app: there is nothing to import here.", true);
            cx.notify();
        }
    }

    /// Read `path`, or the newest ForkLift store when `None`; ask for the
    /// file when there is none, or when macOS keeps filejam out of it.
    #[cfg(target_os = "macos")]
    fn read_forklift(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        use crate::{forklift, remote::runtime};

        enum Found {
            Read(Result<forklift::Favorites, String>),
            Nothing,
            Blocked(String),
        }

        let chosen = path.is_some();
        let task = runtime().spawn_blocking(move || {
            let path = match path {
                Some(path) => path,
                None => match forklift::find() {
                    Ok(Some(path)) => path,
                    Ok(None) => return Found::Nothing,
                    Err(error) => return Found::Blocked(error),
                },
            };
            match forklift::read(&path) {
                Err(error) if !chosen && error.contains("did not let filejam") => {
                    Found::Blocked(error)
                }
                read => Found::Read(read),
            }
        });
        cx.spawn(async move |this, cx| {
            let found = task
                .await
                .unwrap_or_else(|error| Found::Read(Err(error.to_string())));
            this.update(cx, |this, cx| {
                match found {
                    Found::Read(Ok(favorites)) => this.plan_import(favorites, cx),
                    Found::Read(Err(error)) => this.set_status(error, true),
                    Found::Nothing => {
                        this.set_status(
                            "No ForkLift favorites found. Choose Favorites.sqlite or Favorites.json.",
                            false,
                        );
                        this.pick_forklift_file(cx);
                    }
                    Found::Blocked(error) => {
                        this.set_status(error, true);
                        this.pick_forklift_file(cx);
                    }
                }
                cx.notify();
            })
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    fn pick_forklift_file(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update(cx, |this, cx| this.read_forklift(Some(path), cx));
            }
        })
        .detach();
    }

    #[cfg(target_os = "macos")]
    fn plan_import(&mut self, favorites: crate::forklift::Favorites, cx: &mut Context<Self>) {
        let mut taken: Vec<String> = self.stored.sites.iter().map(|s| s.id.clone()).collect();
        let mut known: Vec<(String, String, u16)> =
            self.stored.sites.iter().map(server_key).collect();
        let mut plan = ImportPlan {
            sites: Vec::new(),
            folders: Vec::new(),
            existing: 0,
            unsupported: favorites.unsupported,
        };
        for server in favorites.servers {
            let site = Site {
                id: store::site_id(&server.name, &taken),
                name: server.name,
                host: server.host,
                port: server.port.unwrap_or(22),
                user: server.user,
                remote_path: server.path,
                local_path: None,
                key_path: None,
                use_agent: true,
                color: server.color.map(str::to_string),
            };
            let key = server_key(&site);
            if known.contains(&key) {
                plan.existing += 1;
                continue;
            }
            known.push(key);
            taken.push(site.id.clone());
            plan.sites.push(site);
        }
        for folder in favorites.folders {
            let favorite = Favorite {
                path: folder.clone(),
                site: None,
            };
            if !self.stored.favorites.contains(&favorite) && !plan.folders.contains(&folder) {
                plan.folders.push(folder);
            }
        }
        if plan.sites.is_empty() && plan.folders.is_empty() {
            self.set_status("Everything in ForkLift's favorites is already here.", false);
        } else {
            self.modal = Some(Modal::Import(plan));
        }
        cx.notify();
    }

    pub fn apply_import(&mut self, plan: ImportPlan, cx: &mut Context<Self>) {
        let (sites, folders) = (plan.sites.len(), plan.folders.len());
        self.stored.sites.extend(plan.sites);
        self.stored
            .sites
            .sort_by(|a, b| fs::natural(&a.name, &b.name));
        self.stored
            .favorites
            .extend(plan.folders.into_iter().map(|path| Favorite { path, site: None }));
        self.persist();
        self.set_status(
            format!(
                "Imported {} and {} from ForkLift.",
                count(sites, "site"),
                count(folders, "folder")
            ),
            false,
        );
        cx.notify();
    }
}

pub fn count(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

fn server_key(site: &Site) -> (String, String, u16) {
    (site.host.to_lowercase(), site.user.clone(), site.port)
}

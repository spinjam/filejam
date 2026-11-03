//! Open in Editor: a file goes to the text editor chosen in Settings. A local
//! file opens where it is; a remote one is downloaded to a scratch copy, and
//! every save to that copy is uploaded back, the way ForkLift does it.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, SystemTime},
};

use gpui::Context;

use crate::{
    fs::{self, Entry, Kind},
    transfer::{Job, Side},
};

use super::{Direction, Location, Workspace};

/// How often the scratch copies are looked at for a save.
const WATCH_EVERY: Duration = Duration::from_secs(1);

/// A remote file open in the editor, through its scratch copy.
pub struct Edited {
    pub site: String,
    /// The folder on the site and the file's name in it.
    pub remote_dir: String,
    pub name: String,
    /// The folder of the scratch copy, which holds only it.
    pub local_dir: PathBuf,
    /// The copy's modification time when last downloaded or uploaded.
    pub seen: Option<SystemTime>,
    pub uploading: bool,
    /// The last upload failed: the copy holds changes the site lacks.
    pub unsaved: bool,
}

impl Edited {
    fn local_path(&self) -> PathBuf {
        self.local_dir.join(&self.name)
    }
}

impl Workspace {
    /// Open the pane's chosen files, or `items` when given, in the editor.
    pub fn open_in_editor(&mut self, ix: usize, items: Option<Vec<Entry>>, cx: &mut Context<Self>) {
        let items: Vec<Entry> = items
            .unwrap_or_else(|| self.panes[ix].targets())
            .into_iter()
            .filter(|entry| !entry.kind.is_dir())
            .collect();
        if items.is_empty() {
            return;
        }
        match self.panes[ix].location.clone() {
            Location::Local => {
                for entry in items {
                    let path = self.panes[ix].join(&entry.name);
                    self.launch_editor(PathBuf::from(path), cx);
                }
            }
            Location::Remote(site) => {
                for entry in items {
                    self.edit_remote(ix, &site, entry, cx);
                }
            }
        }
    }

    fn edit_remote(&mut self, ix: usize, site: &str, entry: Entry, cx: &mut Context<Self>) {
        let remote_dir = self.panes[ix].path.clone();
        // Already open: the copy there is the one being watched.
        if let Some(edited) = self.editing.iter().find(|edited| {
            edited.site == site && edited.remote_dir == remote_dir && edited.name == entry.name
        }) {
            let path = edited.local_path();
            self.launch_editor(path, cx);
            return;
        }
        let Some(side) = self.side_of(ix) else {
            self.set_status("Not connected.", true);
            cx.notify();
            return;
        };
        let local_dir = scratch_root().join(format!("{}-{}", std::process::id(), self.next_edit));
        self.next_edit += 1;
        if let Err(error) = std::fs::create_dir_all(&local_dir) {
            self.set_status(format!("Could not make a copy to edit: {error}"), true);
            cx.notify();
            return;
        }
        let job = Job {
            from: side,
            from_dir: remote_dir.clone(),
            items: vec![(entry.name.clone(), entry.kind)],
            to: Side::Local,
            to_dir: local_dir.to_string_lossy().into_owned(),
        };
        let label = format!("{} → editor", entry.name);
        let done = self.begin_transfer(job, label, Direction::Download, vec![site.to_string()], cx);
        let site = site.to_string();
        cx.spawn(async move |this, cx| {
            let result = done.await;
            this.update(cx, |this, cx| {
                if let Err(error) = result {
                    let _ = std::fs::remove_dir_all(&local_dir);
                    this.set_status(format!("Could not open {}: {error}", entry.name), true);
                    cx.notify();
                    return;
                }
                // The site went away while the file came down.
                if !this.conns.contains_key(&site) {
                    let _ = std::fs::remove_dir_all(&local_dir);
                    return;
                }
                let edited = Edited {
                    seen: modified(&local_dir.join(&entry.name)),
                    site,
                    remote_dir,
                    name: entry.name,
                    local_dir,
                    uploading: false,
                    unsaved: false,
                };
                let path = edited.local_path();
                this.editing.push(edited);
                this.launch_editor(path, cx);
                this.watch_edits(cx);
            })
        })
        .detach();
    }

    /// Start the editor on `path`, reporting in the status line if it would
    /// not start.
    fn launch_editor(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let editor = self
            .stored
            .settings
            .editor
            .clone()
            .filter(|editor| !editor.trim().is_empty());
        let mut command = editor_command(editor.as_deref(), &path);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let name = editor.unwrap_or_else(|| "the text editor".into());
                self.set_status(format!("Could not start {name}: {error}"), true);
                cx.notify();
                return;
            }
        };
        // `open -a` exits at once, and fails when no app has that name.
        cx.spawn(async move |this, cx| {
            let output = cx
                .background_executor()
                .spawn(async move { child.wait_with_output() })
                .await;
            let failed = match output {
                Ok(output) if output.status.success() => None,
                Ok(output) => Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
                Err(error) => Some(error.to_string()),
            };
            if let Some(error) = failed {
                let _ = this.update(cx, |this, cx| {
                    let name = editor.unwrap_or_else(|| "the text editor".into());
                    let text = if error.is_empty() {
                        format!(
                            "Could not open {} in {name}.",
                            fs::file_name(&path.to_string_lossy())
                        )
                    } else {
                        format!("Could not open in {name}: {error}")
                    };
                    this.set_status(text, true);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Look at the scratch copies while any are open, uploading each save.
    fn watch_edits(&mut self, cx: &mut Context<Self>) {
        if self.watching {
            return;
        }
        self.watching = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(WATCH_EVERY).await;
                let going = this
                    .update(cx, |this, cx| {
                        if this.editing.is_empty() {
                            this.watching = false;
                            return false;
                        }
                        this.upload_saves(cx);
                        true
                    })
                    .unwrap_or(false);
                if !going {
                    break;
                }
            }
        })
        .detach();
    }

    fn upload_saves(&mut self, cx: &mut Context<Self>) {
        let mut saved = Vec::new();
        for (index, edited) in self.editing.iter_mut().enumerate() {
            if edited.uploading {
                continue;
            }
            let now = modified(&edited.local_path());
            if now.is_some() && now != edited.seen {
                // Taken before the upload: a save during it is caught next time.
                edited.seen = now;
                edited.uploading = true;
                saved.push(index);
            }
        }
        for index in saved {
            let edited = &self.editing[index];
            let Some(remote) = self
                .remote_of(&Location::Remote(edited.site.clone()))
                .cloned()
            else {
                self.editing[index].uploading = false;
                continue;
            };
            let (site, remote_dir, name) = (
                edited.site.clone(),
                edited.remote_dir.clone(),
                edited.name.clone(),
            );
            let job = Job {
                from: Side::Local,
                from_dir: edited.local_dir.to_string_lossy().into_owned(),
                items: vec![(name.clone(), Kind::File)],
                to: Side::Remote(remote),
                to_dir: remote_dir.clone(),
            };
            let label = format!("{name} → {}", fs::file_name(&remote_dir));
            let local_dir = edited.local_dir.clone();
            let done = self.begin_transfer(job, label, Direction::Upload, vec![site.clone()], cx);
            cx.spawn(async move |this, cx| {
                let result = done.await;
                this.update(cx, |this, cx| {
                    let Some(edited) = this
                        .editing
                        .iter_mut()
                        .find(|edited| edited.local_dir == local_dir)
                    else {
                        return;
                    };
                    edited.uploading = false;
                    match result {
                        Ok(()) => {
                            edited.unsaved = false;
                            this.set_status(format!("Uploaded {name}"), false);
                        }
                        Err(error) => {
                            edited.unsaved = true;
                            // Try again on the next look, so a later save still goes up.
                            edited.seen = None;
                            this.set_status(
                                format!(
                                    "Could not upload {name}: {error}. Your changes are kept in {}",
                                    local_dir.display()
                                ),
                                true,
                            );
                        }
                    }
                    // Whichever pane shows the folder lists it again.
                    let location = Location::Remote(site.clone());
                    for ix in 0..2 {
                        if this.panes[ix].location == location && this.panes[ix].path == remote_dir
                        {
                            this.load(ix, cx);
                        }
                    }
                    cx.notify();
                })
            })
            .detach();
        }
    }

    /// Stop watching a site's files, throwing away the copies the site
    /// already has; a copy with changes it lacks is left, and said where.
    pub(super) fn stop_editing(&mut self, site: &str) {
        let (ended, kept): (Vec<Edited>, Vec<Edited>) = std::mem::take(&mut self.editing)
            .into_iter()
            .partition(|edited| edited.site == site);
        self.editing = kept;
        for edited in ended {
            if edited.unsaved {
                self.set_status(
                    format!(
                        "{} was not uploaded. Your changes are kept in {}",
                        edited.name,
                        edited.local_dir.display()
                    ),
                    true,
                );
            } else {
                let _ = std::fs::remove_dir_all(&edited.local_dir);
            }
        }
    }

    /// At quit: throw away every copy the sites already have.
    pub(super) fn stop_all_editing(&mut self) {
        for edited in std::mem::take(&mut self.editing) {
            if !edited.unsaved {
                let _ = std::fs::remove_dir_all(&edited.local_dir);
            }
        }
    }
}

/// Where the scratch copies live, one folder each.
fn scratch_root() -> PathBuf {
    std::env::temp_dir().join("filejam-edit")
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// On macOS the editor is an app's name, opened with `open -a`; elsewhere it
/// is a command, with its arguments, that is given the file last. Unset, the
/// system's own text editor.
fn editor_command(editor: Option<&str>, path: &Path) -> Command {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        match editor {
            Some(app) => command.args(["-a", app.trim()]),
            None => command.arg("-t"),
        };
        command.arg(path);
        command
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut words = editor
            .map(|editor| editor.split_whitespace())
            .into_iter()
            .flatten();
        let mut command = match words.next() {
            Some(program) => {
                let mut command = Command::new(program);
                command.args(words);
                command
            }
            #[cfg(target_os = "windows")]
            None => Command::new("notepad"),
            #[cfg(not(target_os = "windows"))]
            None => Command::new("xdg-open"),
        };
        command.arg(path);
        command
    }
}

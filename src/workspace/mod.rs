//! The window: a sidebar of places and sites, two panes side by side, the
//! transfers drawer under them. All state lives here, in one entity, so an
//! operation that spans both panes — a copy, a drop — is one method.

mod form;
mod pane;
mod render;

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use gpui::{App, AppContext, Context, Entity, FocusHandle, Focusable, ScrollStrategy, Window};
use gpui_component::{
    input::{InputEvent, InputState},
    resizable::ResizableState,
};
use tokio::sync::mpsc;

use crate::{
    fs::{self, Entry, SortKey},
    remote::{Credentials, HostKeyQuestion, Remote, runtime},
    store::{self, Secret, Site, Stored},
    theme::{self, Theme},
    transfer::{self, Job, Progress, Side},
};

pub use form::SiteForm;
pub use pane::{Location, Pane};

pub enum Conn {
    Connecting,
    Connected(Remote),
    Failed,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Upload,
    Download,
    Copy,
}

pub enum TransferState {
    Running,
    Done,
    Failed(String),
}

pub struct TransferRow {
    pub id: u64,
    pub label: String,
    pub direction: Direction,
    pub progress: Arc<Progress>,
    pub state: TransferState,
}

pub enum Modal {
    Site(Box<SiteForm>),
    NewFolder {
        pane: usize,
        input: Entity<InputState>,
    },
    Rename {
        pane: usize,
        from: String,
        input: Entity<InputState>,
    },
    Delete {
        pane: usize,
        items: Vec<Entry>,
    },
    Replace {
        from: usize,
        items: Vec<Entry>,
        clashes: Vec<String>,
    },
}

/// What a drag between the panes carries.
#[derive(Clone)]
pub struct DraggedFiles {
    pub pane: usize,
    pub items: Vec<Entry>,
}

pub struct StatusLine {
    pub text: String,
    pub error: bool,
}

pub struct Workspace {
    pub focus: FocusHandle,
    pub stored: Stored,
    pub conns: HashMap<String, Conn>,
    pub panes: [Pane; 2],
    pub active: usize,
    pub sidebar_visible: bool,
    pub shell_split: Entity<ResizableState>,
    pub pane_split: Entity<ResizableState>,
    pub transfers: Vec<TransferRow>,
    pub transfers_visible: bool,
    next_transfer: u64,
    polling: bool,
    pub modal: Option<Modal>,
    pub host_questions: VecDeque<HostKeyQuestion>,
    asker: mpsc::UnboundedSender<HostKeyQuestion>,
    pub status: Option<StatusLine>,
    /// Set by anything that wants the focused pane's list to take focus on
    /// the next frame: the element has to be mounted first.
    pub wants_focus: bool,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let stored = store::load();
        let home = fs::home().to_string_lossy().into_owned();
        let panes = [
            Pane::new(home.clone(), window, cx),
            Pane::new(home, window, cx),
        ];

        let (asker, mut questions) = mpsc::unbounded_channel::<HostKeyQuestion>();
        cx.spawn(async move |this, cx| {
            while let Some(question) = questions.recv().await {
                if this
                    .update(cx, |this, cx| {
                        this.host_questions.push_back(question);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        for ix in 0..2 {
            let filter = panes[ix].filter.clone();
            cx.subscribe_in(
                &filter,
                window,
                move |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        let show_hidden = this.stored.settings.show_hidden;
                        this.panes[ix].rebuild(show_hidden, cx);
                        cx.notify();
                    }
                },
            )
            .detach();
            let path_input = panes[ix].path_input.clone();
            cx.subscribe_in(
                &path_input,
                window,
                move |this, input, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let typed = input.read(cx).value().trim().to_string();
                        this.panes[ix].editing_path = false;
                        if !typed.is_empty() {
                            this.go_to_typed(ix, typed, cx);
                        }
                        this.focus_pane(ix, window, cx);
                    }
                    InputEvent::Blur => {
                        this.panes[ix].editing_path = false;
                        cx.notify();
                    }
                    _ => {}
                },
            )
            .detach();
        }

        let mut this = Self {
            focus: cx.focus_handle(),
            stored,
            conns: HashMap::new(),
            panes,
            active: 0,
            sidebar_visible: true,
            shell_split: cx.new(|_| ResizableState::default()),
            pane_split: cx.new(|_| ResizableState::default()),
            transfers: Vec::new(),
            transfers_visible: false,
            next_transfer: 0,
            polling: false,
            modal: None,
            host_questions: VecDeque::new(),
            asker,
            status: None,
            wants_focus: true,
        };
        // The right pane starts one level into home, on Downloads when there
        // is one: two identical panes read as one pane drawn twice.
        let downloads = fs::home().join("Downloads");
        if downloads.is_dir() {
            this.panes[1].path = downloads.to_string_lossy().into_owned();
        }
        this.load(0, cx);
        this.load(1, cx);
        this
    }

    pub fn site(&self, id: &str) -> Option<&Site> {
        self.stored.sites.iter().find(|site| site.id == id)
    }

    pub fn remote_of(&self, location: &Location) -> Option<&Remote> {
        match location {
            Location::Local => None,
            Location::Remote(id) => match self.conns.get(id) {
                Some(Conn::Connected(remote)) => Some(remote),
                _ => None,
            },
        }
    }

    fn side_of(&self, ix: usize) -> Option<Side> {
        match &self.panes[ix].location {
            Location::Local => Some(Side::Local),
            location => self.remote_of(location).cloned().map(Side::Remote),
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(StatusLine {
            text: text.into(),
            error,
        });
    }

    pub fn focus_pane(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active = ix;
        window.focus(&self.panes[ix].focus, cx);
        cx.notify();
    }

    // --- navigation -----------------------------------------------------------

    /// Go somewhere, remembering where the pane was for Back.
    pub fn navigate(
        &mut self,
        ix: usize,
        location: Location,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let pane = &mut self.panes[ix];
        if pane.location == location && pane.path == path {
            self.load(ix, cx);
            return;
        }
        pane.back.push((pane.location.clone(), pane.path.clone()));
        pane.forward.clear();
        self.go(ix, location, path, cx);
    }

    fn go(&mut self, ix: usize, location: Location, path: String, cx: &mut Context<Self>) {
        let pane = &mut self.panes[ix];
        if pane.location != location || pane.path != path {
            pane.selected.clear();
            pane.cursor = None;
        }
        pane.location = location;
        pane.path = path;
        self.load(ix, cx);
    }

    /// List the pane's folder again, off the UI thread.
    pub fn load(&mut self, ix: usize, cx: &mut Context<Self>) {
        let side = self.side_of(ix);
        let pane = &mut self.panes[ix];
        pane.generation += 1;
        let generation = pane.generation;
        let path = pane.path.clone();
        pane.loading = true;
        cx.notify();
        let Some(side) = side else {
            pane.loading = false;
            pane.error = Some("Not connected.".into());
            pane.entries.clear();
            pane.rows.clear();
            return;
        };
        let task = runtime().spawn(async move {
            match side {
                Side::Local => tokio::task::spawn_blocking(move || fs::list_local(&path))
                    .await
                    .map_err(|error| error.to_string())?,
                Side::Remote(remote) => remote.list(&path).await,
            }
        });
        cx.spawn(async move |this, cx| {
            let listed = task.await.unwrap_or_else(|error| Err(error.to_string()));
            this.update(cx, |this, cx| {
                let show_hidden = this.stored.settings.show_hidden;
                let pane = &mut this.panes[ix];
                if pane.generation != generation {
                    return;
                }
                pane.loading = false;
                match listed {
                    Ok(entries) => {
                        pane.error = None;
                        pane.entries = entries;
                    }
                    Err(error) => {
                        pane.error = Some(error);
                        pane.entries.clear();
                    }
                }
                pane.rebuild(show_hidden, cx);
                if let Some(cursor) = pane.cursor {
                    pane.scroll.scroll_to_item(cursor, ScrollStrategy::Nearest);
                }
                cx.notify();
            })
        })
        .detach();
    }

    pub fn refresh(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.load(ix, cx);
    }

    pub fn go_up(&mut self, ix: usize, cx: &mut Context<Self>) {
        let pane = &self.panes[ix];
        if let Some(parent) = pane.parent() {
            let left = fs::file_name(&pane.path);
            let location = pane.location.clone();
            self.navigate(ix, location, parent, cx);
            self.panes[ix].reveal = Some(left);
        }
    }

    pub fn go_back(&mut self, ix: usize, cx: &mut Context<Self>) {
        let pane = &mut self.panes[ix];
        if let Some((location, path)) = pane.back.pop() {
            pane.forward
                .push((pane.location.clone(), pane.path.clone()));
            let location = self.usable(location);
            self.go(ix, location.0, location.1.unwrap_or(path), cx);
        }
    }

    pub fn go_forward(&mut self, ix: usize, cx: &mut Context<Self>) {
        let pane = &mut self.panes[ix];
        if let Some((location, path)) = pane.forward.pop() {
            pane.back.push((pane.location.clone(), pane.path.clone()));
            let location = self.usable(location);
            self.go(ix, location.0, location.1.unwrap_or(path), cx);
        }
    }

    /// History may name a site that has since disconnected: that step lands
    /// on the local home instead of on an error.
    fn usable(&self, location: Location) -> (Location, Option<String>) {
        if location == Location::Local || self.remote_of(&location).is_some() {
            (location, None)
        } else {
            (
                Location::Local,
                Some(fs::home().to_string_lossy().into_owned()),
            )
        }
    }

    pub fn go_home(&mut self, ix: usize, cx: &mut Context<Self>) {
        let location = self.panes[ix].location.clone();
        let home = match self.remote_of(&location) {
            Some(remote) => remote.home.clone(),
            None => fs::home().to_string_lossy().into_owned(),
        };
        self.navigate(ix, location, home, cx);
    }

    /// A path typed into the bar: resolved by the server on a remote pane,
    /// `~` expanded on a local one.
    fn go_to_typed(&mut self, ix: usize, typed: String, cx: &mut Context<Self>) {
        let location = self.panes[ix].location.clone();
        match self.remote_of(&location).cloned() {
            Some(remote) => {
                let task = runtime().spawn(async move { remote.resolve(&typed).await });
                cx.spawn(async move |this, cx| {
                    let resolved = task.await.unwrap_or_else(|error| Err(error.to_string()));
                    this.update(cx, |this, cx| match resolved {
                        Ok(path) => this.navigate(ix, location, path, cx),
                        Err(error) => {
                            this.set_status(error, true);
                            cx.notify();
                        }
                    })
                })
                .detach();
            }
            None => {
                let path = match typed.strip_prefix('~') {
                    Some(rest) => format!("{}{rest}", fs::home().to_string_lossy()),
                    None => typed,
                };
                if std::path::Path::new(&path).is_dir() {
                    self.navigate(ix, Location::Local, path, cx);
                } else {
                    self.set_status(format!("{path} is not a folder."), true);
                    cx.notify();
                }
            }
        }
    }

    pub fn open_row(&mut self, ix: usize, row: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.panes[ix].entry_at(row).cloned() else {
            return;
        };
        if entry.kind.is_dir() {
            let path = self.panes[ix].join(&entry.name);
            let location = self.panes[ix].location.clone();
            self.navigate(ix, location, path, cx);
        } else if !self.panes[ix].is_remote() {
            // A local file opens in whatever the system opens it with.
            let path = self.panes[ix].join(&entry.name);
            open_with_system(&path);
        } else {
            // A remote file: downloaded to the other pane if it is local.
            self.panes[ix].select_only(row);
            self.copy_to_other(cx);
        }
    }

    pub fn sort_by(&mut self, ix: usize, key: SortKey, cx: &mut Context<Self>) {
        let pane = &mut self.panes[ix];
        if pane.sort == key {
            pane.ascending = !pane.ascending;
        } else {
            pane.sort = key;
            pane.ascending = true;
        }
        let show_hidden = self.stored.settings.show_hidden;
        self.panes[ix].rebuild(show_hidden, cx);
        cx.notify();
    }

    pub fn move_cursor(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        let pane = &mut self.panes[self.active];
        if pane.rows.is_empty() {
            return;
        }
        let last = pane.rows.len() as isize - 1;
        let next = match pane.cursor {
            Some(cursor) => (cursor as isize + delta).clamp(0, last),
            None => 0,
        } as usize;
        if extend {
            pane.select_range(next);
        } else {
            pane.select_only(next);
        }
        pane.scroll.scroll_to_item(next, ScrollStrategy::Nearest);
        cx.notify();
    }

    pub fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        self.stored.settings.show_hidden = !self.stored.settings.show_hidden;
        let show_hidden = self.stored.settings.show_hidden;
        for ix in 0..2 {
            self.panes[ix].rebuild(show_hidden, cx);
        }
        self.persist();
        cx.notify();
    }

    pub fn cycle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let all = Theme::all();
        let current = theme::theme(cx).name;
        let index = all.iter().position(|t| t.name == current).unwrap_or(0);
        let next = all[(index + 1) % all.len()];
        theme::install_theme(next, window, cx);
        self.stored.settings.theme = Some(next.name.to_string());
        self.persist();
        self.set_status(format!("Theme: {}", next.name), false);
        cx.notify();
    }

    pub fn persist(&mut self) {
        if let Err(error) = store::save(&self.stored) {
            self.set_status(format!("Could not save settings: {error}"), true);
        }
    }

    // --- sites ----------------------------------------------------------------

    /// Open a site in a pane, connecting first if it is not connected.
    pub fn open_site(&mut self, id: String, ix: usize, cx: &mut Context<Self>) {
        let Some(site) = self.site(&id).cloned() else {
            return;
        };
        match self.conns.get(&id) {
            Some(Conn::Connected(remote)) if !remote.is_closed() => {
                let path = site
                    .remote_path
                    .clone()
                    .unwrap_or_else(|| remote.home.clone());
                self.navigate(ix, Location::Remote(id), path, cx);
                return;
            }
            Some(Conn::Connecting) => return,
            _ => {}
        }
        self.conns.insert(id.clone(), Conn::Connecting);
        self.set_status(format!("Connecting to {}…", site.address()), false);
        cx.notify();
        let asker = self.asker.clone();
        let task = runtime().spawn(async move {
            let site_id = site.id.clone();
            let credentials = tokio::task::spawn_blocking(move || Credentials {
                password: store::secret(&site_id, Secret::Password).ok().flatten(),
                passphrase: store::secret(&site_id, Secret::Passphrase).ok().flatten(),
            })
            .await
            .unwrap_or_default();
            Remote::connect(site, credentials, asker).await
        });
        cx.spawn(async move |this, cx| {
            let connected = task.await.unwrap_or_else(|error| Err(error.to_string()));
            this.update(cx, |this, cx| {
                let Some(site) = this.site(&id).cloned() else {
                    return;
                };
                match connected {
                    Ok(remote) => {
                        let path = site
                            .remote_path
                            .clone()
                            .unwrap_or_else(|| remote.home.clone());
                        this.conns.insert(id.clone(), Conn::Connected(remote));
                        this.set_status(format!("Connected to {}", site.address()), false);
                        this.navigate(ix, Location::Remote(id), path, cx);
                        if let Some(local) = site.local_path.filter(|p| !p.trim().is_empty()) {
                            this.navigate(1 - ix, Location::Local, local, cx);
                        }
                    }
                    Err(error) => {
                        this.conns.insert(id, Conn::Failed);
                        this.set_status(error, true);
                    }
                }
                cx.notify();
            })
        })
        .detach();
    }

    pub fn disconnect(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(Conn::Connected(remote)) = self.conns.remove(id) {
            runtime().spawn(async move { remote.disconnect().await });
        }
        let home = fs::home().to_string_lossy().into_owned();
        for ix in 0..2 {
            if self.panes[ix].location == Location::Remote(id.to_string()) {
                self.navigate(ix, Location::Local, home.clone(), cx);
            }
        }
        if let Some(site) = self.site(id) {
            let name = site.name.clone();
            self.set_status(format!("Disconnected from {name}"), false);
        }
        cx.notify();
    }

    pub fn save_site(
        &mut self,
        site: Site,
        password: String,
        passphrase: String,
    ) -> Result<(), String> {
        store::set_secret(&site.id, Secret::Password, &password)?;
        store::set_secret(&site.id, Secret::Passphrase, &passphrase)?;
        match self.stored.sites.iter_mut().find(|s| s.id == site.id) {
            Some(existing) => *existing = site,
            None => self.stored.sites.push(site),
        }
        self.stored
            .sites
            .sort_by(|a, b| fs::natural(&a.name, &b.name));
        store::save(&self.stored)
    }

    pub fn remove_site(&mut self, id: &str, cx: &mut Context<Self>) {
        self.disconnect(id, cx);
        self.stored.sites.retain(|site| site.id != id);
        store::forget_secrets(id);
        self.persist();
        self.modal = None;
        cx.notify();
    }

    // --- file operations ------------------------------------------------------

    pub fn copy_to_other(&mut self, cx: &mut Context<Self>) {
        let from = self.active;
        let items = self.panes[from].targets();
        self.copy_items(from, items, cx);
    }

    /// Copy into the other pane's folder, asking first if names clash there.
    pub fn copy_items(&mut self, from: usize, items: Vec<Entry>, cx: &mut Context<Self>) {
        if items.is_empty() {
            return;
        }
        let to = 1 - from;
        if self.panes[from].location == self.panes[to].location
            && self.panes[from].path == self.panes[to].path
        {
            self.set_status("Both panes show the same folder.", true);
            cx.notify();
            return;
        }
        let clashes: Vec<String> = items
            .iter()
            .filter(|item| self.panes[to].entries.iter().any(|e| e.name == item.name))
            .map(|item| item.name.clone())
            .collect();
        if clashes.is_empty() {
            self.start_transfer(from, items, cx);
        } else {
            self.modal = Some(Modal::Replace {
                from,
                items,
                clashes,
            });
            cx.notify();
        }
    }

    pub fn start_transfer(&mut self, from: usize, items: Vec<Entry>, cx: &mut Context<Self>) {
        let to = 1 - from;
        let (Some(from_side), Some(to_side)) = (self.side_of(from), self.side_of(to)) else {
            self.set_status("Not connected.", true);
            cx.notify();
            return;
        };
        let direction = match (from_side.is_remote(), to_side.is_remote()) {
            (false, true) => Direction::Upload,
            (true, false) => Direction::Download,
            _ => Direction::Copy,
        };
        let to_dir = self.panes[to].path.clone();
        let to_location = self.panes[to].location.clone();
        let label = match items.as_slice() {
            [one] => format!("{} → {}", one.name, fs::file_name(&to_dir)),
            many => format!("{} items → {}", many.len(), fs::file_name(&to_dir)),
        };
        let job = Job {
            from: from_side,
            from_dir: self.panes[from].path.clone(),
            items: items
                .iter()
                .map(|item| (item.name.clone(), item.kind))
                .collect(),
            to: to_side,
            to_dir: to_dir.clone(),
        };
        let progress = Arc::new(Progress::default());
        let id = self.next_transfer;
        self.next_transfer += 1;
        self.transfers.push(TransferRow {
            id,
            label,
            direction,
            progress: progress.clone(),
            state: TransferState::Running,
        });
        self.transfers_visible = true;
        let task = runtime().spawn(transfer::run(job, progress));
        cx.spawn(async move |this, cx| {
            let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
            this.update(cx, |this, cx| {
                if let Some(row) = this.transfers.iter_mut().find(|row| row.id == id) {
                    row.state = match result {
                        Ok(()) => TransferState::Done,
                        Err(error) => TransferState::Failed(error),
                    };
                }
                // Whichever pane now shows the destination lists it again.
                for ix in 0..2 {
                    if this.panes[ix].location == to_location && this.panes[ix].path == to_dir {
                        this.load(ix, cx);
                    }
                }
                cx.notify();
            })
        })
        .detach();
        self.poll_transfers(cx);
        cx.notify();
    }

    /// Repaint while anything is moving; stops itself when nothing is.
    fn poll_transfers(&mut self, cx: &mut Context<Self>) {
        if self.polling {
            return;
        }
        self.polling = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                let running = this
                    .update(cx, |this, cx| {
                        cx.notify();
                        let running = this
                            .transfers
                            .iter()
                            .any(|row| matches!(row.state, TransferState::Running));
                        if !running {
                            this.polling = false;
                        }
                        running
                    })
                    .unwrap_or(false);
                if !running {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn cancel_transfer(&mut self, id: u64) {
        if let Some(row) = self.transfers.iter().find(|row| row.id == id) {
            row.progress.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn clear_finished(&mut self, cx: &mut Context<Self>) {
        self.transfers
            .retain(|row| matches!(row.state, TransferState::Running));
        cx.notify();
    }

    pub fn ask_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Folder name")
                .default_value("untitled folder")
        });
        self.watch_enter(&input, window, cx);
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.modal = Some(Modal::NewFolder {
            pane: self.active,
            input,
        });
        cx.notify();
    }

    pub fn ask_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ix = self.active;
        let Some(entry) = self.panes[ix].targets().into_iter().next() else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(entry.name.clone()));
        self.watch_enter(&input, window, cx);
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.modal = Some(Modal::Rename {
            pane: ix,
            from: entry.name,
            input,
        });
        cx.notify();
    }

    pub fn ask_delete(&mut self, cx: &mut Context<Self>) {
        let ix = self.active;
        let items = self.panes[ix].targets();
        if items.is_empty() {
            return;
        }
        self.modal = Some(Modal::Delete { pane: ix, items });
        cx.notify();
    }

    /// Enter in a dialog's field confirms the dialog.
    fn watch_enter(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.confirm_modal(window, cx);
            }
        })
        .detach();
    }

    pub fn dismiss_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(question) = self.host_questions.pop_front() {
            let _ = question.reply.send(false);
        } else {
            self.modal = None;
        }
        let active = self.active;
        self.focus_pane(active, window, cx);
    }

    pub fn answer_host_key(&mut self, trust: bool, cx: &mut Context<Self>) {
        if let Some(question) = self.host_questions.pop_front() {
            let _ = question.reply.send(trust);
        }
        cx.notify();
    }

    pub fn confirm_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.host_questions.is_empty() {
            self.answer_host_key(true, cx);
            return;
        }
        let Some(modal) = self.modal.take() else {
            return;
        };
        match modal {
            Modal::Site(form) => {
                if let Err(form) = self.submit_form(form, window, cx) {
                    self.modal = Some(Modal::Site(form));
                }
            }
            Modal::NewFolder { pane, input } => {
                let name = input.read(cx).value().trim().to_string();
                if valid_name(&name) {
                    self.run_op(pane, Op::Mkdir(name), cx);
                } else {
                    self.modal = Some(Modal::NewFolder { pane, input });
                }
            }
            Modal::Rename { pane, from, input } => {
                let to = input.read(cx).value().trim().to_string();
                if to == from {
                } else if valid_name(&to) {
                    self.run_op(pane, Op::Rename(from, to), cx);
                } else {
                    self.modal = Some(Modal::Rename { pane, from, input });
                }
            }
            Modal::Delete { pane, items } => self.run_op(
                pane,
                Op::Delete(items.into_iter().map(|e| e.name).collect()),
                cx,
            ),
            Modal::Replace { from, items, .. } => self.start_transfer(from, items, cx),
        }
        if self.modal.is_none() {
            let active = self.active;
            self.focus_pane(active, window, cx);
        }
        cx.notify();
    }

    fn run_op(&mut self, ix: usize, op: Op, cx: &mut Context<Self>) {
        let Some(side) = self.side_of(ix) else {
            return;
        };
        let dir = self.panes[ix].path.clone();
        let reveal = match &op {
            Op::Mkdir(name) | Op::Rename(_, name) => Some(name.clone()),
            Op::Delete(_) => None,
        };
        let task = runtime().spawn(async move {
            match (&side, op) {
                (Side::Local, Op::Mkdir(name)) => fs::local_mkdir(&fs::local_join(&dir, &name)),
                (Side::Local, Op::Rename(from, to)) => {
                    fs::local_rename(&fs::local_join(&dir, &from), &fs::local_join(&dir, &to))
                }
                (Side::Local, Op::Delete(names)) => {
                    for name in names {
                        fs::local_delete(&fs::local_join(&dir, &name))?;
                    }
                    Ok(())
                }
                (Side::Remote(remote), Op::Mkdir(name)) => {
                    remote.mkdir(&fs::remote_join(&dir, &name)).await
                }
                (Side::Remote(remote), Op::Rename(from, to)) => {
                    remote
                        .rename(&fs::remote_join(&dir, &from), &fs::remote_join(&dir, &to))
                        .await
                }
                (Side::Remote(remote), Op::Delete(names)) => {
                    for name in names {
                        remote.delete(&fs::remote_join(&dir, &name)).await?;
                    }
                    Ok(())
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
            this.update(cx, |this, cx| {
                if let Err(error) = result {
                    this.set_status(error, true);
                }
                for other in 0..2 {
                    if this.panes[other].location == this.panes[ix].location
                        && this.panes[other].path == this.panes[ix].path
                    {
                        if other == ix {
                            this.panes[other].reveal = reveal.clone();
                            if let Some(name) = &reveal {
                                this.panes[other].selected.clear();
                                this.panes[other].selected.insert(name.clone());
                            }
                        }
                        this.load(other, cx);
                    }
                }
                cx.notify();
            })
        })
        .detach();
    }

    pub fn drop_files(&mut self, dragged: &DraggedFiles, onto: usize, cx: &mut Context<Self>) {
        if dragged.pane == onto {
            return;
        }
        self.copy_items(dragged.pane, dragged.items.clone(), cx);
    }
}

enum Op {
    Mkdir(String),
    Rename(String, String),
    Delete(Vec<String>),
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/')
}

fn open_with_system(path: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

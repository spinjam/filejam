//! One side of the window: where it is, what it lists, what is selected.

use std::collections::HashSet;

use gpui::{App, AppContext, Entity, FocusHandle, UniformListScrollHandle, Window};
use gpui_component::{input::InputState, resizable::ResizableState};

use crate::fs::{self, Entry, SortKey};

use super::tree::FolderTree;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    Local,
    /// A saved site, by id.
    Remote(String),
}

/// A tab while another is in front of its pane: everything [`Pane::trade`]
/// swaps. Filter text is kept as text, since the pane has the one input.
pub struct Tab {
    pub location: Location,
    pub path: String,
    pub entries: Vec<Entry>,
    pub rows: Vec<usize>,
    pub selected: HashSet<String>,
    pub cursor: Option<usize>,
    pub anchor: Option<usize>,
    pub sort: SortKey,
    pub ascending: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub back: Vec<(Location, String)>,
    pub forward: Vec<(Location, String)>,
    pub reveal: Option<String>,
    pub tree: FolderTree,
    pub filter: String,
}

impl Tab {
    /// Nothing listed yet. An empty `path` on a site means wherever the site
    /// starts, decided once it has connected.
    pub fn new(location: Location, path: String) -> Self {
        let mut tree = FolderTree::new(false);
        tree.reset(matches!(location, Location::Remote(_)));
        Self {
            location,
            path,
            entries: Vec::new(),
            rows: Vec::new(),
            selected: HashSet::new(),
            cursor: None,
            anchor: None,
            sort: SortKey::Name,
            ascending: true,
            loading: false,
            error: None,
            back: Vec::new(),
            forward: Vec::new(),
            reveal: None,
            tree,
            filter: String::new(),
        }
    }
}

pub struct Pane {
    /// Every tab in strip order. The one in front is `None`: its state is
    /// the fields below.
    pub tabs: Vec<Option<Tab>>,
    pub location: Location,
    pub path: String,
    pub entries: Vec<Entry>,
    /// Indices into `entries`, in the order shown: hidden files and the
    /// filter taken out, sorted.
    pub rows: Vec<usize>,
    /// By name, so a refresh keeps what was picked.
    pub selected: HashSet<String>,
    /// The row the keyboard is on.
    pub cursor: Option<usize>,
    /// Where a shift-click or shift-arrow range starts.
    pub anchor: Option<usize>,
    pub sort: SortKey,
    pub ascending: bool,
    pub filter: Entity<InputState>,
    pub path_input: Entity<InputState>,
    pub editing_path: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub back: Vec<(Location, String)>,
    pub forward: Vec<(Location, String)>,
    pub scroll: UniformListScrollHandle,
    pub focus: FocusHandle,
    /// Bumped by every navigation, so a listing that lands after the user
    /// has moved on is dropped rather than shown.
    pub generation: u64,
    /// The name to put the cursor on once the next listing lands: the folder
    /// just left by going up, or the one just made.
    pub reveal: Option<String>,
    /// The folder tree beside the listing, when it is switched on.
    pub tree: FolderTree,
    pub tree_split: Entity<ResizableState>,
}

impl Pane {
    pub fn new(path: String, tree: bool, window: &mut Window, cx: &mut App) -> Self {
        Self {
            tabs: vec![None],
            location: Location::Local,
            path,
            entries: Vec::new(),
            rows: Vec::new(),
            selected: HashSet::new(),
            cursor: None,
            anchor: None,
            sort: SortKey::Name,
            ascending: true,
            filter: cx.new(|cx| InputState::new(window, cx).placeholder("Filter")),
            path_input: cx.new(|cx| InputState::new(window, cx).placeholder("Go to folder…")),
            editing_path: false,
            loading: false,
            error: None,
            back: Vec::new(),
            forward: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            focus: cx.focus_handle(),
            generation: 0,
            reveal: None,
            tree: FolderTree::new(tree),
            tree_split: cx.new(|_| ResizableState::default()),
        }
    }

    /// Trade everything shown with the other pane: where it is, what it
    /// lists, what is picked, its history and its tree. The inputs, focus
    /// and scroll stay with the side, and so does whether the tree is on.
    pub fn swap_contents(&mut self, other: &mut Pane) {
        use std::mem::swap;
        swap(&mut self.tabs, &mut other.tabs);
        swap(&mut self.location, &mut other.location);
        swap(&mut self.path, &mut other.path);
        swap(&mut self.entries, &mut other.entries);
        swap(&mut self.rows, &mut other.rows);
        swap(&mut self.selected, &mut other.selected);
        swap(&mut self.cursor, &mut other.cursor);
        swap(&mut self.anchor, &mut other.anchor);
        swap(&mut self.sort, &mut other.sort);
        swap(&mut self.ascending, &mut other.ascending);
        swap(&mut self.loading, &mut other.loading);
        swap(&mut self.error, &mut other.error);
        swap(&mut self.back, &mut other.back);
        swap(&mut self.forward, &mut other.forward);
        swap(&mut self.reveal, &mut other.reveal);
        swap(&mut self.tree, &mut other.tree);
        swap(&mut self.tree.visible, &mut other.tree.visible);
        self.editing_path = false;
        other.editing_path = false;
    }

    /// Trade the tab in front for `tab`. The inputs, focus and scroll stay
    /// with the side, and so does whether the tree is on.
    pub fn trade(&mut self, tab: &mut Tab) {
        use std::mem::swap;
        swap(&mut self.location, &mut tab.location);
        swap(&mut self.path, &mut tab.path);
        swap(&mut self.entries, &mut tab.entries);
        swap(&mut self.rows, &mut tab.rows);
        swap(&mut self.selected, &mut tab.selected);
        swap(&mut self.cursor, &mut tab.cursor);
        swap(&mut self.anchor, &mut tab.anchor);
        swap(&mut self.sort, &mut tab.sort);
        swap(&mut self.ascending, &mut tab.ascending);
        swap(&mut self.loading, &mut tab.loading);
        swap(&mut self.error, &mut tab.error);
        swap(&mut self.back, &mut tab.back);
        swap(&mut self.forward, &mut tab.forward);
        swap(&mut self.reveal, &mut tab.reveal);
        swap(&mut self.tree, &mut tab.tree);
        swap(&mut self.tree.visible, &mut tab.tree.visible);
    }

    /// The index of the tab in front.
    pub fn front(&self) -> usize {
        self.tabs.iter().position(Option::is_none).unwrap_or(0)
    }

    pub fn tab_location(&self, tab: usize) -> &Location {
        match self.tabs.get(tab).and_then(Option::as_ref) {
            Some(behind) => &behind.location,
            None => &self.location,
        }
    }

    pub fn tab_path(&self, tab: usize) -> &str {
        match self.tabs.get(tab).and_then(Option::as_ref) {
            Some(behind) => &behind.path,
            None => &self.path,
        }
    }

    /// The tab on `location`, the one in front if that is one of them.
    pub fn tab_of(&self, location: &Location) -> Option<usize> {
        if self.location == *location {
            return Some(self.front());
        }
        (0..self.tabs.len()).find(|&tab| self.tab_location(tab) == location)
    }

    pub fn is_remote(&self) -> bool {
        matches!(self.location, Location::Remote(_))
    }

    pub fn join(&self, name: &str) -> String {
        if self.is_remote() {
            fs::remote_join(&self.path, name)
        } else {
            fs::local_join(&self.path, name)
        }
    }

    pub fn parent(&self) -> Option<String> {
        if self.is_remote() {
            fs::remote_parent(&self.path)
        } else {
            fs::local_parent(&self.path)
        }
    }

    pub fn entry_at(&self, row: usize) -> Option<&Entry> {
        self.rows
            .get(row)
            .and_then(|&index| self.entries.get(index))
    }

    /// Rebuild the shown rows from the entries, keeping the selection and the
    /// cursor on the same names where they still exist.
    pub fn rebuild(&mut self, show_hidden: bool, cx: &App) {
        let cursor_name = self
            .cursor
            .and_then(|row| self.entry_at(row))
            .map(|entry| entry.name.clone());
        let needle = self.filter.read(cx).value().to_lowercase();
        let mut rows: Vec<usize> = (0..self.entries.len())
            .filter(|&index| {
                let entry = &self.entries[index];
                (show_hidden || !entry.is_hidden())
                    && (needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            })
            .collect();
        let (sort, ascending) = (self.sort, self.ascending);
        rows.sort_by(|&a, &b| fs::compare(&self.entries[a], &self.entries[b], sort, ascending));
        self.rows = rows;

        let shown: HashSet<&str> = self
            .rows
            .iter()
            .map(|&index| self.entries[index].name.as_str())
            .collect();
        self.selected.retain(|name| shown.contains(name.as_str()));

        let wanted = self.reveal.take().or(cursor_name);
        self.cursor = wanted
            .and_then(|name| self.rows.iter().position(|&i| self.entries[i].name == name))
            .or_else(|| (!self.rows.is_empty()).then_some(0))
            .map(|row| row.min(self.rows.len().saturating_sub(1)));
        if self.rows.is_empty() {
            self.cursor = None;
        }
        self.anchor = self.cursor;
        self.tree.rebuild(&self.path, show_hidden);
    }

    /// What an action acts on: the selection, or the row under the cursor
    /// when nothing is selected.
    pub fn targets(&self) -> Vec<Entry> {
        let picked: Vec<Entry> = self
            .rows
            .iter()
            .map(|&index| &self.entries[index])
            .filter(|entry| self.selected.contains(&entry.name))
            .cloned()
            .collect();
        if !picked.is_empty() {
            return picked;
        }
        self.cursor
            .and_then(|row| self.entry_at(row))
            .cloned()
            .into_iter()
            .collect()
    }

    pub fn select_only(&mut self, row: usize) {
        self.selected.clear();
        if let Some(entry) = self.entry_at(row) {
            self.selected.insert(entry.name.clone());
        }
        self.cursor = Some(row);
        self.anchor = Some(row);
    }

    pub fn toggle(&mut self, row: usize) {
        if let Some(name) = self.entry_at(row).map(|entry| entry.name.clone())
            && !self.selected.remove(&name)
        {
            self.selected.insert(name);
        }
        self.cursor = Some(row);
        self.anchor = Some(row);
    }

    pub fn select_range(&mut self, row: usize) {
        let anchor = self.anchor.unwrap_or(row);
        let (start, end) = if anchor <= row {
            (anchor, row)
        } else {
            (row, anchor)
        };
        self.selected.clear();
        for r in start..=end {
            if let Some(entry) = self.entry_at(r) {
                self.selected.insert(entry.name.clone());
            }
        }
        self.cursor = Some(row);
    }

    pub fn select_all(&mut self) {
        self.selected = self
            .rows
            .iter()
            .map(|&index| self.entries[index].name.clone())
            .collect();
    }

    /// Selected bytes and count, for the status bar.
    pub fn selection_summary(&self) -> (usize, u64) {
        self.rows
            .iter()
            .map(|&index| &self.entries[index])
            .filter(|entry| self.selected.contains(&entry.name))
            .fold((0, 0), |(count, bytes), entry| {
                (count + 1, bytes + entry.size)
            })
    }
}

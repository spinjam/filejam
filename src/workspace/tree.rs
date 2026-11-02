//! A pane's folder tree: the folders from the root down, opened along the
//! path the pane shows. Children are listed only when a folder is opened,
//! so a tree over a slow link costs a listing per click, not per folder.

use std::collections::{HashMap, HashSet};

use gpui::{ScrollStrategy, UniformListScrollHandle};

use crate::fs::{self, Entry};

/// One line of the tree as drawn.
pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub expanded: bool,
    /// Listed and found to hold no folders: no disclosure chevron.
    pub leaf: bool,
    pub loading: bool,
    pub hidden: bool,
}

#[derive(Default)]
struct Node {
    /// Sub-folder names and whether each is hidden, once listed.
    children: Option<Vec<(String, bool)>>,
    loading: bool,
    error: bool,
}

pub struct FolderTree {
    pub visible: bool,
    remote: bool,
    nodes: HashMap<String, Node>,
    expanded: HashSet<String>,
    pub rows: Vec<TreeRow>,
    pub scroll: UniformListScrollHandle,
    /// Bumped when the pane changes side or site, so a listing for the old
    /// tree is dropped when it lands.
    pub generation: u64,
    /// Scroll the pane's folder into view once its row exists.
    follow: bool,
}

impl FolderTree {
    pub fn new(visible: bool) -> Self {
        Self {
            visible,
            remote: false,
            nodes: HashMap::new(),
            expanded: HashSet::new(),
            rows: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            generation: 0,
            follow: true,
        }
    }

    /// Forget everything: the pane now shows another computer.
    pub fn reset(&mut self, remote: bool) {
        self.remote = remote;
        self.nodes.clear();
        self.expanded.clear();
        self.rows.clear();
        self.generation += 1;
        self.follow = true;
    }

    pub fn join(&self, dir: &str, name: &str) -> String {
        if self.remote {
            fs::remote_join(dir, name)
        } else {
            fs::local_join(dir, name)
        }
    }

    /// Open every folder from the root down to `path`, and `path` itself.
    pub fn reveal(&mut self, path: &str) {
        for (_, ancestor) in fs::crumbs(path, self.remote) {
            self.expanded.insert(ancestor);
        }
        self.follow = true;
    }

    /// Open or close a folder. Returns whether it now needs listing.
    pub fn toggle(&mut self, path: &str) -> bool {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_string());
        }
        self.follow = false;
        self.needs_listing(path)
    }

    fn needs_listing(&self, path: &str) -> bool {
        self.expanded.contains(path)
            && self
                .nodes
                .get(path)
                .is_none_or(|node| node.children.is_none() && !node.loading && !node.error)
    }

    /// Open folders not yet listed, marked as loading: the caller lists them.
    pub fn take_pending(&mut self) -> Vec<String> {
        let pending: Vec<String> = self
            .expanded
            .iter()
            .filter(|path| self.needs_listing(path))
            .cloned()
            .collect();
        for path in &pending {
            self.nodes.entry(path.clone()).or_default().loading = true;
        }
        pending
    }

    /// A listing of `path` landed, from the tree or from the pane itself.
    pub fn set_children(&mut self, path: &str, entries: &[Entry]) {
        let mut folders: Vec<(String, bool)> = entries
            .iter()
            .filter(|entry| entry.kind.is_dir())
            .map(|entry| (entry.name.clone(), entry.is_hidden()))
            .collect();
        folders.sort_by(|a, b| fs::natural(&a.0, &b.0));
        let node = self.nodes.entry(path.to_string()).or_default();
        node.loading = false;
        node.children = Some(folders);
        node.error = false;
    }

    /// `path` could not be listed — a chrooted server's `/`, say. It still
    /// shows the one child that leads to the pane's folder.
    pub fn set_failed(&mut self, path: &str) {
        let node = self.nodes.entry(path.to_string()).or_default();
        node.loading = false;
        node.children = Some(Vec::new());
        node.error = true;
    }

    /// Flatten the open folders into the rows drawn, from the root of `current`.
    pub fn rebuild(&mut self, current: &str, show_hidden: bool) {
        self.rows.clear();
        let Some((label, root)) = fs::crumbs(current, self.remote).into_iter().next() else {
            return;
        };
        let trail: Vec<String> = fs::crumbs(current, self.remote)
            .into_iter()
            .map(|(_, path)| path)
            .collect();
        self.push(root, label, 0, false, show_hidden, &trail);
        if self.follow
            && let Some(row) = self.rows.iter().position(|row| row.path == current)
        {
            self.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
            self.follow = false;
        }
    }

    fn push(
        &mut self,
        path: String,
        name: String,
        depth: usize,
        hidden: bool,
        show_hidden: bool,
        trail: &[String],
    ) {
        let expanded = self.expanded.contains(&path);
        let node = self.nodes.get(&path);
        let loading = node.is_some_and(|node| node.loading);
        let mut children: Vec<(String, bool)> = node
            .and_then(|node| node.children.as_ref())
            .map(|children| {
                children
                    .iter()
                    .filter(|(_, hidden)| show_hidden || !hidden)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if node.is_some_and(|node| node.error)
            && let Some(at) = trail.iter().position(|step| *step == path)
            && let Some(next) = trail.get(at + 1)
        {
            let name = fs::file_name(next);
            let hidden = name.starts_with('.');
            children = vec![(name, hidden)];
        }
        let listed = node.is_some_and(|node| node.children.is_some());
        self.rows.push(TreeRow {
            path: path.clone(),
            name,
            depth,
            expanded,
            leaf: listed && children.is_empty(),
            loading,
            hidden,
        });
        if expanded {
            for (child, hidden) in children {
                let child_path = self.join(&path, &child);
                self.push(child_path, child, depth + 1, hidden, show_hidden, trail);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::Kind;

    fn dir(name: &str) -> Entry {
        Entry {
            name: name.into(),
            kind: Kind::Dir,
            size: 0,
            modified: None,
            mode: None,
        }
    }

    #[test]
    fn revealing_a_path_opens_and_lists_every_ancestor() {
        let mut tree = FolderTree::new(true);
        tree.reset(true);
        tree.reveal("/var/www");
        let mut pending = tree.take_pending();
        pending.sort();
        assert_eq!(pending, vec!["/", "/var", "/var/www"]);
        assert!(tree.take_pending().is_empty(), "already loading");

        tree.set_children("/", &[dir("var"), dir("etc"), dir(".hidden")]);
        tree.set_children("/var", &[dir("www"), dir("log")]);
        tree.set_children("/var/www", &[]);
        tree.rebuild("/var/www", false);
        let shown: Vec<(&str, usize)> = tree
            .rows
            .iter()
            .map(|row| (row.path.as_str(), row.depth))
            .collect();
        assert_eq!(
            shown,
            vec![
                ("/", 0),
                ("/etc", 1),
                ("/var", 1),
                ("/var/log", 2),
                ("/var/www", 2),
            ]
        );
        assert!(tree.rows[4].leaf);
        assert!(!tree.rows[3].leaf, "not listed yet, so it may hold folders");
    }

    #[test]
    fn closing_a_folder_hides_its_children_and_reopening_does_not_relist() {
        let mut tree = FolderTree::new(true);
        tree.reset(true);
        tree.reveal("/");
        tree.take_pending();
        tree.set_children("/", &[dir("srv")]);
        assert!(tree.toggle("/srv"), "never listed");
        tree.take_pending();
        tree.set_children("/srv", &[dir("data")]);
        tree.rebuild("/", false);
        assert_eq!(tree.rows.len(), 3);

        assert!(!tree.toggle("/srv"));
        tree.rebuild("/", false);
        assert_eq!(tree.rows.len(), 2);
        assert!(!tree.toggle("/srv"), "listed already");
    }

    #[test]
    fn an_unlistable_ancestor_still_leads_to_the_pane_folder() {
        let mut tree = FolderTree::new(true);
        tree.reset(true);
        tree.reveal("/home/me");
        tree.take_pending();
        tree.set_failed("/");
        tree.set_failed("/home");
        tree.set_children("/home/me", &[]);
        tree.rebuild("/home/me", false);
        let shown: Vec<&str> = tree.rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(shown, vec!["/", "/home", "/home/me"]);
    }
}

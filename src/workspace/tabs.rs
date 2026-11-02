//! The tabs of a pane. The pane's own fields are the tab in front, and a tab
//! behind waits as a [`Tab`], so nothing that reads a pane has to know tabs
//! exist. A tab behind lists nothing: it is listed again when it comes back.

use gpui::{Context, ScrollStrategy, Window};

use crate::fs;

use super::{Location, Modal, Workspace, pane::Tab};

/// Which tab comes to the front when the one at `closing` goes: the one after
/// it, or the one before if it was the last.
fn next_front(closing: usize, len: usize) -> usize {
    if closing + 1 < len {
        closing + 1
    } else {
        closing.saturating_sub(1)
    }
}

impl Workspace {
    fn busy(&self) -> bool {
        self.modal.is_some() || !self.host_questions.is_empty()
    }

    /// Bring a tab to the front of its pane and list its folder again.
    pub fn show_tab(&mut self, ix: usize, to: usize, window: &mut Window, cx: &mut Context<Self>) {
        let front = self.panes[ix].front();
        if to == front || to >= self.panes[ix].tabs.len() {
            return;
        }
        let leaving = self.panes[ix].filter.read(cx).value().to_string();
        let pane = &mut self.panes[ix];
        let Some(mut tab) = pane.tabs[to].take() else {
            return;
        };
        pane.trade(&mut tab);
        let filter = std::mem::replace(&mut tab.filter, leaving);
        // Listings in flight belong to the tab going behind. Past every
        // generation either tree has had, so none of them lands on this one.
        pane.generation += 1;
        let tree_generation = pane.tree.generation.max(tab.tree.generation) + 1;
        pane.tree.settle(tree_generation);
        pane.editing_path = false;
        pane.tabs[front] = Some(tab);
        pane.filter
            .update(cx, |input, cx| input.set_value(filter, window, cx));
        let show_hidden = self.stored.settings.show_hidden;
        let pane = &mut self.panes[ix];
        pane.rebuild(show_hidden, cx);
        if let Some(cursor) = pane.cursor {
            pane.scroll.scroll_to_item(cursor, ScrollStrategy::Nearest);
        }
        self.load(ix, cx);
    }

    /// A new tab in front of a pane, sorted the way the pane is.
    pub(super) fn open_tab(
        &mut self,
        ix: usize,
        location: Location,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = &mut self.panes[ix];
        let mut tab = Tab::new(location, path);
        tab.sort = pane.sort;
        tab.ascending = pane.ascending;
        pane.tabs.push(Some(tab));
        let last = pane.tabs.len() - 1;
        self.show_tab(ix, last, window, cx);
    }

    /// Cmd-T and the strip's +: the folder in front again.
    pub fn new_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let front = self.panes[ix].front();
        self.duplicate_tab(ix, front, window, cx);
    }

    /// A new tab on a tab's folder, or on home when that tab has no connection
    /// to share.
    pub fn duplicate_tab(
        &mut self,
        ix: usize,
        tab: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || tab >= self.panes[ix].tabs.len() {
            return;
        }
        let pane = &self.panes[ix];
        let (location, path) = (
            pane.tab_location(tab).clone(),
            pane.tab_path(tab).to_string(),
        );
        let (location, path) = if location == Location::Local || self.remote_of(&location).is_some()
        {
            (location, path)
        } else {
            (Location::Local, home())
        };
        self.open_tab(ix, location, path, window, cx);
        self.focus_pane(ix, window, cx);
    }

    /// Move to the tab `delta` along, wrapping at the ends.
    pub fn cycle_tab(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        let ix = self.active;
        let len = self.panes[ix].tabs.len() as isize;
        let to = (self.panes[ix].front() as isize + delta).rem_euclid(len) as usize;
        self.show_tab(ix, to, window, cx);
    }

    /// The × and Cmd-W. The last tab on a site takes the connection with it,
    /// so if that would stop a transfer, ask first.
    pub fn close_tab(&mut self, ix: usize, tab: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() || tab >= self.panes[ix].tabs.len() {
            return;
        }
        if let Location::Remote(id) = self.panes[ix].tab_location(tab)
            && self.tabs_on(id) == 1
            && self.has_running_transfer(id)
        {
            self.modal = Some(Modal::CloseHost {
                site: id.clone(),
                tab: Some((ix, tab)),
            });
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        self.close_tab_now(ix, tab, window, cx);
    }

    /// Close every tab of a pane but one. A tab whose closing would stop a
    /// transfer stays open: this is one click, too quick for a question per tab.
    pub fn close_other_tabs(
        &mut self,
        ix: usize,
        keep: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || keep >= self.panes[ix].tabs.len() {
            return;
        }
        self.show_tab(ix, keep, window, cx);
        let mut kept = 0;
        for tab in (0..self.panes[ix].tabs.len()).rev() {
            if tab == keep {
                continue;
            }
            if let Location::Remote(id) = self.panes[ix].tab_location(tab)
                && self.tabs_on(id) == 1
                && self.has_running_transfer(id)
            {
                kept += 1;
                continue;
            }
            self.close_tab_now(ix, tab, window, cx);
        }
        if kept > 0 {
            self.set_status("Tabs with transfers running stayed open.", false);
        }
        self.focus_pane(ix, window, cx);
    }

    pub(super) fn close_tab_now(
        &mut self,
        ix: usize,
        tab: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let location = self.panes[ix].tab_location(tab).clone();
        self.remove_tab(ix, tab, window, cx);
        if let Location::Remote(id) = location
            && self.tabs_on(&id) == 0
        {
            self.end_connection(&id);
        }
        cx.notify();
    }

    /// A pane is never without a tab: its last one gives way to the local home.
    pub(super) fn remove_tab(
        &mut self,
        ix: usize,
        tab: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.panes[ix].tabs.len() == 1 {
            self.panes[ix]
                .tabs
                .push(Some(Tab::new(Location::Local, home())));
        }
        if tab == self.panes[ix].front() {
            let to = next_front(tab, self.panes[ix].tabs.len());
            self.show_tab(ix, to, window, cx);
        }
        self.panes[ix].tabs.remove(tab);
    }

    /// How many tabs, in either pane, are on a site.
    fn tabs_on(&self, id: &str) -> usize {
        let site = Location::Remote(id.to_string());
        self.panes
            .iter()
            .map(|pane| {
                (0..pane.tabs.len())
                    .filter(|&tab| *pane.tab_location(tab) == site)
                    .count()
            })
            .sum()
    }
}

fn home() -> String {
    fs::home().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::next_front;

    #[test]
    fn closing_a_tab_brings_up_the_one_after_it() {
        assert_eq!(next_front(0, 3), 1);
        assert_eq!(next_front(1, 3), 2);
    }

    #[test]
    fn closing_the_last_tab_brings_up_the_one_before_it() {
        assert_eq!(next_front(2, 3), 1);
    }
}

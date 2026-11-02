//! How the window looks. dbdelve's composition: a titlebar that is also the
//! toolbar, a sidebar of places on the chrome plane, the content planes
//! brighter beside it, a status bar along the bottom.

use std::{ops::Range, sync::atomic::Ordering};

use gpui::{
    AnyElement, AppContext, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder, px, uniform_list,
};
use gpui_component::{
    Disableable, Selectable,
    menu::{ContextMenuExt, PopupMenuItem},
    input::Input,
    resizable::{h_resizable, resizable_panel},
};

use crate::{
    actions::*,
    fs::{self, Entry, Kind, SortKey},
    icons::{icon, icon as icons},
    theme::{self, SiteColor, Theme, layout},
    ui::{self, Control, Status, Tone},
};

use super::{Conn, Direction, DraggedFiles, Location, Modal, Pane, TransferState, Workspace};

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *theme::theme(cx);
        if self.wants_focus {
            self.wants_focus = false;
            let active = self.active;
            window.focus(&self.panes[active].focus, cx);
        }

        let body = if self.sidebar_visible {
            h_resizable("shell-split")
                .with_state(&self.shell_split)
                .child(
                    resizable_panel()
                        .size(px(layout::SIDEBAR_DEFAULT_WIDTH))
                        .size_range(px(layout::SIDEBAR_MIN_WIDTH)..px(layout::SIDEBAR_MAX_WIDTH))
                        .child(self.render_sidebar(t, cx)),
                )
                .child(resizable_panel().child(self.render_content(t, window, cx)))
                .into_any_element()
        } else {
            self.render_content(t, window, cx)
        };

        div()
            .id("workspace")
            .key_context("Workspace")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .font_family(theme::CHROME_FONT)
            .text_size(px(layout::TEXT_MD))
            .text_color(t.text)
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .on_action(
                cx.listener(|this, _: &NewSite, window, cx| this.open_form(None, window, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.sidebar_visible = !this.sidebar_visible;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleTransfers, _, cx| {
                this.transfers_visible = !this.transfers_visible;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleTree, _, cx| this.toggle_tree(this.active, cx)))
            .on_action(cx.listener(|this, _: &AddToFavorites, _, cx| this.favorite_from_pane(cx)))
            .on_action(cx.listener(|this, _: &ImportForklift, _, cx| this.import_forklift(cx)))
            .on_action(cx.listener(|this, _: &ToggleHidden, _, cx| this.toggle_hidden(cx)))
            .on_action(cx.listener(|this, _: &CycleTheme, window, cx| this.cycle_theme(window, cx)))
            .on_action(
                cx.listener(|this, _: &OpenSettings, window, cx| this.open_settings(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(this.active, cx)))
            .on_action(cx.listener(|this, _: &GoUp, _, cx| this.go_up(this.active, cx)))
            .on_action(cx.listener(|this, _: &GoBack, _, cx| this.go_back(this.active, cx)))
            .on_action(cx.listener(|this, _: &GoForward, _, cx| this.go_forward(this.active, cx)))
            .on_action(cx.listener(|this, _: &GoHome, _, cx| this.go_home(this.active, cx)))
            .on_action(cx.listener(|this, _: &GoToPath, window, cx| {
                let ix = this.active;
                this.edit_path(ix, window, cx);
            }))
            .on_action(cx.listener(|this, _: &Open, _, cx| {
                let ix = this.active;
                if let Some(cursor) = this.panes[ix].cursor {
                    this.open_row(ix, cursor, cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &SelectPrevious, _, cx| this.move_cursor(-1, false, cx)),
            )
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.move_cursor(1, false, cx)))
            .on_action(
                cx.listener(|this, _: &ExtendPrevious, _, cx| this.move_cursor(-1, true, cx)),
            )
            .on_action(cx.listener(|this, _: &ExtendNext, _, cx| this.move_cursor(1, true, cx)))
            .on_action(cx.listener(|this, _: &SelectFirst, _, cx| {
                this.move_cursor(isize::MIN / 2, false, cx)
            }))
            .on_action(cx.listener(|this, _: &SelectLast, _, cx| {
                this.move_cursor(isize::MAX / 2, false, cx)
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                let ix = this.active;
                this.panes[ix].select_all();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SwitchPane, window, cx| {
                let other = 1 - this.active;
                this.focus_pane(other, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CopyToOther, _, cx| this.copy_to_other(cx)))
            .on_action(cx.listener(|this, _: &SwapPanes, window, cx| this.swap_panes(window, cx)))
            .on_action(
                cx.listener(|this, _: &NewFolder, window, cx| this.ask_new_folder(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Rename, window, cx| this.ask_rename(window, cx)))
            .on_action(cx.listener(|this, _: &Delete, _, cx| this.ask_delete(cx)))
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.dismiss_modal(window, cx)))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| this.confirm_modal(window, cx)))
            .on_action(cx.listener(|this, _: &Disconnect, _, cx| {
                if let Location::Remote(id) = this.panes[this.active].location.clone() {
                    this.disconnect(&id, cx);
                }
            }))
            .child(self.render_titlebar(t, cx))
            .child(div().flex_1().min_h_0().child(body))
            .child(self.render_status(t))
            .when_some(self.render_modal(t, window, cx), |root, modal| {
                root.child(modal)
            })
    }
}

impl Workspace {
    fn edit_path(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.panes[ix].path.clone();
        self.active = ix;
        self.panes[ix].editing_path = true;
        self.panes[ix].path_input.update(cx, |input, cx| {
            input.set_value(path, window, cx);
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        cx.notify();
    }

    fn site_color(&self, location: &Location) -> Option<SiteColor> {
        match location {
            Location::Local => None,
            Location::Remote(id) => self
                .site(id)
                .and_then(|site| site.color.as_deref())
                .and_then(SiteColor::from_slug),
        }
    }

    fn location_label(&self, location: &Location) -> String {
        match location {
            Location::Local => "This Computer".into(),
            Location::Remote(id) => self
                .site(id)
                .map(|site| site.name.clone())
                .unwrap_or_else(|| id.clone()),
        }
    }

    // --- titlebar -------------------------------------------------------------

    fn render_titlebar(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let ix = self.active;
        let pane = &self.panes[ix];
        let other = &self.panes[1 - ix];
        let has_targets = !pane.targets().is_empty();
        let (copy_icon, copy_label) = match (pane.is_remote(), other.is_remote()) {
            (false, true) => (icons::UPLOAD, "Upload"),
            (true, false) => (icons::DOWNLOAD, "Download"),
            _ => (icons::COPY, "Copy"),
        };

        let leading = vec![
            ui::icon_button(
                "toggle-sidebar",
                icons::SIDEBAR,
                Tone::Quiet,
                Control::Compact,
                t,
            )
            .tooltip("Toggle Sidebar")
            .on_click(cx.listener(|this, _, _, cx| {
                this.sidebar_visible = !this.sidebar_visible;
                cx.notify();
            }))
            .into_any_element(),
            ui::icon_button("back", icons::BACK, Tone::Quiet, Control::Compact, t)
                .tooltip("Back")
                .disabled(pane.back.is_empty())
                .on_click(cx.listener(|this, _, _, cx| this.go_back(this.active, cx)))
                .into_any_element(),
            ui::icon_button("forward", icons::FORWARD, Tone::Quiet, Control::Compact, t)
                .tooltip("Forward")
                .disabled(pane.forward.is_empty())
                .on_click(cx.listener(|this, _, _, cx| this.go_forward(this.active, cx)))
                .into_any_element(),
        ];

        let title = div()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .min_w_0()
            .child(
                icon(if pane.is_remote() {
                    icons::SERVER
                } else {
                    icons::COMPUTER
                })
                .size(px(layout::ICON_SIZE))
                .text_color(t.text_muted),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.location_label(&pane.location)),
            )
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .text_color(t.text_muted)
                    .child(pane.path.clone()),
            )
            .into_any_element();

        let trailing = vec![
            ui::icon_label_button(
                "copy-other",
                copy_icon,
                copy_label,
                Tone::Primary,
                Control::Compact,
                t,
            )
            .tooltip("Copy to the other pane (F5)")
            .disabled(!has_targets)
            .on_click(cx.listener(|this, _, _, cx| this.copy_to_other(cx)))
            .into_any_element(),
            ui::icon_button(
                "new-folder",
                icons::NEW_FOLDER,
                Tone::Quiet,
                Control::Compact,
                t,
            )
            .tooltip("New Folder (F7)")
            .on_click(cx.listener(|this, _, window, cx| this.ask_new_folder(window, cx)))
            .into_any_element(),
            ui::icon_button("rename", icons::RENAME, Tone::Quiet, Control::Compact, t)
                .tooltip("Rename (F2)")
                .disabled(!has_targets)
                .on_click(cx.listener(|this, _, window, cx| this.ask_rename(window, cx)))
                .into_any_element(),
            ui::icon_button("delete", icons::DELETE, Tone::Quiet, Control::Compact, t)
                .tooltip("Delete (F8)")
                .disabled(!has_targets)
                .on_click(cx.listener(|this, _, _, cx| this.ask_delete(cx)))
                .into_any_element(),
            div().w(px(1.)).h(px(16.)).bg(t.border).into_any_element(),
            ui::icon_button("swap-panes", icons::SWAP, Tone::Quiet, Control::Compact, t)
                .tooltip("Swap Panes (⌘U)")
                .on_click(cx.listener(|this, _, window, cx| this.swap_panes(window, cx)))
                .into_any_element(),
            ui::icon_button("refresh", icons::REFRESH, Tone::Quiet, Control::Compact, t)
                .tooltip("Refresh (⌘R)")
                .on_click(cx.listener(|this, _, _, cx| this.refresh(this.active, cx)))
                .into_any_element(),
            ui::icon_button(
                "hidden",
                if self.stored.settings.show_hidden {
                    icons::SHOWN
                } else {
                    icons::HIDDEN
                },
                Tone::Quiet,
                Control::Compact,
                t,
            )
            .tooltip(if self.stored.settings.show_hidden {
                "Hide Hidden Files (⌘⇧.)"
            } else {
                "Show Hidden Files (⌘⇧.)"
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx)))
            .into_any_element(),
            ui::icon_button(
                "transfers",
                icons::TRANSFERS,
                Tone::Quiet,
                Control::Compact,
                t,
            )
            .tooltip("Transfers (⌘⇧T)")
            .on_click(cx.listener(|this, _, _, cx| {
                this.transfers_visible = !this.transfers_visible;
                cx.notify();
            }))
            .into_any_element(),
            ui::icon_button(
                "settings",
                icons::SETTINGS,
                Tone::Quiet,
                Control::Compact,
                t,
            )
            .tooltip("Settings (⌘,)")
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
            .into_any_element(),
        ];

        ui::titlebar(
            t,
            self.site_color(&pane.location),
            leading,
            vec![title],
            trailing,
        )
    }

    // --- sidebar --------------------------------------------------------------

    fn render_sidebar(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let active = &self.panes[self.active];
        let home = fs::home();
        let favorites: Vec<(&'static str, String, &'static str)> = [
            ("Home", home.clone(), icons::HOME),
            ("Desktop", home.join("Desktop"), icons::DESKTOP),
            ("Documents", home.join("Documents"), icons::FOLDER),
            ("Downloads", home.join("Downloads"), icons::DOWNLOAD),
        ]
        .into_iter()
        .filter(|(_, path, _)| path.is_dir())
        .map(|(label, path, glyph)| (label, path.to_string_lossy().into_owned(), glyph))
        .chain(std::iter::once((
            "Computer",
            std::path::MAIN_SEPARATOR.to_string(),
            icons::DISK,
        )))
        .collect();

        let place = |id: SharedString,
                     glyph: &'static str,
                     ink: theme::color::Srgb,
                     label: String,
                     current: bool| {
            div()
                .id(id)
                .h(px(layout::SIDEBAR_ROW_HEIGHT))
                .mx(px(layout::SPACE_SM))
                .px(px(layout::SPACE_SM))
                .flex()
                .items_center()
                .gap(px(layout::SPACE_SM))
                .rounded(px(layout::RADIUS_CONTROL))
                .cursor_pointer()
                .when(current, |row| row.bg(t.element_active))
                .when(!current, |row| row.hover(|style| style.bg(t.element_hover)))
                .child(icon(glyph).size(px(layout::ICON_SIZE)).text_color(ink))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(label),
                )
        };

        let favorites = favorites.into_iter().map(|(label, path, glyph)| {
            let current = active.location == Location::Local && active.path == path;
            place(
                format!("fav-{label}").into(),
                glyph,
                t.text_muted,
                label.into(),
                current,
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                let ix = this.active;
                this.navigate(ix, Location::Local, path.clone(), cx);
                this.focus_pane(ix, window, cx);
            }))
            .into_any_element()
        });

        let weak = cx.entity().downgrade();
        let pinned = self.stored.favorites.iter().enumerate().map(|(index, favorite)| {
            let location = match &favorite.site {
                None => Location::Local,
                Some(id) => Location::Remote(id.clone()),
            };
            let current = active.location == location && active.path == favorite.path;
            let tint = self
                .site_color(&location)
                .map(|c| c.swatch())
                .unwrap_or(t.folder);
            let site_name = favorite
                .site
                .as_ref()
                .map(|id| self.site(id).map(|site| site.name.clone()).unwrap_or_else(|| id.clone()));
            let group: SharedString = format!("pinned-{index}").into();
            let weak = weak.clone();
            place(
                format!("pinned-{index}").into(),
                icons::FOLDER,
                tint,
                super::favorite_name(&favorite.path),
                current,
            )
            .group(group.clone())
            .when_some(site_name, |row, name| {
                row.child(
                    div()
                        .flex_shrink_0()
                        .max_w(px(80.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(px(layout::TEXT_XS))
                        .text_color(t.text_faint)
                        .child(name),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .opacity(0.)
                    .group_hover(group, |style| style.opacity(1.))
                    .child(
                        ui::icon_button(
                            SharedString::from(format!("unpin-{index}")),
                            icons::CLOSE,
                            Tone::Quiet,
                            Control::Inline,
                            t,
                        )
                        .tooltip("Remove from Favorites")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.remove_favorite(index, cx);
                        })),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_favorite(index, cx);
                let ix = this.active;
                this.focus_pane(ix, window, cx);
            }))
            .context_menu(move |menu, _, _| {
                let weak = weak.clone();
                menu.item(
                    PopupMenuItem::new("Remove from Favorites")
                        .icon(icon(icons::CLOSE))
                        .on_click(move |_, _, cx| {
                            let _ = weak.update(cx, |this, cx| this.remove_favorite(index, cx));
                        }),
                )
            })
            .into_any_element()
        });
        let pinned: Vec<AnyElement> = pinned.collect();

        let sites = self.stored.sites.iter().map(|site| {
            let id = site.id.clone();
            let current = active.location == Location::Remote(id.clone());
            let status = match self.conns.get(&id) {
                None => Status::Idle,
                Some(Conn::Connecting) => Status::Connecting,
                Some(Conn::Connected(_)) => Status::Connected,
                Some(Conn::Failed) => Status::Failed,
            };
            let connected = matches!(status, Status::Connected);
            let tint = site.color.as_deref().and_then(SiteColor::from_slug);
            let group: SharedString = format!("site-{id}").into();
            let (open_id, edit_id, eject_id) = (id.clone(), id.clone(), id.clone());
            place(
                format!("site-{id}").into(),
                icons::SERVER,
                tint.map(|c| c.swatch()).unwrap_or(t.text_muted),
                site.name.clone(),
                current,
            )
            .group(group.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .opacity(0.)
                    .group_hover(group.clone(), |style| style.opacity(1.))
                    .child(
                        ui::icon_button(
                            SharedString::from(format!("edit-{edit_id}")),
                            icons::RENAME,
                            Tone::Quiet,
                            Control::Inline,
                            t,
                        )
                        .tooltip("Edit Site")
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.open_form(Some(edit_id.clone()), window, cx);
                            },
                        )),
                    )
                    .when(connected, |actions| {
                        actions.child(
                            ui::icon_button(
                                SharedString::from(format!("eject-{eject_id}")),
                                icons::DISCONNECT,
                                Tone::Quiet,
                                Control::Inline,
                                t,
                            )
                            .tooltip("Disconnect")
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.disconnect(&eject_id, cx);
                                },
                            )),
                        )
                    }),
            )
            .child(ui::status_dot(t, status))
            .on_click(cx.listener(move |this, _, window, cx| {
                let ix = this.active;
                this.open_site(open_id.clone(), ix, cx);
                this.focus_pane(ix, window, cx);
            }))
            .into_any_element()
        });
        let sites: Vec<AnyElement> = sites.collect();
        let no_sites = sites.is_empty();

        let heading = |label: &str| {
            div()
                .px(px(layout::SPACE_LG))
                .pt(px(layout::SPACE_MD))
                .pb(px(layout::SPACE_XS))
                .child(ui::section_label(t, label))
        };

        div()
            .id("sidebar")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .pb(px(layout::SPACE_MD))
            .child(
                div()
                    .id("favorites")
                    .flex()
                    .flex_col()
                    .pb(px(layout::SPACE_XS))
                    .rounded(px(layout::RADIUS_CONTROL))
                    .on_drop(cx.listener(|this, dragged: &DraggedFiles, _, cx| {
                        this.drop_favorites(dragged, cx)
                    }))
                    .drag_over::<DraggedFiles>(|style, _, _, cx| {
                        let t = *theme::theme(cx);
                        style.bg(t.selection)
                    })
                    .child(heading("Favorites"))
                    .children(favorites)
                    .children(pinned),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pr(px(layout::SPACE_SM))
                    .child(heading("Sites"))
                    .child(
                        ui::icon_button("add-site", icons::PLUS, Tone::Quiet, Control::Inline, t)
                            .mt(px(layout::SPACE_SM))
                            .tooltip("New Site (⌘N)")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.open_form(None, window, cx)),
                            ),
                    ),
            )
            .children(sites)
            .when(no_sites, |column| {
                column.child(
                    div()
                        .mx(px(layout::SPACE_LG))
                        .mt(px(layout::SPACE_XS))
                        .flex()
                        .flex_col()
                        .gap(px(layout::SPACE_SM))
                        .child(
                            div()
                                .text_size(px(layout::TEXT_SM))
                                .text_color(t.text_faint)
                                .child("No sites yet. Add a server to browse it over SFTP."),
                        )
                        .child(
                            ui::icon_label_button(
                                "first-site",
                                icons::PLUS,
                                "New Site",
                                Tone::Primary,
                                Control::Compact,
                                t,
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| this.open_form(None, window, cx)),
                            ),
                        ),
                )
            })
    }

    // --- content --------------------------------------------------------------

    fn render_content(&self, t: Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let panes = h_resizable("pane-split")
            .with_state(&self.pane_split)
            .child(resizable_panel().child(self.render_pane(0, t, window, cx)))
            .child(resizable_panel().child(self.render_pane(1, t, window, cx)));
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().child(panes))
            .when(self.transfers_visible, |column| {
                column.child(self.render_transfers(t, cx))
            })
            .into_any_element()
    }

    fn render_pane(
        &self,
        ix: usize,
        t: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane = &self.panes[ix];
        let focused = self.active == ix;
        let plane = if focused {
            t.data_glass()
        } else {
            t.panel_glass()
        };
        div()
            .id(("pane", ix))
            .track_focus(&pane.focus)
            .key_context(FILE_LIST)
            .size_full()
            .flex()
            .flex_col()
            .bg(plane)
            .when(ix == 1, |pane| pane.border_l_1().border_color(t.border))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    if this.active != ix || !this.panes[ix].focus.is_focused(window) {
                        this.focus_pane(ix, window, cx);
                    }
                }),
            )
            .on_drop(cx.listener(move |this, dragged: &DraggedFiles, _, cx| {
                this.drop_files(dragged, ix, cx)
            }))
            .drag_over::<DraggedFiles>(move |style, dragged, _, cx| {
                if dragged.pane == ix {
                    style
                } else {
                    let t = *theme::theme(cx);
                    style.bg(t.selection)
                }
            })
            .child(self.render_pane_header(ix, t, window, cx))
            .child(div().flex_1().min_h_0().child(self.render_pane_body(ix, t, cx)))
            .into_any_element()
    }

    /// The listing, with the folder tree beside it when that is on.
    fn render_pane_body(&self, ix: usize, t: Theme, cx: &mut Context<Self>) -> AnyElement {
        let pane = &self.panes[ix];
        let listing = div()
            .size_full()
            .flex()
            .flex_col()
            .child(column_headers(ix, pane, pane.is_remote(), t, cx))
            .child(self.render_listing(ix, t, cx))
            .into_any_element();
        if !pane.tree.visible {
            return listing;
        }
        h_resizable(("tree-split", ix))
            .with_state(&pane.tree_split)
            .child(
                resizable_panel()
                    .size(px(layout::TREE_DEFAULT_WIDTH))
                    .size_range(px(layout::TREE_MIN_WIDTH)..px(layout::TREE_MAX_WIDTH))
                    .child(self.render_tree(ix, t, cx)),
            )
            .child(resizable_panel().child(listing))
            .into_any_element()
    }

    fn render_tree(&self, ix: usize, t: Theme, cx: &mut Context<Self>) -> AnyElement {
        let tree = &self.panes[ix].tree;
        div()
            .size_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(t.border)
            .child(
                uniform_list(
                    ("tree", ix),
                    tree.rows.len(),
                    cx.processor(move |this, range: Range<usize>, _window, cx| {
                        this.render_tree_rows(ix, range, cx)
                    }),
                )
                .track_scroll(&tree.scroll)
                .flex_1()
                .py(px(layout::SPACE_XS)),
            )
            .into_any_element()
    }

    fn render_tree_rows(
        &mut self,
        ix: usize,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let t = *theme::theme(cx);
        let pane = &self.panes[ix];
        let focused = self.active == ix;
        let remote = pane.is_remote();
        range
            .filter_map(|n| {
                let row = pane.tree.rows.get(n)?;
                let current = row.path == pane.path;
                let path = row.path.clone();
                let toggle_path = row.path.clone();
                let location = pane.location.clone();
                let (menu_path, menu_location) = (row.path.clone(), pane.location.clone());
                let weak = cx.entity().downgrade();
                let (glyph, tint) = if row.depth == 0 {
                    (if remote { icons::SERVER } else { icons::DISK }, t.text_muted)
                } else if row.expanded && !row.leaf {
                    (icons::FOLDER_OPEN, t.folder)
                } else {
                    (icons::FOLDER, t.folder)
                };
                let disclosure = if row.leaf {
                    div().size(px(layout::TREE_CHEVRON_SIZE)).into_any_element()
                } else {
                    div()
                        .id(SharedString::from(format!("tree-toggle-{ix}-{n}")))
                        .size(px(layout::TREE_CHEVRON_SIZE))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(layout::RADIUS_CONTROL - 2.))
                        .hover(|style| style.bg(t.element_hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_tree_folder(ix, &toggle_path, cx);
                        }))
                        .child(
                            icon(if row.expanded {
                                icons::CHEVRON_DOWN
                            } else {
                                icons::CHEVRON_RIGHT
                            })
                            .size(px(12.))
                            .text_color(if row.loading { t.text_faint } else { t.text_muted }),
                        )
                        .into_any_element()
                };
                Some(
                    div()
                        .id(SharedString::from(format!("tree-{ix}-{n}")))
                        .h(px(layout::TREE_ROW_HEIGHT))
                        .mx(px(layout::SPACE_XS))
                        .pl(px(layout::SPACE_XS + row.depth as f32 * layout::TREE_INDENT))
                        .pr(px(layout::SPACE_SM))
                        .flex()
                        .items_center()
                        .gap(px(layout::SPACE_XS))
                        .rounded(px(layout::RADIUS_CONTROL))
                        .cursor_pointer()
                        .when(current, |r| {
                            r.bg(if focused { t.selection } else { t.element_active })
                        })
                        .when(!current, |r| r.hover(|style| style.bg(t.element_hover)))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.click_count() >= 2 {
                                this.toggle_tree_folder(ix, &path, cx);
                            } else {
                                this.navigate(ix, location.clone(), path.clone(), cx);
                            }
                            this.focus_pane(ix, window, cx);
                        }))
                        .child(disclosure)
                        .child(icon(glyph).size(px(layout::ICON_SIZE)).text_color(tint))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(layout::TEXT_SM))
                                .when(row.hidden, |name| name.text_color(t.text_muted))
                                .when(current, |name| name.font_weight(FontWeight::MEDIUM))
                                .child(row.name.clone()),
                        )
                        .context_menu(move |menu, _, _| {
                            let weak = weak.clone();
                            let (location, path) = (menu_location.clone(), menu_path.clone());
                            menu.item(
                                PopupMenuItem::new("Add to Favorites")
                                    .icon(icon(icons::FOLDER))
                                    .on_click(move |_, _, cx| {
                                        let (location, path) = (location.clone(), path.clone());
                                        let _ = weak.update(cx, |this, cx| {
                                            this.add_favorite(location, path, cx)
                                        });
                                    }),
                            )
                        })
                        .into_any_element(),
                )
            })
            .collect()
    }

    fn render_pane_header(
        &self,
        ix: usize,
        t: Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane = &self.panes[ix];
        let remote = pane.is_remote();
        let tint = self.site_color(&pane.location);

        let chip = div()
            .flex_shrink_0()
            .h(px(layout::CONTROL_HEIGHT_COMPACT))
            .px(px(layout::SPACE_SM))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_XS + 2.))
            .rounded(px(layout::RADIUS_CONTROL))
            .bg(t.element_hover)
            .child(
                icon(if remote {
                    icons::SERVER
                } else {
                    icons::COMPUTER
                })
                .size(px(layout::ICON_SIZE))
                .text_color(tint.map(|c| c.swatch()).unwrap_or(t.text_muted)),
            )
            .child(
                div()
                    .text_size(px(layout::TEXT_SM))
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .child(self.location_label(&pane.location)),
            );

        let path = if pane.editing_path {
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Input::new(&pane.path_input)
                        .appearance(false)
                        .p_0()
                        .text_size(px(layout::TEXT_SM)),
                )
                .into_any_element()
        } else {
            let crumbs = fs::crumbs(&pane.path, remote);
            let count = crumbs.len();
            div()
                .id(("crumbs", ix))
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .overflow_hidden()
                .cursor_text()
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    if event.click_count() >= 2 {
                        this.edit_path(ix, window, cx);
                    }
                }))
                .children(crumbs.into_iter().enumerate().map(|(n, (label, path))| {
                    let last = n + 1 == count;
                    let location = pane.location.clone();
                    div()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .when(n > 0, |crumb| {
                            crumb.child(
                                icon(icons::CHEVRON_RIGHT)
                                    .size(px(12.))
                                    .text_color(t.text_faint),
                            )
                        })
                        .child(
                            div()
                                .id(SharedString::from(format!("crumb-{ix}-{n}")))
                                .px(px(layout::SPACE_XS))
                                .rounded(px(layout::RADIUS_CONTROL - 2.))
                                .text_size(px(layout::TEXT_SM))
                                .whitespace_nowrap()
                                .cursor_pointer()
                                .text_color(if last { t.text } else { t.text_muted })
                                .when(last, |crumb| crumb.font_weight(FontWeight::MEDIUM))
                                .hover(|style| style.bg(t.element_hover))
                                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                    if event.click_count() == 1 {
                                        cx.stop_propagation();
                                        this.navigate(ix, location.clone(), path.clone(), cx);
                                    }
                                }))
                                .child(label),
                        )
                }))
                .into_any_element()
        };

        let filter = div()
            .w(px(150.))
            .flex_shrink_0()
            .h(px(layout::CONTROL_HEIGHT_COMPACT))
            .px(px(layout::SPACE_SM))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_XS))
            .rounded(px(layout::RADIUS_CONTROL))
            .bg(t.element_hover)
            .child(icon(icons::SEARCH).size(px(12.)).text_color(t.text_faint))
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&pane.filter)
                        .appearance(false)
                        .p_0()
                        .text_size(px(layout::TEXT_SM)),
                ),
            );

        div()
            .h(px(layout::PANE_HEADER_HEIGHT))
            .flex_shrink_0()
            .px(px(layout::SPACE_SM))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .border_b_1()
            .border_color(t.border)
            .child(
                ui::icon_button(
                    ("tree-toggle", ix),
                    icons::FOLDER_TREE,
                    Tone::Quiet,
                    Control::Inline,
                    t,
                )
                .selected(pane.tree.visible)
                .tooltip(if pane.tree.visible {
                    "Hide Folder Tree (⌥⌘T)"
                } else {
                    "Show Folder Tree (⌥⌘T)"
                })
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_tree(ix, cx))),
            )
            .child(
                ui::icon_button(("up", ix), icons::UP, Tone::Quiet, Control::Inline, t)
                    .tooltip("Enclosing Folder (⌘↑)")
                    .disabled(pane.parent().is_none())
                    .on_click(cx.listener(move |this, _, _, cx| this.go_up(ix, cx))),
            )
            .child(chip)
            .child(path)
            .child(filter)
    }

    fn render_listing(&self, ix: usize, t: Theme, cx: &mut Context<Self>) -> AnyElement {
        let pane = &self.panes[ix];
        let message = |text: String, color| {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p(px(layout::SPACE_LG))
                .text_size(px(layout::TEXT_SM))
                .text_color(color)
                .child(text)
                .into_any_element()
        };
        if let Some(error) = &pane.error {
            return message(error.clone(), t.danger);
        }
        if pane.rows.is_empty() {
            if pane.loading {
                return message("Loading…".into(), t.text_faint);
            }
            let filtered = !pane.filter.read(cx).value().is_empty();
            return message(
                if filtered {
                    "No matches."
                } else {
                    "This folder is empty."
                }
                .into(),
                t.text_faint,
            );
        }
        uniform_list(
            ("listing", ix),
            pane.rows.len(),
            cx.processor(move |this, range: Range<usize>, _window, cx| {
                this.render_rows(ix, range, cx)
            }),
        )
        .track_scroll(&pane.scroll)
        .flex_1()
        .into_any_element()
    }

    fn render_rows(
        &mut self,
        ix: usize,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let t = *theme::theme(cx);
        let pane = &self.panes[ix];
        let focused = self.active == ix;
        let remote = pane.is_remote();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        range
            .filter_map(|row| {
                let entry = pane.entry_at(row)?.clone();
                let selected = pane.selected.contains(&entry.name);
                let cursor = pane.cursor == Some(row) && focused;
                let dragged = if selected {
                    pane.targets()
                } else {
                    vec![entry.clone()]
                };
                let (glyph, tint) = file_icon(&entry, t);
                let weak = cx.entity().downgrade();
                let folder = entry.kind.is_dir().then(|| pane.join(&entry.name));
                let here = pane.path.clone();
                let location = pane.location.clone();
                let label = match dragged.as_slice() {
                    [one] => one.name.clone(),
                    many => format!("{} items", many.len()),
                };
                Some(
                    div()
                        .id(SharedString::from(format!("row-{ix}-{row}")))
                        .w_full()
                        .h(px(layout::ROW_HEIGHT))
                        .px(px(layout::SPACE_MD))
                        .flex()
                        .items_center()
                        .gap(px(layout::SPACE_SM))
                        .text_size(px(layout::TEXT_MD))
                        .when(row % 2 == 1, |r| {
                            r.bg(t.element_hover.flatten(t.bg).alpha(0.35))
                        })
                        .when(selected, |r| {
                            r.bg(if focused {
                                t.selection
                            } else {
                                t.element_active
                            })
                        })
                        .when(cursor && !selected, |r| r.bg(t.element_active))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.active = ix;
                            window.focus(&this.panes[ix].focus, cx);
                            let modifiers = event.modifiers();
                            let pane = &mut this.panes[ix];
                            if event.click_count() >= 2 {
                                this.open_row(ix, row, cx);
                            } else if modifiers.platform || modifiers.control {
                                pane.toggle(row);
                            } else if modifiers.shift {
                                pane.select_range(row);
                            } else {
                                pane.select_only(row);
                            }
                            cx.notify();
                        }))
                        .on_drag(
                            DraggedFiles {
                                pane: ix,
                                items: dragged,
                            },
                            move |_, _, _, cx| {
                                let label = label.clone();
                                cx.new(|_| DragPreview { label, glyph })
                            },
                        )
                        .child(
                            icon(glyph)
                                .size(px(layout::FILE_ICON_SIZE))
                                .text_color(tint),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .when(entry.is_hidden(), |name| name.text_color(t.text_muted))
                                .child(entry.name.clone()),
                        )
                        .child(
                            cell(layout::SIZE_COLUMN_WIDTH, t.text_muted)
                                .justify_end()
                                .child(if entry.kind.is_dir() {
                                    "—".to_string()
                                } else {
                                    ui::human_bytes(entry.size)
                                }),
                        )
                        .child(
                            cell(layout::DATE_COLUMN_WIDTH, t.text_muted).child(
                                entry
                                    .modified
                                    .map(|m| ui::short_date(m, now))
                                    .unwrap_or_default(),
                            ),
                        )
                        .child(if remote {
                            cell(layout::MODE_COLUMN_WIDTH, t.text_muted)
                                .font_family(theme::MONO_FONT)
                                .text_size(px(layout::TEXT_SM))
                                .child(entry.mode.map(ui::mode_string).unwrap_or_default())
                        } else {
                            cell(layout::MODE_COLUMN_WIDTH, t.text_muted).child(entry.kind_label())
                        })
                        .context_menu(move |menu, _, _| {
                            let favorite = weak.clone();
                            let current = weak.clone();
                            let (folder, here, location) =
                                (folder.clone(), here.clone(), location.clone());
                            let here_location = location.clone();
                            menu.item(
                                PopupMenuItem::new("Add to Favorites")
                                    .icon(icon(icons::FOLDER))
                                    .disabled(folder.is_none())
                                    .on_click(move |_, _, cx| {
                                        if let Some(path) = folder.clone() {
                                            let location = location.clone();
                                            let _ = favorite.update(cx, |this, cx| {
                                                this.add_favorite(location, path, cx)
                                            });
                                        }
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Add Current Folder to Favorites").on_click(
                                    move |_, _, cx| {
                                        let (location, path) =
                                            (here_location.clone(), here.clone());
                                        let _ = current.update(cx, |this, cx| {
                                            this.add_favorite(location, path, cx)
                                        });
                                    },
                                ),
                            )
                        })
                        .into_any_element(),
                )
            })
            .collect()
    }

    // --- transfers ------------------------------------------------------------

    fn render_transfers(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.transfers.iter().rev().map(|row| {
            let id = row.id;
            let fraction = row.progress.fraction();
            let done = row.progress.bytes_done.load(Ordering::Relaxed);
            let total = row.progress.bytes_total.load(Ordering::Relaxed);
            let (glyph, color) = match row.direction {
                Direction::Upload => (icons::UPLOAD, t.accent),
                Direction::Download => (icons::DOWNLOAD, t.success),
                Direction::Copy => (icons::COPY, t.text_muted),
            };
            let (detail, detail_color, bar_color, end): (String, _, _, AnyElement) =
                match &row.state {
                    TransferState::Running => (
                        format!(
                            "{} of {}{}",
                            ui::human_bytes(done),
                            ui::human_bytes(total),
                            match row.progress.current() {
                                name if name.is_empty() => String::new(),
                                name => format!(" · {name}"),
                            }
                        ),
                        t.text_muted,
                        t.accent,
                        ui::icon_button(
                            ("cancel", id as usize),
                            icons::CLOSE,
                            Tone::Quiet,
                            Control::Inline,
                            t,
                        )
                        .tooltip("Cancel")
                        .on_click(cx.listener(move |this, _, _, _| this.cancel_transfer(id)))
                        .into_any_element(),
                    ),
                    TransferState::Done => (
                        format!("{} · done", ui::human_bytes(total)),
                        t.text_muted,
                        t.success,
                        icon(icons::DONE)
                            .size(px(layout::ICON_SIZE))
                            .text_color(t.success)
                            .into_any_element(),
                    ),
                    TransferState::Failed(error) => (
                        error.clone(),
                        t.danger,
                        t.danger,
                        icon(icons::FAILED)
                            .size(px(layout::ICON_SIZE))
                            .text_color(t.danger)
                            .into_any_element(),
                    ),
                };
            let fraction = if matches!(row.state, TransferState::Done) {
                1.0
            } else {
                fraction
            };
            div()
                .px(px(layout::SPACE_LG))
                .py(px(layout::SPACE_SM))
                .flex()
                .items_center()
                .gap(px(layout::SPACE_MD))
                .border_b_1()
                .border_color(t.border)
                .child(
                    icon(glyph)
                        .size(px(layout::FILE_ICON_SIZE))
                        .text_color(color),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(layout::SPACE_XS))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(layout::SPACE_SM))
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(row.label.clone()),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .text_size(px(layout::TEXT_SM))
                                        .text_color(detail_color)
                                        .child(detail),
                                ),
                        )
                        .child(ui::progress(t, fraction, bar_color)),
                )
                .child(end)
                .into_any_element()
        });
        let empty = self.transfers.is_empty();
        div()
            .h(px(layout::TRANSFERS_HEIGHT))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(t.border)
            .child(
                div()
                    .h(px(layout::COLUMN_HEADER_HEIGHT + 4.))
                    .px(px(layout::SPACE_LG))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(t.border)
                    .child(ui::section_label(t, "Transfers"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(layout::SPACE_XS))
                            .child(
                                ui::button("clear-finished", "Clear Finished", Tone::Quiet, Control::Inline, t)
                                    .on_click(cx.listener(|this, _, _, cx| this.clear_finished(cx))),
                            )
                            .child(
                                ui::icon_button("close-transfers", icons::CLOSE, Tone::Quiet, Control::Inline, t)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.transfers_visible = false;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .id("transfer-rows")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(rows)
                    .when(empty, |list| {
                        list.child(
                            div()
                                .p(px(layout::SPACE_LG))
                                .text_size(px(layout::TEXT_SM))
                                .text_color(t.text_faint)
                                .child("Nothing transferred yet. Select files and press F5, or drag them onto the other pane."),
                        )
                    }),
            )
    }

    // --- status bar -----------------------------------------------------------

    fn render_status(&self, t: Theme) -> impl IntoElement {
        let pane = &self.panes[self.active];
        let status = match &pane.location {
            Location::Local => Status::Connected,
            Location::Remote(id) => match self.conns.get(id) {
                Some(Conn::Connected(_)) => Status::Connected,
                Some(Conn::Connecting) => Status::Connecting,
                Some(Conn::Failed) => Status::Failed,
                None => Status::Idle,
            },
        };
        let where_ = match &pane.location {
            Location::Local => "Local".to_string(),
            Location::Remote(id) => self.site(id).map(|site| site.address()).unwrap_or_default(),
        };
        let (selected, bytes) = pane.selection_summary();
        let summary = if selected > 0 {
            format!(
                "{selected} of {} selected · {}",
                pane.rows.len(),
                ui::human_bytes(bytes)
            )
        } else {
            format!("{} items", pane.rows.len())
        };
        let running: Vec<_> = self
            .transfers
            .iter()
            .filter(|row| matches!(row.state, TransferState::Running))
            .collect();
        let transfers = (!running.is_empty()).then(|| {
            let done: u64 = running
                .iter()
                .map(|r| r.progress.bytes_done.load(Ordering::Relaxed))
                .sum();
            let total: u64 = running
                .iter()
                .map(|r| r.progress.bytes_total.load(Ordering::Relaxed))
                .sum();
            let percent = if total > 0 { done * 100 / total } else { 0 };
            format!(
                "{} transfer{} · {percent}%",
                running.len(),
                if running.len() == 1 { "" } else { "s" }
            )
        });

        div()
            .h(px(layout::STATUS_HEIGHT))
            .flex_shrink_0()
            .px(px(layout::SPACE_MD))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .border_t_1()
            .border_color(t.border)
            .text_size(px(layout::TEXT_SM))
            .text_color(t.text_muted)
            .child(ui::status_dot(t, status))
            .child(div().flex_shrink_0().child(where_))
            .when_some(self.status.as_ref(), |bar, line| {
                bar.child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_color(if line.error { t.danger } else { t.text_faint })
                        .child(format!("— {}", line.text)),
                )
            })
            .child(div().flex_1())
            .when_some(transfers, |bar, text| {
                bar.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(layout::SPACE_XS))
                        .text_color(t.accent)
                        .child(icon(icons::TRANSFERS).size(px(12.)).text_color(t.accent))
                        .child(text),
                )
            })
            .child(div().flex_shrink_0().child(summary))
    }

    // --- modals ---------------------------------------------------------------

    fn render_modal(
        &self,
        t: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let card = if let Some(question) = self.host_questions.front() {
            ui::dialog(t)
                .mt(px(120.))
                .child(dialog_title(t, icons::TRUST, "Trust this server?"))
                .child(dialog_text(
                    t,
                    format!(
                        "{}:{} is not in your known hosts. Check that its fingerprint matches \
                         the one your server administrator gave you before trusting it.",
                        question.host, question.port
                    ),
                ))
                .child(
                    div()
                        .p(px(layout::SPACE_MD))
                        .rounded(px(layout::RADIUS_CONTROL))
                        .bg(t.element_hover)
                        .flex()
                        .flex_col()
                        .gap(px(layout::SPACE_XS))
                        .child(
                            div()
                                .text_size(px(layout::TEXT_XS))
                                .text_color(t.text_faint)
                                .child(question.algorithm.clone()),
                        )
                        .child(
                            div()
                                .font_family(theme::MONO_FONT)
                                .text_size(px(layout::TEXT_SM))
                                .child(question.fingerprint.clone()),
                        ),
                )
                .child(dialog_buttons(
                    t,
                    cx,
                    "Cancel",
                    ui::accent_button("trust", "Trust", t, cx)
                        .on_click(cx.listener(|this, _, _, cx| this.answer_host_key(true, cx)))
                        .into_any_element(),
                ))
                .into_any_element()
        } else {
            match self.modal.as_ref()? {
                Modal::Site(form) => self.render_form(form, t, cx),
                Modal::Settings => self.render_settings(t, window, cx),
                Modal::NewFolder { input, .. } => ui::dialog(t)
                    .mt(px(120.))
                    .child(dialog_title(t, icons::NEW_FOLDER, "New Folder"))
                    .child(field(t, input))
                    .child(dialog_buttons(
                        t,
                        cx,
                        "Cancel",
                        ui::accent_button("create", "Create", t, cx)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.confirm_modal(window, cx)),
                            )
                            .into_any_element(),
                    ))
                    .into_any_element(),
                Modal::Rename { from, input, .. } => ui::dialog(t)
                    .mt(px(120.))
                    .child(dialog_title(t, icons::RENAME, &format!("Rename “{from}”")))
                    .child(field(t, input))
                    .child(dialog_buttons(
                        t,
                        cx,
                        "Cancel",
                        ui::accent_button("do-rename", "Rename", t, cx)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.confirm_modal(window, cx)),
                            )
                            .into_any_element(),
                    ))
                    .into_any_element(),
                Modal::Delete { items, pane } => {
                    let where_ = if self.panes[*pane].is_remote() {
                        "from the server"
                    } else {
                        "from this computer"
                    };
                    let what = match items.as_slice() {
                        [one] => format!("“{}”", one.name),
                        many => format!("{} items", many.len()),
                    };
                    ui::dialog(t)
                        .mt(px(120.))
                        .child(dialog_title(t, icons::DELETE, &format!("Delete {what}?")))
                        .child(dialog_text(
                            t,
                            format!(
                                "They are removed {where_} permanently, folders with everything in them. This cannot be undone."
                            ),
                        ))
                        .child(dialog_buttons(
                            t,
                            cx,
                            "Cancel",
                            ui::button("do-delete", "Delete", Tone::Danger, Control::Standard, t)
                                .on_click(cx.listener(|this, _, window, cx| this.confirm_modal(window, cx)))
                                .into_any_element(),
                        ))
                        .into_any_element()
                }
                Modal::Import(plan) => {
                    let adds = match (plan.sites.len(), plan.folders.len()) {
                        (0, folders) => super::import::count(folders, "local folder"),
                        (sites, 0) => super::import::count(sites, "site"),
                        (sites, folders) => format!(
                            "{} and {}",
                            super::import::count(sites, "site"),
                            super::import::count(folders, "local folder")
                        ),
                    };
                    let mut lines = vec![format!(
                        "{adds} will be added to filejam{}.",
                        if plan.folders.is_empty() { "" } else { " (folders go to Favorites)" }
                    )];
                    match plan.existing {
                        0 => {}
                        1 => lines.push(
                            "1 server is already here (same host, user and port) and is left as it is."
                                .into(),
                        ),
                        n => lines.push(format!(
                            "{n} servers are already here (same host, user and port) and are left as they are."
                        )),
                    }
                    match plan.unsupported {
                        0 => {}
                        1 => lines.push("1 favorite is not SFTP (FTP, S3, WebDAV…) and is skipped.".into()),
                        n => lines.push(format!(
                            "{n} favorites are not SFTP (FTP, S3, WebDAV…) and are skipped."
                        )),
                    }
                    lines.push(
                        "Passwords stay in ForkLift: sites sign in with your SSH agent or keys. \
                         Add a password with Edit Site where one is needed."
                            .into(),
                    );
                    let names: Vec<String> = plan.sites.iter().map(|s| s.name.clone()).collect();
                    ui::dialog(t)
                        .mt(px(120.))
                        .child(dialog_title(t, icons::SERVER, "Import from ForkLift"))
                        .children(lines.into_iter().map(|line| dialog_text(t, line)))
                        .when(!names.is_empty(), |card| {
                            card.child(
                                div()
                                    .id("import-names")
                                    .max_h(px(160.))
                                    .overflow_y_scroll()
                                    .p(px(layout::SPACE_SM))
                                    .rounded(px(layout::RADIUS_CONTROL))
                                    .bg(t.element_hover)
                                    .text_size(px(layout::TEXT_SM))
                                    .text_color(t.text_muted)
                                    .children(names.into_iter().map(|name| div().child(name))),
                            )
                        })
                        .child(dialog_buttons(
                            t,
                            cx,
                            "Cancel",
                            ui::accent_button("do-import", "Import", t, cx)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.confirm_modal(window, cx)
                                }))
                                .into_any_element(),
                        ))
                        .into_any_element()
                }
                Modal::Replace { clashes, .. } => {
                    let what = match clashes.as_slice() {
                        [one] => format!("“{one}” already exists there."),
                        many => format!("{} items already exist there.", many.len()),
                    };
                    ui::dialog(t)
                        .mt(px(120.))
                        .child(dialog_title(t, icons::COPY, "Replace existing items?"))
                        .child(dialog_text(
                            t,
                            format!("{what} Files are overwritten; folders are merged."),
                        ))
                        .child(dialog_buttons(
                            t,
                            cx,
                            "Cancel",
                            ui::button("do-replace", "Replace", Tone::Danger, Control::Standard, t)
                                .on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.confirm_modal(window, cx)
                                    }),
                                )
                                .into_any_element(),
                        ))
                        .into_any_element()
                }
            }
        };
        Some(
            ui::scrim(t)
                .key_context(MODAL)
                .id("modal")
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(card)
                .into_any_element(),
        )
    }
}

fn column_headers(
    ix: usize,
    pane: &Pane,
    remote: bool,
    t: Theme,
    cx: &mut Context<Workspace>,
) -> impl IntoElement {
    let header = |id: &'static str, label: &'static str, key: Option<SortKey>| {
        let current = key == Some(pane.sort);
        div()
            .id(SharedString::from(format!("{id}-{ix}")))
            .h_full()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_XS))
            .text_size(px(layout::TEXT_XS))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if current { t.text } else { t.text_faint })
            .child(label)
            .when(current, |h| {
                h.child(
                    icon(if pane.ascending {
                        icons::SORT_UP
                    } else {
                        icons::SORT_DOWN
                    })
                    .size(px(12.))
                    .text_color(t.text_muted),
                )
            })
            .when_some(key, |h, key| {
                h.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.sort_by(ix, key, cx)))
            })
    };
    div()
        .h(px(layout::COLUMN_HEADER_HEIGHT))
        .flex_shrink_0()
        .px(px(layout::SPACE_MD))
        .flex()
        .items_center()
        .gap(px(layout::SPACE_SM))
        .border_b_1()
        .border_color(t.border)
        .child(div().w(px(layout::FILE_ICON_SIZE)))
        .child(header("h-name", "NAME", Some(SortKey::Name)).flex_1())
        .child(
            header("h-size", "SIZE", Some(SortKey::Size))
                .w(px(layout::SIZE_COLUMN_WIDTH))
                .justify_end(),
        )
        .child(
            header("h-date", "MODIFIED", Some(SortKey::Modified)).w(px(layout::DATE_COLUMN_WIDTH)),
        )
        .child(if remote {
            header("h-mode", "PERMISSIONS", None).w(px(layout::MODE_COLUMN_WIDTH))
        } else {
            header("h-kind", "KIND", Some(SortKey::Kind)).w(px(layout::MODE_COLUMN_WIDTH))
        })
}

fn cell(width: f32, color: theme::color::Srgb) -> gpui::Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .flex()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(layout::TEXT_SM))
        .text_color(color)
}

/// An icon per kind of file, and the one hue folders always wear.
fn file_icon(entry: &Entry, t: Theme) -> (&'static str, theme::color::Srgb) {
    match entry.kind {
        Kind::Dir => return (icons::FOLDER, t.folder),
        Kind::LinkDir => return (icons::FOLDER, t.folder),
        Kind::Link => return (icons::SYMLINK, t.text_muted),
        Kind::File => {}
    }
    let extension = fs::extension(&entry.name).unwrap_or("").to_lowercase();
    let glyph = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "heic" | "tiff" => {
            icons::FILE_IMAGE
        }
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "dmg" => {
            icons::FILE_ARCHIVE
        }
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => icons::FILE_AUDIO,
        "mp4" | "mov" | "mkv" | "avi" | "webm" => icons::FILE_VIDEO,
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "conf" | "env" => icons::FILE_JSON,
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" => icons::FILE_TERMINAL,
        "csv" | "tsv" | "xls" | "xlsx" | "ods" => icons::FILE_SHEET,
        "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "rb" | "go" | "c" | "h" | "cpp" | "java"
        | "php" | "html" | "css" | "scss" | "swift" | "kt" | "sql" | "lua" => icons::FILE_CODE,
        "txt" | "md" | "log" | "pdf" | "doc" | "docx" | "rtf" => icons::FILE_TEXT,
        _ => icons::FILE,
    };
    (glyph, t.text_muted)
}

fn dialog_title(t: Theme, glyph: &'static str, title: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(layout::SPACE_SM))
        .child(
            icon(glyph)
                .size(px(layout::FILE_ICON_SIZE))
                .text_color(t.text_muted),
        )
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .text_size(px(layout::TEXT_LG - 1.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
}

fn dialog_text(t: Theme, text: String) -> impl IntoElement {
    div()
        .text_size(px(layout::TEXT_MD))
        .text_color(t.text_muted)
        .child(text)
}

fn field(t: Theme, input: &gpui::Entity<gpui_component::input::InputState>) -> impl IntoElement {
    div()
        .h(px(layout::CONTROL_HEIGHT))
        .px(px(layout::SPACE_MD))
        .flex()
        .items_center()
        .rounded(px(layout::RADIUS_CONTROL))
        .border_1()
        .border_color(t.border_strong)
        .bg(t.element_hover)
        .child(
            div()
                .flex_1()
                .child(Input::new(input).appearance(false).p_0()),
        )
}

fn dialog_buttons(
    t: Theme,
    cx: &mut Context<Workspace>,
    cancel: &'static str,
    primary: AnyElement,
) -> impl IntoElement {
    div()
        .pt(px(layout::SPACE_SM))
        .flex()
        .justify_end()
        .gap(px(layout::SPACE_SM))
        .child(
            ui::button("dialog-cancel", cancel, Tone::Quiet, Control::Standard, t)
                .on_click(cx.listener(|this, _, window, cx| this.dismiss_modal(window, cx))),
        )
        .child(primary)
}

/// What follows the pointer while files are dragged to the other pane.
pub struct DragPreview {
    label: String,
    glyph: &'static str,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *theme::theme(cx);
        div()
            .px(px(layout::SPACE_SM))
            .py(px(layout::SPACE_XS))
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .rounded(px(layout::RADIUS_CONTROL))
            .border_1()
            .border_color(t.border_strong)
            .bg(t.overlay)
            .shadow_lg()
            .font_family(theme::CHROME_FONT)
            .text_size(px(layout::TEXT_MD))
            .text_color(t.text)
            .child(
                icon(self.glyph)
                    .size(px(layout::ICON_SIZE))
                    .text_color(t.text_muted),
            )
            .child(self.label.clone())
    }
}

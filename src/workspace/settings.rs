//! The settings card: the colour theme, picked from previews painted in each
//! theme's own tokens, and the few switches that outlive a session.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    icons::{icon, icon as icons},
    theme::{Theme, layout},
    ui,
};

use super::{Modal, Workspace};

impl Workspace {
    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Some(Modal::Settings);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(super) fn render_settings(
        &self,
        t: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let top = layout::FORM_TOP + layout::SPACE_LG;
        let chosen = self.stored.settings.theme.as_deref();

        let system = theme_card(
            "theme-system",
            "Match System",
            chosen.is_none(),
            t,
            div()
                .size_full()
                .flex()
                .child(div().flex_1().h_full().child(preview(Theme::black())))
                .child(div().flex_1().h_full().child(preview(Theme::light()))),
        )
        .on_click(cx.listener(|this, _, window, cx| this.choose_theme(None, window, cx)));

        let themes = Theme::all().into_iter().map(|theme| {
            let name = theme.name;
            theme_card(
                SharedString::from(format!("theme-{name}")),
                name.strip_prefix("Filejam ").unwrap_or(name),
                chosen == Some(name),
                t,
                preview(theme),
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.choose_theme(Some(name), window, cx)),
            )
            .into_any_element()
        });

        let appearance = div()
            .flex()
            .flex_wrap()
            .gap(px(layout::SPACE_MD))
            .child(system)
            .children(themes);

        let show_hidden = self.stored.settings.show_hidden;
        let files = ui::well(
            t,
            [ui::option_row(
                t,
                "Show hidden files",
                "Dotfiles appear in both panes, dimmed.",
                ui::switch("settings-hidden", show_hidden, t)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
            )],
        );

        let header = div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_SM))
            .child(
                icon(icons::SETTINGS)
                    .size(px(layout::FILE_ICON_SIZE))
                    .text_color(t.text_muted),
            )
            .child(
                div()
                    .text_size(px(layout::TEXT_LG - 1.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Settings"),
            );

        // The themes run far past a short window, so they scroll between a
        // header and a Done that stay put, under the one switch they would
        // otherwise bury.
        let body = div()
            .id("settings-body")
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(layout::FORM_SECTION_GAP))
            .child(section(t, "Files", files))
            .child(section(t, "Theme", appearance));

        ui::dialog(t)
            .track_focus(&self.focus)
            .w(px(layout::SETTINGS_WIDTH))
            .mt(px(top))
            .max_h(ui::card_max_height(window, top))
            .gap(px(layout::FORM_SECTION_GAP))
            .child(header)
            .child(body)
            .child(
                div().flex_shrink_0().flex().justify_end().child(
                    ui::accent_button("settings-done", "Done", t, cx).on_click(
                        cx.listener(|this, _, window, cx| this.dismiss_modal(window, cx)),
                    ),
                ),
            )
            .into_any_element()
    }
}

fn section(t: Theme, label: &str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(layout::SPACE_SM))
        .child(ui::section_label(t, label).pl(px(layout::SPACE_XS)))
        .child(body)
}

/// A theme to pick: its preview, its name under it, ringed in the accent
/// while it is the one in use.
fn theme_card(
    id: impl Into<gpui::ElementId>,
    label: &str,
    selected: bool,
    t: Theme,
    preview: impl IntoElement,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(layout::THEME_CARD_WIDTH))
        .p(px(layout::SPACE_XS))
        .flex()
        .flex_col()
        .gap(px(layout::SPACE_XS + 2.))
        .rounded(px(layout::RADIUS_PANEL))
        .border_2()
        .border_color(if selected {
            t.accent.opaque()
        } else {
            t.accent.alpha(0.)
        })
        .cursor_pointer()
        .when(!selected, |card| {
            card.hover(|style| style.bg(t.element_hover))
        })
        .child(
            div()
                .h(px(layout::THEME_PREVIEW_HEIGHT))
                .rounded(px(layout::RADIUS_CONTROL))
                .overflow_hidden()
                .border_1()
                .border_color(t.border_strong)
                .child(preview),
        )
        .child(
            div()
                .px(px(layout::SPACE_XS))
                .flex()
                .items_center()
                .gap(px(layout::SPACE_XS))
                .text_size(px(layout::TEXT_SM))
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if selected { t.text } else { t.text_muted })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(label.to_string()),
                )
                .when(selected, |row| {
                    row.child(
                        icon(icons::DONE)
                            .size(px(layout::ICON_SIZE - 2.))
                            .text_color(t.accent),
                    )
                }),
        )
}

/// filejam in miniature, painted in `p`'s tokens: the sidebar on the chrome
/// plane, a pane header, a few rows with one selected, the accent button.
fn preview(p: Theme) -> impl IntoElement {
    let line =
        |width: f32, color: gpui::Hsla| div().w(px(width)).h(px(3.)).rounded_full().bg(color);
    let row = |selected: bool, width: f32| {
        div()
            .h(px(9.))
            .px(px(4.))
            .flex()
            .items_center()
            .gap(px(3.))
            .when(selected, |row| row.bg(p.selection.flatten(p.bg)))
            .child(div().size(px(5.)).rounded(px(1.)).bg(p.folder))
            .child(line(width, p.text_muted.into()))
    };
    div()
        .size_full()
        .flex()
        .child(
            div()
                .w(px(26.))
                .h_full()
                .bg(p.surface)
                .pt(px(6.))
                .px(px(4.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(line(14., p.text_faint.into()))
                .child(line(10., p.text_faint.into()))
                .child(line(16., p.text_faint.into())),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .bg(p.bg)
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(12.))
                        .px(px(4.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .bg(p.panel)
                        .child(line(20., p.text.into()))
                        .child(div().w(px(12.)).h(px(5.)).rounded_full().bg(p.accent)),
                )
                .child(
                    div()
                        .pt(px(3.))
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(row(false, 28.))
                        .child(row(true, 22.))
                        .child(row(false, 32.))
                        .child(row(false, 18.)),
                ),
        )
}

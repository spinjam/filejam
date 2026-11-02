//! The shared widget vocabulary, lifted from dbdelve's: buttons, chrome, wells
//! and readouts every surface is assembled from, so two panels asking the same
//! kind of question cannot end up looking like two different applications.

use gpui::{
    AnyElement, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    InteractiveElementExt, POPOVER_BLUR,
    button::{Button, ButtonCustomVariant, ButtonVariants},
};

use crate::{
    icons::{icon, icon as icons},
    theme::{SiteColor, Theme, layout},
};

pub const CLIENT_TITLEBAR: bool = !cfg!(target_os = "linux");

/// filejam's own titlebar, drawn where the platform's would be. The drag
/// region is a child covering what is left of the row, because a drag region
/// swallows the clicks a button needs: anything interactive goes in `leading`
/// or `trailing`, outside it.
pub fn titlebar(
    t: Theme,
    tint: Option<SiteColor>,
    leading: Vec<AnyElement>,
    middle: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
) -> impl IntoElement {
    div()
        .h(px(layout::TITLEBAR_HEIGHT))
        .w_full()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(layout::SPACE_SM))
        .border_b_1()
        .border_color(t.border)
        .when_some(tint, |row, color| row.bg(color.band(t)))
        .pl(px(if cfg!(target_os = "macos") {
            layout::TITLEBAR_LEADING_INSET
        } else {
            layout::SPACE_SM
        }))
        .when(!cfg!(target_os = "windows"), |row| {
            row.pr(px(layout::SPACE_SM))
        })
        .children(leading)
        .child(
            div()
                .id("titlebar")
                .when(CLIENT_TITLEBAR, |strip| {
                    strip
                        .window_control_area(gpui::WindowControlArea::Drag)
                        .on_double_click(|_, window, _| window.titlebar_double_click())
                })
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(px(layout::SPACE_SM))
                .children(middle),
        )
        .children(trailing)
        .when(cfg!(target_os = "windows"), |row| {
            row.child(caption_buttons(t))
        })
}

/// Minimise, maximise and close, for Windows, which draws none into a
/// transparent titlebar.
fn caption_buttons(t: Theme) -> impl IntoElement {
    let button = |id: &'static str, path: &'static str, area, hover: gpui::Hsla| {
        div()
            .id(id)
            .w(px(layout::TITLEBAR_HEIGHT * 1.25))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .window_control_area(area)
            .hover(move |style| style.bg(hover))
            .child(icon(path).size(px(layout::ICON_SIZE)).text_color(t.text))
    };
    div()
        .h_full()
        .flex()
        .flex_shrink_0()
        .child(button(
            "minimize",
            icons::MINIMIZE,
            gpui::WindowControlArea::Min,
            t.element_hover.into(),
        ))
        .child(button(
            "maximize",
            icons::MAXIMIZE,
            gpui::WindowControlArea::Max,
            t.element_hover.into(),
        ))
        .child(button(
            "close",
            icons::CLOSE,
            gpui::WindowControlArea::Close,
            t.danger.into(),
        ))
}

/// The card every modal is drawn on. `occlude`, or a click on one of its
/// buttons also lands on the file list beneath.
pub fn dialog(t: Theme) -> Div {
    div()
        .occlude()
        .w(px(layout::DIALOG_WIDTH))
        .p(px(layout::SPACE_LG))
        .flex()
        .flex_col()
        .gap(px(layout::SPACE_MD))
        .border_1()
        .border_color(t.border_strong)
        .rounded(px(layout::RADIUS_PANEL))
        .map(|card| raised_card(card, t))
}

/// A floating card's fill, frosted when it lets the window through.
pub fn frosted_card<E: Styled>(card: E, t: Theme) -> E {
    let fill = t.overlay_glass();
    let card = card.bg(fill);
    if fill.a < 1.0 {
        card.backdrop_blur(POPOVER_BLUR)
    } else {
        card
    }
}

/// [`frosted_card`] with a shadow when it is solid.
pub fn raised_card<E: Styled>(card: E, t: Theme) -> E {
    let card = frosted_card(card, t);
    if t.overlay_glass().a < 1.0 {
        card
    } else {
        card.shadow_lg()
    }
}

/// The tallest a floating card can be with its top `top` below the window's and
/// a margin left under it, so nothing at its foot falls off the window.
pub fn card_max_height(window: &gpui::Window, top: f32) -> gpui::Pixels {
    window.viewport_size().height - px(top + layout::SPACE_LG)
}

/// The dim over the window behind a modal, which also centres it.
pub fn scrim(t: Theme) -> Div {
    div()
        .absolute()
        .inset_0()
        .occlude()
        .bg(t.scrim())
        .flex()
        .justify_center()
        .items_start()
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Primary,
    Quiet,
    Danger,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Control {
    Standard,
    Compact,
    Inline,
}

impl Tone {
    pub fn ink(self, t: Theme) -> crate::theme::color::Srgb {
        match self {
            Tone::Primary => t.text,
            Tone::Quiet => t.text_muted,
            Tone::Danger => t.on_accent,
        }
    }
}

impl Control {
    pub fn height(self) -> f32 {
        match self {
            Control::Standard => layout::CONTROL_HEIGHT,
            Control::Compact => layout::CONTROL_HEIGHT_COMPACT,
            Control::Inline => layout::CONTROL_HEIGHT_INLINE,
        }
    }

    pub fn padding(self) -> f32 {
        (self.height() - self.text_size()) / 2.0
    }

    pub fn text_size(self) -> f32 {
        match self {
            Control::Standard => layout::TEXT_MD,
            Control::Compact | Control::Inline => layout::TEXT_SM,
        }
    }
}

/// A filejam button: gpui-component's mechanism (tooltip, focus ring,
/// disabled gate), measured off filejam's own layout scale.
pub fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    tone: Tone,
    size: Control,
    t: Theme,
) -> Button {
    control(id, tone, size)
        .px(px(size.padding()))
        .when(size == Control::Standard, |standard| {
            standard.min_w(px(layout::CONTROL_MIN_WIDTH))
        })
        .child(button_label(label, tone, size, t))
}

/// A button with an icon before its words.
pub fn icon_label_button(
    id: impl Into<gpui::ElementId>,
    path: &'static str,
    label: impl Into<SharedString>,
    tone: Tone,
    size: Control,
    t: Theme,
) -> Button {
    control(id, tone, size).px(px(size.padding())).child(
        div()
            .flex()
            .items_center()
            .gap(px(layout::SPACE_XS + 2.))
            .child(
                icon(path)
                    .size(px(layout::ICON_SIZE))
                    .text_color(tone.ink(t)),
            )
            .child(button_label(label, tone, size, t)),
    )
}

/// A button that is only its icon, square.
pub fn icon_button(
    id: impl Into<gpui::ElementId>,
    path: &'static str,
    tone: Tone,
    size: Control,
    t: Theme,
) -> Button {
    control(id, tone, size).w(px(size.height())).p_0().child(
        icon(path)
            .size(px(layout::ICON_SIZE))
            .text_color(tone.ink(t)),
    )
}

pub fn button_label(
    label: impl Into<SharedString>,
    tone: Tone,
    size: Control,
    t: Theme,
) -> impl IntoElement {
    div()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .line_height(gpui::relative(1.))
        .text_size(px(size.text_size()))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tone.ink(t))
        .child(label.into())
}

pub fn control(id: impl Into<gpui::ElementId>, tone: Tone, size: Control) -> Button {
    Button::new(id)
        .map(|button| match tone {
            Tone::Primary => button.primary(),
            Tone::Quiet => button.ghost(),
            Tone::Danger => button.danger(),
        })
        .h(px(size.height()))
}

/// The one action a surface exists to take, on the accent.
pub fn accent_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    t: Theme,
    cx: &gpui::App,
) -> Button {
    let size = Control::Standard;
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(t.accent.into())
                .foreground(t.on_accent.into())
                .hover(t.accent.alpha(layout::ACCENT_HOVER).into())
                .active(t.accent.alpha(layout::ACCENT_PRESSED).into()),
        )
        .bg(t.accent)
        .h(px(size.height()))
        .px(px(size.padding()))
        .min_w(px(layout::CONTROL_MIN_WIDTH))
        .child(
            div()
                .whitespace_nowrap()
                .line_height(gpui::relative(1.))
                .text_size(px(size.text_size()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.on_accent)
                .child(label.into()),
        )
}

/// Rows set into the surface they sit on, a hairline between each two.
pub fn well(t: Theme, rows: impl IntoIterator<Item = AnyElement>) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .rounded(px(layout::RADIUS_WELL))
        .border_1()
        .border_color(t.border)
        .bg(t.element_hover);
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(div().mx(px(layout::SPACE_LG)).h(px(1.)).bg(t.border));
        }
        card = card.child(row);
    }
    card
}

/// One labelled row of a form well: the label in a fixed column, the field
/// filling the rest.
pub fn form_row(t: Theme, label: &'static str, field: impl IntoElement) -> AnyElement {
    div()
        .h(px(layout::FORM_ROW_HEIGHT))
        .px(px(layout::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(layout::SPACE_MD))
        .child(
            div()
                .w(px(layout::FORM_LABEL_WIDTH))
                .flex_shrink_0()
                .text_size(px(layout::TEXT_MD))
                .text_color(t.text_muted)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(field))
        .into_any_element()
}

/// A choice in a well: its title, what it does under that, its control beside.
pub fn option_row(
    t: Theme,
    title: &'static str,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> AnyElement {
    div()
        .px(px(layout::SPACE_LG))
        .py(px(layout::SPACE_MD))
        .flex()
        .items_center()
        .gap(px(layout::SPACE_MD))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(layout::TEXT_MD))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.text)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(layout::TEXT_SM))
                        .text_color(t.text_muted)
                        .child(description.into()),
                ),
        )
        .child(control)
        .into_any_element()
}

pub fn switch(id: impl Into<gpui::ElementId>, on: bool, t: Theme) -> Button {
    let width = layout::SWITCH_WIDTH;
    let height = layout::SWITCH_HEIGHT;
    let thumb = layout::SWITCH_THUMB_SIZE;
    let inset = (height - thumb) / 2.;
    control(id, Tone::Quiet, Control::Inline)
        .flex_shrink_0()
        .w(px(width))
        .h(px(height))
        .p_0()
        .rounded_full()
        .child(
            div()
                .relative()
                .size_full()
                .rounded_full()
                .bg(if on {
                    t.accent.opaque()
                } else {
                    t.border_strong
                })
                .child(
                    div()
                        .absolute()
                        .top(px(inset))
                        .left(px(if on { width - thumb - inset } else { inset }))
                        .size(px(thumb))
                        .rounded_full()
                        .bg(if on { t.on_accent } else { t.text_muted }),
                ),
        )
}

/// A site colour as a dot to pick, or None as a struck-out ring.
pub fn swatch(
    id: impl Into<gpui::ElementId>,
    color: Option<SiteColor>,
    selected: bool,
    t: Theme,
) -> Button {
    let size = layout::SWATCH_SIZE + 2. * (layout::SWATCH_RING_GAP + 1.);
    control(id, Tone::Quiet, Control::Inline)
        .w(px(size))
        .h(px(size))
        .p_0()
        .rounded_full()
        .child(
            div()
                .size_full()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(if selected {
                    t.text.opaque()
                } else {
                    t.text.alpha(0.)
                })
                .child(match color {
                    Some(color) => div()
                        .size(px(layout::SWATCH_SIZE))
                        .rounded_full()
                        .bg(color.swatch())
                        .into_any_element(),
                    None => icon(icons::NO_COLOR)
                        .size(px(layout::SWATCH_SIZE))
                        .text_color(t.text_faint)
                        .into_any_element(),
                }),
        )
}

/// Small, uppercase, dim: the names under it are what the eye lands on.
pub fn section_label(t: Theme, label: &str) -> Div {
    div()
        .text_size(px(layout::TEXT_XS))
        .font_weight(FontWeight::MEDIUM)
        .text_color(t.text_faint)
        .child(label.to_uppercase())
}

pub enum Status {
    Idle,
    Connecting,
    Connected,
    Failed,
}

/// A connection's state as one dot: faint idle, muted connecting, green
/// connected, red failed. Never the site's own colour.
pub fn status_dot(t: Theme, status: Status) -> Div {
    let fill = match status {
        Status::Idle => t.text_faint,
        Status::Connecting => t.text_muted,
        Status::Connected => t.success,
        Status::Failed => t.danger,
    };
    div()
        .flex_shrink_0()
        .size(px(layout::SPACE_XS + 2.))
        .rounded_full()
        .bg(fill)
}

/// A thin progress track.
pub fn progress(t: Theme, fraction: f32, color: crate::theme::color::Srgb) -> Div {
    div()
        .h(px(layout::PROGRESS_HEIGHT))
        .w_full()
        .rounded_full()
        .bg(t.element_active)
        .child(
            div()
                .h_full()
                .rounded_full()
                .bg(color)
                .w(gpui::relative(fraction.clamp(0., 1.))),
        )
}

/// Bytes at the precision a person reads them.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// `rwxr-xr-x` from a mode's permission bits.
pub fn mode_string(mode: u32) -> String {
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    bits.iter()
        .map(|(bit, ch)| if mode & bit != 0 { *ch } else { '-' })
        .collect()
}

/// A Unix time as the list shows it: today's files by the clock, older ones
/// by the date.
pub fn short_date(unix: i64, now: i64) -> String {
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let Ok(at) = time::OffsetDateTime::from_unix_timestamp(unix) else {
        return String::new();
    };
    let at = at.to_offset(offset);
    let today = time::OffsetDateTime::from_unix_timestamp(now)
        .map(|now| now.to_offset(offset).date())
        .ok();
    let format = if Some(at.date()) == today {
        time::macros::format_description!("Today, [hour]:[minute]")
    } else {
        time::macros::format_description!("[day] [month repr:short] [year], [hour]:[minute]")
    };
    at.format(&format).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_the_way_a_person_reads_them() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1_500), "1.5 KB");
        assert_eq!(human_bytes(3_200_000_000), "3.2 GB");
    }

    #[test]
    fn modes_spell_out_their_bits() {
        assert_eq!(mode_string(0o755), "rwxr-xr-x");
        assert_eq!(mode_string(0o640), "rw-r-----");
    }
}

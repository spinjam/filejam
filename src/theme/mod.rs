//! filejam's theme, trimmed from dbdelve's.
//!
//! The same two rules: numbers drive layout ([`layout`]) and never mix with
//! colour tokens, and the planes are told apart by tone rather than hairlines —
//! the closer a surface is to the files, the more light it gets.

pub mod color;

use color::{Oklch, Rgba, Srgb, contrast_ratio};
use gpui::{App, Window};

pub mod layout {
    pub const SPACE_XS: f32 = 4.0;
    pub const SPACE_SM: f32 = 8.0;
    pub const SPACE_MD: f32 = 12.0;
    pub const SPACE_LG: f32 = 16.0;

    pub const TEXT_XS: f32 = 11.0;
    pub const TEXT_SM: f32 = 12.0;
    pub const TEXT_MD: f32 = 13.0;
    pub const TEXT_LG: f32 = 16.0;

    pub const ICON_SIZE: f32 = 14.0;
    pub const FILE_ICON_SIZE: f32 = 16.0;

    pub const RADIUS_CONTROL: f32 = 6.0;
    pub const RADIUS_PANEL: f32 = 10.0;
    pub const RADIUS_LARGE: f32 = 16.0;
    pub const RADIUS_WELL: f32 = 12.0;

    pub const CONTROL_HEIGHT: f32 = 32.0;
    pub const CONTROL_HEIGHT_COMPACT: f32 = 24.0;
    pub const CONTROL_HEIGHT_INLINE: f32 = 20.0;
    pub const CONTROL_MIN_WIDTH: f32 = 76.0;

    pub const TITLEBAR_HEIGHT: f32 = CONTROL_HEIGHT_COMPACT + 2.0 * SPACE_SM;
    pub const TITLEBAR_LEADING_INSET: f32 = 78.0;
    pub const STATUS_HEIGHT: f32 = 32.0;
    pub const PANE_HEADER_HEIGHT: f32 = CONTROL_HEIGHT_COMPACT + 2.0 * SPACE_SM;
    pub const COLUMN_HEADER_HEIGHT: f32 = 28.0;
    pub const ROW_HEIGHT: f32 = 26.0;
    pub const SIDEBAR_ROW_HEIGHT: f32 = 28.0;

    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 220.0;
    pub const SIDEBAR_MIN_WIDTH: f32 = 170.0;
    pub const SIDEBAR_MAX_WIDTH: f32 = 400.0;
    pub const TRANSFERS_HEIGHT: f32 = 220.0;

    pub const SIZE_COLUMN_WIDTH: f32 = 84.0;
    pub const DATE_COLUMN_WIDTH: f32 = 140.0;
    pub const MODE_COLUMN_WIDTH: f32 = 96.0;

    pub const DIALOG_WIDTH: f32 = 420.0;
    pub const FORM_WIDTH: f32 = 560.0;
    pub const FORM_TOP: f32 = 44.0;
    pub const FORM_PADDING: f32 = 20.0;
    pub const FORM_LOGO_TILE: f32 = 40.0;
    pub const FORM_LOGO_SIZE: f32 = 20.0;
    pub const FORM_TITLE_SIZE: f32 = 17.0;
    pub const FORM_TITLE_LINE: f32 = 22.0;
    pub const FORM_SECTION_GAP: f32 = 18.0;
    pub const FORM_LABEL_WIDTH: f32 = 120.0;
    pub const FORM_ROW_HEIGHT: f32 = 44.0;
    pub const FORM_PORT_WIDTH: f32 = 84.0;

    pub const SWATCH_SIZE: f32 = 18.0;
    pub const SWATCH_RING_GAP: f32 = 2.0;
    pub const SWITCH_WIDTH: f32 = 32.0;
    pub const SWITCH_HEIGHT: f32 = 18.0;
    pub const SWITCH_THUMB_SIZE: f32 = 14.0;
    pub const PROGRESS_HEIGHT: f32 = 4.0;
    pub const ACCENT_HOVER: f32 = 0.9;
    pub const ACCENT_PRESSED: f32 = 0.8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
}

impl Appearance {
    pub fn of_system(appearance: gpui::WindowAppearance) -> Self {
        match appearance {
            gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Self::Light,
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

impl gpui::Global for Theme {}

pub fn theme(cx: &App) -> &Theme {
    cx.global::<Theme>()
}

pub const CHROME_FONT: &str = "IBM Plex Sans";
pub const MONO_FONT: &str = "Lilex";

impl From<Srgb> for gpui::Hsla {
    fn from(c: Srgb) -> Self {
        c.opaque().into()
    }
}

impl From<Rgba> for gpui::Hsla {
    fn from(c: Rgba) -> Self {
        gpui::Rgba {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
        .into()
    }
}

impl From<Srgb> for gpui::Fill {
    fn from(c: Srgb) -> Self {
        gpui::Hsla::from(c).into()
    }
}

impl From<Rgba> for gpui::Fill {
    fn from(c: Rgba) -> Self {
        gpui::Hsla::from(c).into()
    }
}

const NEUTRAL_HUE: f32 = 260.0;
const NEUTRAL_CHROMA: f32 = 0.004;
const HAIRLINE_DARK: f32 = 0.09;
const HAIRLINE_LIGHT: f32 = 0.12;
const OPACITY_DEFAULT: f32 = 0.72;
const LIGHT_GLASS_OPACITY: f32 = 0.79;
const PANEL_ALPHA: f32 = 0.14;
const DATA_ALPHA: f32 = 0.14;
const OVERLAY_ALPHA_FLOOR: f32 = 0.15;
const POPUP_TEXT_CONTRAST: f32 = 4.5;
const POPUP_MUTED_CONTRAST: f32 = 3.0;
const SCRIM_DARK: f32 = 0.35;
const SCRIM_LIGHT: f32 = 0.19;

fn neutral(lightness: f32) -> Srgb {
    Oklch::new(lightness, NEUTRAL_CHROMA, NEUTRAL_HUE).to_srgb()
}

const WHITE: Srgb = Srgb::new(1.0, 1.0, 1.0);
const BLACK: Srgb = Srgb::new(0.0, 0.0, 0.0);
const TRANSPARENT: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// A site's colour: the titlebar band while it is in front, its dot in the
/// sidebar. Same palette as dbdelve's connection colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteColor {
    Gray,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

const SWATCH_LIGHTNESS: f32 = 0.65;
const BAND_LIGHTNESS_DARK: f32 = 0.35;
const BAND_LIGHTNESS_LIGHT: f32 = 0.88;
const GLASS_BAND_ALPHA: f32 = 0.40;
const BAND_CHROME_GAP: f32 = 0.04;

impl SiteColor {
    pub const ALL: [SiteColor; 7] = [
        Self::Gray,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Gray => "gray",
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.slug() == s)
    }

    pub fn swatch(self) -> Srgb {
        self.at(SWATCH_LIGHTNESS)
    }

    /// The titlebar tint while this site is in the focused pane.
    pub fn band(self, theme: Theme) -> Rgba {
        if theme.is_glass {
            return self.swatch().alpha(GLASS_BAND_ALPHA);
        }
        let (fixed, away) = match theme.appearance {
            Appearance::Dark => (BAND_LIGHTNESS_DARK, 1.0),
            Appearance::Light => (BAND_LIGHTNESS_LIGHT, -1.0),
        };
        let chrome = Oklch::from_srgb(theme.surface).l;
        let lightness = if (fixed - chrome).abs() < BAND_CHROME_GAP {
            chrome + away * BAND_CHROME_GAP
        } else {
            fixed
        };
        self.at(lightness).opaque()
    }

    fn at(self, lightness: f32) -> Srgb {
        let (chroma, hue) = match self {
            Self::Gray => (0.012, 265.0),
            Self::Red => (0.190, 25.0),
            Self::Orange => (0.150, 55.0),
            Self::Yellow => (0.130, 90.0),
            Self::Green => (0.150, 145.0),
            Self::Blue => (0.140, 250.0),
            Self::Purple => (0.170, 305.0),
        };
        Oklch::new(lightness, chroma, hue).to_srgb()
    }
}

/// Every colour filejam paints. Flat fields: a token you have to go looking
/// for gets duplicated instead of reused.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub appearance: Appearance,
    pub is_glass: bool,
    pub opacity: f32,

    /// The content plane, brightest: the file lists.
    pub bg: Srgb,
    /// One step behind: the focused pane's header, the dialogs' fields.
    pub panel: Srgb,
    /// Chrome, furthest back: sidebar, titlebar, status bar.
    pub surface: Srgb,
    /// The floating plane: dialogs and menus.
    pub overlay: Srgb,

    pub element_hover: Rgba,
    pub element_active: Rgba,
    pub control: Srgb,

    pub border: Rgba,
    pub border_strong: Rgba,

    pub text: Srgb,
    pub text_muted: Srgb,
    pub text_faint: Srgb,

    pub accent: Srgb,
    pub on_accent: Srgb,
    pub selection: Rgba,
    pub cursor: Srgb,

    pub danger: Srgb,
    pub success: Srgb,

    /// Folder icons: the one hue a listing always carries.
    pub folder: Srgb,
}

impl Theme {
    pub fn all() -> Vec<Self> {
        vec![
            Self::black(),
            Self::dark(),
            Self::dark().glassed("Filejam Dark Glass"),
            Self::light(),
            Self::light().glassed("Filejam Light Glass"),
        ]
    }

    pub fn frost(self) -> Rgba {
        self.surface.alpha(self.tint(self.opacity))
    }

    pub fn panel_glass(self) -> Rgba {
        self.panel.alpha(self.tint(PANEL_ALPHA))
    }

    pub fn data_glass(self) -> Rgba {
        self.bg.alpha(self.tint(DATA_ALPHA))
    }

    pub fn scrim(self) -> Rgba {
        BLACK.alpha(match self.appearance {
            Appearance::Dark => SCRIM_DARK,
            Appearance::Light => SCRIM_LIGHT,
        })
    }

    /// Every floating card. Opaque themes get their overlay tone; glass themes
    /// the thinnest tint of it that still holds the text — see dbdelve's
    /// `Theme::overlay_glass` for the long version.
    pub fn overlay_glass(self) -> Rgba {
        if !self.is_glass {
            return self.overlay.opaque();
        }
        let desktop = match self.appearance {
            Appearance::Dark => WHITE,
            Appearance::Light => BLACK,
        };
        let backdrop = self.data_glass().flatten(self.frost().flatten(desktop));
        let holds = |alpha: f32| {
            let card = self.overlay.alpha(alpha).flatten(backdrop);
            contrast_ratio(self.text, card) >= POPUP_TEXT_CONTRAST
                && contrast_ratio(self.text_muted, card) >= POPUP_MUTED_CONTRAST
        };
        let alpha = (0..=40)
            .map(|step| OVERLAY_ALPHA_FLOOR + (1.0 - OVERLAY_ALPHA_FLOOR) * step as f32 / 40.0)
            .find(|&alpha| holds(alpha))
            .unwrap_or(1.0);
        self.overlay.alpha(alpha)
    }

    fn tint(self, alpha: f32) -> f32 {
        if self.is_glass { alpha } else { 1.0 }
    }

    pub fn window_background(self) -> gpui::WindowBackgroundAppearance {
        if self.is_glass {
            gpui::WindowBackgroundAppearance::Blurred
        } else {
            gpui::WindowBackgroundAppearance::Opaque
        }
    }

    pub fn apply_to_components(self, cx: &mut App) {
        gpui_component::Theme::update(cx, |component| {
            component.shadow = false;
            component.radius = gpui::px(layout::RADIUS_CONTROL);
            component.radius_lg = gpui::px(layout::RADIUS_LARGE);
            component.font_size = gpui::px(16.0);
            component.mono_font_size = gpui::px(layout::TEXT_MD);
            component.font_family = CHROME_FONT.into();
            component.mono_font_family = MONO_FONT.into();

            component.colors.background = self.frost().into();
            component.colors.foreground = self.text.into();
            component.colors.input = self.border.into();
            component.colors.border = self.border.into();
            component.colors.caret = self.cursor.into();
            component.colors.selection = self.selection.into();
            component.colors.ring = self.accent.into();
            component.colors.blue = self.accent.into();
            component.colors.muted = self.surface.into();
            component.colors.muted_foreground = self.text_muted.into();
            component.colors.popover = self.overlay_glass().into();
            component.colors.popover_foreground = self.text.into();

            let control_hover = self.element_active.flatten(self.control);
            let control_active = self.element_active.flatten(control_hover);
            component.colors.primary = self.control.into();
            component.colors.primary_foreground = self.text.into();
            component.colors.primary_hover = control_hover.into();
            component.colors.primary_active = control_active.into();
            component.colors.button_primary = self.control.into();
            component.colors.button_primary_foreground = self.text.into();
            component.colors.button_primary_hover = control_hover.into();
            component.colors.button_primary_active = control_active.into();
            component.colors.secondary = self.control.into();
            component.colors.secondary_foreground = self.text.into();
            component.colors.secondary_hover = control_hover.into();
            component.colors.secondary_active = control_active.into();
            component.colors.accent = self.element_active.into();
            component.colors.accent_foreground = self.text.into();
            component.colors.danger = self.danger.into();
            component.colors.danger_foreground = self.on_accent.into();
            component.colors.danger_hover = self.element_hover.flatten(self.danger).into();
            component.colors.danger_active = self.element_active.flatten(self.danger).into();
            component.colors.button_danger = component.colors.danger;
            component.colors.button_danger_foreground = component.colors.danger_foreground;
            component.colors.button_danger_hover = component.colors.danger_hover;
            component.colors.button_danger_active = component.colors.danger_active;
            component.colors.success = self.success.into();
            component.colors.success_foreground = self.on_accent.into();
            component.colors.list = self.surface.into();
            component.colors.list_even = self.surface.into();
            component.colors.list_head = self.surface.into();
            component.colors.list_hover = self.element_hover.into();
            component.colors.list_active = self.selection.into();
            component.colors.list_active_border = self.accent.into();
            component.colors.scrollbar = TRANSPARENT.into();
            component.colors.scrollbar_thumb = self.border_strong.into();
            component.colors.scrollbar_thumb_hover = self.element_active.into();
            component.colors.progress_bar = self.accent.into();
        });
    }

    fn glassed(self, name: &'static str) -> Self {
        let tone = |plane: Srgb, lightness: f32| {
            let own = Oklch::from_srgb(plane);
            Oklch::new(lightness, own.c, own.h).to_srgb()
        };
        let (bg, panel, surface, overlay, control, opacity) = match self.appearance {
            Appearance::Dark => (0.275, 0.215, 0.130, 0.165, 0.340, OPACITY_DEFAULT),
            Appearance::Light => (0.985, 0.970, 0.950, 0.985, 0.900, LIGHT_GLASS_OPACITY),
        };
        Self {
            name,
            is_glass: true,
            opacity,
            bg: tone(self.bg, bg),
            panel: tone(self.panel, panel),
            surface: tone(self.surface, surface),
            overlay: tone(self.overlay, overlay),
            control: tone(self.control, control),
            ..self
        }
    }

    pub fn black() -> Self {
        Self {
            name: "Filejam Black",
            bg: neutral(0.155),
            panel: neutral(0.148),
            surface: neutral(0.140),
            overlay: neutral(0.200),
            control: neutral(0.300),
            ..Self::dark()
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Filejam Dark",
            appearance: Appearance::Dark,
            is_glass: false,
            opacity: OPACITY_DEFAULT,

            bg: neutral(0.300),
            panel: neutral(0.260),
            surface: neutral(0.220),
            overlay: neutral(0.350),

            element_hover: WHITE.alpha(0.05),
            element_active: WHITE.alpha(0.09),
            control: neutral(0.380),

            border: WHITE.alpha(HAIRLINE_DARK),
            border_strong: WHITE.alpha(0.16),

            text: neutral(0.93),
            text_muted: neutral(0.76),
            text_faint: neutral(0.62),

            accent: Oklch::new(0.70, 0.15, 250.0).to_srgb(),
            on_accent: neutral(0.14),
            selection: Oklch::new(0.68, 0.15, 250.0).to_srgb().alpha(0.28),
            cursor: Oklch::new(0.72, 0.14, 250.0).to_srgb(),

            danger: Oklch::new(0.70, 0.19, 25.0).to_srgb(),
            success: Oklch::new(0.72, 0.15, 150.0).to_srgb(),
            folder: Oklch::new(0.74, 0.12, 235.0).to_srgb(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Filejam Light",
            appearance: Appearance::Light,
            is_glass: false,
            opacity: OPACITY_DEFAULT,

            bg: WHITE,
            panel: neutral(0.972),
            surface: neutral(0.940),
            overlay: WHITE,

            element_hover: BLACK.alpha(0.04),
            element_active: BLACK.alpha(0.08),
            control: neutral(0.920),

            border: BLACK.alpha(HAIRLINE_LIGHT),
            border_strong: BLACK.alpha(0.20),

            text: neutral(0.26),
            text_muted: neutral(0.45),
            text_faint: neutral(0.58),

            accent: Oklch::new(0.52, 0.17, 250.0).to_srgb(),
            on_accent: WHITE,
            selection: Oklch::new(0.52, 0.17, 250.0).to_srgb().alpha(0.20),
            cursor: Oklch::new(0.48, 0.18, 250.0).to_srgb(),

            danger: Oklch::new(0.52, 0.20, 25.0).to_srgb(),
            success: Oklch::new(0.52, 0.15, 150.0).to_srgb(),
            folder: Oklch::new(0.58, 0.14, 240.0).to_srgb(),
        }
    }
}

pub fn starting_theme(name: Option<&str>, system: Appearance) -> Theme {
    name.and_then(|name| Theme::all().into_iter().find(|theme| theme.name == name))
        .unwrap_or_else(|| match system {
            Appearance::Dark => Theme::black(),
            Appearance::Light => Theme::light(),
        })
}

/// Push a theme everywhere it is read. The vibrant view has to be asked for
/// again on every switch, not once at startup.
pub fn install_theme(theme: Theme, window: &mut Window, cx: &mut App) {
    theme.apply_to_components(cx);
    cx.set_global(theme);
    window.set_background_appearance(theme.window_background());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_holds_its_body_text() {
        for theme in Theme::all() {
            for plane in [theme.bg, theme.panel, theme.surface] {
                assert!(
                    contrast_ratio(theme.text, plane) >= 4.5,
                    "{}: body text fails on a plane",
                    theme.name
                );
                assert!(
                    contrast_ratio(theme.text_muted, plane) >= 3.0,
                    "{}: muted text fails on a plane",
                    theme.name
                );
            }
        }
    }
}

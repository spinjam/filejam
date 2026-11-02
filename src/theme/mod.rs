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
    pub const TAB_BAR_HEIGHT: f32 = CONTROL_HEIGHT_COMPACT + 2.0 * SPACE_XS;
    pub const TAB_MIN_WIDTH: f32 = 84.0;
    pub const TAB_MAX_WIDTH: f32 = 180.0;
    pub const COLUMN_HEADER_HEIGHT: f32 = 28.0;
    pub const ROW_HEIGHT: f32 = 26.0;
    pub const SIDEBAR_ROW_HEIGHT: f32 = 28.0;

    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 220.0;
    pub const SIDEBAR_MIN_WIDTH: f32 = 170.0;
    pub const SIDEBAR_MAX_WIDTH: f32 = 400.0;
    pub const TREE_DEFAULT_WIDTH: f32 = 200.0;
    pub const TREE_MIN_WIDTH: f32 = 120.0;
    pub const TREE_MAX_WIDTH: f32 = 480.0;
    pub const TREE_ROW_HEIGHT: f32 = 24.0;
    pub const TREE_INDENT: f32 = 14.0;
    pub const TREE_CHEVRON_SIZE: f32 = 16.0;
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

    pub const SETTINGS_WIDTH: f32 = 560.0;
    pub const THEME_CARD_WIDTH: f32 = 164.0;
    pub const THEME_PREVIEW_HEIGHT: f32 = 64.0;

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

/// The editor palettes' selection wash.
const PALETTE_SELECTION_ALPHA: f32 = 0.28;

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
    /// Every theme, in the order the picker lists them: every dark one, then
    /// every light one. The families and their palettes are dbdelve's.
    pub fn all() -> Vec<Self> {
        vec![
            Self::black(),
            Self::dark(),
            Self::dark().glassed("Filejam Dark Glass"),
            Self::gruvbox_dark(),
            Self::gruvbox_dark().glassed("Gruvbox Dark Glass"),
            Self::catppuccin_mocha(),
            Self::catppuccin_mocha().glassed("Catppuccin Mocha Glass"),
            Self::github_dark(),
            Self::github_dark().glassed("GitHub Dark Glass"),
            Self::tokyo_night(),
            Self::tokyo_night().glassed("Tokyo Night Glass"),
            Self::dracula(),
            Self::dracula().glassed("Dracula Glass"),
            Self::one_dark(),
            Self::one_dark().glassed("One Dark Glass"),
            Self::light(),
            Self::light().glassed("Filejam Light Glass"),
            Self::gruvbox_light(),
            Self::gruvbox_light().glassed("Gruvbox Light Glass"),
            Self::catppuccin_latte(),
            Self::catppuccin_latte().glassed("Catppuccin Latte Glass"),
            Self::github_light(),
            Self::github_light().glassed("GitHub Light Glass"),
            Self::tokyo_night_day(),
            Self::tokyo_night_day().glassed("Tokyo Night Day Glass"),
            Self::dracula_light(),
            Self::dracula_light().glassed("Dracula Light Glass"),
            Self::one_light(),
            Self::one_light().glassed("One Light Glass"),
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

    // The editor palettes below are their upstream hex. Where an upstream
    // colour misses a contrast floor dbdelve holds them to, it is moved in Oklch
    // lightness only (lifted on a dark theme, darkened on a light one), and
    // the line says from what.

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "Gruvbox Dark",

            bg: Srgb::from_hex(0x32302f),
            panel: Srgb::from_hex(0x282828),
            surface: Srgb::from_hex(0x1d2021),
            overlay: Srgb::from_hex(0x3c3836),
            control: Srgb::from_hex(0x504945),

            text: Srgb::from_hex(0xfbf1c7),
            text_muted: Srgb::from_hex(0xd5c4a1),
            text_faint: Srgb::from_hex(0x928374),

            accent: Srgb::from_hex(0x83a598),
            folder: Srgb::from_hex(0x83a598),
            on_accent: Srgb::from_hex(0x1d2021),
            selection: Srgb::from_hex(0x83a598).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0xebdbb2),

            // Lifted from #fb4934.
            danger: Srgb::from_hex(0xff674f),
            success: Srgb::from_hex(0xb8bb26),

            ..Self::dark()
        }
    }

    pub fn gruvbox_light() -> Self {
        Self {
            name: "Gruvbox Light",

            bg: Srgb::from_hex(0xfbf1c7),
            panel: Srgb::from_hex(0xf2e5bc),
            surface: Srgb::from_hex(0xebdbb2),
            overlay: Srgb::from_hex(0xf9f5d7),
            control: Srgb::from_hex(0xd5c4a1),

            text: Srgb::from_hex(0x282828),
            text_muted: Srgb::from_hex(0x504945),
            text_faint: Srgb::from_hex(0x928374),

            accent: Srgb::from_hex(0x076678),
            folder: Srgb::from_hex(0x076678),
            on_accent: Srgb::from_hex(0xf9f5d7),
            selection: Srgb::from_hex(0x076678).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x3c3836),

            danger: Srgb::from_hex(0x9d0006),
            success: Srgb::from_hex(0x79740e),

            ..Self::light()
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha",

            // Base and crust nudged from #1e1e2e and #11111b: upstream they sit
            // 7 and 8 levels off mantle, under or on the 8-level plane floor.
            bg: Srgb::from_hex(0x1f1f30),
            panel: Srgb::from_hex(0x181825),
            surface: Srgb::from_hex(0x10101a),
            overlay: Srgb::from_hex(0x313244),
            control: Srgb::from_hex(0x45475a),

            text: Srgb::from_hex(0xcdd6f4),
            text_muted: Srgb::from_hex(0xbac2de),
            text_faint: Srgb::from_hex(0x9399b2),

            accent: Srgb::from_hex(0xb4befe),
            folder: Srgb::from_hex(0xb4befe),
            on_accent: Srgb::from_hex(0x11111b),
            selection: Srgb::from_hex(0xb4befe).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0xf5e0dc),

            danger: Srgb::from_hex(0xf38ba8),
            success: Srgb::from_hex(0xa6e3a1),

            ..Self::dark()
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            name: "Catppuccin Latte",

            // Base nudged from #eff1f5, 7.7 levels off mantle.
            bg: Srgb::from_hex(0xf0f2f6),
            panel: Srgb::from_hex(0xe6e9ef),
            surface: Srgb::from_hex(0xdce0e8),
            overlay: Srgb::from_hex(0xf0f2f6),
            control: Srgb::from_hex(0xbcc0cc),

            // Darkened from #4c4f69, too soft as body text.
            text: Srgb::from_hex(0x34364f),
            text_muted: Srgb::from_hex(0x5c5f77),
            text_faint: Srgb::from_hex(0x7c7f93),

            // Darkened from #7287fd.
            accent: Srgb::from_hex(0x6174e8),
            folder: Srgb::from_hex(0x6174e8),
            on_accent: Srgb::from_hex(0xdce0e8),
            selection: Srgb::from_hex(0x7287fd).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0xdc8a78),

            // Darkened from #d20f39 and #40a02b.
            danger: Srgb::from_hex(0xd10d38),
            success: Srgb::from_hex(0x319218),

            ..Self::light()
        }
    }

    pub fn github_dark() -> Self {
        Self {
            name: "GitHub Dark",

            bg: Srgb::from_hex(0x161b22),
            panel: Srgb::from_hex(0x0d1117),
            surface: Srgb::from_hex(0x010409),
            overlay: Srgb::from_hex(0x161b22),
            control: Srgb::from_hex(0x21262d),

            text: Srgb::from_hex(0xe6edf3),
            text_muted: Srgb::from_hex(0x9198a1),
            text_faint: Srgb::from_hex(0x6e7681),

            accent: Srgb::from_hex(0x4493f8),
            folder: Srgb::from_hex(0x4493f8),
            on_accent: WHITE,
            selection: Srgb::from_hex(0x4493f8).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x4493f8),

            // Lifted from #f85149, which falls under 4.5 on the glass chrome.
            danger: Srgb::from_hex(0xfa534b),
            success: Srgb::from_hex(0x3fb950),

            ..Self::dark()
        }
    }

    pub fn github_light() -> Self {
        Self {
            name: "GitHub Light",

            bg: WHITE,
            // Nudged from #f6f8fa, 7 levels off the canvas.
            panel: Srgb::from_hex(0xf4f6f8),
            surface: Srgb::from_hex(0xe6eaef),
            overlay: WHITE,
            control: Srgb::from_hex(0xf6f8fa),

            text: Srgb::from_hex(0x1f2328),
            text_muted: Srgb::from_hex(0x59636e),
            text_faint: Srgb::from_hex(0x818b98),

            accent: Srgb::from_hex(0x0969da),
            folder: Srgb::from_hex(0x0969da),
            on_accent: WHITE,
            selection: Srgb::from_hex(0x0969da).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x0969da),

            danger: Srgb::from_hex(0xd1242f),
            success: Srgb::from_hex(0x1a7f37),

            ..Self::light()
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night",

            bg: Srgb::from_hex(0x1f2335),
            panel: Srgb::from_hex(0x1a1b26),
            surface: Srgb::from_hex(0x0c0e14),
            overlay: Srgb::from_hex(0x292e42),
            control: Srgb::from_hex(0x3b4261),

            // Lifted from #c0caf5, too soft as body text, in glass most of all.
            text: Srgb::from_hex(0xc7d2fd),
            text_muted: Srgb::from_hex(0xa9b1d6),
            text_faint: Srgb::from_hex(0x737aa2),

            accent: Srgb::from_hex(0x7aa2f7),
            folder: Srgb::from_hex(0x7aa2f7),
            on_accent: Srgb::from_hex(0x1a1b26),
            selection: Srgb::from_hex(0x7aa2f7).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0xc0caf5),

            danger: Srgb::from_hex(0xf7768e),
            success: Srgb::from_hex(0x9ece6a),

            ..Self::dark()
        }
    }

    pub fn tokyo_night_day() -> Self {
        Self {
            name: "Tokyo Night Day",

            bg: Srgb::from_hex(0xe1e2e7),
            panel: Srgb::from_hex(0xd0d5e3),
            surface: Srgb::from_hex(0xc1c9df),
            overlay: Srgb::from_hex(0xe1e2e7),
            control: Srgb::from_hex(0xa8aecb),

            // Darkened from #3760bf, far too soft on this page.
            text: Srgb::from_hex(0x031f7b),
            // Darkened from #6172b0.
            text_muted: Srgb::from_hex(0x42518c),
            text_faint: Srgb::from_hex(0x68709a),

            // Darkened from #2e7de9.
            accent: Srgb::from_hex(0x2272dd),
            folder: Srgb::from_hex(0x2272dd),
            on_accent: Srgb::from_hex(0xe1e2e7),
            selection: Srgb::from_hex(0x2e7de9).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x3760bf),

            // Darkened from #f52a65.
            danger: Srgb::from_hex(0xbb003a),
            success: Srgb::from_hex(0x587539),

            ..Self::light()
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula",

            bg: Srgb::from_hex(0x343746),
            panel: Srgb::from_hex(0x282a36),
            // Darkened from #21222c.
            surface: Srgb::from_hex(0x1f202a),
            // Not current-line #44475a: the purple accent misses 3:1 over it.
            overlay: Srgb::from_hex(0x343746),
            control: Srgb::from_hex(0x44475a),

            // Dracula has no secondary text, so the two lower tiers are its
            // comment (#6272a4) lifted until each clears its floor.
            text: Srgb::from_hex(0xf8f8f2),
            text_muted: Srgb::from_hex(0x8ea0d5),
            text_faint: Srgb::from_hex(0x7f91c5),

            accent: Srgb::from_hex(0xbd93f9),
            folder: Srgb::from_hex(0xbd93f9),
            on_accent: Srgb::from_hex(0x282a36),
            selection: Srgb::from_hex(0xbd93f9).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0xf8f8f2),

            // Lifted from #ff5555.
            danger: Srgb::from_hex(0xff7773),
            success: Srgb::from_hex(0x50fa7b),

            ..Self::dark()
        }
    }

    /// Alucard, Dracula's official light variant.
    pub fn dracula_light() -> Self {
        Self {
            name: "Dracula Light",

            bg: Srgb::from_hex(0xfffbeb),
            panel: Srgb::from_hex(0xece9df),
            surface: Srgb::from_hex(0xdedccf),
            overlay: Srgb::from_hex(0xfffbeb),
            control: Srgb::from_hex(0xcfcfde),

            // Alucard has no secondary text either: both lower tiers are its
            // comment, the muted one darkened from #6c664b.
            text: Srgb::from_hex(0x1f1f1f),
            text_muted: Srgb::from_hex(0x676147),
            text_faint: Srgb::from_hex(0x6c664b),

            accent: Srgb::from_hex(0x644ac9),
            folder: Srgb::from_hex(0x644ac9),
            on_accent: Srgb::from_hex(0xfffbeb),
            selection: Srgb::from_hex(0x644ac9).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x1f1f1f),

            // Darkened from #cb3a2a.
            danger: Srgb::from_hex(0xc33223),
            success: Srgb::from_hex(0x14710a),

            ..Self::light()
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "One Dark",

            bg: Srgb::from_hex(0x282c34),
            // Nudged from #21252b, 7.7 levels off the editor's page.
            panel: Srgb::from_hex(0x20242a),
            surface: Srgb::from_hex(0x181a1f),
            overlay: Srgb::from_hex(0x2c313a),
            control: Srgb::from_hex(0x3e4451),

            // One Dark's highlighted UI text, not its #abb2bf foreground, which
            // is too soft as body text and stays the muted tier.
            text: Srgb::from_hex(0xd7dae0),
            text_muted: Srgb::from_hex(0xabb2bf),
            text_faint: Srgb::from_hex(0x7f848e),

            accent: Srgb::from_hex(0x61afef),
            folder: Srgb::from_hex(0x61afef),
            on_accent: Srgb::from_hex(0x282c34),
            selection: Srgb::from_hex(0x61afef).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x528bff),

            // Lifted from #e06c75.
            danger: Srgb::from_hex(0xe46f78),
            success: Srgb::from_hex(0x98c379),

            ..Self::dark()
        }
    }

    pub fn one_light() -> Self {
        Self {
            name: "One Light",

            bg: Srgb::from_hex(0xfafafa),
            panel: Srgb::from_hex(0xeaeaeb),
            surface: Srgb::from_hex(0xdbdbdc),
            overlay: Srgb::from_hex(0xfafafa),
            control: Srgb::from_hex(0xe5e5e6),

            // Darkened from #383a42, #696c77 and #a0a1a7.
            text: Srgb::from_hex(0x33353d),
            text_muted: Srgb::from_hex(0x5d606b),
            text_faint: Srgb::from_hex(0x909197),

            accent: Srgb::from_hex(0x4078f2),
            folder: Srgb::from_hex(0x4078f2),
            on_accent: Srgb::from_hex(0xfafafa),
            selection: Srgb::from_hex(0x4078f2).alpha(PALETTE_SELECTION_ALPHA),
            cursor: Srgb::from_hex(0x526fff),

            // Darkened from #e45649 and #50a14f.
            danger: Srgb::from_hex(0xc2352c),
            success: Srgb::from_hex(0x3b8c3b),

            ..Self::light()
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

    fn check(theme: Theme, what: &str, fg: Srgb, bg: Srgb, minimum: f32) {
        let ratio = contrast_ratio(fg, bg);
        assert!(
            ratio >= minimum,
            "{}: {what} is {ratio:.2}:1, under {minimum}",
            theme.name
        );
    }

    #[test]
    fn every_theme_holds_its_text() {
        for t in Theme::all() {
            for plane in [t.bg, t.panel, t.surface] {
                check(t, "body text", t.text, plane, 4.5);
                check(t, "muted text", t.text_muted, plane, 3.0);
            }
            check(t, "faint text", t.text_faint, t.bg, 3.0);
            check(t, "text on a button", t.text, t.control, 4.5);
            check(t, "text on a card", t.text, t.overlay, 4.5);
            check(t, "accent", t.accent, t.bg, 3.0);
            check(t, "ink on the accent", t.on_accent, t.accent, 3.0);
            check(t, "danger", t.danger, t.bg, 4.5);
            check(t, "ink on danger", t.on_accent, t.danger, 3.0);
            check(t, "success", t.success, t.surface, 3.0);
            check(t, "a folder", t.folder, t.bg, 3.0);
            check(
                t,
                "a selected folder",
                t.folder,
                t.selection.flatten(t.bg),
                2.0,
            );
        }
    }

    #[test]
    fn a_coloured_titlebar_keeps_its_text_in_every_theme() {
        for t in Theme::all() {
            for color in SiteColor::ALL {
                let band = color.band(t).flatten(t.surface);
                check(t, "the titlebar's text", t.text, band, 4.5);
                check(t, "the titlebar's muted text", t.text_muted, band, 3.0);
            }
        }
    }

    #[test]
    fn every_theme_can_be_found_again_by_its_name() {
        for t in Theme::all() {
            assert_eq!(starting_theme(Some(t.name), Appearance::Dark).name, t.name);
        }
    }

    #[test]
    fn every_dark_theme_is_listed_before_every_light_one() {
        let all = Theme::all();
        let first_light = all
            .iter()
            .position(|t| t.appearance == Appearance::Light)
            .unwrap();
        assert!(
            all[first_light..]
                .iter()
                .all(|t| t.appearance == Appearance::Light)
        );
    }
}

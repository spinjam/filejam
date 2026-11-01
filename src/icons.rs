//! filejam's icon set: Lucide as Rust data (`icondata_lu`), served from
//! memory at the paths GPUI asks for, the way dbdelve does it. GPUI paints an
//! SVG as a mask tinted by the element's text colour, so an icon is whatever
//! colour the theme token beside it is.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_component::Icon;
use icondata_core::IconData;

const ICONS: &[(&str, &IconData)] = &[
    // Asked for by gpui-component's own widgets.
    ("icons/chevron-down.svg", icondata_lu::LuChevronDown),
    ("icons/chevron-right.svg", icondata_lu::LuChevronRight),
    ("icons/chevron-left.svg", icondata_lu::LuChevronLeft),
    ("icons/chevron-up.svg", icondata_lu::LuChevronUp),
    ("icons/chevrons-up-down.svg", icondata_lu::LuChevronsUpDown),
    ("icons/close.svg", icondata_lu::LuX),
    ("icons/check.svg", icondata_lu::LuCheck),
    ("icons/minus.svg", icondata_lu::LuMinus),
    ("icons/square.svg", icondata_lu::LuSquare),
    ("icons/search.svg", icondata_lu::LuSearch),
    ("icons/eye.svg", icondata_lu::LuEye),
    ("icons/eye-off.svg", icondata_lu::LuEyeOff),
    ("icons/loader-circle.svg", icondata_lu::LuLoaderCircle),
    ("icons/ellipsis.svg", icondata_lu::LuEllipsis),
    ("icons/arrow-up.svg", icondata_lu::LuArrowUp),
    ("icons/arrow-down.svg", icondata_lu::LuArrowDown),
    // filejam's own.
    ("icons/arrow-left.svg", icondata_lu::LuArrowLeft),
    ("icons/arrow-right.svg", icondata_lu::LuArrowRight),
    ("icons/arrow-right-left.svg", icondata_lu::LuArrowRightLeft),
    ("icons/sort-ascending.svg", icondata_lu::LuArrowUpNarrowWide),
    (
        "icons/sort-descending.svg",
        icondata_lu::LuArrowDownWideNarrow,
    ),
    ("icons/file.svg", icondata_lu::LuFile),
    ("icons/file-text.svg", icondata_lu::LuFileText),
    ("icons/file-image.svg", icondata_lu::LuFileImage),
    ("icons/file-archive.svg", icondata_lu::LuFileArchive),
    ("icons/file-code.svg", icondata_lu::LuFileCode),
    ("icons/file-audio.svg", icondata_lu::LuFileAudio),
    ("icons/file-video.svg", icondata_lu::LuFileVideo),
    ("icons/file-json.svg", icondata_lu::LuFileJson),
    ("icons/file-terminal.svg", icondata_lu::LuFileTerminal),
    ("icons/file-spreadsheet.svg", icondata_lu::LuFileSpreadsheet),
    ("icons/file-symlink.svg", icondata_lu::LuFileSymlink),
    ("icons/folder.svg", icondata_lu::LuFolder),
    ("icons/folder-open.svg", icondata_lu::LuFolderOpen),
    ("icons/folder-plus.svg", icondata_lu::LuFolderPlus),
    ("icons/server.svg", icondata_lu::LuServer),
    ("icons/house.svg", icondata_lu::LuHouse),
    ("icons/monitor.svg", icondata_lu::LuMonitor),
    ("icons/laptop.svg", icondata_lu::LuLaptop),
    ("icons/hard-drive.svg", icondata_lu::LuHardDrive),
    ("icons/download.svg", icondata_lu::LuDownload),
    ("icons/upload.svg", icondata_lu::LuUpload),
    ("icons/refresh-cw.svg", icondata_lu::LuRefreshCw),
    ("icons/trash.svg", icondata_lu::LuTrash2),
    ("icons/pencil.svg", icondata_lu::LuPencil),
    ("icons/copy.svg", icondata_lu::LuCopy),
    ("icons/plus.svg", icondata_lu::LuPlus),
    ("icons/panel-left.svg", icondata_lu::LuPanelLeft),
    ("icons/panel-bottom.svg", icondata_lu::LuPanelBottom),
    ("icons/plug.svg", icondata_lu::LuPlug),
    ("icons/unplug.svg", icondata_lu::LuUnplug),
    ("icons/key.svg", icondata_lu::LuKey),
    ("icons/settings.svg", icondata_lu::LuSettings),
    ("icons/circle-check.svg", icondata_lu::LuCircleCheck),
    ("icons/circle-alert.svg", icondata_lu::LuCircleAlert),
    ("icons/sun-moon.svg", icondata_lu::LuSunMoon),
    ("icons/ban.svg", icondata_lu::LuBan),
    ("icons/shield-check.svg", icondata_lu::LuShieldCheck),
    ("icons/globe.svg", icondata_lu::LuGlobe),
];

/// filejam's own names for what it draws, so a call site names a thing
/// rather than a file.
pub mod icon {
    pub const CHEVRON_RIGHT: &str = "icons/chevron-right.svg";
    pub const CLOSE: &str = "icons/close.svg";
    pub const MINIMIZE: &str = "icons/minus.svg";
    pub const MAXIMIZE: &str = "icons/square.svg";
    pub const SEARCH: &str = "icons/search.svg";
    pub const BACK: &str = "icons/arrow-left.svg";
    pub const FORWARD: &str = "icons/arrow-right.svg";
    pub const UP: &str = "icons/arrow-up.svg";
    pub const SORT_UP: &str = "icons/sort-ascending.svg";
    pub const SORT_DOWN: &str = "icons/sort-descending.svg";
    pub const FILE: &str = "icons/file.svg";
    pub const FILE_TEXT: &str = "icons/file-text.svg";
    pub const FILE_IMAGE: &str = "icons/file-image.svg";
    pub const FILE_ARCHIVE: &str = "icons/file-archive.svg";
    pub const FILE_CODE: &str = "icons/file-code.svg";
    pub const FILE_AUDIO: &str = "icons/file-audio.svg";
    pub const FILE_VIDEO: &str = "icons/file-video.svg";
    pub const FILE_JSON: &str = "icons/file-json.svg";
    pub const FILE_TERMINAL: &str = "icons/file-terminal.svg";
    pub const FILE_SHEET: &str = "icons/file-spreadsheet.svg";
    pub const SYMLINK: &str = "icons/file-symlink.svg";
    pub const FOLDER: &str = "icons/folder.svg";
    pub const NEW_FOLDER: &str = "icons/folder-plus.svg";
    pub const SERVER: &str = "icons/server.svg";
    pub const HOME: &str = "icons/house.svg";
    pub const DESKTOP: &str = "icons/monitor.svg";
    pub const COMPUTER: &str = "icons/laptop.svg";
    pub const DISK: &str = "icons/hard-drive.svg";
    pub const DOWNLOAD: &str = "icons/download.svg";
    pub const UPLOAD: &str = "icons/upload.svg";
    pub const REFRESH: &str = "icons/refresh-cw.svg";
    pub const DELETE: &str = "icons/trash.svg";
    pub const RENAME: &str = "icons/pencil.svg";
    pub const COPY: &str = "icons/copy.svg";
    pub const PLUS: &str = "icons/plus.svg";
    pub const SIDEBAR: &str = "icons/panel-left.svg";
    pub const TRANSFERS: &str = "icons/panel-bottom.svg";
    pub const DISCONNECT: &str = "icons/unplug.svg";
    pub const DONE: &str = "icons/circle-check.svg";
    pub const FAILED: &str = "icons/circle-alert.svg";
    pub const THEME: &str = "icons/sun-moon.svg";
    pub const NO_COLOR: &str = "icons/ban.svg";
    pub const TRUST: &str = "icons/shield-check.svg";
    pub const HIDDEN: &str = "icons/eye-off.svg";
    pub const SHOWN: &str = "icons/eye.svg";
}

pub fn icon(path: &'static str) -> Icon {
    Icon::empty().path(path)
}

pub struct Icons;

impl AssetSource for Icons {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, data)| Cow::Owned(document(data).into_bytes())))
    }

    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS.iter().map(|(name, _)| (*name).into()).collect())
    }
}

/// Wrap Lucide's path data in the SVG document GPUI's renderer expects.
fn document(icon: &IconData) -> String {
    let attribute = |name: &str, value: Option<&str>| {
        value
            .map(|value| format!(r#" {name}="{value}""#))
            .unwrap_or_default()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"{}{}{}{}{}{}{}{}>{}</svg>"#,
        attribute("viewBox", icon.view_box),
        attribute("width", icon.width),
        attribute("height", icon.height),
        attribute("fill", icon.fill),
        attribute("stroke", icon.stroke),
        attribute("stroke-width", icon.stroke_width),
        attribute("stroke-linecap", icon.stroke_linecap),
        attribute("stroke-linejoin", icon.stroke_linejoin),
        icon.data
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_icon_resolves_to_a_document() {
        use icon::*;
        for path in [
            CHEVRON_RIGHT,
            CLOSE,
            MINIMIZE,
            MAXIMIZE,
            SEARCH,
            BACK,
            FORWARD,
            UP,
            SORT_UP,
            SORT_DOWN,
            FILE,
            FILE_TEXT,
            FILE_IMAGE,
            FILE_ARCHIVE,
            FILE_CODE,
            FILE_AUDIO,
            FILE_VIDEO,
            FILE_JSON,
            FILE_TERMINAL,
            FILE_SHEET,
            SYMLINK,
            FOLDER,
            NEW_FOLDER,
            SERVER,
            HOME,
            DESKTOP,
            COMPUTER,
            DISK,
            DOWNLOAD,
            UPLOAD,
            REFRESH,
            DELETE,
            RENAME,
            COPY,
            PLUS,
            SIDEBAR,
            TRANSFERS,
            DISCONNECT,
            DONE,
            FAILED,
            THEME,
            NO_COLOR,
            TRUST,
            HIDDEN,
            SHOWN,
        ] {
            assert!(Icons.load(path).unwrap().is_some(), "{path} draws nothing");
        }
    }
}

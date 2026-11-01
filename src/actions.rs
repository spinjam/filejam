//! Every action filejam dispatches, and the keys bound to them. The menu bar
//! names the same actions, so the menus are where a keystroke is discovered.

use gpui::{KeyBinding, actions};

actions!(
    filejam,
    [
        Quit,
        NewSite,
        ToggleSidebar,
        ToggleTransfers,
        ToggleHidden,
        CycleTheme,
        OpenSettings,
        Refresh,
        GoUp,
        GoBack,
        GoForward,
        GoHome,
        GoToPath,
        Open,
        SelectPrevious,
        SelectNext,
        ExtendPrevious,
        ExtendNext,
        SelectFirst,
        SelectLast,
        SelectAll,
        SwitchPane,
        CopyToOther,
        NewFolder,
        Rename,
        Delete,
        Dismiss,
        Confirm,
        Disconnect,
    ]
);

/// The key context of a focused file list.
pub const FILE_LIST: &str = "FileList";
/// The key context of an open dialog.
pub const MODAL: &str = "Modal";

pub fn bindings() -> Vec<KeyBinding> {
    let list = Some(FILE_LIST);
    let modal = Some(MODAL);
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-n", NewSite, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-alt-s", ToggleSidebar, None),
        KeyBinding::new("cmd-shift-t", ToggleTransfers, None),
        KeyBinding::new("cmd-shift-.", ToggleHidden, None),
        KeyBinding::new("cmd-r", Refresh, None),
        KeyBinding::new("cmd-up", GoUp, list),
        KeyBinding::new("backspace", GoUp, list),
        KeyBinding::new("cmd-[", GoBack, None),
        KeyBinding::new("cmd-]", GoForward, None),
        KeyBinding::new("cmd-shift-h", GoHome, None),
        KeyBinding::new("cmd-l", GoToPath, None),
        KeyBinding::new("enter", Open, list),
        KeyBinding::new("cmd-down", Open, list),
        KeyBinding::new("up", SelectPrevious, list),
        KeyBinding::new("down", SelectNext, list),
        KeyBinding::new("shift-up", ExtendPrevious, list),
        KeyBinding::new("shift-down", ExtendNext, list),
        KeyBinding::new("home", SelectFirst, list),
        KeyBinding::new("end", SelectLast, list),
        KeyBinding::new("cmd-a", SelectAll, list),
        KeyBinding::new("tab", SwitchPane, list),
        KeyBinding::new("f5", CopyToOther, list),
        KeyBinding::new("cmd-shift-c", CopyToOther, list),
        KeyBinding::new("f7", NewFolder, list),
        KeyBinding::new("cmd-shift-n", NewFolder, None),
        KeyBinding::new("f2", Rename, list),
        KeyBinding::new("f8", Delete, list),
        KeyBinding::new("cmd-backspace", Delete, list),
        KeyBinding::new("escape", Dismiss, modal),
        KeyBinding::new("cmd-enter", Confirm, modal),
        KeyBinding::new("cmd-shift-d", Disconnect, None),
    ]
}

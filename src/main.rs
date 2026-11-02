// Release builds are a GUI app; debug keeps the console for `cargo run` output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
#[cfg(target_os = "macos")]
mod forklift;
mod fs;
mod icons;
mod remote;
mod store;
mod theme;
mod transfer;
mod ui;
mod workspace;

use std::borrow::Cow;

use gpui::{
    App, AppContext, Menu, MenuItem, TitlebarOptions, WindowDecorations, WindowOptions, point, px,
    size,
};
use gpui_component::Root;

use actions::*;
use icons::Icons;
use theme::layout;
use workspace::Workspace;

/// The platform's window buttons, which filejam positions but does not draw.
const TRAFFIC_LIGHT_DIAMETER: f32 = 14.0;

fn main() {
    gpui_platform::application()
        .with_assets(Icons)
        .run(|cx: &mut App| {
            cx.text_system()
                .add_fonts(
                    guic_gpui_assets::BUNDLED_FONTS
                        .iter()
                        .map(|font| Cow::Borrowed(*font))
                        .collect(),
                )
                .expect("bundled fonts must be loadable");
            gpui_component::init(cx);

            let stored = store::load();
            let theme = theme::starting_theme(
                stored.settings.theme.as_deref(),
                theme::Appearance::of_system(cx.window_appearance()),
            );
            theme.apply_to_components(cx);
            cx.set_global(theme);

            cx.bind_keys(actions::bindings());
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
            cx.set_menus(menus());

            let options = WindowOptions {
                window_background: theme.window_background(),
                window_bounds: Some(gpui::WindowBounds::centered(size(px(1280.), px(800.)), cx)),
                window_min_size: Some(size(px(760.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("filejam".into()),
                    appears_transparent: ui::CLIENT_TITLEBAR,
                    traffic_light_position: cfg!(target_os = "macos").then(|| {
                        point(
                            px(layout::SPACE_MD),
                            px((layout::TITLEBAR_HEIGHT - TRAFFIC_LIGHT_DIAMETER) / 2.),
                        )
                    }),
                }),
                window_decorations: Some(WindowDecorations::Server),
                app_id: Some("filejam".into()),
                ..Default::default()
            };

            // Root must be the window's first layer, or the inputs' popups
            // panic looking for it.
            cx.open_window(options, |window, cx| {
                let workspace = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| Root::new(workspace, window, cx))
            })
            .expect("failed to open window");

            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}

fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "filejam".into(),
            disabled: false,
            items: vec![
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Quit filejam", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            disabled: false,
            items: vec![
                MenuItem::action("New Site…", NewSite),
                MenuItem::action("New Folder", NewFolder),
                MenuItem::action("Add to Favorites", AddToFavorites),
                MenuItem::separator(),
                MenuItem::action("Copy to Other Pane", CopyToOther),
                MenuItem::action("Rename", Rename),
                MenuItem::action("Delete", Delete),
                MenuItem::separator(),
                MenuItem::action("Disconnect", Disconnect),
                #[cfg(target_os = "macos")]
                MenuItem::separator(),
                #[cfg(target_os = "macos")]
                MenuItem::action("Import from ForkLift…", ImportForklift),
            ],
        },
        Menu {
            name: "View".into(),
            disabled: false,
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle Transfers", ToggleTransfers),
                MenuItem::action("Toggle Folder Tree", ToggleTree),
                MenuItem::action("Show Hidden Files", ToggleHidden),
                MenuItem::separator(),
                MenuItem::action("Refresh", Refresh),
                MenuItem::action("Next Theme", CycleTheme),
            ],
        },
        Menu {
            name: "Go".into(),
            disabled: false,
            items: vec![
                MenuItem::action("Back", GoBack),
                MenuItem::action("Forward", GoForward),
                MenuItem::action("Enclosing Folder", GoUp),
                MenuItem::action("Home", GoHome),
                MenuItem::action("Go to Folder…", GoToPath),
                MenuItem::separator(),
                MenuItem::action("Switch Pane", SwitchPane),
            ],
        },
    ]
}

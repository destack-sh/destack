use crate::menu::{Menu, MenuName, Place, QUIT_MENU_ID, START_MENU_ID};
use tauri::image::Image;
use tauri::menu::PredefinedMenuItem;
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager};

/// The tray's template glyph: black with alpha, 22 points at twice the scale.
const ICON: &[u8] = include_bytes!("../../icons/tray.png");

/// The tray icon's identity.
const TRAY_ID: &str = "desktop";

/// The tray icon, which opens the desktop while it shows no window.
pub struct Tray(TrayIcon);

impl Tray {
    /// Show the tray icon with its menu.
    pub fn open(app: &AppHandle) -> tauri::Result<Self> {
        // offer a new window of the start link and quitting below the host's items, titled once the host runs
        let start =
            tauri::menu::MenuItem::with_id(app, START_MENU_ID, "New Window", true, None::<&str>)?;
        let separator = PredefinedMenuItem::separator(app)?;
        let quit = tauri::menu::MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
        let menu = tauri::menu::Menu::with_items(app, &[&start, &separator, &quit])?;
        app.state::<Menu>().add_titled(start)?;
        app.state::<Menu>().add_titled(quit)?;
        app.state::<Menu>()
            .add(MenuName::Tray, Place::Tray(menu.clone()), 0)?;

        // show the glyph as a template the system tints
        let icon = TrayIconBuilder::with_id(TRAY_ID)
            .icon(Image::from_bytes(ICON)?)
            .icon_as_template(true)
            .tooltip(app.package_info().name.clone())
            .menu(&menu)
            .build(app)?;

        Ok(Self(icon))
    }

    /// Show the desktop's status as the text the tray icon shows on hover.
    pub fn set_tooltip(&self, tooltip: &str) -> tauri::Result<()> {
        self.0.set_tooltip(Some(tooltip))
    }
}

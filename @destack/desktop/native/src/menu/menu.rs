use crate::host::lock_error;
use crate::menu::accelerator;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, IsMenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};

/// The menu item opening a new window of the start link.
pub const START_MENU_ID: &str = "start.open";

/// The menu item quitting the desktop, which otherwise runs on without windows.
pub const QUIT_MENU_ID: &str = "application.quit";

/// A native menu the host fills.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MenuName {
    /// The tray's menu.
    Tray,
    /// The application menu's first submenu.
    Application,
}

/// An item the host places in a native menu: a label, an action, a toggle or a separator.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MenuItem {
    /// A line the person reads but cannot choose.
    Label {
        /// The item's identity.
        id: String,
        /// The text.
        title: String,
    },
    /// An item the person chooses to run an action.
    Action {
        /// The item's identity, which a choice reports back.
        id: String,
        /// The label.
        title: String,
        /// The key combination choosing it, such as `mod+,`.
        shortcut: Option<String>,
    },
    /// An item with a check mark the person toggles.
    Toggle {
        /// The item's identity, which a choice reports back.
        id: String,
        /// The label.
        title: String,
        /// Whether the item shows its check mark.
        is_checked: bool,
    },
    /// A line between groups of items.
    Separator {},
}

/// A menu showing the host's items from a position.
pub enum Place {
    /// The tray's menu.
    Tray(tauri::menu::Menu<Wry>),
    /// The application menu's first submenu.
    Application(Submenu<Wry>),
}

/// A place showing the host's items: its menu, where they start and the items shown now.
struct Placement {
    /// The menu.
    place: Place,
    /// The position of the first item.
    position: usize,
    /// The native items showing the host's items, with their trailing separator.
    shown: Vec<MenuItemKind<Wry>>,
    /// The identities of the host's items shown, which choices report back.
    ids: Vec<String>,
}

/// The places showing the host's items, and the shell's own items the host titles.
#[derive(Default)]
pub struct Menu {
    /// The places showing the host's items, by menu.
    places: Mutex<HashMap<MenuName, Placement>>,
    /// The shell's own items, there before and without the host, which it titles in its language.
    titled: Mutex<Vec<tauri::menu::MenuItem<Wry>>>,
}

impl Menu {
    /// Add one of the shell's own items, which the host titles once it runs.
    pub fn add_titled(&self, item: tauri::menu::MenuItem<Wry>) -> tauri::Result<()> {
        self.titled
            .lock()
            .map_err(|_| tauri::Error::Io(lock_error()))?
            .push(item);

        Ok(())
    }

    /// Title the shell's own items in the host's language: a new window and quitting.
    pub fn title(&self, start: &str, quit: &str) -> tauri::Result<()> {
        let titled = self.titled.lock().map_err(|_| tauri::Error::Io(lock_error()))?;
        for item in titled.iter() {
            // title each by its identity
            if item.id() == START_MENU_ID {
                item.set_text(start)?;
            } else if item.id() == QUIT_MENU_ID {
                item.set_text(quit)?;
            }
        }

        Ok(())
    }

    /// Show a menu's host items in a place from a position.
    pub fn add(&self, menu: MenuName, place: Place, position: usize) -> tauri::Result<()> {
        let placement = Placement {
            place,
            position,
            shown: Vec::new(),
            ids: Vec::new(),
        };
        self.places
            .lock()
            .map_err(|_| tauri::Error::Io(lock_error()))?
            .insert(menu, placement);

        Ok(())
    }

    /// Replace a menu's host items, followed by a separator while there are any.
    pub fn set(&self, app: &AppHandle, menu: MenuName, items: Vec<MenuItem>) -> tauri::Result<()> {
        // find the menu's place, which a platform without it leaves out
        let mut places = self.places.lock().map_err(|_| tauri::Error::Io(lock_error()))?;
        let Some(placement) = places.get_mut(&menu) else {
            return Ok(());
        };

        // remove the items shown before
        for item in placement.shown.drain(..) {
            placement.place.remove(&item)?;
        }

        // show the new items and their separator
        let mut created = items
            .iter()
            .map(|item| item.native(app))
            .collect::<tauri::Result<Vec<_>>>()?;
        if !items.is_empty() {
            created.push(MenuItemKind::Predefined(PredefinedMenuItem::separator(
                app,
            )?));
        }
        for (offset, item) in created.iter().enumerate() {
            placement.place.insert(item, placement.position + offset)?;
        }
        placement.shown = created;
        placement.ids = items.into_iter().filter_map(MenuItem::into_id).collect();

        Ok(())
    }

    /// Report whether an item is one of the host's, failing on a poisoned lock.
    pub fn contains(&self, id: &str) -> io::Result<bool> {
        let places = self.places.lock().map_err(|_| lock_error())?;

        Ok(places
            .values()
            .flat_map(|placement| &placement.ids)
            .any(|shown| shown == id))
    }
}

impl MenuItem {
    /// Build the native item showing the host item.
    fn native(&self, app: &AppHandle) -> tauri::Result<MenuItemKind<Wry>> {
        Ok(match self {
            // a line nobody chooses
            Self::Label { id, title } => MenuItemKind::MenuItem(
                tauri::menu::MenuItem::with_id(app, id, title, false, None::<&str>)?,
            ),
            // an action, with its key combination in the native syntax
            Self::Action {
                id,
                title,
                shortcut: combination,
            } => {
                let combination = combination.as_deref().map(accelerator);
                MenuItemKind::MenuItem(tauri::menu::MenuItem::with_id(
                    app,
                    id,
                    title,
                    true,
                    combination.as_deref(),
                )?)
            }
            // a toggle with its check mark
            Self::Toggle {
                id,
                title,
                is_checked,
            } => MenuItemKind::Check(CheckMenuItem::with_id(
                app,
                id,
                title,
                true,
                *is_checked,
                None::<&str>,
            )?),
            // a line between groups
            Self::Separator {} => MenuItemKind::Predefined(PredefinedMenuItem::separator(app)?),
        })
    }

    /// Take the item's identity, none for a separator.
    fn into_id(self) -> Option<String> {
        match self {
            Self::Label { id, .. } | Self::Action { id, .. } | Self::Toggle { id, .. } => Some(id),
            Self::Separator {} => None,
        }
    }
}

impl Place {
    /// Insert an item at a position.
    fn insert(&self, item: &dyn IsMenuItem<Wry>, position: usize) -> tauri::Result<()> {
        match self {
            Self::Tray(menu) => menu.insert(item, position),
            Self::Application(submenu) => submenu.insert(item, position),
        }
    }

    /// Remove an item.
    fn remove(&self, item: &dyn IsMenuItem<Wry>) -> tauri::Result<()> {
        match self {
            Self::Tray(menu) => menu.remove(item),
            Self::Application(submenu) => submenu.remove(item),
        }
    }
}

impl Menu {
    /// Build the platform's default application menu with a new window item.
    pub fn application(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<Wry>> {
        let menu = tauri::menu::Menu::default(app)?;
        let start = tauri::menu::MenuItem::with_id(
            app,
            START_MENU_ID,
            "New Window",
            true,
            Some("CmdOrCtrl+N"),
        )?;

        // find the default File menu, which Linux leaves out
        let file = menu
            .items()?
            .into_iter()
            .filter_map(|item| item.as_submenu().cloned())
            .find(|submenu| submenu.text().is_ok_and(|text| text == "File"));

        // lead the File menu with the new window item
        app.state::<Menu>().add_titled(start.clone())?;
        match file {
            Some(file) => file.prepend(&start)?,
            None => menu.prepend(&Submenu::with_items(app, "File", true, &[&start])?)?,
        }

        // show the host's items below the application menu's first item
        if let Some(application) = menu
            .items()?
            .first()
            .and_then(|item| item.as_submenu().cloned())
        {
            app.state::<Menu>()
                .add(MenuName::Application, Place::Application(application), 1)?;
        }

        Ok(menu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read each kind of menu item in the host's line format, refusing unknown fields.
    #[test]
    fn test_read_menu_items() {
        let line = r#"[{"kind":"label","id":"serving.label.0","title":"Serving 2 spaces"},{"kind":"action","id":"settings.open","title":"Settings…","shortcut":"mod+,"},{"kind":"toggle","id":"autostart.toggle","title":"Open at Login","isChecked":false},{"kind":"separator"}]"#;
        let items: Vec<MenuItem> = serde_json::from_str(line).unwrap();
        assert_eq!(
            items,
            [
                MenuItem::Label {
                    id: "serving.label.0".into(),
                    title: "Serving 2 spaces".into(),
                },
                MenuItem::Action {
                    id: "settings.open".into(),
                    title: "Settings…".into(),
                    shortcut: Some("mod+,".into()),
                },
                MenuItem::Toggle {
                    id: "autostart.toggle".into(),
                    title: "Open at Login".into(),
                    is_checked: false,
                },
                MenuItem::Separator {},
            ]
        );
        let refused = serde_json::from_str::<MenuItem>(r#"{"kind":"separator","id":"x"}"#)
            .map(|_| ())
            .map_err(|error| error.to_string());
        assert_eq!(
            refused,
            Err("unknown field `id`, there are no fields".to_owned())
        );
    }
}

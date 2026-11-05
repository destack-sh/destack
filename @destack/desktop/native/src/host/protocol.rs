use crate::app::{Bounds, Window, browse};
use crate::badge::Badge;
use crate::banner::Banner;
use crate::dialog::Dialog;
use crate::menu::Shortcut;
use crate::menu::Tray;
use crate::menu::{Menu, MenuItem, MenuName};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{self, Write};
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

/// The host's private input pipe, closed once the native windows exit.
pub type Input = Arc<Mutex<Option<ChildStdin>>>;

/// Commands accepted only from the desktop host's private output pipe.
#[derive(Deserialize)]
#[serde(tag = "action", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Command {
    // windows
    /// Open a window, or focus the one with the identity.
    #[serde(rename = "window.open")]
    OpenWindow(Window),
    /// Bring a window into focus.
    #[serde(rename = "window.focus")]
    FocusWindow {
        /// The window's identity.
        id: String,
    },
    /// Close a window without reporting it closed.
    #[serde(rename = "window.close")]
    CloseWindow {
        /// The window's identity.
        id: String,
    },

    // banners and the badge
    /// Show a banner, replacing the one with the same identity.
    #[serde(rename = "banner.show")]
    ShowBanner(Banner),
    /// Withdraw a shown banner.
    #[serde(rename = "banner.withdraw")]
    WithdrawBanner {
        /// The banner's identity.
        id: String,
    },
    /// Show a count on the application icon.
    #[serde(rename = "badge.set")]
    SetBadge {
        /// The count to show, or none to clear the badge.
        count: Option<i64>,
    },

    // dialogs
    /// Ask the person a question in a modal dialog with two answers.
    #[serde(rename = "dialog.ask")]
    AskDialog(Dialog),

    // menus, the tray, shortcuts and the browser
    /// Replace the host's items in a native menu.
    #[serde(rename = "menu.set")]
    SetMenu {
        /// The menu.
        menu: MenuName,
        /// The items, in order.
        items: Vec<MenuItem>,
    },
    /// Show the desktop's status on the tray icon.
    #[serde(rename = "tray.set")]
    SetTray {
        /// The text the icon shows on hover.
        tooltip: String,
    },
    /// Register a global shortcut, replacing the one with the same identity.
    #[serde(rename = "shortcut.register")]
    RegisterShortcut {
        /// The shortcut's identity, which a press reports back.
        id: String,
        /// The key combination, such as `alt+space`.
        shortcut: String,
    },
    /// Release a global shortcut, which presses then no longer report.
    #[serde(rename = "shortcut.unregister")]
    UnregisterShortcut {
        /// The shortcut's identity.
        id: String,
    },
    /// Open a web location in the default browser, such as a sign-in.
    #[serde(rename = "browser.open")]
    OpenBrowser {
        /// The location.
        url: tauri::Url,
    },

    /// Title the shell's own items in the host's language.
    #[serde(rename = "menu.title")]
    TitleMenu {
        /// The title of the item opening a new window.
        start: String,
        /// The title of the item quitting the desktop.
        quit: String,
    },

    // application
    /// Open the desktop at login, or stop opening it.
    #[serde(rename = "autostart.set")]
    SetAutostart {
        /// Whether the desktop opens at login.
        is_enabled: bool,
    },
    /// Close every window and exit.
    #[serde(rename = "application.quit")]
    QuitApplication {},
}

impl Command {
    /// Apply a host command on the native application's main thread.
    pub fn apply(self, app: &AppHandle, input: Input) -> Result<(), Box<dyn Error>> {
        match self {
            // windows
            Self::OpenWindow(window) => {
                let label = window.id.clone();
                window.open(app, input)?;

                // show the current badge on the new window
                if let Some(window) = app.get_webview_window(&label) {
                    app.state::<Badge>().show(&window)?;
                }
            }
            Self::FocusWindow { id } => {
                if let Some(window) = app.get_webview_window(&id) {
                    window.unminimize()?;
                    window.set_focus()?;
                }
            }
            Self::CloseWindow { id } => {
                if let Some(window) = app.get_webview_window(&id) {
                    window.destroy()?;
                }
            }

            // banners and the badge
            Self::ShowBanner(banner) => banner.show(app)?,
            Self::WithdrawBanner { id } => Banner::withdraw(app, &id)?,
            Self::SetBadge { count } => app.state::<Badge>().set(app, count)?,

            // dialogs
            Self::AskDialog(dialog) => dialog.ask(app, input),

            // menus, the tray, shortcuts and the browser
            Self::SetMenu { menu, items } => app.state::<Menu>().set(app, menu, items)?,
            Self::TitleMenu { start, quit } => app.state::<Menu>().title(&start, &quit)?,
            Self::SetTray { tooltip } => app.state::<Tray>().set_tooltip(&tooltip)?,
            Self::RegisterShortcut { id, shortcut } => {
                app.state::<Shortcut>()
                    .register(app, input, id, &shortcut)?;
            }
            Self::UnregisterShortcut { id } => app.state::<Shortcut>().unregister(app, &id)?,
            Self::OpenBrowser { url } => browse(app, &input, &url),

            // application
            Self::SetAutostart { is_enabled: true } => app.autolaunch().enable()?,
            Self::SetAutostart { is_enabled: false } => app.autolaunch().disable()?,
            Self::QuitApplication {} => app.exit(0),
        }

        Ok(())
    }

    /// Read the action of the command as the host sends it, such as `window.open`.
    pub fn action(&self) -> &'static str {
        match self {
            Self::OpenWindow(_) => "window.open",
            Self::FocusWindow { .. } => "window.focus",
            Self::CloseWindow { .. } => "window.close",
            Self::ShowBanner(_) => "banner.show",
            Self::WithdrawBanner { .. } => "banner.withdraw",
            Self::SetBadge { .. } => "badge.set",
            Self::AskDialog(_) => "dialog.ask",
            Self::SetMenu { .. } => "menu.set",
            Self::TitleMenu { .. } => "menu.title",
            Self::SetTray { .. } => "tray.set",
            Self::RegisterShortcut { .. } => "shortcut.register",
            Self::UnregisterShortcut { .. } => "shortcut.unregister",
            Self::OpenBrowser { .. } => "browser.open",
            Self::SetAutostart { .. } => "autostart.set",
            Self::QuitApplication {} => "application.quit",
        }
    }

    /// Report whether the command runs off the main thread, which it would block.
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            Self::RegisterShortcut { .. } | Self::SetAutostart { .. }
        )
    }
}

/// Native observations returned to the desktop host.
#[derive(Serialize)]
#[serde(tag = "action")]
pub enum Event {
    // windows
    /// The person closed a window.
    #[serde(rename = "window.closed")]
    WindowClosed {
        /// The window's identity.
        id: String,
    },
    /// A window navigated off its origin, which the shell stopped.
    #[serde(rename = "window.navigated")]
    WindowNavigated {
        /// The window's identity.
        id: String,
        /// The location.
        url: String,
    },
    /// The person moved or resized a window.
    #[serde(rename = "window.moved")]
    WindowMoved {
        /// The window's identity.
        id: String,
        /// Where the window sits now.
        bounds: Bounds,
    },
    /// A window came into focus.
    #[serde(rename = "window.focused")]
    WindowFocused {
        /// The window's identity.
        id: String,
    },

    // banners
    /// The person chose an action beside a banner.
    #[serde(rename = "banner.acted")]
    BannerActed {
        /// The banner's identity.
        id: String,
        /// The action's declared name.
        name: String,
        /// The text the action asked for.
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// The person dismissed a banner.
    #[serde(rename = "banner.dismissed")]
    BannerDismissed {
        /// The banner's identity.
        id: String,
    },

    // dialogs
    /// The person answered a dialog.
    #[serde(rename = "dialog.answered")]
    DialogAnswered {
        /// The dialog's identity.
        id: String,
        /// Whether the person allowed.
        #[serde(rename = "isAllowed")]
        is_allowed: bool,
    },

    // links, menus and shortcuts
    /// Another application opened a link, or the person asked for a new window.
    #[serde(rename = "link.opened")]
    LinkOpened {
        /// The link, or none for the start link.
        url: Option<String>,
    },
    /// The person chose a host item of the tray or the application menu.
    #[serde(rename = "menu.chosen")]
    MenuChosen {
        /// The item's identity.
        item: String,
    },
    /// The person pressed a registered global shortcut.
    #[serde(rename = "shortcut.pressed")]
    ShortcutPressed {
        /// The shortcut's identity.
        id: String,
    },

    // application
    /// The person launched the desktop again, or clicked its icon without a window showing.
    #[serde(rename = "application.reopened")]
    ApplicationReopened {},
    /// A command or a native step failed.
    #[serde(rename = "shell.failed")]
    ShellFailed {
        /// What failed: a command's action, such as `window.open`, or a native step.
        command: String,
        /// Why it failed.
        message: String,
    },
}

impl Event {
    /// Describe a failed command or native step, for the host to report.
    pub fn failed(command: &str, error: impl std::fmt::Display) -> Self {
        Self::ShellFailed {
            command: command.to_owned(),
            message: error.to_string(),
        }
    }

    /// Send one complete observation while the host input remains open.
    pub fn send(&self, input: &Mutex<Option<ChildStdin>>) -> io::Result<()> {
        // serialize messages under one writer lock
        let mut input = input.lock().map_err(|_| lock_error())?;
        if let Some(input) = input.as_mut() {
            serde_json::to_writer(&mut *input, self)?;
            input.write_all(b"\n")?;
            input.flush()?;
        }

        Ok(())
    }

    /// Send an observation from a native callback, exiting once the host can no longer receive it.
    pub fn report(&self, app: &AppHandle, input: &Mutex<Option<ChildStdin>>) {
        if let Err(error) = self.send(input) {
            eprintln!("{error}");
            app.exit(1);
        }
    }
}

/// Report a poisoned lock after a panic on another thread.
pub fn lock_error() -> io::Error {
    io::Error::other("desktop lock failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read every host command in the host's line format, refusing unknown fields.
    #[test]
    fn test_read_host_commands() {
        let lines = [
            r#"{"action":"window.open","id":"window-1","kind":"view","url":"http://notes--personal--florian.localhost:7470/home","profile":"00000000-0000-0000-0000-000000000002","title":"notes","bounds":{"x":10,"y":20,"width":800,"height":600}}"#,
            r#"{"action":"window.open","id":"palette","kind":"palette","url":"http://home--personal--florian.localhost:7470/command","profile":"00000000-0000-0000-0000-000000000003","title":"Commands"}"#,
            r#"{"action":"window.focus","id":"window-1"}"#,
            r#"{"action":"window.close","id":"window-1"}"#,
            r#"{"action":"banner.show","id":"delivery-1","title":"Bob mentioned you","body":"can you check the intro?","interruption":"timeSensitive","actions":[{"name":"reply","title":"Reply","isDestructive":false,"text":{"placeholder":"Reply","button":"Send"}}]}"#,
            r#"{"action":"banner.withdraw","id":"delivery-1"}"#,
            r#"{"action":"badge.set","count":3}"#,
            r#"{"action":"badge.set","count":null}"#,
            r#"{"action":"menu.set","menu":"tray","items":[{"kind":"action","id":"update.restartToUpdate","title":"Restart to Update"}]}"#,
            r#"{"action":"menu.title","start":"Neues Fenster","quit":"Beenden"}"#,
            r#"{"action":"tray.set","tooltip":"Destack: Serving 2 spaces"}"#,
            r#"{"action":"shortcut.register","id":"palette","shortcut":"alt+space"}"#,
            r#"{"action":"shortcut.unregister","id":"palette"}"#,
            r#"{"action":"browser.open","url":"https://destack.app/sign-in"}"#,
            r#"{"action":"autostart.set","isEnabled":true}"#,
            r#"{"action":"dialog.ask","id":"window-1","title":"Allow notes to act for you?","body":"read note","allow":"Allow","deny":"Don't Allow"}"#,
            r#"{"action":"application.quit"}"#,
        ];
        let read = lines.map(|line| match serde_json::from_str::<Command>(line) {
            Ok(Command::OpenWindow(window)) => format!("open {} {:?}", window.id, window.bounds),
            Ok(Command::FocusWindow { id }) => format!("focus {id}"),
            Ok(Command::CloseWindow { id }) => format!("close {id}"),
            Ok(Command::ShowBanner(banner)) => format!(
                "show {} {:?} {}",
                banner.id, banner.interruption, banner.actions[0].name
            ),
            Ok(Command::WithdrawBanner { id }) => format!("withdraw {id}"),
            Ok(Command::SetBadge { count }) => format!("badge {count:?}"),
            Ok(Command::SetMenu { menu, items }) => format!("menu {menu:?} {}", items.len()),
            Ok(Command::TitleMenu { start, quit }) => format!("title {start} {quit}"),
            Ok(Command::SetTray { tooltip }) => format!("tray {tooltip}"),
            Ok(Command::RegisterShortcut { id, shortcut }) => {
                format!("shortcut {id} {shortcut}")
            }
            Ok(Command::UnregisterShortcut { id }) => format!("unregister {id}"),
            Ok(Command::OpenBrowser { url }) => format!("browse {url}"),
            Ok(Command::SetAutostart { is_enabled }) => format!("autostart {is_enabled}"),
            Ok(Command::AskDialog(dialog)) => format!(
                "ask {} {} {} {} {}",
                dialog.id, dialog.title, dialog.body, dialog.allow, dialog.deny
            ),
            Ok(Command::QuitApplication {}) => "quit".to_owned(),
            Err(error) => format!("invalid {error}"),
        });
        assert_eq!(
            read,
            [
                "open window-1 Some(Bounds { x: 10.0, y: 20.0, width: 800.0, height: 600.0 })",
                "open palette None",
                "focus window-1",
                "close window-1",
                "show delivery-1 TimeSensitive reply",
                "withdraw delivery-1",
                "badge Some(3)",
                "badge None",
                "menu Tray 1",
                "title Neues Fenster Beenden",
                "tray Destack: Serving 2 spaces",
                "shortcut palette alt+space",
                "unregister palette",
                "browse https://destack.app/sign-in",
                "autostart true",
                "ask window-1 Allow notes to act for you? read note Allow Don't Allow",
                "quit",
            ]
        );

        // refuse fields a command does not take
        let unknown = [
            r#"{"action":"badge.set","count":1,"label":"new"}"#,
            r#"{"action":"window.open","id":"window-1","kind":"view","url":"https://destack.app/","profile":"00000000-0000-0000-0000-000000000002","title":"notes","label":"x"}"#,
        ];
        let refused = unknown.map(|line| match serde_json::from_str::<Command>(line) {
            Ok(_) => "accepted".to_owned(),
            Err(error) => error.to_string(),
        });
        assert_eq!(
            refused,
            [
                "unknown field `label`, expected `count`",
                "unknown field `label`, expected one of `id`, `kind`, `url`, `profile`, `title`, `bounds`",
            ]
        );
    }

    /// Write every native observation tagged with its action.
    #[test]
    fn test_write_native_events() -> serde_json::Result<()> {
        let bounds = Bounds {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        };
        let lines = [
            Event::WindowClosed {
                id: "window-1".into(),
            },
            Event::WindowNavigated {
                id: "window-1".into(),
                url: "http://notes--personal--florian.localhost:7470/editor".into(),
            },
            Event::WindowMoved {
                id: "window-1".into(),
                bounds,
            },
            Event::WindowFocused {
                id: "window-1".into(),
            },
            Event::BannerActed {
                id: "delivery-1".into(),
                name: "reply".into(),
                text: Some("on it".into()),
            },
            Event::BannerDismissed {
                id: "delivery-1".into(),
            },
            Event::LinkOpened {
                url: Some("destack://notes--personal--florian/home".into()),
            },
            Event::LinkOpened { url: None },
            Event::MenuChosen {
                item: "update.0".into(),
            },
            Event::ShortcutPressed {
                id: "palette".into(),
            },
            Event::ApplicationReopened {},
        ]
        .iter()
        .map(serde_json::to_string)
        .collect::<serde_json::Result<Vec<_>>>()?;
        assert_eq!(
            lines,
            [
                r#"{"action":"window.closed","id":"window-1"}"#,
                r#"{"action":"window.navigated","id":"window-1","url":"http://notes--personal--florian.localhost:7470/editor"}"#,
                r#"{"action":"window.moved","id":"window-1","bounds":{"x":10.0,"y":20.0,"width":800.0,"height":600.0}}"#,
                r#"{"action":"window.focused","id":"window-1"}"#,
                r#"{"action":"banner.acted","id":"delivery-1","name":"reply","text":"on it"}"#,
                r#"{"action":"banner.dismissed","id":"delivery-1"}"#,
                r#"{"action":"link.opened","url":"destack://notes--personal--florian/home"}"#,
                r#"{"action":"link.opened","url":null}"#,
                r#"{"action":"menu.chosen","item":"update.0"}"#,
                r#"{"action":"shortcut.pressed","id":"palette"}"#,
                r#"{"action":"application.reopened"}"#,
            ]
        );

        Ok(())
    }
}

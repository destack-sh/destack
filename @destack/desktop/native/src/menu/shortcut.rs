use crate::host::{Event, Input, lock_error};
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// The global shortcuts the host registered, by identity.
#[derive(Default)]
pub struct Shortcut(Mutex<HashMap<String, tauri_plugin_global_shortcut::Shortcut>>);

impl Shortcut {
    /// Register a shortcut under an identity, replacing the one before, and report each press.
    pub fn register(
        &self,
        app: &AppHandle,
        input: Input,
        id: String,
        text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let shortcut = parse(text)?;
        let mut registered = self.0.lock().map_err(|_| lock_error())?;

        // release the shortcut registered under the identity before
        let shortcuts = app.global_shortcut();
        if let Some(previous) = registered.remove(&id) {
            shortcuts.unregister(previous)?;
        }

        // report each press under the identity
        let pressed = id.clone();
        shortcuts.on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                Event::ShortcutPressed {
                    id: pressed.clone(),
                }
                .report(app, &input);
            }
        })?;
        registered.insert(id, shortcut);

        Ok(())
    }

    /// Release the shortcut registered under an identity, if any.
    pub fn unregister(&self, app: &AppHandle, id: &str) -> Result<(), Box<dyn Error>> {
        let mut registered = self.0.lock().map_err(|_| lock_error())?;
        if let Some(previous) = registered.remove(id) {
            app.global_shortcut().unregister(previous)?;
        }

        Ok(())
    }
}

/// Read a shortcut in the settings' keybinding syntax, such as `mod+shift+k`.
pub fn parse(text: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, Box<dyn Error>> {
    Ok(tauri_plugin_global_shortcut::Shortcut::from_str(
        &accelerator(text),
    )?)
}

/// Write a key combination of the settings' keybinding syntax in the native accelerator syntax.
pub fn accelerator(text: &str) -> String {
    text.split('+')
        .map(|token| match token.to_ascii_lowercase().as_str() {
            "mod" => "CmdOrCtrl".to_owned(),
            "cmd" | "meta" => "Super".to_owned(),
            _ => token.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    /// Read the default palette shortcut and a command keybinding, and refuse unknown keys.
    #[test]
    fn test_parse_shortcuts() {
        let read = ["alt+space", "mod+shift+k", "ctrl+alt+p"].map(|text| {
            parse(text)
                .map(|shortcut| (shortcut.mods, shortcut.key))
                .ok()
        });
        let command = if cfg!(target_os = "macos") {
            Modifiers::SUPER
        } else {
            Modifiers::CONTROL
        };
        assert_eq!(
            read,
            [
                Some((Modifiers::ALT, Code::Space)),
                Some((command | Modifiers::SHIFT, Code::KeyK)),
                Some((Modifiers::CONTROL | Modifiers::ALT, Code::KeyP)),
            ]
        );
        let refused = parse("alt+nothing")
            .map(|_| ())
            .map_err(|error| error.to_string());
        assert_eq!(
            refused,
            Err("Couldn't recognize \"nothing\" as a valid key for hotkey, if you feel like it should be, please report this to https://github.com/tauri-apps/muda".to_owned())
        );
    }
}

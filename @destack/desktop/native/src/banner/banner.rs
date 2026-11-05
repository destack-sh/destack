use serde::Deserialize;
use std::error::Error;
use tauri::AppHandle;

#[cfg(target_os = "macos")]
use crate::banner::AppleBanners;
#[cfg(target_os = "macos")]
use tauri::Manager;

#[cfg(not(target_os = "macos"))]
use std::io;
#[cfg(not(target_os = "macos"))]
use tauri_plugin_notification::NotificationExt;

/// A banner the host shows: a notification's text and the actions offered beside it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Banner {
    /// The banner's identity, which withdrawals and events name.
    pub id: String,
    /// The headline.
    pub title: String,
    /// The line below the headline.
    pub subtitle: Option<String>,
    /// The text.
    pub body: String,
    /// How strongly the banner interrupts, which the system's focus modes follow.
    pub interruption: Interruption,
    /// The actions offered beside it.
    pub actions: Vec<BannerAction>,
}

/// How strongly a banner interrupts, as Apple's interruption levels name it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Interruption {
    /// Listed without sound or a banner.
    Passive,
    /// Shown as the person's focus allows.
    Active,
    /// Shown through a focus that allows time-sensitive banners.
    TimeSensitive,
    /// Shown through every focus and the silent switch.
    Critical,
}

/// An action offered beside a banner, as the declaring notification names it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BannerAction {
    /// The action's declared name, which an act reports back.
    pub name: String,
    /// The button's label.
    pub title: String,
    /// Whether the action destroys or declines.
    pub is_destructive: bool,
    /// The text field the action asks for.
    pub text: Option<BannerText>,
}

/// The text field a banner action asks for.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannerText {
    /// The field's placeholder.
    pub placeholder: String,
    /// The label of the button sending the text.
    pub button: String,
}

impl Banner {
    /// Show the banner through the system's notification center with its actions.
    #[cfg(target_os = "macos")]
    pub fn show(self, app: &AppHandle) -> Result<(), Box<dyn Error>> {
        app.state::<AppleBanners>().show(self)
    }

    /// Withdraw a shown banner.
    #[cfg(target_os = "macos")]
    pub fn withdraw(app: &AppHandle, id: &str) -> Result<(), Box<dyn Error>> {
        app.state::<AppleBanners>().withdraw(id)
    }

    /// Show the banner through the system's notifications, refusing actions only macOS offers.
    #[cfg(not(target_os = "macos"))]
    pub fn show(self, app: &AppHandle) -> Result<(), Box<dyn Error>> {
        // refuse actions, which only macOS banners offer
        if !self.actions.is_empty() {
            return Err(io::Error::other("banners offer actions only on macOS").into());
        }

        // show the subtitle as the body's first line, which these banners lack
        let body = match self.subtitle {
            Some(subtitle) => format!("{subtitle}\n{}", self.body),
            None => self.body,
        };
        app.notification()
            .builder()
            .title(self.title)
            .body(body)
            .show()?;

        Ok(())
    }

    /// Refuse withdrawing a banner, which only macOS banners allow.
    #[cfg(not(target_os = "macos"))]
    pub fn withdraw(_app: &AppHandle, _id: &str) -> Result<(), Box<dyn Error>> {
        Err(io::Error::other("banners are withdrawn only on macOS").into())
    }
}

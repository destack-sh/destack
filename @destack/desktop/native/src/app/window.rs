use crate::host::{Event, Input};
use serde::{Deserialize, Serialize};
use std::io;
use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

/// The host suffix of the views a device host serves, as RFC 6761 reserves it for loopback.
const LOCAL_SUFFIX: &str = ".localhost";

/// The width of a new view window, in logical pixels.
const WINDOW_WIDTH: f64 = 1100.0;

/// The height of a new view window, in logical pixels.
const WINDOW_HEIGHT: f64 = 760.0;

/// The width of the command palette, in logical pixels, as Spotlight and Raycast size theirs.
const PALETTE_WIDTH: f64 = 680.0;

/// The height of the command palette, in logical pixels.
const PALETTE_HEIGHT: f64 = 420.0;

/// What a window shows.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    /// A view, or an object's opening, in a window the person places.
    View,
    /// The command palette, floating above the other windows until it loses focus.
    Palette,
}

/// A window's place on its screen, in logical pixels from the screen's top left corner.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

/// A window of one view or of the command palette, as its origin serves it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// The window's identity, which later commands and events name.
    pub id: String,
    /// What the window shows.
    pub kind: WindowKind,
    /// The location, which its origin authorized.
    pub url: tauri::Url,
    /// The other origins the window navigates within, such as the universe signing it in.
    pub origins: Vec<tauri::Url>,
    /// The browser profile isolating the origin's storage.
    pub profile: Uuid,
    /// The title bar's text.
    pub title: String,
    /// Where the window sits, the shell's choice when absent.
    pub bounds: Option<Bounds>,
}

impl Window {
    /// Require encrypted or loopback locations without credentials, its own and its origins'.
    pub fn validate(&self) -> io::Result<()> {
        if !std::iter::once(&self.url)
            .chain(&self.origins)
            .all(Window::is_supported)
        {
            return Err(io::Error::other("unsupported view URL"));
        }

        Ok(())
    }

    /// Report whether a location is encrypted or on loopback (RFC 8252 7.3), without credentials.
    fn is_supported(url: &tauri::Url) -> bool {
        let is_local = url.scheme() == "http"
            && url.host_str().is_some_and(|host| {
                host.ends_with(LOCAL_SUFFIX) || host == "localhost" || host == "127.0.0.1"
            });

        (url.scheme() == "https" || is_local)
            && url.username().is_empty()
            && url.password().is_none()
    }

    /// Open the window with isolated storage, handing locations on other origins to the host.
    pub fn open(self, app: &AppHandle, input: Input) -> tauri::Result<()> {
        self.validate()?;

        // focus an existing window without changing its security context
        if let Some(window) = app.get_webview_window(&self.id) {
            window.set_focus()?;

            return Ok(());
        }

        // confine navigation to the window's origins, letting the host open locations on other origins
        let origins: Vec<_> = std::iter::once(&self.url)
            .chain(&self.origins)
            .map(tauri::Url::origin)
            .collect();
        let navigation = input.clone();
        let opening = input.clone();
        let (id, kind) = (self.id.clone(), self.kind);
        let (navigated, opened) = (id.clone(), id.clone());
        let builder = WebviewWindowBuilder::new(app, &self.id, WebviewUrl::External(self.url))
            .title(self.title)
            .on_navigation(move |destination| {
                if origins.contains(&destination.origin()) {
                    return true;
                }
                navigate(&navigation, &navigated, destination);

                false
            })
            .on_new_window(move |destination, _| {
                navigate(&opening, &opened, &destination);

                NewWindowResponse::Deny
            });

        // size a view where it last sat, and float the palette in the middle of the screen
        let builder = match (kind, self.bounds) {
            (WindowKind::View, Some(bounds)) => builder
                .inner_size(bounds.width, bounds.height)
                .position(bounds.x, bounds.y),
            (WindowKind::View, None) => builder.inner_size(WINDOW_WIDTH, WINDOW_HEIGHT),
            (WindowKind::Palette, _) => builder
                .inner_size(PALETTE_WIDTH, PALETTE_HEIGHT)
                .center()
                .always_on_top(true)
                .decorations(false)
                .resizable(false)
                .skip_taskbar(true),
        };

        // separate each origin's storage across native webviews
        #[cfg(target_os = "macos")]
        let builder = builder.data_store_identifier(*self.profile.as_bytes());
        #[cfg(not(target_os = "macos"))]
        let builder = builder.data_directory(
            app.path()
                .app_local_data_dir()?
                .join("profile")
                .join(self.profile.to_string()),
        );
        let window = builder.build()?;

        // report what the person does to the window
        let handle = app.clone();
        let observed = window.clone();
        window.on_window_event(move |event| {
            observe(&handle, &input, &observed, &id, kind, event);
        });

        Ok(())
    }
}

/// Report a native window event to the host, closing the palette once it loses focus.
fn observe(
    app: &AppHandle,
    input: &Input,
    window: &WebviewWindow,
    id: &str,
    kind: WindowKind,
    event: &WindowEvent,
) {
    let id = id.to_owned();
    match (kind, event) {
        // report a view the person closes, which a host's close never does
        (WindowKind::View, WindowEvent::CloseRequested { .. }) => {
            Event::WindowClosed { id }.report(app, input);
        }
        // report where the person placed a view
        (WindowKind::View, WindowEvent::Moved(_) | WindowEvent::Resized(_)) => {
            match bounds(window) {
                Ok(bounds) => Event::WindowMoved { id, bounds }.report(app, input),
                Err(error) => Event::failed("window.moved", error).report(app, input),
            }
        }
        // report a window coming into focus
        (_, WindowEvent::Focused(true)) => Event::WindowFocused { id }.report(app, input),
        // close the palette once the person moves on, reporting it as a closed window
        (WindowKind::Palette, WindowEvent::Focused(false)) => match window.destroy() {
            Ok(()) => Event::WindowClosed { id }.report(app, input),
            Err(error) => Event::failed("window.close", error).report(app, input),
        },
        // leave other events to the shell
        _ => {}
    }
}

/// Read where a window sits, in logical pixels.
fn bounds(window: &WebviewWindow) -> tauri::Result<Bounds> {
    let scale = window.scale_factor()?;
    let position: LogicalPosition<f64> = window.outer_position()?.to_logical(scale);
    let size: LogicalSize<f64> = window.inner_size()?.to_logical(scale);

    Ok(Bounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

/// Ask the host to open a location a window navigated to off its origin.
fn navigate(input: &Input, id: &str, url: &tauri::Url) {
    let event = Event::WindowNavigated {
        id: id.to_owned(),
        url: url.to_string(),
    };
    if let Err(error) = event.send(input) {
        eprintln!("{error}");
    }
}

/// Remove a browser profile's stored data, reporting a failure to the host.
pub fn remove_profile(app: &AppHandle, input: &Input, profile: Uuid) {
    let handle = app.clone();
    let input = input.clone();
    tauri::async_runtime::spawn(async move {
        // remove the data store the profile's windows used
        #[cfg(target_os = "macos")]
        let result = handle
            .remove_data_store(*profile.as_bytes())
            .await
            .map_err(|error| error.to_string());
        #[cfg(not(target_os = "macos"))]
        let result = remove_directory(&handle, profile);

        // report a failure
        if let Err(error) = result {
            Event::failed("profile.remove", error).report(&handle, &input);
        }
    });
}

/// Remove a profile's data directory, a profile never used being removed already.
#[cfg(not(target_os = "macos"))]
fn remove_directory(app: &AppHandle, profile: Uuid) -> Result<(), String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("profile")
        .join(profile.to_string());
    match std::fs::remove_dir_all(directory) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(()),
    }
}

/// Open a web location in the default browser, reporting a refused one to the host.
pub fn browse(app: &AppHandle, input: &Input, url: &tauri::Url) {
    // open only web locations
    let result = if matches!(url.scheme(), "https" | "http") {
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|error| error.to_string())
    } else {
        Err(format!("unsupported browser URL {url}"))
    };
    if let Err(error) = result {
        Event::failed("browser.open", error).report(app, input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Accept encrypted origins and loopback views, rejecting other schemes and credentials.
    #[test]
    fn test_validate_window_origin() {
        for (url, accepted) in [
            ("https://notes.personal.florian.destack.app/home", true),
            ("http://notes.personal.florian.localhost:7777/home", true),
            ("http://127.0.0.1:3210/", true),
            ("http://localhost.example.com/", false),
            ("http://destack.app/", false),
            ("file:///etc/passwd", false),
            ("javascript:alert(1)", false),
            ("https://user:password@destack.app/", false),
        ] {
            let window = Window {
                id: "window-1".into(),
                kind: WindowKind::View,
                url: url.parse().unwrap(),
                origins: vec!["https://destack.app".parse().unwrap()],
                profile: Uuid::nil(),
                title: "Notes".into(),
                bounds: None,
            };
            let result = window.validate().map_err(|error| error.to_string());
            let expected = if accepted {
                Ok(())
            } else {
                Err("unsupported view URL".to_owned())
            };
            assert_eq!(result, expected, "{url}");
        }

        // refuse an unencrypted origin to navigate within
        let window = Window {
            id: "window-1".into(),
            kind: WindowKind::View,
            url: "https://notes.personal.florian.destack.app/home"
                .parse()
                .unwrap(),
            origins: vec!["http://destack.app".parse().unwrap()],
            profile: Uuid::nil(),
            title: "Notes".into(),
            bounds: None,
        };
        assert_eq!(
            window.validate().map_err(|error| error.to_string()),
            Err("unsupported view URL".to_owned())
        );
    }
}

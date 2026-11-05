use super::installation;
use crate::badge::Badge;
use crate::host::Host;
use crate::host::{Command, Event, Input, lock_error};
use crate::menu::{Menu, QUIT_MENU_ID, START_MENU_ID, Shortcut, Tray};
use std::error::Error;
use std::io;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager, RunEvent};

#[cfg(target_os = "macos")]
use crate::banner::AppleBanners;

/// The TypeScript host the first instance starts, reaped after the application exits.
type Hosted = Arc<Mutex<Option<Host>>>;

/// Run native windows as this user's one desktop, releasing the host on every exit path.
pub fn main() -> Result<(), Box<dyn Error>> {
    // follow the activated release when opening an existing Windows installation shortcut
    if installation::forward()? {
        return Ok(());
    }

    // hand a second launch's link to this desktop, or reopen it, and start the host once this is the first instance
    let input: Input = Arc::new(Mutex::new(None));
    let hosted: Hosted = Arc::new(Mutex::new(None));
    let (opened, started, host) = (input.clone(), input.clone(), hosted.clone());
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            move |app, arguments, _| {
                let url = arguments
                    .into_iter()
                    .skip(1)
                    .find(|word| !word.starts_with('-'));
                let event = match url {
                    Some(url) => Event::LinkOpened { url: Some(url) },
                    None => Event::ApplicationReopened {},
                };
                event.report(app, &opened);
            },
        ))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Badge::default())
        .manage(Menu::default())
        .manage(Shortcut::default())
        .menu(Menu::application)
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.manage(AppleBanners::open(started.clone()));
            #[cfg(not(target_os = "macos"))]
            app.handle().plugin(tauri_plugin_notification::init())?;

            app.manage(Tray::open(app.handle())?);
            start(app.handle(), &started, &host)?;

            Ok(())
        })
        .build(tauri::generate_context!())?;

    // forward menu, tray, dock and link requests to the host
    let events = input.clone();
    let code = app.run_return(move |app, event| answer(app, &events, event));

    // close the host input after the native application exits, then reap the host
    input.lock().map_err(|_| lock_error())?.take();
    let stopped = match hosted.lock().map_err(|_| lock_error())?.take() {
        Some(mut host) => host.wait().and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "desktop host exited with {status}"
                )))
            }
        }),
        None => Ok(()),
    };

    // report both failures when the native application and host cleanup fail
    match (code, stopped) {
        (0, Ok(())) => Ok(()),
        (code, Ok(())) => Err(io::Error::other(format!("desktop exited with {code}")).into()),
        (0, Err(error)) => Err(error.into()),
        (code, Err(stopped)) => Err(io::Error::other(format!(
            "desktop exited with {code}; host shutdown failed: {stopped}"
        ))
        .into()),
    }
}

/// Start the host with private pipes and apply its commands until it ends its output.
fn start(app: &AppHandle, input: &Input, hosted: &Hosted) -> Result<(), Box<dyn Error>> {
    // store the host's input for the native events, and the host for reaping
    let mut host = Host::start()?;
    let commands = host.commands()?;
    *input.lock().map_err(|_| lock_error())? = host.child.stdin.take();
    *hosted.lock().map_err(|_| lock_error())? = Some(host);

    // deliver commands without blocking the native event loop
    let handle = app.clone();
    let input = input.clone();
    thread::spawn(move || deliver(&handle, &input, commands));

    Ok(())
}

/// Apply each host command, exiting once the host ends its output.
fn deliver(app: &AppHandle, input: &Input, commands: Receiver<io::Result<Command>>) {
    for message in commands {
        // stop at an unreadable command
        let command = match message {
            Ok(command) => command,
            Err(error) => {
                eprintln!("{error}");
                break;
            }
        };

        // apply a blocking command here, and the others on the native event loop, reporting a failure to the host
        let action = command.action();
        let result = if command.is_blocking() {
            if let Err(error) = command.apply(app, input.clone()) {
                Event::failed(action, error).report(app, input);
            }
            Ok(())
        } else {
            let (handle, input) = (app.clone(), input.clone());
            app.run_on_main_thread(move || {
                // report a failed command, leaving the other windows open
                if let Err(error) = command.apply(&handle, input.clone()) {
                    Event::failed(action, error).report(&handle, &input);
                }
            })
        };
        if let Err(error) = result {
            eprintln!("{error}");
            break;
        }
    }
    app.exit(1);
}

/// Answer a native application event: new windows, host items, quitting and opened links.
fn answer(app: &AppHandle, input: &Input, event: RunEvent) {
    match event {
        // open a window of the start link
        RunEvent::MenuEvent(event) if event.id() == START_MENU_ID => {
            Event::LinkOpened { url: None }.report(app, input);
        }
        // quit, which closing the last window does not
        RunEvent::MenuEvent(event) if event.id() == QUIT_MENU_ID => app.exit(0),
        // report the host's items the person chooses, and a failed lookup
        RunEvent::MenuEvent(event) => {
            let item = event.id().as_ref().to_owned();
            match app.state::<Menu>().contains(&item) {
                Ok(true) => Event::MenuChosen { item }.report(app, input),
                Ok(false) => {}
                Err(error) => Event::failed("menu.chosen", error).report(app, input),
            }
        }
        // run on after the last window closes
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        // reopen the desktop when the dock icon is clicked without a window showing
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => Event::ApplicationReopened {}.report(app, input),
        // open each link another application opened
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => {
            for url in urls {
                Event::LinkOpened {
                    url: Some(url.into()),
                }
                .report(app, input);
            }
        }
        // leave other native events to the shell
        _ => {}
    }
}

use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, HWND_TOPMOST, ICON_BIG, ICON_SMALL, SW_RESTORE, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SendMessageW, SetWindowPos, SetWindowTextW, ShowWindow, WM_SETICON,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::display::{WindowChromeKind, WindowIconSet, WindowVisibility};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::constants::{WINDOW_ICON_BIG_DEFAULT, WINDOW_ICON_SMALL_DEFAULT};
use super::core::{apply_window_style, normalize_opacity, show_command};
use super::geometry::refresh_window_snapshot;
use super::icon::{
    best_icon_index, create_hicon, decode_window_icons, destroy_owned_icons, icon_target_dimensions,
};
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // decode and validate title payload
    let title = unsafe { title.as_str()? }.to_string();
    let title_wide = core_platform::wide_from_str("title", &title)?;

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setTitle",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply host title update
    let status = unsafe { SetWindowTextW(host_state.hwnd, title_wide.as_ptr()) };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setTitle",
            "SetWindowTextW",
            "failed to set window title",
        ));
    }

    // update cached title value
    host_state.title = title;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setIcons",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // decode icons and build new host icon handles
    let (new_small_icon, new_big_icon) = match icons {
        Some(icon_set) => {
            let decoded = decode_window_icons(icon_set)?;
            let (small_width, small_height, big_width, big_height) =
                icon_target_dimensions(WINDOW_ICON_SMALL_DEFAULT, WINDOW_ICON_BIG_DEFAULT);
            let small_index = best_icon_index(&decoded, small_width, small_height);
            let big_index = best_icon_index(&decoded, big_width, big_height);

            let small_icon =
                create_hicon(&decoded[small_index], "destack.display.window.setIcons")?;
            let big_icon =
                match create_hicon(&decoded[big_index], "destack.display.window.setIcons") {
                    Ok(icon) => icon,
                    Err(error) => {
                        destroy_owned_icons(small_icon, 0);
                        return Err(error);
                    }
                };

            (small_icon, big_icon)
        }
        None => (0, 0),
    };

    // apply host icon handles
    unsafe {
        let _ = SendMessageW(
            host_state.hwnd,
            WM_SETICON,
            ICON_SMALL as usize,
            new_small_icon,
        );
        let _ = SendMessageW(host_state.hwnd, WM_SETICON, ICON_BIG as usize, new_big_icon);
    }

    // swap cached icon handles
    let previous_small_icon = host_state.icon_small;
    let previous_big_icon = host_state.icon_big;
    host_state.icon_small = new_small_icon;
    host_state.icon_big = new_big_icon;

    // release stale icon handles
    let stale_small_icon = if previous_small_icon == new_small_icon {
        0
    } else {
        previous_small_icon
    };
    let stale_big_icon = if previous_big_icon == new_big_icon {
        0
    } else {
        previous_big_icon
    };
    destroy_owned_icons(stale_small_icon, stale_big_icon);

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setVisibility",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // resolve one host show command for this transition
    let show_window_command = if visibility == WindowVisibility::Visible
        && previous.visibility != WindowVisibility::Visible
    {
        SW_RESTORE
    } else {
        show_command(visibility)
    };

    // apply host visibility transition
    unsafe {
        ShowWindow(host_state.hwnd, show_window_command);
        UpdateWindow(host_state.hwnd);
    }

    // refresh cached state and publish deltas
    host_state.visibility = visibility;
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setResizable",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply cached mutation and rollback on host failure
    let previous_resizable = host_state.resizable;
    host_state.resizable = resizable;
    if let Err(error) = apply_window_style(&host_state, "destack.display.window.setResizable") {
        host_state.resizable = previous_resizable;
        return Err(error);
    }

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setDecorated",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply cached mutation and rollback on host failure
    let previous_decorated = host_state.decorated;
    host_state.decorated = decorated;
    if let Err(error) = apply_window_style(&host_state, "destack.display.window.setDecorated") {
        host_state.decorated = previous_decorated;
        return Err(error);
    }

    Ok(())
}

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply host topmost transition
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            if alwaysontop {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setAlwaysOnTop",
            "SetWindowPos",
            "failed to update topmost state",
        ));
    }

    // update cached topmost state
    host_state.always_on_top = alwaysontop;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setChrome",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for rollback and delta publication
    let previous = host_state.clone();

    // apply cached mutation and rollback on host failure
    host_state.chrome = chrome;
    if let Err(error) = apply_window_style(&host_state, "destack.display.window.setChrome") {
        host_state.chrome = previous.chrome;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for rollback and delta publication
    let previous = host_state.clone();

    // apply cached mutation and rollback on host failure
    host_state.mouse_passthrough = passthrough;
    if let Err(error) =
        apply_window_style(&host_state, "destack.display.window.setMousePassthrough")
    {
        host_state.mouse_passthrough = previous.mouse_passthrough;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    // validate opacity payload
    let opacity = normalize_opacity(opacity, "opacity")?;

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setOpacity",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for rollback and delta publication
    let previous = host_state.clone();

    // apply cached mutation and rollback on host failure
    host_state.opacity = opacity;
    if let Err(error) = apply_window_style(&host_state, "destack.display.window.setOpacity") {
        host_state.opacity = previous.opacity;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Read one window opacity value.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.opacity",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // write cached opacity value
    unsafe {
        *out = host_state.opacity;
    }

    Ok(())
}

/// Set one window taskbar-visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for rollback and delta publication
    let previous = host_state.clone();

    // apply cached mutation and rollback on host failure
    host_state.taskbar_visible = visible;
    if let Err(error) = apply_window_style(&host_state, "destack.display.window.setTaskbarVisible")
    {
        host_state.taskbar_visible = previous.taskbar_visible;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

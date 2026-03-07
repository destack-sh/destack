use objc2_app_kit::{NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask};
use objc2_core_graphics::{kCGFloatingWindowLevel, kCGNormalWindowLevel};
use objc2_foundation::NSString;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowChromeKind, WindowIconSet, WindowVisibility, unsupported};
use crate::platform::resource::WindowHandle;
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::{icon, reconcile};
use crate::platform::display::unix::appkit::event::publish_state_deltas;
use crate::platform::display::unix::appkit::{core as appkit_core, resource as display_resource};

/// Apply one requested visibility state to one native AppKit window.
fn apply_window_visibility(window: &objc2_app_kit::NSWindow, visibility: WindowVisibility) {
    match visibility {
        WindowVisibility::Visible => {
            if window.isMiniaturized() {
                window.deminiaturize(None);
            }

            if window.isZoomed() && !window.styleMask().contains(NSWindowStyleMask::FullScreen) {
                window.zoom(None);
            }

            window.makeKeyAndOrderFront(None);
        }
        WindowVisibility::Hidden => {
            window.orderOut(None);
        }
        WindowVisibility::Minimized => {
            if !window.isMiniaturized() {
                window.miniaturize(None);
            }
        }
        WindowVisibility::Maximized => {
            if window.isMiniaturized() {
                window.deminiaturize(None);
            }

            if !window.isZoomed() {
                window.zoom(None);
            }
        }
    }
}

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.always_on_top = always_on_top;
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
        |host| {
            if always_on_top {
                host.window
                    .setLevel(kCGFloatingWindowLevel as NSWindowLevel);
            } else {
                host.window.setLevel(kCGNormalWindowLevel as NSWindowLevel);
            }
            Ok(())
        },
    )?;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.decorated = decorated;
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setDecorated",
        |host| {
            let mut style = host.window.styleMask();
            if decorated {
                style.insert(
                    NSWindowStyleMask::Titled
                        | NSWindowStyleMask::Closable
                        | NSWindowStyleMask::Miniaturizable,
                );
            } else {
                style.remove(
                    NSWindowStyleMask::Titled
                        | NSWindowStyleMask::Closable
                        | NSWindowStyleMask::Miniaturizable,
                );
            }
            host.window.setStyleMask(style);
            Ok(())
        },
    )?;

    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.resizable = resizable;
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setResizable",
        |host| {
            let mut style = host.window.styleMask();
            if resizable {
                style.insert(NSWindowStyleMask::Resizable);
            } else {
                style.remove(NSWindowStyleMask::Resizable);
            }
            host.window.setStyleMask(style);
            Ok(())
        },
    )?;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setChrome",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.chrome = chrome;
    let next = host_state.clone();
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setChrome",
        |host| {
            let behavior = match chrome {
                WindowChromeKind::Popup => NSWindowCollectionBehavior::Transient,
                _ => NSWindowCollectionBehavior::Managed,
            };
            host.window.setCollectionBehavior(behavior);
            Ok(())
        },
    )?;

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setIcons",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    drop(host_state);

    let decoded_icons = icon::decode_window_icon_images(icons)?;
    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setIcons",
        |host| {
            let icon_image = icon::window_icon_image(decoded_icons.as_deref())?;
            let mut retained_icon = host.window_icon.borrow_mut();
            host.window.setMiniwindowImage(icon_image.as_deref());
            *retained_icon = icon_image;
            Ok(())
        },
    )?;

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            "opacity",
            "window opacity must be between 0.0 and 1.0",
        ));
    }

    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setOpacity",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.opacity = opacity;
    let next = host_state.clone();
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setOpacity",
        |host| {
            host.window.setAlphaValue(opacity);
            host.window.setOpaque(opacity >= 1.0);
            Ok(())
        },
    )?;

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

/// Read one window opacity.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.opacity",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    unsafe {
        *out = host_state.opacity;
    }

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let title = unsafe { title.as_str()? }.to_string();
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTitle",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.title = title.clone();
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setTitle",
        |host| {
            let title = NSString::from_str(&title);
            host.window.setTitle(&title);
            Ok(())
        },
    )?;

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setVisibility",
        |host| {
            apply_window_visibility(&host.window, visibility);
            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state_with_visibility(
        &runtime_state,
        window_handle,
        Some(visibility),
    )?;

    Ok(())
}

/// Set taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    drop(host_state);

    unsafe {
        unsupported::destack_display_window_set_taskbar_visible(context, window_handle, visible)
    }
}

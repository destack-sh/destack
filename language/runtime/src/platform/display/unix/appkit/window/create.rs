use std::sync::{Arc, Mutex};

use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSPanel, NSPasteboardTypeFileURL, NSPasteboardTypeString,
    NSWindow, NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask,
};
use objc2_core_graphics::{kCGFloatingWindowLevel, kCGNormalWindowLevel};
use objc2_foundation::{NSArray, NSSize, NSString};

use crate::diagnostic::RuntimeResult;
use crate::host::os::apple::call::with_process_main_context_marker_if_needed;
use crate::platform;
use crate::platform::display::{WindowModeOptions, WindowOptions, WindowRole, WindowVisibility};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::constants::DEFAULT_WINDOW_OPACITY;
use super::cursor::apply_cursor_policy;
use super::{core, drop, geometry, mode, options, reconcile, relation};
use crate::platform::display::unix::appkit;
use crate::platform::display::unix::appkit::core::AppKitWindowHost;
use crate::platform::display::unix::appkit::core::delegate::AppKitWindowDelegate;
use crate::platform::display::unix::appkit::event;

/// Create one native AppKit window object.
fn create_native_window(
    mtm: MainThreadMarker,
    options: &WindowOptions,
) -> objc2::rc::Retained<NSWindow> {
    let rect = options::window_rect(options.size_logical, options.position);
    let style_mask = options::style_mask_for_options(options);

    // use panels for non-toplevel roles
    if options.role != WindowRole::Toplevel {
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            rect,
            style_mask | NSWindowStyleMask::NonactivatingPanel,
            NSBackingStoreType::Buffered,
            false,
        );

        unsafe {
            panel.setReleasedWhenClosed(false);
        }

        return panel.into_super();
    }

    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            style_mask,
            NSBackingStoreType::Buffered,
            false,
        )
    };

    unsafe {
        window.setReleasedWhenClosed(false);
    }

    window
}

/// Open one window.
pub(crate) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    platform::core::ensure_out(out, "out")?;
    let title = unsafe { options.title.as_str()? };

    // validate caller size inputs
    if options.size_logical.width <= 0.0 || options.size_logical.height <= 0.0 {
        return Err(platform::core::invalid_argument(
            "sizeLogical",
            "window logical width and height must be greater than zero",
        ));
    }

    // validate host-supported option lanes
    geometry::validate_size_constraints(options.constraints, "destack.display.window.open")?;
    options::validate_unsupported_open_options(options)?;

    // resolve the initial target display before touching AppKit state
    let resolved_display = mode::mode_display(options.mode).or(options.display);

    if let Some(display) = resolved_display {
        appkit::resource::resolve_display_id(context, display, "destack.display.window.open")?;
    }

    // apply role-specific host defaults
    let (resolved_chrome, resolved_decorated, resolved_taskbar_visible, resolved_always_on_top) =
        options::resolve_role_open_defaults(
            options.role,
            options.chrome,
            options.decorated,
            options.taskbar_visible,
            options.always_on_top,
        );

    let mut resolved_options = options;
    resolved_options.chrome = resolved_chrome;
    resolved_options.decorated = resolved_decorated;
    resolved_options.taskbar_visible = resolved_taskbar_visible;
    resolved_options.always_on_top = resolved_always_on_top;
    resolved_options.display = resolved_display;

    // register the runtime-visible host state before creating the host window
    let host_state = Arc::new(Mutex::new(options::initial_host_state(
        &resolved_options,
        title,
    )));
    let entry = appkit::resource::window_resource_entry(Arc::clone(&host_state));
    let resource_id =
        context
            .worker()
            .resources
            .insert(context.world(), entry, Some(context.engine()));
    let window_handle = resource::WindowHandle(resource_id);
    let runtime_state = appkit::core::runtime_state(context);
    let host_state_for_delegate = Arc::clone(&host_state);
    let runtime_state_for_delegate = Arc::clone(&runtime_state);
    let initial_display_frame = if let Some(display) = resolved_options.display {
        Some(mode::display_frame(
            context,
            display,
            "destack.display.window.open",
        )?)
    } else {
        None
    };

    // create and configure the native window on the AppKit main thread
    with_process_main_context_marker_if_needed(|mtm| {
        let application = NSApplication::sharedApplication(mtm);
        let window = create_native_window(mtm, &resolved_options);
        let native_title = NSString::from_str(title);
        window.setTitle(&native_title);
        let mut collection_behavior =
            NSWindowCollectionBehavior::Managed | NSWindowCollectionBehavior::FullScreenPrimary;

        if resolved_options.taskbar_visible {
            collection_behavior.insert(NSWindowCollectionBehavior::ParticipatesInCycle);
            window.setExcludedFromWindowsMenu(false);
        } else {
            collection_behavior.insert(NSWindowCollectionBehavior::IgnoresCycle);
            window.setExcludedFromWindowsMenu(true);
        }

        if resolved_options.role != WindowRole::Toplevel {
            collection_behavior.insert(NSWindowCollectionBehavior::Transient);
        }

        if resolved_options.role == WindowRole::Overlay {
            collection_behavior.insert(NSWindowCollectionBehavior::CanJoinAllSpaces);
            collection_behavior.insert(NSWindowCollectionBehavior::Stationary);
        }

        window.setCollectionBehavior(collection_behavior);
        window.setOpaque(!resolved_options.transparent);
        window.setAlphaValue(resolved_options.opacity.unwrap_or(DEFAULT_WINDOW_OPACITY));
        window.setIgnoresMouseEvents(resolved_options.mouse_passthrough.unwrap_or(false));
        window.setCanHide(resolved_options.taskbar_visible);

        // apply z-order policy
        if resolved_options.always_on_top {
            window.setLevel(kCGFloatingWindowLevel as NSWindowLevel);
        } else {
            window.setLevel(kCGNormalWindowLevel as NSWindowLevel);
        }

        // apply aspect ratio and size constraints
        if let Some(aspect_ratio) = resolved_options.aspect_ratio {
            window.setContentAspectRatio(NSSize::new(
                aspect_ratio.numerator as f64,
                aspect_ratio.denominator as f64,
            ));
        }

        if let Some(constraints) = resolved_options.constraints {
            if let Some(minimum) = constraints.min {
                window.setContentMinSize(NSSize::new(minimum.width, minimum.height));
            }

            if let Some(maximum) = constraints.max {
                window.setContentMaxSize(NSSize::new(maximum.width, maximum.height));
            }
        }

        // place the window onto the requested display
        if let Some(frame) = initial_display_frame {
            if matches!(
                resolved_options.mode,
                WindowModeOptions::WindowBorderlessModeOptions(_)
            ) {
                window.setFrame_display(frame, true);
            } else if resolved_options.position.is_none() {
                window.setFrameOrigin(frame.origin);
            }
        }

        // install delegate and drag-and-drop registration before showing the window
        let delegate = AppKitWindowDelegate::new(
            mtm,
            Arc::clone(&runtime_state_for_delegate),
            window_handle,
            Arc::clone(&host_state_for_delegate),
        );
        let protocol: &ProtocolObject<dyn objc2_app_kit::NSWindowDelegate> = delegate.as_protocol();
        window.setDelegate(Some(protocol));
        let drag_types = NSArray::from_slice(&[unsafe { NSPasteboardTypeFileURL }, unsafe {
            NSPasteboardTypeString
        }]);
        window.registerForDraggedTypes(&drag_types);

        // apply initial visibility
        if resolved_options.visibility == WindowVisibility::Visible {
            window.makeKeyAndOrderFront(None);
        } else if resolved_options.visibility == WindowVisibility::Minimized {
            window.makeKeyAndOrderFront(None);
            window.miniaturize(None);
        } else {
            window.orderOut(None);
        }

        // apply initial borderless fullscreen after the window becomes known to AppKit
        if matches!(
            resolved_options.mode,
            WindowModeOptions::WindowBorderlessModeOptions(_)
        ) {
            window.toggleFullScreen(None);
        }

        // activate the application so the new window participates in the desktop session
        #[allow(deprecated)]
        application.activateIgnoringOtherApps(resolved_options.focus_on_show);

        {
            let mut host_state = host_state_for_delegate
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            core::refresh_host_state_geometry(&mut host_state, &window);
        }

        let mut state = runtime_state_for_delegate
            .main_thread_state
            .get(mtm)
            .borrow_mut();
        state.windows.insert(
            window_handle,
            AppKitWindowHost {
                host_state: Arc::clone(&host_state_for_delegate),
                window,
                _delegate: delegate,
                drop_session: Default::default(),
                window_icon: Default::default(),
                text_input: Default::default(),
            },
        );
        Ok(())
    })?;

    // apply parent and modal relationships after the host window exists
    let initial_owner = resolved_options.transient_for.or(resolved_options.parent);
    let initial_modal = resolved_options.modal.unwrap_or(false);

    if let Err(error) = relation::apply_host_relationship_state(
        &runtime_state,
        window_handle,
        None,
        false,
        initial_owner,
        initial_modal,
        "destack.display.window.open",
    ) {
        appkit::core::with_main_thread_state(&runtime_state, |state| {
            let mut state = state.borrow_mut();

            if let Some(host) = state.windows.remove(&window_handle) {
                drop::cancel_active_drop_session(&runtime_state, window_handle, &host);
                host.window.unregisterDraggedTypes();
                host.window.close();
            }
        });

        drop(context.worker().resources.remove(
            context.world(),
            window_handle.0,
            Some(context.engine()),
        ));

        return Err(error);
    }

    // reconcile the host state, then publish creation side effects
    reconcile::refresh_host_window_binding(&runtime_state, window_handle)?;

    event::publish_window_created(&runtime_state, window_handle);
    apply_cursor_policy(&runtime_state);

    unsafe {
        *out = window_handle;
    }

    Ok(())
}

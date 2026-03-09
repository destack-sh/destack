use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowPhysicalSize, WindowPosition, WindowSizeConstraints,
};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::core::frame_top_left_point_from_desktop_position;
use super::reconcile;
use crate::platform::display::unix::appkit::event::publish_state_deltas;
use crate::platform::display::unix::appkit::{core as appkit_core, resource as display_resource};

const UNBOUNDED_WINDOW_SIZE: f64 = 10_000_000.0;

/// Validate one optional size-constraint payload.
pub(crate) fn validate_size_constraints(
    constraints: Option<WindowSizeConstraints>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(constraints) = constraints else {
        return Ok(());
    };

    if let Some(minimum) = constraints.min
        && (minimum.width <= 0.0 || minimum.height <= 0.0)
    {
        return Err(core_platform::invalid_argument(
            "constraints",
            format!("{operation}: minimum logical size must be greater than zero"),
        ));
    }

    if let Some(maximum) = constraints.max
        && (maximum.width <= 0.0 || maximum.height <= 0.0)
    {
        return Err(core_platform::invalid_argument(
            "constraints",
            format!("{operation}: maximum logical size must be greater than zero"),
        ));
    }

    Ok(())
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setPosition",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.position = position;
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setPosition",
        |host| {
            let top_left = frame_top_left_point_from_desktop_position(position);
            host.window.setFrameTopLeftPoint(top_left);
            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state(&runtime_state, window_handle)
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    validate_size_constraints(constraints, "destack.display.window.setSizeConstraints")?;

    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    host_state.constraints = constraints;
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setSizeConstraints",
        |host| {
            if let Some(constraints) = constraints {
                if let Some(minimum) = constraints.min {
                    host.window
                        .setContentMinSize(NSSize::new(minimum.width, minimum.height));
                } else {
                    host.window.setContentMinSize(NSSize::new(0.0, 0.0));
                }

                if let Some(maximum) = constraints.max {
                    host.window
                        .setContentMaxSize(NSSize::new(maximum.width, maximum.height));
                } else {
                    host.window.setContentMaxSize(NSSize::new(
                        UNBOUNDED_WINDOW_SIZE,
                        UNBOUNDED_WINDOW_SIZE,
                    ));
                }
            } else {
                host.window.setContentMinSize(NSSize::new(0.0, 0.0));
                host.window
                    .setContentMaxSize(NSSize::new(UNBOUNDED_WINDOW_SIZE, UNBOUNDED_WINDOW_SIZE));
            }
            Ok(())
        },
    )?;

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            "size",
            "window logical width and height must be greater than zero",
        ));
    }

    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setSizeLogical",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let scale_factor = (host_state.scale_factor_milli as f64 / 1000.0).max(1.0);
    host_state.size_logical = size;
    host_state.size_physical = crate::platform::display::WindowPhysicalSize {
        width: (size.width * scale_factor).round().max(1.0) as u32,
        height: (size.height * scale_factor).round().max(1.0) as u32,
    };
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setSizeLogical",
        |host| {
            let current_frame = host.window.frame();
            let current_top_left = NSPoint::new(
                current_frame.origin.x,
                current_frame.origin.y + current_frame.size.height,
            );
            let desired_content =
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size.width, size.height));
            let mut desired_frame = host.window.frameRectForContentRect(desired_content);
            desired_frame.origin = NSPoint::new(
                current_top_left.x,
                current_top_left.y - desired_frame.size.height,
            );

            host.window.setFrame_display(desired_frame, true);
            Ok(())
        },
    )?;

    reconcile::reconcile_host_window_state(&runtime_state, window_handle)?;

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    if size.width == 0 || size.height == 0 {
        return Err(core_platform::invalid_argument(
            "size",
            "window physical width and height must be greater than zero",
        ));
    }

    let runtime_state = appkit_core::runtime_state(context);
    let scale_factor = appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setSizePhysical",
        |host| Ok(host.window.backingScaleFactor()),
    )?;
    let scale_factor = scale_factor.max(1.0);
    let logical = WindowLogicalSize {
        width: (size.width as f64) / scale_factor,
        height: (size.height as f64) / scale_factor,
    };

    unsafe { window_set_size_logical(context, window_handle, logical) }
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn window_set_aspect_ratio(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    aspect_ratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    if let Some(aspect_ratio) = aspect_ratio
        && (aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must be greater than zero",
        ));
    }

    let runtime_state = appkit_core::runtime_state(context);
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setAspectRatio",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.aspect_ratio = aspect_ratio;
    let next = host_state.clone();
    drop(host_state);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setAspectRatio",
        |host| {
            if let Some(aspect_ratio) = aspect_ratio {
                host.window.setContentAspectRatio(NSSize::new(
                    aspect_ratio.numerator as f64,
                    aspect_ratio.denominator as f64,
                ));
            } else {
                host.window.setContentAspectRatio(NSSize::new(0.0, 0.0));
            }
            Ok(())
        },
    )?;

    publish_state_deltas(&runtime_state, window_handle, &previous, &next);

    Ok(())
}

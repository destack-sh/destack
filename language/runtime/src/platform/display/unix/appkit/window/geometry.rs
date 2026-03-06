use objc2_foundation::{NSPoint, NSSize};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowPhysicalSize, WindowPosition, WindowSizeConstraints,
};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::event::{publish_state_deltas, publish_window_position_changed};
use super::super::{core as appkit_core, resource as display_resource};
use super::{reconcile, runtime};

const UNBOUNDED_WINDOW_SIZE: f64 = 10_000_000.0;

/// Validate one optional size-constraint payload.
pub(crate) fn validate_size_constraints(
    constraints: Option<WindowSizeConstraints>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(constraints) = constraints else {
        return Ok(());
    };

    if let Some(minimum) = constraints.min {
        if minimum.width <= 0.0 || minimum.height <= 0.0 {
            return Err(core_platform::invalid_argument(
                "constraints",
                format!("{operation}: minimum logical size must be greater than zero"),
            ));
        }
    }

    if let Some(maximum) = constraints.max {
        if maximum.width <= 0.0 || maximum.height <= 0.0 {
            return Err(core_platform::invalid_argument(
                "constraints",
                format!("{operation}: maximum logical size must be greater than zero"),
            ));
        }
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
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setPosition",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setPosition")?;
    let previous_position = binding.position;
    binding.position = position;
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setPosition",
        |host| {
            host.window
                .setFrameOrigin(NSPoint::new(position.x as f64, position.y as f64));
            Ok(())
        },
    )?;

    publish_window_position_changed(&runtime_state, window_handle, previous_position, position);
    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    validate_size_constraints(constraints, "destack.display.window.setSizeConstraints")?;

    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setSizeConstraints")?;
    binding.constraints = constraints;
    drop(binding);

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
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setSizeLogical",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setSizeLogical")?;
    drop(binding);

    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setSizeLogical",
        |host| {
            host.window
                .setContentSize(NSSize::new(size.width, size.height));
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
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setAspectRatio",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setAspectRatio")?;
    let previous = binding.clone();
    binding.aspect_ratio = aspect_ratio;
    let next = binding.clone();
    drop(binding);

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

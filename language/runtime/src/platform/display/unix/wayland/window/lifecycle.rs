use std::sync::{Arc, Mutex};

use wayland_client::Proxy;
use wayland_client::backend::ObjectId;
use wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1,
};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1;
use wayland_protocols::xdg::shell::client::{xdg_popup, xdg_positioner, xdg_toplevel};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowModeOptions, WindowOptions,
    WindowPosition, WindowRole, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::model::{WaylandWindowBinding, WaylandWindowHost};
use super::super::{core as backend_core, event, monitor, resource as display_resource};
use super::{
    clamp_logical_aspect, clamp_logical_size, decoration_mode_for_window, drop,
    logical_to_physical, mode_display, mode_display_mode, normalize_logical_size,
    normalize_opacity, opacity_multiplier, resolve_window_binding, validate_size_constraints,
};

/// Resolve role-specific defaults for one wayland window open request.
fn resolve_role_open_defaults(
    role: WindowRole,
    taskbar_visible: bool,
    always_on_top: bool,
) -> (bool, bool) {
    // keep explicit caller values for top-level windows
    if role == WindowRole::Toplevel {
        return (taskbar_visible, always_on_top);
    }

    // popup and overlay roles are transient or layered surfaces by construction
    if role == WindowRole::Popup {
        return (false, false);
    }

    (false, true)
}

/// Validate unsupported open-option lanes for this wayland backend.
fn validate_unsupported_open_options(options: WindowOptions) -> RuntimeResult<()> {
    // reject hidden open visibility: xdg-shell has no portable hidden lane
    if options.visibility == WindowVisibility::Hidden {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit desktop placement requests: wayland toplevel placement is compositor-owned
    if options.position.is_some() {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit always-on-top requests for non-overlay roles
    if options.always_on_top && options.role != WindowRole::Overlay {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject explicit taskbar policy requests for top-level roles
    if !options.taskbar_visible && options.role == WindowRole::Toplevel {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // reject initial aspect-ratio locks: no standard xdg-shell request lane exists
    if let Some(aspect_ratio) = options.aspect_ratio {
        if aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0 {
            return Err(core_platform::invalid_argument(
                "options.aspectRatio",
                "aspect ratio numerator and denominator must be greater than zero",
            ));
        }

        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    Ok(())
}

/// Resolve one scale-factor value from one optional display selection.
fn resolve_scale_factor_milli(
    context: &BindingCallContext,
    display: Option<resource::DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<u32> {
    // return display-specific scale when display handle is configured
    if let Some(display) = display {
        let snapshot = monitor::snapshot_by_display_handle(context, display, operation)?;
        return Ok(snapshot.descriptor.scale_factor_milli.max(1));
    }

    // otherwise use primary-monitor scale when available
    let snapshots = monitor::enumerate_monitor_snapshots(context)?;
    if let Some(snapshot) = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary)
    {
        return Ok(snapshot.descriptor.scale_factor_milli.max(1));
    }

    Ok(1000)
}

/// Resolve one initial display selection from window options.
fn resolve_initial_display(
    options: WindowOptions,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    let mode_display = mode_display(options.mode);

    // reject conflicting explicit display selections
    if let (Some(display), Some(mode_display)) = (options.display, mode_display)
        && display != mode_display
    {
        return Err(core_platform::invalid_argument(
            "options",
            "options.display must match options.mode display when both are set",
        ));
    }

    Ok(mode_display.or(options.display))
}

/// Validate one initial mode payload.
fn validate_initial_mode(
    context: &BindingCallContext,
    mode: WindowModeOptions,
    display: Option<resource::DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject exclusive fullscreen on wayland: this backend only supports compositor fullscreen
    if matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // resolve display handle when configured
    let Some(display) = display else {
        return Ok(());
    };
    let snapshot = monitor::snapshot_by_display_handle(context, display, operation)?;

    // validate optional exclusive mode payload against host-reported modes
    let Some(display_mode) = mode_display_mode(mode) else {
        return Ok(());
    };

    if !snapshot.modes.contains(&display_mode) {
        return Err(core_platform::invalid_argument(
            "mode",
            "display mode is not supported by the target display",
        ));
    }

    Ok(())
}

/// Resolve one optional parent toplevel id for one new window.
#[derive(Clone)]
struct ParentRoleIds {
    /// Parent xdg-surface object id.
    xdg_surface: Option<ObjectId>,
    /// Parent xdg-toplevel object id.
    xdg_toplevel: Option<ObjectId>,
    /// Parent layer-surface object id.
    layer_surface: Option<ObjectId>,
}

/// Resolve one optional parent host object set for one new window.
fn resolve_parent_roles(
    context: &BindingCallContext,
    parent: Option<resource::WindowHandle>,
    transient_for: Option<resource::WindowHandle>,
    operation: &'static str,
) -> RuntimeResult<Option<ParentRoleIds>> {
    let owner = transient_for.or(parent);
    let Some(owner) = owner else {
        return Ok(None);
    };

    // resolve owner window binding and enforce owner-thread affinity
    let owner_binding = resolve_window_binding(context, owner, operation)?;
    let owner_binding = owner_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    Ok(Some(ParentRoleIds {
        xdg_surface: owner_binding.host.xdg_surface.clone(),
        xdg_toplevel: owner_binding.host.xdg_toplevel.clone(),
        layer_surface: owner_binding.host.layer_surface.clone(),
    }))
}

/// Apply one requested mode payload to one xdg_toplevel.
fn apply_mode(
    mode: WindowModeOptions,
    toplevel: &xdg_toplevel::XdgToplevel,
    output: Option<wayland_client::protocol::wl_output::WlOutput>,
) {
    match mode {
        // clear compositor fullscreen and maximize lanes
        WindowModeOptions::WindowWindowedModeOptions(_) => {
            toplevel.unset_fullscreen();
            toplevel.unset_maximized();
        }
        // enter compositor fullscreen lane with optional output preference
        WindowModeOptions::WindowBorderlessModeOptions(_) => {
            toplevel.set_fullscreen(output.as_ref());
        }
        // map exclusive requests to compositor fullscreen lane
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_) => {
            toplevel.set_fullscreen(output.as_ref());
        }
    }
}

/// Apply one requested visibility payload to one xdg_toplevel.
fn apply_visibility(visibility: WindowVisibility, toplevel: &xdg_toplevel::XdgToplevel) {
    match visibility {
        // request minimization for minimized lane
        WindowVisibility::Minimized => {
            toplevel.set_minimized();
        }
        // hidden open requests are rejected before this helper
        WindowVisibility::Hidden => {}
        // request maximization for maximized lane
        WindowVisibility::Maximized => {
            toplevel.set_maximized();
        }
        // clear maximize lane for visible windowed presentation
        WindowVisibility::Visible => {
            toplevel.unset_maximized();
        }
    }
}

/// Created host object ids for one window-open request.
struct CreatedWindowHostIds {
    /// Created wl-surface object id.
    surface: ObjectId,
    /// Created xdg-surface object id when role requires it.
    xdg_surface: Option<ObjectId>,
    /// Created xdg-toplevel object id when role requires it.
    xdg_toplevel: Option<ObjectId>,
    /// Created xdg-popup object id when role requires it.
    xdg_popup: Option<ObjectId>,
    /// Created layer-surface object id when role requires it.
    layer_surface: Option<ObjectId>,
    /// Created xdg-decoration object id when requested.
    xdg_decoration: Option<ObjectId>,
    /// Created xdg-dialog object id when requested.
    xdg_dialog: Option<ObjectId>,
    /// Created alpha-modifier surface object id when requested.
    alpha_modifier_surface: Option<ObjectId>,
    /// Created fractional-scale object id when requested.
    fractional_scale: Option<ObjectId>,
    /// Created viewport object id when requested.
    viewport: Option<ObjectId>,
}

/// Create one wayland surface-role host object set for one binding.
fn create_window_host(
    context: &BindingCallContext,
    binding: &Arc<Mutex<WaylandWindowBinding>>,
    operation: &'static str,
) -> RuntimeResult<CreatedWindowHostIds> {
    // snapshot mutable window state before opening native host objects
    let binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());
    let display_handle = binding_guard.display;
    let (
        window_id,
        role,
        mode,
        visibility,
        title,
        resizable,
        constraints,
        size_logical,
        size_physical,
        parent_toplevel,
        decorated,
        chrome,
        modal,
        opacity,
    ) = (
        binding_guard.id.clone(),
        binding_guard.role,
        binding_guard.mode,
        binding_guard.visibility,
        binding_guard.title.clone(),
        binding_guard.resizable,
        binding_guard.constraints,
        binding_guard.size_logical,
        binding_guard.size_physical,
        binding_guard.parent,
        binding_guard.decorated,
        binding_guard.chrome,
        binding_guard.modal,
        binding_guard.opacity,
    );
    drop(binding_guard);

    // resolve display identifier from the stored display handle
    let display = display_handle
        .map(|display| display_resource::resolve_display_id(context, display, operation))
        .transpose()?;

    let transient_for = {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        binding.transient_for
    };
    let parent_roles = resolve_parent_roles(context, parent_toplevel, transient_for, operation)?;

    // create wayland objects and apply initial shell requests
    backend_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            let queue_handle = event_queue.handle();

            let compositor = dispatch_state
                .compositor
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            let token = backend_core::WaylandWindowDispatchToken {
                window_id,
                binding: Arc::downgrade(binding),
            };
            let surface = compositor.create_surface(&queue_handle, token.clone());

            let mut ids = CreatedWindowHostIds {
                surface: surface.id(),
                xdg_surface: None,
                xdg_toplevel: None,
                xdg_popup: None,
                layer_surface: None,
                xdg_decoration: None,
                xdg_dialog: None,
                alpha_modifier_surface: None,
                fractional_scale: None,
                viewport: None,
            };

            // attach one optional viewport lane for scale coordination
            if let Some(viewporter) = dispatch_state.viewporter.as_ref().cloned() {
                let viewport = viewporter.get_viewport(&surface, &queue_handle, ());
                let destination_width =
                    size_logical.width.round().clamp(1.0, i32::MAX as f64) as i32;
                let destination_height =
                    size_logical.height.round().clamp(1.0, i32::MAX as f64) as i32;
                viewport.set_destination(destination_width, destination_height);
                ids.viewport = Some(viewport.id());
            }

            // attach one optional fractional-scale lane for per-surface scale events
            if let Some(manager) = dispatch_state.fractional_scale_manager.as_ref().cloned() {
                let fractional_scale =
                    manager.get_fractional_scale(&surface, &queue_handle, token.clone());
                ids.fractional_scale = Some(fractional_scale.id());
            }

            // create one role-specific surface object set
            match role {
                // toplevel role: xdg_toplevel backed window
                WindowRole::Toplevel => {
                    let wm_base = dispatch_state
                        .wm_base
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| core_platform::not_supported(operation))?;
                    let xdg_surface =
                        wm_base.get_xdg_surface(&surface, &queue_handle, token.clone());
                    let toplevel = xdg_surface.get_toplevel(&queue_handle, token.clone());
                    ids.xdg_surface = Some(xdg_surface.id());
                    ids.xdg_toplevel = Some(toplevel.id());

                    // reject unsupported initial decoration and chrome requests
                    let needs_decoration_control =
                        !decorated || chrome != WindowChromeKind::Standard;
                    if needs_decoration_control && dispatch_state.decoration_manager.is_none() {
                        return Err(core_platform::not_supported(operation));
                    }

                    // create one optional xdg-decoration lane before the first commit
                    if let Some(manager) = dispatch_state.decoration_manager.as_ref().cloned() {
                        let decoration = manager.get_toplevel_decoration(
                            &toplevel,
                            &queue_handle,
                            token.clone(),
                        );
                        let mode = decoration_mode_for_window(chrome, decorated);
                        decoration.set_mode(mode);
                        ids.xdg_decoration = Some(decoration.id());
                    }

                    // apply initial title and parent relationships
                    toplevel.set_title(title);

                    if let Some(parent_toplevel_id) = parent_roles
                        .as_ref()
                        .and_then(|roles| roles.xdg_toplevel.clone())
                        && !parent_toplevel_id.is_null()
                        && let Ok(parent_toplevel) = backend_core::resolve_xdg_toplevel(
                            connection,
                            parent_toplevel_id,
                            operation,
                        )
                    {
                        toplevel.set_parent(Some(&parent_toplevel));
                    }

                    // apply one optional initial modal request
                    if modal {
                        let manager = dispatch_state
                            .dialog_manager
                            .as_ref()
                            .cloned()
                            .ok_or_else(|| core_platform::not_supported(operation))?;
                        let dialog = manager.get_xdg_dialog(&toplevel, &queue_handle, ());
                        dialog.set_modal();
                        ids.xdg_dialog = Some(dialog.id());
                    }

                    // apply initial mode and visibility state
                    let output = display
                        .as_deref()
                        .and_then(backend_core::output_global_name_from_display_id)
                        .and_then(|global_name| {
                            dispatch_state.outputs_by_global.get(&global_name).cloned()
                        });
                    apply_mode(mode, &toplevel, output);
                    apply_visibility(visibility, &toplevel);

                    // apply initial resizable and logical-size-constraint policy
                    if !resizable {
                        let width = size_physical.width.min(i32::MAX as u32) as i32;
                        let height = size_physical.height.min(i32::MAX as u32) as i32;
                        toplevel.set_min_size(width.max(1), height.max(1));
                        toplevel.set_max_size(width.max(1), height.max(1));
                    } else if let Some(constraints) = constraints {
                        if let Some(minimum) = constraints.min {
                            let width = minimum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                            let height = minimum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                            toplevel.set_min_size(width, height);
                        }
                        if let Some(maximum) = constraints.max {
                            let width = maximum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                            let height = maximum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                            toplevel.set_max_size(width, height);
                        }
                    }
                }
                // popup role: xdg_popup backed transient surface
                WindowRole::Popup => {
                    let wm_base = dispatch_state
                        .wm_base
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| core_platform::not_supported(operation))?;
                    let parent_surface = parent_roles
                        .as_ref()
                        .and_then(|roles| roles.xdg_surface.clone())
                        .filter(|value| !value.is_null())
                        .map(|parent_surface_id| {
                            backend_core::resolve_xdg_surface(
                                connection,
                                parent_surface_id,
                                operation,
                            )
                        })
                        .transpose()?;
                    let parent_layer_id = parent_roles
                        .as_ref()
                        .and_then(|roles| roles.layer_surface.clone())
                        .filter(|value| !value.is_null());
                    if parent_surface.is_none() && parent_layer_id.is_none() {
                        return Err(core_platform::invalid_argument(
                            "options",
                            "popup windows require one parent or transientFor window",
                        ));
                    }

                    let width = size_physical.width.min(i32::MAX as u32) as i32;
                    let height = size_physical.height.min(i32::MAX as u32) as i32;
                    let positioner = wm_base.create_positioner(&queue_handle, ());
                    positioner.set_size(width.max(1), height.max(1));
                    positioner.set_anchor_rect(0, 0, width.max(1), height.max(1));
                    positioner.set_anchor(xdg_positioner::Anchor::TopLeft);
                    positioner.set_gravity(xdg_positioner::Gravity::TopLeft);
                    positioner.set_constraint_adjustment(
                        xdg_positioner::ConstraintAdjustment::SlideX
                            | xdg_positioner::ConstraintAdjustment::SlideY
                            | xdg_positioner::ConstraintAdjustment::FlipX
                            | xdg_positioner::ConstraintAdjustment::FlipY,
                    );

                    let xdg_surface =
                        wm_base.get_xdg_surface(&surface, &queue_handle, token.clone());
                    let popup = xdg_surface.get_popup(
                        parent_surface.as_ref(),
                        &positioner,
                        &queue_handle,
                        token.clone(),
                    );
                    ids.xdg_surface = Some(xdg_surface.id());
                    ids.xdg_popup = Some(popup.id());

                    // attach popup to layer parent when parent role is one layer surface
                    if let Some(parent_layer_id) = parent_layer_id
                        && let Ok(parent_layer_surface) =
                            zwlr_layer_surface_v1::ZwlrLayerSurfaceV1::from_id(
                                connection,
                                parent_layer_id,
                            )
                    {
                        parent_layer_surface.get_popup(&popup);
                    }
                }
                // overlay role: layer-shell backed surface
                WindowRole::Overlay => {
                    let manager = dispatch_state
                        .layer_shell_manager
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| core_platform::not_supported(operation))?;
                    let output = display
                        .as_deref()
                        .and_then(backend_core::output_global_name_from_display_id)
                        .and_then(|global_name| {
                            dispatch_state.outputs_by_global.get(&global_name).cloned()
                        });
                    let layer_surface = manager.get_layer_surface(
                        &surface,
                        output.as_ref(),
                        zwlr_layer_shell_v1::Layer::Overlay,
                        String::from("destack"),
                        &queue_handle,
                        token.clone(),
                    );
                    layer_surface.set_size(size_physical.width.max(1), size_physical.height.max(1));
                    ids.layer_surface = Some(layer_surface.id());
                }
            }

            // apply one optional initial surface opacity through alpha-modifier
            if (opacity - 1.0).abs() > f64::EPSILON {
                let manager = dispatch_state
                    .alpha_modifier_manager
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;
                let alpha_surface = manager.get_surface(&surface, &queue_handle, ());
                alpha_surface.set_multiplier(opacity_multiplier(opacity));
                ids.alpha_modifier_surface = Some(alpha_surface.id());
            }

            // request one presentation feedback callback for this commit
            backend_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                token.clone(),
            );

            // commit the base surface and flush request bytes
            surface.commit();
            backend_core::flush_queue(event_queue, operation)?;

            Ok(ids)
        },
    )
}

/// Open one wayland window and register it in runtime resources.
pub(crate) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    // validate output pointer and normalize initial options
    core_platform::ensure_out(out, "out")?;
    let title = unsafe { options.title.as_str()?.to_string() };
    let (resolved_taskbar_visible, resolved_always_on_top) =
        resolve_role_open_defaults(options.role, options.taskbar_visible, options.always_on_top);
    let mut size_logical = normalize_logical_size(options.size_logical, "options.sizeLogical")?;
    validate_size_constraints(options.constraints, "options.constraints")?;
    if options.opacity.is_some() {
        normalize_opacity(options.opacity.unwrap_or(1.0), "options.opacity")?;
    }

    // validate unsupported initial option lanes for this backend
    validate_unsupported_open_options(options)?;

    // validate and normalize initial display and mode selections
    let display = resolve_initial_display(options)?;
    validate_initial_mode(
        context,
        options.mode,
        display,
        "destack.display.window.open",
    )?;

    // reject modal requests without any owner relationship
    if options.modal.unwrap_or(false) && options.parent.is_none() && options.transient_for.is_none()
    {
        return Err(core_platform::invalid_argument(
            "options.modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // reject modal requests for non-toplevel roles
    if options.modal.unwrap_or(false) && options.role != WindowRole::Toplevel {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // clamp initial logical size by constraints and optional aspect ratio
    size_logical = clamp_logical_size(size_logical, options.constraints);
    size_logical = clamp_logical_aspect(size_logical, options.aspect_ratio);

    // resolve initial scale factor and derive physical size
    let scale_factor_milli =
        resolve_scale_factor_milli(context, display, "destack.display.window.open")?;
    let size_physical = logical_to_physical(size_logical, scale_factor_milli);

    // allocate one runtime binding before host object creation
    let host_id = backend_core::next_window_host_id(context);
    let window_id = format!("wayland-window-{host_id}");
    let position = options.position.unwrap_or(WindowPosition { x: 0, y: 0 });
    let visibility = options.visibility;
    let focused = false;
    let opacity = options.opacity.unwrap_or(1.0);

    let binding = Arc::new(Mutex::new(WaylandWindowBinding {
        id: window_id.clone(),
        host: WaylandWindowHost {
            id: host_id,
            surface: ObjectId::null(),
            xdg_surface: None,
            xdg_toplevel: None,
            xdg_popup: None,
            layer_surface: None,
            xdg_decoration: None,
            xdg_dialog: None,
            alpha_modifier_surface: None,
            fractional_scale: None,
            viewport: None,
            locked_pointer: None,
            confined_pointer: None,
        },
        owner_thread_id: std::thread::current().id(),
        title,
        role: options.role,
        mode: options.mode,
        display,
        resizable: options.resizable,
        decorated: options.decorated,
        chrome: options.chrome,
        taskbar_visible: resolved_taskbar_visible,
        transparent: options.transparent,
        opacity,
        always_on_top: resolved_always_on_top,
        parent: options.parent,
        transient_for: options.transient_for,
        modal: options.modal.unwrap_or(false),
        mouse_passthrough: options.mouse_passthrough.unwrap_or(false),
        aspect_ratio: options.aspect_ratio,
        visibility,
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        position,
        size_logical,
        size_physical,
        scale_factor_milli,
        focused,
        safe_area_insets: None,
        theme: WindowTheme::Unknown,
        close_requested_emitted: false,
        destroyed_emitted: false,
    }));

    // create host objects and store native object ids in the binding
    let host_ids = create_window_host(context, &binding, "destack.display.window.open")?;
    {
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        binding.host.surface = host_ids.surface;
        binding.host.xdg_surface = host_ids.xdg_surface;
        binding.host.xdg_toplevel = host_ids.xdg_toplevel;
        binding.host.xdg_popup = host_ids.xdg_popup;
        binding.host.layer_surface = host_ids.layer_surface;
        binding.host.xdg_decoration = host_ids.xdg_decoration;
        binding.host.xdg_dialog = host_ids.xdg_dialog;
        binding.host.alpha_modifier_surface = host_ids.alpha_modifier_surface;
        binding.host.fractional_scale = host_ids.fractional_scale;
        binding.host.viewport = host_ids.viewport;
    }

    // register resource entry and backend window-id mapping
    let resource_id = context.runtime().resources.insert(
        display_resource::window_resource_entry(Arc::clone(&binding)),
        Some(context.engine()),
    );
    let window_handle = resource::WindowHandle(resource_id);

    let runtime_state = backend_core::runtime_state(context);
    {
        let mut windows_by_id = runtime_state
            .windows_by_id
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        windows_by_id.insert(window_id.clone(), window_handle);
    }
    {
        let surface_id = {
            let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            binding.host.surface.clone()
        };
        let token = backend_core::WaylandWindowDispatchToken {
            window_id: window_id.clone(),
            binding: Arc::downgrade(&binding),
        };
        let mut window_tokens_by_surface = runtime_state
            .window_tokens_by_surface
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_tokens_by_surface.insert(surface_id, token);
    }

    // publish one created event for the new window
    event::publish_window_created(&runtime_state, window_handle);

    // write output handle for caller
    unsafe {
        *out = window_handle;
    }

    Ok(())
}

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(context, window_handle, "destack.display.window.close")?;
    let (
        window_id,
        host_surface,
        host_xdg_surface,
        host_xdg_toplevel,
        host_xdg_popup,
        host_layer_surface,
        host_xdg_decoration,
        host_xdg_dialog,
        host_alpha_modifier_surface,
        host_fractional_scale,
        host_viewport,
        host_locked_pointer,
        host_confined_pointer,
        should_publish_close_requested,
    ) = {
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        // clear transient drop-session state before closing this window
        if let Err(error) = drop::reset_drop_state(context, &mut binding) {
            return Err(error);
        }

        // capture whether this close path should emit close-requested
        let should_publish_close_requested = !binding.close_requested_emitted;
        binding.close_requested_emitted = true;

        // mark this window as destroyed for this lifetime
        binding.destroyed_emitted = true;

        (
            binding.id.clone(),
            binding.host.surface.clone(),
            binding.host.xdg_surface.clone(),
            binding.host.xdg_toplevel.clone(),
            binding.host.xdg_popup.clone(),
            binding.host.layer_surface.clone(),
            binding.host.xdg_decoration.clone(),
            binding.host.xdg_dialog.clone(),
            binding.host.alpha_modifier_surface.clone(),
            binding.host.fractional_scale.clone(),
            binding.host.viewport.clone(),
            binding.host.locked_pointer.clone(),
            binding.host.confined_pointer.clone(),
            should_publish_close_requested,
        )
    };
    let host_surface_for_destroy = host_surface.clone();

    // destroy native wayland objects for this window
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.close",
        |connection, event_queue, dispatch_state| {
            // clear stale pointer-focus serial lanes before this surface is destroyed
            backend_core::clear_pointer_focus_for_surface(dispatch_state, &host_surface);

            if let Some(host_xdg_dialog) = host_xdg_dialog.clone()
                && !host_xdg_dialog.is_null()
                && let Ok(proxy) = xdg_dialog_v1::XdgDialogV1::from_id(connection, host_xdg_dialog)
            {
                proxy.destroy();
            }

            if let Some(host_xdg_decoration) = host_xdg_decoration.clone()
                && !host_xdg_decoration.is_null()
                && let Ok(proxy) = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                    connection,
                    host_xdg_decoration,
                )
            {
                proxy.destroy();
            }

            if let Some(host_alpha_modifier_surface) = host_alpha_modifier_surface.clone()
                && !host_alpha_modifier_surface.is_null()
                && let Ok(proxy) = wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1::from_id(
                    connection,
                    host_alpha_modifier_surface,
                )
            {
                proxy.destroy();
            }

            if let Some(host_fractional_scale) = host_fractional_scale.clone()
                && !host_fractional_scale.is_null()
                && let Ok(proxy) = wp_fractional_scale_v1::WpFractionalScaleV1::from_id(
                    connection,
                    host_fractional_scale,
                )
            {
                proxy.destroy();
            }

            if let Some(host_viewport) = host_viewport.clone()
                && !host_viewport.is_null()
                && let Ok(proxy) = wp_viewport::WpViewport::from_id(connection, host_viewport)
            {
                proxy.destroy();
            }

            if let Some(host_locked_pointer) = host_locked_pointer.clone()
                && !host_locked_pointer.is_null()
                && let Ok(proxy) = zwp_locked_pointer_v1::ZwpLockedPointerV1::from_id(
                    connection,
                    host_locked_pointer,
                )
            {
                proxy.destroy();
            }

            if let Some(host_confined_pointer) = host_confined_pointer.clone()
                && !host_confined_pointer.is_null()
                && let Ok(proxy) = zwp_confined_pointer_v1::ZwpConfinedPointerV1::from_id(
                    connection,
                    host_confined_pointer,
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_popup) = host_xdg_popup.clone()
                && !host_xdg_popup.is_null()
                && let Ok(proxy) = xdg_popup::XdgPopup::from_id(connection, host_xdg_popup)
            {
                proxy.destroy();
            }

            if let Some(host_layer_surface) = host_layer_surface.clone()
                && !host_layer_surface.is_null()
                && let Ok(proxy) = zwlr_layer_surface_v1::ZwlrLayerSurfaceV1::from_id(
                    connection,
                    host_layer_surface,
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_toplevel) = host_xdg_toplevel.clone()
                && !host_xdg_toplevel.is_null()
                && let Ok(proxy) = backend_core::resolve_xdg_toplevel(
                    connection,
                    host_xdg_toplevel,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            if let Some(host_xdg_surface) = host_xdg_surface.clone()
                && !host_xdg_surface.is_null()
                && let Ok(proxy) = backend_core::resolve_xdg_surface(
                    connection,
                    host_xdg_surface,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            if !host_surface.is_null()
                && let Ok(proxy) = backend_core::resolve_wl_surface(
                    connection,
                    host_surface_for_destroy,
                    "destack.display.window.close",
                )
            {
                proxy.destroy();
            }

            backend_core::flush_queue(event_queue, "destack.display.window.close")?;

            Ok(())
        },
    )?;

    // remove backend window-id mapping
    let runtime_state = backend_core::runtime_state(context);
    {
        let mut windows_by_id = runtime_state
            .windows_by_id
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        windows_by_id.remove(&window_id);
    }
    {
        let mut window_tokens_by_surface = runtime_state
            .window_tokens_by_surface
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_tokens_by_surface.remove(&host_surface);
    }

    // remove resource table entry and reject stale handles
    let removed = context
        .runtime()
        .resources
        .remove(window_handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(backend_core::window_not_found(
            "destack.display.window.close",
            window_handle,
        ));
    }

    // publish one close-requested event when this is the first close path
    if should_publish_close_requested {
        event::publish_window_close_requested(&runtime_state, window_handle);
    }

    // publish one destroyed event for the closed window
    event::publish_window_destroyed(&runtime_state, window_handle);

    Ok(())
}

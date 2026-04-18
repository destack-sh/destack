use std::sync::{Arc, Mutex};

use wayland_client::Proxy;
use wayland_client::backend::ObjectId;
use wayland_protocols::xdg::shell::client::{xdg_positioner, xdg_toplevel};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowModeOptions, WindowOptions,
    WindowPosition, WindowRole, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{
    clamp_logical_aspect, clamp_logical_size, decoration_mode_for_window, logical_to_physical,
    normalize_logical_size, normalize_opacity, opacity_multiplier, resolve_initial_display,
    resolve_role_open_defaults, resolve_scale_factor_milli, resolve_window_host_state,
    validate_initial_mode, validate_size_constraints, validate_unsupported_open_options,
};
use crate::platform::display::unix::wayland::model::{WaylandWindowHost, WaylandWindowHostState};
use crate::platform::display::unix::wayland::{
    core as wayland_core, event, resource as display_resource,
};

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

    // resolve owner window host state and validate relationship
    let owner_host_state = resolve_window_host_state(context, owner, operation)?;
    let owner_host_state = owner_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    Ok(Some(ParentRoleIds {
        xdg_surface: owner_host_state.host.xdg_surface.clone(),
        xdg_toplevel: owner_host_state.host.xdg_toplevel.clone(),
        layer_surface: owner_host_state.host.layer_surface.clone(),
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

/// Create one wayland surface-role host object set for one host state.
fn create_window_host(
    context: &BindingCallContext,
    host_state: &Arc<Mutex<WaylandWindowHostState>>,
    operation: &'static str,
) -> RuntimeResult<CreatedWindowHostIds> {
    // snapshot mutable window state before opening native host objects
    let host_state_guard = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let display_handle = host_state_guard.display;
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
        host_state_guard.id.clone(),
        host_state_guard.role,
        host_state_guard.mode,
        host_state_guard.visibility,
        host_state_guard.title.clone(),
        host_state_guard.resizable,
        host_state_guard.constraints,
        host_state_guard.size_logical,
        host_state_guard.size_physical,
        host_state_guard.parent,
        host_state_guard.decorated,
        host_state_guard.chrome,
        host_state_guard.modal,
        host_state_guard.opacity,
    );
    drop(host_state_guard);

    // resolve display identifier from the stored display handle
    let display = display_handle
        .map(|display| display_resource::resolve_display_id(context, display, operation))
        .transpose()?;

    let transient_for = {
        let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        host_state.transient_for
    };
    let parent_roles = resolve_parent_roles(context, parent_toplevel, transient_for, operation)?;

    // create wayland objects and apply initial shell requests
    wayland_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            let queue_handle = event_queue.handle();

            let compositor = dispatch_state
                .globals
                .compositor
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            let token = wayland_core::WaylandWindowDispatchToken {
                window_id,
                host_state: Arc::downgrade(host_state),
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
            if let Some(viewporter) = dispatch_state.globals.viewporter.as_ref().cloned() {
                let viewport = viewporter.get_viewport(&surface, &queue_handle, ());
                let destination_width =
                    size_logical.width.round().clamp(1.0, i32::MAX as f64) as i32;
                let destination_height =
                    size_logical.height.round().clamp(1.0, i32::MAX as f64) as i32;
                viewport.set_destination(destination_width, destination_height);
                ids.viewport = Some(viewport.id());
            }

            // attach one optional fractional-scale lane for per-surface scale events
            if let Some(manager) = dispatch_state
                .globals
                .fractional_scale_manager
                .as_ref()
                .cloned()
            {
                let fractional_scale =
                    manager.get_fractional_scale(&surface, &queue_handle, token.clone());
                ids.fractional_scale = Some(fractional_scale.id());
            }

            // create one role-specific surface object set
            match role {
                // toplevel role: xdg_toplevel backed window
                WindowRole::Toplevel => {
                    let wm_base = dispatch_state
                        .globals
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
                    if needs_decoration_control
                        && dispatch_state.globals.decoration_manager.is_none()
                    {
                        return Err(core_platform::not_supported(operation));
                    }

                    // create one optional xdg-decoration lane before the first commit
                    if let Some(manager) =
                        dispatch_state.globals.decoration_manager.as_ref().cloned()
                    {
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
                        && let Ok(parent_toplevel) = wayland_core::resolve_xdg_toplevel(
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
                            .globals
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
                        .and_then(wayland_core::output_global_name_from_display_id)
                        .and_then(|global_name| {
                            dispatch_state
                                .output
                                .outputs_by_global
                                .get(&global_name)
                                .cloned()
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
                        .globals
                        .wm_base
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| core_platform::not_supported(operation))?;
                    let parent_surface = parent_roles
                        .as_ref()
                        .and_then(|roles| roles.xdg_surface.clone())
                        .filter(|value| !value.is_null())
                        .map(|parent_surface_id| {
                            wayland_core::resolve_xdg_surface(
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
                        .globals
                        .layer_shell_manager
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| core_platform::not_supported(operation))?;
                    let output = display
                        .as_deref()
                        .and_then(wayland_core::output_global_name_from_display_id)
                        .and_then(|global_name| {
                            dispatch_state
                                .output
                                .outputs_by_global
                                .get(&global_name)
                                .cloned()
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
                    .globals
                    .alpha_modifier_manager
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;
                let alpha_surface = manager.get_surface(&surface, &queue_handle, ());
                alpha_surface.set_multiplier(opacity_multiplier(opacity));
                ids.alpha_modifier_surface = Some(alpha_surface.id());
            }

            // request one presentation feedback callback for this commit
            wayland_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                token.clone(),
            );

            // commit the base surface and flush request bytes
            surface.commit();
            wayland_core::flush_queue(event_queue, operation)?;

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
    let runtime_state = wayland_core::runtime_state(context);

    // allocate one runtime host state before host object creation
    let window_id = format!("wayland-window-{}", runtime_state.next_window_host_id());
    let position = options.position.unwrap_or(WindowPosition { x: 0, y: 0 });
    let visibility = options.visibility;
    let focused = false;
    let opacity = options.opacity.unwrap_or(1.0);

    let host_state = Arc::new(Mutex::new(WaylandWindowHostState {
        id: window_id.clone(),
        host: WaylandWindowHost {
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
        title,
        role: options.role,
        mode: options.mode,
        pending_mode: None,
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

    // create host objects and store native object ids in the host state
    let host_ids = create_window_host(context, &host_state, "destack.display.window.open")?;
    {
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        host_state.host.surface = host_ids.surface;
        host_state.host.xdg_surface = host_ids.xdg_surface;
        host_state.host.xdg_toplevel = host_ids.xdg_toplevel;
        host_state.host.xdg_popup = host_ids.xdg_popup;
        host_state.host.layer_surface = host_ids.layer_surface;
        host_state.host.xdg_decoration = host_ids.xdg_decoration;
        host_state.host.xdg_dialog = host_ids.xdg_dialog;
        host_state.host.alpha_modifier_surface = host_ids.alpha_modifier_surface;
        host_state.host.fractional_scale = host_ids.fractional_scale;
        host_state.host.viewport = host_ids.viewport;
    }

    // register resource entry and backend window-id mapping
    let resource_id = context.worker().resources.insert(
        context.world(),
        display_resource::window_resource_entry(context, Arc::clone(&host_state)),
        Some(context.engine()),
    );
    let window_handle = resource::WindowHandle(resource_id);

    runtime_state.register_window_handle(window_id.clone(), window_handle);

    {
        let surface_id = {
            let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
            host_state.host.surface.clone()
        };
        let token = wayland_core::WaylandWindowDispatchToken {
            window_id: window_id.clone(),
            host_state: Arc::downgrade(&host_state),
        };
        runtime_state.register_window_token(surface_id, token);
    }

    // publish one created event for the new window
    event::publish_window_created(&runtime_state, window_handle);

    // write output handle for caller
    unsafe {
        *out = window_handle;
    }

    Ok(())
}

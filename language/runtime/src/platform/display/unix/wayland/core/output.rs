use std::collections::{BTreeMap, HashMap};

use wayland_client::protocol::wl_output;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::color_management::v1::client::wp_color_management_output_v1;

use crate::platform::display::{DisplayMode, DisplayOrientation};

use super::{
    WAYLAND_DEFAULT_BIT_DEPTH, WAYLAND_DEFAULT_REFRESH_MILLI_HZ, WaylandConnectionDispatchState,
    WaylandWlrAdaptiveSyncState, orientation_from_transform, parse_mode_flags,
};

/// Runtime collector snapshot for one wlr-output-management head object.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandWlrOutputHeadState {
    /// Logical output name advertised by the compositor.
    pub(crate) name: Option<String>,
    /// Enumerated mode object ids for this output head.
    pub(crate) mode_ids: Vec<wayland_client::backend::ObjectId>,
    /// Optional adaptive-sync support state for this output head.
    pub(crate) adaptive_sync: Option<WaylandWlrAdaptiveSyncState>,
}

/// Runtime collector snapshot for one wlr-output-management mode object.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandWlrOutputModeState {
    /// Mode width in physical pixels.
    pub(crate) width: Option<u32>,
    /// Mode height in physical pixels.
    pub(crate) height: Option<u32>,
    /// Mode refresh rate in milli-hertz.
    pub(crate) refresh_milli_hz: Option<u32>,
    /// Whether this mode is compositor preferred.
    pub(crate) is_preferred: bool,
}

/// Runtime collector snapshot for one wl_output global.
#[derive(Debug, Clone)]
pub(crate) struct WaylandOutputSnapshot {
    /// Stable wl_registry global name.
    pub(crate) global_name: u32,
    /// Logical output name from wl_output.name when present.
    pub(crate) logical_name: Option<String>,
    /// Human-readable description from wl_output.description when present.
    pub(crate) description: Option<String>,
    /// Make token reported by geometry events.
    pub(crate) make: Option<String>,
    /// Model token reported by geometry events.
    pub(crate) model: Option<String>,
    /// Desktop-space x coordinate.
    pub(crate) x: i32,
    /// Desktop-space y coordinate.
    pub(crate) y: i32,
    /// Physical width in millimeters.
    pub(crate) width_mm: u32,
    /// Physical height in millimeters.
    pub(crate) height_mm: u32,
    /// Output scale factor.
    pub(crate) scale_factor: u32,
    /// Output transform orientation.
    pub(crate) orientation: DisplayOrientation,
    /// Enumerated mode list.
    pub(crate) modes: Vec<DisplayMode>,
    /// Current mode when reported.
    pub(crate) current_mode: Option<DisplayMode>,
    /// Desktop-preferred mode when reported.
    pub(crate) desktop_mode: Option<DisplayMode>,
}

impl WaylandOutputSnapshot {
    /// Create one empty output snapshot for one wl_registry global.
    pub(crate) fn from_global_name(global_name: u32) -> Self {
        Self {
            global_name,
            logical_name: None,
            description: None,
            make: None,
            model: None,
            x: 0,
            y: 0,
            width_mm: 0,
            height_mm: 0,
            scale_factor: 1,
            orientation: DisplayOrientation::Landscape,
            modes: Vec::new(),
            current_mode: None,
            desktop_mode: None,
        }
    }
}

/// Mutable output and topology state for one wayland connection lane.
#[derive(Debug, Default)]
pub(crate) struct WaylandOutputState {
    /// Output head snapshots keyed by head object id.
    pub(crate) wlr_output_heads_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandWlrOutputHeadState>,
    /// Output mode snapshots keyed by mode object id.
    pub(crate) wlr_output_modes_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandWlrOutputModeState>,
    /// Bound color-management output objects keyed by wl_output global name.
    pub(crate) color_outputs_by_global:
        HashMap<u32, wp_color_management_output_v1::WpColorManagementOutputV1>,
    /// Bound output globals keyed by registry name.
    pub(crate) outputs_by_global: HashMap<u32, wl_output::WlOutput>,
    /// Output snapshots keyed by wl_registry global name.
    pub(crate) output_snapshots_by_global: BTreeMap<u32, WaylandOutputSnapshot>,
    /// Whether output topology changed during the last dispatch turn.
    pub(crate) monitor_topology_dirty: bool,
}

impl WaylandOutputState {
    /// Mark monitor topology as dirty for this dispatch turn.
    pub(crate) fn mark_monitor_topology_dirty(&mut self) {
        self.monitor_topology_dirty = true;
    }

    /// Take and clear the monitor-topology dirty flag.
    pub(crate) fn take_monitor_topology_dirty(&mut self) -> bool {
        let is_dirty = self.monitor_topology_dirty;
        self.monitor_topology_dirty = false;
        is_dirty
    }
}

impl Dispatch<wl_output::WlOutput, u32> for WaylandConnectionDispatchState {
    /// Handle wl_output events.
    fn event(
        state: &mut Self,
        _proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        output_name: &u32,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // any wl_output event can change the published topology snapshot
        state.output.mark_monitor_topology_dirty();

        let Some(output) = state.output.output_snapshots_by_global.get_mut(output_name) else {
            return;
        };

        match event {
            // capture geometry metadata and orientation
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel: _,
                make,
                model,
                transform,
            } => {
                output.x = x;
                output.y = y;
                output.width_mm = physical_width.max(0) as u32;
                output.height_mm = physical_height.max(0) as u32;
                output.make = if make.is_empty() { None } else { Some(make) };
                output.model = if model.is_empty() { None } else { Some(model) };
                output.orientation = orientation_from_transform(transform);
            }
            // capture mode metadata and current or preferred mode markers
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                if width <= 0 || height <= 0 {
                    return;
                }

                let refresh_milli_hz = if refresh <= 0 {
                    WAYLAND_DEFAULT_REFRESH_MILLI_HZ
                } else {
                    refresh as u32
                };
                let mode = DisplayMode {
                    width: width as u32,
                    height: height as u32,
                    refresh_milli_hz,
                    format: 0,
                    bit_depth: WAYLAND_DEFAULT_BIT_DEPTH,
                };
                if !output.modes.contains(&mode) {
                    output.modes.push(mode);
                }

                let (is_current, is_preferred) = parse_mode_flags(flags);
                if is_current {
                    output.current_mode = Some(mode);
                }
                if is_preferred {
                    output.desktop_mode = Some(mode);
                }
            }
            // capture scale updates
            wl_output::Event::Scale { factor } => {
                output.scale_factor = factor.max(1) as u32;
            }
            // capture logical names
            wl_output::Event::Name { name } => {
                if !name.is_empty() {
                    output.logical_name = Some(name);
                }
            }
            // capture human-readable descriptions
            wl_output::Event::Description { description } => {
                if !description.is_empty() {
                    output.description = Some(description);
                }
            }
            _ => {}
        }
    }
}

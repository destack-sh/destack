#[cfg(any(unix, windows))]
use std::sync::{Arc, OnceLock};

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
#[cfg(any(unix, windows))]
use crate::diagnostic::RuntimeResult;
#[cfg(any(unix, windows))]
use crate::runtime::BindingCallContext;
#[cfg(any(unix, windows))]
use crate::runtime::process::service::CachedServiceHandle;

#[cfg(unix)]
use super::host::{
    UnixInputMonitorRuntimeState, UnixInputMonitorService, unix_input_monitor_service,
};
#[cfg(windows)]
use super::host::{
    WindowsRawInputRuntimeState, WindowsRawInputService, WindowsXInputService,
    windows_raw_input_service, windows_xinput_service,
};

/// Agent-owned input module state.
#[derive(Default)]
pub(crate) struct PlatformInputState {
    /// Shared unix input-monitor service handle for this agent.
    #[cfg(unix)]
    unix_input_monitor_service: CachedServiceHandle<UnixInputMonitorService>,
    /// Agent-owned unix input-monitor state.
    #[cfg(unix)]
    unix_input_monitor_runtime_state: OnceLock<Arc<UnixInputMonitorRuntimeState>>,
    /// Shared windows raw-input service handle for this agent.
    #[cfg(windows)]
    windows_raw_input_service: CachedServiceHandle<WindowsRawInputService>,
    /// Shared windows xinput packet service handle for this agent.
    #[cfg(windows)]
    windows_xinput_service: CachedServiceHandle<WindowsXInputService>,
    /// Agent-owned windows raw-input state.
    #[cfg(windows)]
    windows_raw_input_runtime_state: OnceLock<Arc<WindowsRawInputRuntimeState>>,
}

impl std::fmt::Debug for PlatformInputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformInputState")
            .finish_non_exhaustive()
    }
}

impl PlatformInputState {
    /// Return whether any agent-owned input state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(unix)]
        if self.unix_input_monitor_runtime_state.get().is_some() {
            return true;
        }

        #[cfg(windows)]
        if self.windows_raw_input_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one input-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformInputImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformInputImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.input".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

    /// Return one shared unix input-monitor service handle for this agent.
    #[cfg(unix)]
    pub(crate) fn unix_input_monitor_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<UnixInputMonitorService>> {
        self.unix_input_monitor_service
            .get_or_try_init(|| unix_input_monitor_service(operation))
    }

    /// Return one agent-owned unix input-monitor state.
    #[cfg(unix)]
    pub(crate) fn unix_input_monitor_runtime_state(
        &self,
        ctx: &BindingCallContext,
    ) -> Arc<UnixInputMonitorRuntimeState> {
        Arc::clone(
            self.unix_input_monitor_runtime_state
                .get_or_init(|| Arc::new(UnixInputMonitorRuntimeState::new(ctx.agent().id))),
        )
    }

    /// Return one shared windows raw-input service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn windows_raw_input_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsRawInputService>> {
        self.windows_raw_input_service
            .get_or_try_init(|| windows_raw_input_service(operation))
    }

    /// Return one shared windows xinput packet service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn windows_xinput_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsXInputService>> {
        self.windows_xinput_service
            .get_or_try_init(|| windows_xinput_service(operation))
    }

    /// Return one agent-owned windows raw-input state.
    #[cfg(windows)]
    pub(crate) fn windows_raw_input_runtime_state(
        &self,
        ctx: &BindingCallContext,
    ) -> Arc<WindowsRawInputRuntimeState> {
        Arc::clone(
            self.windows_raw_input_runtime_state
                .get_or_init(|| Arc::new(WindowsRawInputRuntimeState::new(ctx.agent().id))),
        )
    }
}

/// Materialized input platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformInputImage;

impl Capture for PlatformInputState {
    type Image = PlatformInputImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one input platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one input platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}

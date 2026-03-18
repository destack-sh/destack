use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::runtime::process::service::ServiceHandle;

#[cfg(target_os = "linux")]
use super::bluetooth::{LinuxBluetoothService, linux_bluetooth_service};
#[cfg(target_os = "linux")]
use super::camera::{LinuxCameraWatchService, linux_camera_watch_service};
#[cfg(target_os = "macos")]
use super::camera::{MacosCameraWatchService, macos_camera_watch_service};
#[cfg(unix)]
use super::serial::{UnixSerialWatchService, unix_serial_watch_service};
#[cfg(windows)]
use super::serial::{WindowsSerialService, windows_serial_service};
use super::usb::{UsbService, usb_service};

/// Agent-owned device module state.
#[derive(Default)]
pub(crate) struct PlatformDeviceState {
    /// Shared Linux BlueZ service handle for this agent.
    #[cfg(target_os = "linux")]
    linux_bluetooth_service: ServiceHandle<LinuxBluetoothService>,
    /// Shared Linux camera topology service handle for this agent.
    #[cfg(target_os = "linux")]
    linux_camera_watch_service: ServiceHandle<LinuxCameraWatchService>,
    /// Shared macOS camera topology service handle for this agent.
    #[cfg(target_os = "macos")]
    macos_camera_watch_service: ServiceHandle<MacosCameraWatchService>,
    /// Shared unix serial topology service handle for this agent.
    #[cfg(unix)]
    unix_serial_watch_service: ServiceHandle<UnixSerialWatchService>,
    /// Shared Windows serial ingress service handle for this agent.
    #[cfg(windows)]
    windows_serial_service: ServiceHandle<WindowsSerialService>,
    /// Shared USB service handle for this agent.
    usb_service: ServiceHandle<UsbService>,
    /// Number of live device resources that currently hold runtime activity.
    runtime_activity_count: Arc<AtomicUsize>,
}

impl std::fmt::Debug for PlatformDeviceState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformDeviceState")
            .finish_non_exhaustive()
    }
}

impl PlatformDeviceState {
    /// Return one shared Linux BlueZ service handle for this agent.
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_bluetooth_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<LinuxBluetoothService>> {
        self.linux_bluetooth_service
            .get_or_try_init(|| linux_bluetooth_service(operation))
    }

    /// Return one shared Linux camera topology service handle for this agent.
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_camera_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<LinuxCameraWatchService>> {
        self.linux_camera_watch_service
            .get_or_try_init(|| linux_camera_watch_service(operation))
    }

    /// Return one shared macOS camera topology service handle for this agent.
    #[cfg(target_os = "macos")]
    pub(crate) fn macos_camera_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<MacosCameraWatchService>> {
        self.macos_camera_watch_service
            .get_or_try_init(|| macos_camera_watch_service(operation))
    }

    /// Return one shared unix serial topology service handle for this agent.
    #[cfg(unix)]
    pub(crate) fn unix_serial_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<UnixSerialWatchService>> {
        self.unix_serial_watch_service
            .get_or_try_init(|| unix_serial_watch_service(operation))
    }

    /// Return one shared Windows serial ingress service handle for this agent.
    #[cfg(windows)]
    pub(crate) fn windows_serial_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsSerialService>> {
        self.windows_serial_service
            .get_or_try_init(|| windows_serial_service(operation))
    }

    /// Return one shared USB service handle for this agent.
    pub(crate) fn usb_service(&self, operation: &'static str) -> RuntimeResult<Arc<UsbService>> {
        self.usb_service.get_or_try_init(|| usb_service(operation))
    }

    /// Return whether any agent-owned device runtime state is active.
    fn has_runtime_state(&self) -> bool {
        self.runtime_activity_count.load(Ordering::Acquire) != 0
    }

    /// Capture one device-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformDeviceImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformDeviceImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.device".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

    /// Retain one unit of device runtime activity until the returned finalizer runs.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn retain_runtime_activity(&self) -> PlatformDeviceActivityFinalizer {
        self.runtime_activity_count.fetch_add(1, Ordering::AcqRel);

        PlatformDeviceActivityFinalizer {
            activity_count: self.runtime_activity_count.clone(),
        }
    }

    /// Wrap one resource finalizer with device runtime-activity tracking.
    pub(crate) fn wrap_finalizer<F>(&self, finalizer: F) -> PlatformDeviceTrackedFinalizer<F>
    where
        F: ResourceFinalizer + 'static,
    {
        self.runtime_activity_count.fetch_add(1, Ordering::AcqRel);

        PlatformDeviceTrackedFinalizer {
            activity_count: self.runtime_activity_count.clone(),
            finalizer,
        }
    }
}

/// Resource finalizer that releases one retained device runtime-activity unit.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) struct PlatformDeviceActivityFinalizer {
    /// Shared live device activity counter.
    activity_count: Arc<AtomicUsize>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl ResourceFinalizer for PlatformDeviceActivityFinalizer {
    /// Release one retained device runtime-activity unit.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        let previous = self.activity_count.fetch_sub(1, Ordering::AcqRel);
        assert!(previous != 0, "device runtime activity count underflow");
    }
}

/// Resource finalizer that releases device runtime activity after backend cleanup.
pub(crate) struct PlatformDeviceTrackedFinalizer<F>
where
    F: ResourceFinalizer + 'static,
{
    /// Shared live device activity counter.
    activity_count: Arc<AtomicUsize>,
    /// Wrapped backend resource finalizer.
    finalizer: F,
}

impl<F> ResourceFinalizer for PlatformDeviceTrackedFinalizer<F>
where
    F: ResourceFinalizer + 'static,
{
    /// Finalize the wrapped resource and release one device runtime-activity unit.
    fn finalize(self: Box<Self>, resource_id: ResourceId) {
        let Self {
            activity_count,
            finalizer,
        } = *self;

        Box::new(finalizer).finalize(resource_id);

        let previous = activity_count.fetch_sub(1, Ordering::AcqRel);
        assert!(previous != 0, "device runtime activity count underflow");
    }
}

/// Materialized device platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformDeviceImage;

impl Capture for PlatformDeviceState {
    type Image = PlatformDeviceImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one device platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one device platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}

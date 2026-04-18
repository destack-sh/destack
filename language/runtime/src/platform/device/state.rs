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
#[cfg(target_os = "android")]
use super::midi::host::AndroidBackendDescription;
#[cfg(target_os = "linux")]
use super::midi::host::{AlsaService, JackService, alsa_service, jack_service};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use super::midi::host::{CoreMidiService, core_midi_service};
#[cfg(windows)]
use super::midi::host::{
    WinMmService, WinRtService, WindowsMidiService, windows_midi_service, winmm_service,
    winrt_service,
};
#[cfg(test)]
use super::serial::SerialTestRegistry;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::serial::{UnixSerialWatchService, unix_serial_watch_service};
#[cfg(windows)]
use super::serial::{WindowsSerialService, windows_serial_service};
use super::usb::{UsbService, usb_service};
#[cfg(target_os = "android")]
use crate::runtime::BindingCallContext;

/// Worker-owned device module state.
#[derive(Default)]
pub(crate) struct PlatformDeviceState {
    /// Cached Android MIDI backend description for this worker.
    #[cfg(target_os = "android")]
    android_backend_description: std::sync::OnceLock<AndroidBackendDescription>,
    /// Shared ALSA service handle for this worker.
    #[cfg(target_os = "linux")]
    alsa_service: ServiceHandle<AlsaService>,
    /// Shared JACK service handle for this worker.
    #[cfg(target_os = "linux")]
    jack_service: ServiceHandle<JackService>,
    /// Shared CoreMIDI service handle for this worker.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    core_midi_service: ServiceHandle<CoreMidiService>,
    /// Shared WinRT service handle for this worker.
    #[cfg(windows)]
    winrt_service: ServiceHandle<WinRtService>,
    /// Shared Windows MIDI service handle for this worker.
    #[cfg(windows)]
    windows_midi_service: ServiceHandle<WindowsMidiService>,
    /// Shared WinMM service handle for this worker.
    #[cfg(windows)]
    winmm_service: ServiceHandle<WinMmService>,
    /// Shared Linux BlueZ service handle for this worker.
    #[cfg(target_os = "linux")]
    linux_bluetooth_service: ServiceHandle<LinuxBluetoothService>,
    /// Shared Linux camera topology service handle for this worker.
    #[cfg(target_os = "linux")]
    linux_camera_watch_service: ServiceHandle<LinuxCameraWatchService>,
    /// Shared macOS camera topology service handle for this worker.
    #[cfg(target_os = "macos")]
    macos_camera_watch_service: ServiceHandle<MacosCameraWatchService>,
    /// Shared unix serial topology service handle for this worker.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    unix_serial_watch_service: ServiceHandle<UnixSerialWatchService>,
    /// Shared Windows serial ingress service handle for this worker.
    #[cfg(windows)]
    windows_serial_service: ServiceHandle<WindowsSerialService>,
    /// Shared test-only virtual serial registry for this worker.
    #[cfg(test)]
    serial_test_registry: Arc<SerialTestRegistry>,
    /// Shared USB service handle for this worker.
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
    /// Return one cached Android MIDI backend description for this worker.
    #[cfg(target_os = "android")]
    pub(crate) fn describe_android_backend(
        &self,
        binding: &BindingCallContext,
        operation: &'static str,
    ) -> RuntimeResult<AndroidBackendDescription> {
        if let Some(description) = self.android_backend_description.get() {
            return Ok(*description);
        }

        let description = super::midi::host::describe_backend(binding, operation)?;
        let _ = self.android_backend_description.set(description);

        Ok(*self
            .android_backend_description
            .get()
            .unwrap_or(&description))
    }

    /// Return one shared ALSA service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn alsa_service(&self, operation: &'static str) -> RuntimeResult<Arc<AlsaService>> {
        self.alsa_service
            .get_or_try_init(|| alsa_service(operation))
    }

    /// Return one shared JACK service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn jack_service(&self, operation: &'static str) -> RuntimeResult<Arc<JackService>> {
        self.jack_service
            .get_or_try_init(|| jack_service(operation))
    }

    /// Return one shared CoreMIDI service handle for this worker.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub(crate) fn core_midi_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<CoreMidiService>> {
        self.core_midi_service
            .get_or_try_init(|| core_midi_service(operation))
    }

    /// Return one shared WinRT service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn winrt_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WinRtService>> {
        self.winrt_service
            .get_or_try_init(|| winrt_service(operation))
    }

    /// Return one shared Windows MIDI service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn windows_midi_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsMidiService>> {
        self.windows_midi_service
            .get_or_try_init(|| windows_midi_service(operation))
    }

    /// Return one shared WinMM service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn winmm_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WinMmService>> {
        self.winmm_service
            .get_or_try_init(|| winmm_service(operation))
    }

    /// Return one shared Linux BlueZ service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_bluetooth_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<LinuxBluetoothService>> {
        self.linux_bluetooth_service
            .get_or_try_init(|| linux_bluetooth_service(operation))
    }

    /// Return one shared Linux camera topology service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_camera_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<LinuxCameraWatchService>> {
        self.linux_camera_watch_service
            .get_or_try_init(|| linux_camera_watch_service(operation))
    }

    /// Return one shared macOS camera topology service handle for this worker.
    #[cfg(target_os = "macos")]
    pub(crate) fn macos_camera_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<MacosCameraWatchService>> {
        self.macos_camera_watch_service
            .get_or_try_init(|| macos_camera_watch_service(operation))
    }

    /// Return one shared unix serial topology service handle for this worker.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn unix_serial_watch_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<UnixSerialWatchService>> {
        self.unix_serial_watch_service
            .get_or_try_init(|| unix_serial_watch_service(operation))
    }

    /// Return one shared Windows serial ingress service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn windows_serial_service(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Arc<WindowsSerialService>> {
        self.windows_serial_service
            .get_or_try_init(|| windows_serial_service(operation))
    }

    /// Return the shared virtual serial registry for this worker.
    #[cfg(test)]
    pub(crate) fn serial_test_registry(&self) -> Arc<SerialTestRegistry> {
        Arc::clone(&self.serial_test_registry)
    }

    /// Return one shared USB service handle for this worker.
    pub(crate) fn usb_service(&self, operation: &'static str) -> RuntimeResult<Arc<UsbService>> {
        self.usb_service.get_or_try_init(|| usb_service(operation))
    }

    /// Return whether any worker-owned device runtime state is active.
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
pub(crate) struct PlatformDeviceActivityFinalizer {
    /// Shared live device activity counter.
    activity_count: Arc<AtomicUsize>,
}

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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

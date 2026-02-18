#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;

use crate::platform::input::{InputDeviceInfo, InputEvent, InputReadMode};
use crate::platform::resource;

/// Close one input device.
///
/// Close one opened input device endpoint and release host resources.
/// Pending unread events are discarded according to host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_close(
    _context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.close")).boxed())
}

/// List available input devices.
///
/// Enumerate host input devices and return stable identifiers and typed device metadata.
/// Device ordering and hotplug visibility follow host input subsystem semantics.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one discoverable input backend.
/// Uses evdev device-node enumeration on Linux, global-session and terminal discovery on macOS, terminal input discovery on other Unix hosts, and console plus raw-state discovery on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_list(
    _context: &RuntimeCallContext,
    out: *mut NativeSlice<InputDeviceInfo>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.list")).boxed())
}

/// Open one input device.
///
/// Open one input device endpoint for event reads and optional control operations.
/// Exclusive-grab behavior and permission checks are host-defined.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one openable input backend.
/// Uses evdev device-node open on Linux, global-session or terminal-device open on macOS, terminal-device open on other Unix hosts, and duplicated console-input handles or raw-state handles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_open(
    _context: &RuntimeCallContext,
    out: *mut resource::InputDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.open")).boxed())
}

/// Close one global input event monitor.
///
/// Close one opened monitor stream and release host subscription resources.
/// Pending unread monitor events are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_close(
    _context: &RuntimeCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorClose",
    ))
    .boxed())
}

/// Open one global input event monitor.
///
/// Open one monitor stream that reports host input topology events, including connect and disconnect.
/// Monitor streams are independent from per-device data streams.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses inotify-backed `/dev/input` monitor events on Linux with snapshot fallback when watcher setup is unavailable, global-session subscriptions on macOS, terminal-device scans on other Unix hosts, and raw-input device-change subscriptions on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_open(
    _context: &RuntimeCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorOpen",
    ))
    .boxed())
}

/// Read one global input monitor event.
///
/// Read one pending monitor event from the global input monitor stream.
/// This stream is the canonical source for device connect and disconnect events.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses blocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_read(
    _context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorRead",
    ))
    .boxed())
}

/// Poll one global input monitor event without blocking.
///
/// Poll one pending monitor event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses nonblocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_try_read(
    _context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.monitorTryRead",
    ))
    .boxed())
}

/// Read one input event.
///
/// Read one pending input event from one opened device stream.
/// Per-device streams report control and motion events for that device and exclude global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses evdev event reads on Linux, global-session state polling on macOS, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_read(
    _context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
}

/// Read one batch of input events.
///
/// Read up to `maxEvents` events from one opened device stream in one call.
/// Batch ordering matches backend delivery order and excludes global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses batched reads when supported and runtime looped reads otherwise.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_read_batch(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.readBatch",
    ))
    .boxed())
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where exclusive grab is not defined by host policy.
/// Uses EVIOCGRAB on Linux, returns notSupported for global-session and terminal-backed Unix input, and uses SetConsoleMode capture toggles on Windows console input.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_grab(
    _context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enable);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.setGrab")).boxed())
}

/// Select event decoding mode for one input stream.
///
/// Select translated or raw decoding mode for one opened input endpoint.
/// Hosts can return notSupported when raw mode is unavailable for the selected endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses per-stream runtime mode selection on Linux evdev and macOS session backends, termios raw and cooked mode updates on Unix TTY paths, and SetConsoleMode updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_read_mode(
    _context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setReadMode",
    ))
    .boxed())
}

/// Poll one input event without blocking.
///
/// Poll one pending input event from one opened device stream and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses nonblocking evdev reads on Linux, nonblocking global-session polling on macOS, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_try_read(
    _context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.tryRead")).boxed())
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use std::{mem, ptr, thread};

use parking_lot::{Condvar, Mutex};
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HID_USAGE_DIGITIZER_PEN, HID_USAGE_DIGITIZER_TOUCH_PAD, HID_USAGE_DIGITIZER_TOUCH_SCREEN,
    HID_USAGE_PAGE_DIGITIZER, HID_USAGE_PAGE_SENSOR, HIDP_BUTTON_CAPS, HIDP_CAPS,
    HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS, HidD_FreePreparsedData, HidD_GetFeature,
    HidD_GetInputReport, HidD_GetPreparsedData, HidD_SetFeature, HidD_SetOutputReport,
    HidP_GetButtonCaps, HidP_GetCaps, HidP_GetUsageValue, HidP_GetUsages, HidP_GetValueCaps,
    HidP_Input, HidP_MaxUsageListLength, PHIDP_PREPARSED_DATA,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_CLASS_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, HANDLE,
    HINSTANCE, HWND, INVALID_HANDLE_VALUE, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows_sys::Win32::UI::Input::Touch::{
    CloseTouchInputHandle, GetTouchInputInfo, RegisterTouchWindow, TOUCHEVENTF_DOWN,
    TOUCHEVENTF_MOVE, TOUCHEVENTF_UP, TOUCHINPUT, TOUCHINPUTMASKF_CONTACTAREA, TWF_WANTPALM,
};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTDEVICELIST, RAWINPUTHEADER, RID_DEVICE_INFO, RID_INPUT, RIDEV_DEVNOTIFY,
    RIDEV_INPUTSINK, RIDEV_PAGEONLY, RIDI_DEVICEINFO, RIDI_DEVICENAME, RIM_TYPEHID,
    RIM_TYPEKEYBOARD, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GIDC_ARRIVAL, GIDC_REMOVAL,
    GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HWND_MESSAGE, PostQuitMessage,
    PostThreadMessageW, RI_KEY_BREAK, RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP,
    RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP, RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_3_UP,
    RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP, RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP,
    RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, RegisterClassW, SetWindowLongPtrW, WM_DESTROY, WM_INPUT,
    WM_INPUT_DEVICE_CHANGE, WM_NCCREATE, WM_QUIT, WM_TOUCH, WNDCLASSW,
};

use super::core as windows_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::process_ingress_loop as run_windows_ingress_loop;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputDeviceCapabilities, InputDeviceCapabilityKind,
    InputDeviceEventPayload, InputDeviceKind, InputEvent, InputEventAction,
    InputGamepadBatteryState, InputGamepadBatteryStatus, InputGamepadButtonState,
    InputGamepadConnectionType, InputGamepadMappingType, InputGamepadState, InputKeyEventPayload,
    InputMonitorChangeEvent, InputMonitorConnectEvent, InputMonitorDisconnectEvent,
    InputMonitorEvent, InputMonitorEventMetadata, InputPointerButtonEventPayload,
    InputPointerMotionEventPayload, InputRawHidReport, InputScrollEventPayload,
    InputSensorDescriptor, InputSensorKind, InputSensorSample, InputTouchContactPhase,
    InputTouchContactState, InputTouchState,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::service::{ProcessSubscriberRegistry, Service};
use crate::runtime::{
    BindingCallContext, ExecutionMode, ExecutionPolicy, WorkerId, start_with_policy,
};

/// Prefix for monitor event device identifiers derived from raw device handles.
pub(super) const WINDOWS_INPUT_MONITOR_ID_PREFIX: &str = "raw:device:";
/// Pseudo-device identifier used for monitor queue overflow notifications.
const WINDOWS_INPUT_RAW_MONITOR_ID: &str = "raw:monitor";
/// Maximum queued keyboard or mouse packets before oldest-drop backpressure.
pub(super) const RAW_INPUT_QUEUE_LIMIT: usize = 8192;
/// Maximum queued monitor packets before oldest-drop backpressure.
pub(super) const RAW_MONITOR_QUEUE_LIMIT: usize = 1024;
/// Maximum queued raw-hid packets before oldest-drop backpressure.
pub(super) const RAW_HID_QUEUE_LIMIT: usize = 4096;
/// Maximum queued touch snapshots before oldest-drop backpressure.
pub(super) const RAW_TOUCH_QUEUE_LIMIT: usize = 2048;
/// Event code for raw keyboard and mouse queue overflow notifications.
const RAW_INPUT_OVERFLOW_CODE: u32 = 0xffff_ff01;
/// Event code for raw monitor queue overflow notifications.
const RAW_MONITOR_OVERFLOW_CODE: u32 = 0xffff_ff02;
/// Raw mouse movement flag for absolute coordinates.
const RAW_MOUSE_MOVE_ABSOLUTE: u16 = 0x0001;
/// Raw mouse movement flag for virtual-desktop absolute coordinates.
const RAW_MOUSE_VIRTUAL_DESKTOP: u16 = 0x0002;
/// Raw mouse absolute coordinate normalization divisor.
const RAW_MOUSE_COORDINATE_MAX: f64 = 65535.0;
/// Win32 virtual-key code for generic shift.
const VK_SHIFT: usize = 0x10;
/// Win32 virtual-key code for generic control.
const VK_CONTROL: usize = 0x11;
/// Win32 virtual-key code for generic alt.
const VK_MENU: usize = 0x12;
/// Win32 virtual-key code for caps lock.
const VK_CAPITAL: usize = 0x14;
/// Win32 virtual-key code for scroll lock.
const VK_SCROLL: usize = 0x91;
/// Win32 virtual-key code for num lock.
const VK_NUMLOCK: usize = 0x90;
/// Win32 virtual-key code for left shift.
const VK_LSHIFT: usize = 0xA0;
/// Win32 virtual-key code for right shift.
const VK_RSHIFT: usize = 0xA1;
/// Win32 virtual-key code for left control.
const VK_LCONTROL: usize = 0xA2;
/// Win32 virtual-key code for right control.
const VK_RCONTROL: usize = 0xA3;
/// Win32 virtual-key code for left alt.
const VK_LMENU: usize = 0xA4;
/// Win32 virtual-key code for right alt.
const VK_RMENU: usize = 0xA5;
/// Win32 virtual-key code for left windows key.
const VK_LWIN: usize = 0x5B;
/// Win32 virtual-key code for right windows key.
const VK_RWIN: usize = 0x5C;
/// CONTROL_KEY_STATE bit for right alt.
const RIGHT_ALT_PRESSED: u32 = 0x0001;
/// CONTROL_KEY_STATE bit for left alt.
const LEFT_ALT_PRESSED: u32 = 0x0002;
/// CONTROL_KEY_STATE bit for right control.
const RIGHT_CTRL_PRESSED: u32 = 0x0004;
/// CONTROL_KEY_STATE bit for left control.
const LEFT_CTRL_PRESSED: u32 = 0x0008;
/// CONTROL_KEY_STATE bit for shift.
const SHIFT_PRESSED: u32 = 0x0010;
/// CONTROL_KEY_STATE bit for num lock.
const NUMLOCK_ON: u32 = 0x0020;
/// CONTROL_KEY_STATE bit for scroll lock.
const SCROLLLOCK_ON: u32 = 0x0040;
/// CONTROL_KEY_STATE bit for caps lock.
const CAPSLOCK_ON: u32 = 0x0080;
/// CONTROL_KEY_STATE bit for enhanced keys.
const ENHANCED_KEY: u32 = 0x0100;

/// Generic HID usage page.
const HID_USAGE_PAGE_GENERIC: u16 = 1;
/// Generic HID button usage page.
const HID_USAGE_PAGE_BUTTON: u16 = 9;
/// Generic HID mouse usage.
const HID_USAGE_GENERIC_MOUSE: u16 = 2;
/// Generic HID joystick usage.
const HID_USAGE_GENERIC_JOYSTICK: u16 = 4;
/// Generic HID gamepad usage.
const HID_USAGE_GENERIC_GAMEPAD: u16 = 5;
/// Generic HID keyboard usage.
const HID_USAGE_GENERIC_KEYBOARD: u16 = 6;
/// Generic HID usage for x axis.
const HID_USAGE_GENERIC_X: u16 = 0x30;
/// Generic HID usage for y axis.
const HID_USAGE_GENERIC_Y: u16 = 0x31;
/// Generic HID usage for z axis.
const HID_USAGE_GENERIC_Z: u16 = 0x32;
/// Generic HID usage for rotation x axis.
const HID_USAGE_GENERIC_RX: u16 = 0x33;
/// Generic HID usage for rotation y axis.
const HID_USAGE_GENERIC_RY: u16 = 0x34;
/// Generic HID usage for rotation z axis.
const HID_USAGE_GENERIC_RZ: u16 = 0x35;
/// Generic HID usage for slider axis.
const HID_USAGE_GENERIC_SLIDER: u16 = 0x36;
/// Generic HID usage for dial axis.
const HID_USAGE_GENERIC_DIAL: u16 = 0x37;
/// Generic HID usage for wheel axis.
const HID_USAGE_GENERIC_WHEEL: u16 = 0x38;
/// Generic HID usage for hat switch.
const HID_USAGE_GENERIC_HAT_SWITCH: u16 = 0x39;
/// Sony USB vendor id.
const SONY_VENDOR_ID: u16 = 0x054c;
/// Generic HID digitizer finger usage.
const HID_USAGE_DIGITIZER_FINGER: u16 = 34;
/// Generic HID sensor usage for 3d accelerometer.
const HID_USAGE_SENSOR_ACCELEROMETER_3D: u16 = 0x73;
/// Generic HID sensor usage for 3d gyrometer.
const HID_USAGE_SENSOR_GYROMETER_3D: u16 = 0x76;
/// Generic HID sensor usage for 3d magnetometer.
const HID_USAGE_SENSOR_MAGNETOMETER_3D: u16 = 0x83;
/// Generic HID sensor usage for gravity vector.
const HID_USAGE_SENSOR_GRAVITY_VECTOR: u16 = 0x8c;
/// Generic HID sensor usage for linear acceleration.
const HID_USAGE_SENSOR_LINEAR_ACCELERATION: u16 = 0x8d;
/// Generic HID sensor usage for orientation.
const HID_USAGE_SENSOR_DEVICE_ORIENTATION: u16 = 0x8a;
/// Generic HID usage for dpad up.
const HID_USAGE_GENERIC_DPAD_UP: u16 = 0x90;
/// Generic HID usage for dpad down.
const HID_USAGE_GENERIC_DPAD_DOWN: u16 = 0x91;
/// Generic HID usage for dpad right.
const HID_USAGE_GENERIC_DPAD_RIGHT: u16 = 0x92;
/// Generic HID usage for dpad left.
const HID_USAGE_GENERIC_DPAD_LEFT: u16 = 0x93;
/// Generic HID usage for system-main-menu button.
const HID_USAGE_GENERIC_SYSTEM_MAIN_MENU: u16 = 0x85;
/// Sony usb report id for dualshock 4 effects packets.
const SONY_DUALSHOCK4_USB_EFFECTS_REPORT_ID: u8 = 0x05;
/// Sony usb report length for dualshock 4 effects packets.
const SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES: u16 = 32;
/// Sony usb report id for dualsense effects packets.
const SONY_DUALSENSE_USB_EFFECTS_REPORT_ID: u8 = 0x02;
/// Sony usb report length for dualsense effects packets.
const SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES: u16 = 48;
/// Standardized gamepad axis count in runtime snapshots.
const STANDARD_GAMEPAD_AXIS_COUNT: usize = 4;
/// Standardized gamepad button count in runtime snapshots.
const STANDARD_GAMEPAD_BUTTON_COUNT: usize = 17;

/// One hid value-capability span used for runtime axis and sensor projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawHidValueCapability {
    /// HID usage page.
    pub(super) usage_page: u16,
    /// Lower inclusive usage in this span.
    pub(super) usage_min: u16,
    /// Upper inclusive usage in this span.
    pub(super) usage_max: u16,
    /// Link-collection id for hid parser lookups.
    pub(super) link_collection: u16,
    /// Report id for this capability span.
    pub(super) report_id: u8,
    /// Logical minimum value.
    pub(super) logical_min: i32,
    /// Logical maximum value.
    pub(super) logical_max: i32,
}

/// One hid button-capability span used for runtime button projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawHidButtonCapability {
    /// HID usage page.
    pub(super) usage_page: u16,
    /// Lower inclusive usage in this span.
    pub(super) usage_min: u16,
    /// Upper inclusive usage in this span.
    pub(super) usage_max: u16,
    /// Link-collection id for hid parser lookups.
    pub(super) link_collection: u16,
    /// Report id for this capability span.
    pub(super) report_id: u8,
}

/// Null-terminated message-only window class name.
const CLASS_NAME: &[u16] = &[
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b't' as u16,
    b'a' as u16,
    b'c' as u16,
    b'k' as u16,
    b'R' as u16,
    b'a' as u16,
    b'w' as u16,
    b'I' as u16,
    b'n' as u16,
    b'p' as u16,
    b'u' as u16,
    b't' as u16,
    0,
];

/// Raw-input device descriptor used by list, open, and capability queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RawInputDeviceDescriptor {
    /// Stable runtime identifier.
    pub(super) id: String,
    /// Stable instance identifier.
    pub(super) instance_id: String,
    /// Stable hardware identifier.
    pub(super) hardware_id: String,
    /// Host-visible display name.
    pub(super) name: String,
    /// Raw Win32 device path when available.
    pub(super) raw_path: Option<String>,
    /// Runtime device kind.
    pub(super) kind: InputDeviceKind,
    /// Vendor identifier when available.
    pub(super) vendor_id: u16,
    /// Product identifier when available.
    pub(super) product_id: u16,
    /// Top-level HID usage page when available.
    pub(super) usage_page: u16,
    /// Top-level HID usage when available.
    pub(super) usage: u16,
    /// HID report size in bytes when available.
    pub(super) report_size: u16,
    /// HID output report size in bytes when available.
    pub(super) output_report_size: u16,
    /// HID feature report size in bytes when available.
    pub(super) feature_report_size: u16,
    /// Logical key count.
    pub(super) key_count: u16,
    /// Logical button count.
    pub(super) button_count: u16,
    /// Logical axis count.
    pub(super) axis_count: u16,
    /// Whether this device supports text semantics.
    pub(super) supports_text: bool,
    /// Whether this device supports rumble semantics.
    pub(super) supports_rumble: bool,
    /// Whether this device supports battery-state semantics.
    pub(super) supports_battery: bool,
    /// Whether this device supports light-control semantics.
    pub(super) supports_light: bool,
    /// Whether this device supports pointer-grab semantics.
    pub(super) supports_pointer_grab: bool,
    /// Whether this device supports raw HID semantics.
    pub(super) supports_raw_hid: bool,
    /// Whether this device supports sensor streams.
    pub(super) supports_sensors: bool,
    /// Whether this device supports player-index assignment.
    pub(super) supports_player_index: bool,
    /// Flattened hid value-capability spans derived from parser metadata.
    pub(super) value_capabilities: Vec<RawHidValueCapability>,
    /// Flattened hid button-capability spans derived from parser metadata.
    pub(super) button_capabilities: Vec<RawHidButtonCapability>,
}

/// Raw input packet captured from keyboard or mouse streams.
#[derive(Debug, Clone)]
struct RawInputPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Stable runtime source device identifier.
    device_id: String,
    /// Runtime event kind.
    kind: windows_core::WindowsInputEventKind,
    /// Runtime event action.
    action: InputEventAction,
    /// Backend event code.
    code: u32,
    /// Backend scan code.
    scan_code: u32,
    /// Scalar event value.
    value: i64,
    /// Pointer x delta or coordinate payload.
    x: f64,
    /// Pointer y delta or coordinate payload.
    y: f64,
    /// Horizontal wheel delta.
    wheel_x: f64,
    /// Vertical wheel delta.
    wheel_y: f64,
    /// Pointer-button bitset snapshot.
    buttons: u32,
    /// Modifier bitfield.
    modifiers: u32,
    /// Whether this event is a key-repeat.
    repeat: bool,
}

/// Raw monitor packet captured from device-change notifications.
#[derive(Debug, Clone)]
struct RawMonitorPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Monitor action.
    action: InputEventAction,
    /// Stable monitor device identifier.
    device_id: String,
    /// Runtime device kind observed for this topology event.
    device_kind: InputDeviceKind,
    /// Backend event code.
    code: u32,
    /// Scalar payload value.
    value: i64,
}

/// Raw-hid packet captured from one HID report.
#[derive(Debug, Clone)]
struct RawHidPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Monotonic per-device sequence number.
    sequence: u64,
    /// Stable runtime source device identifier.
    device_id: String,
    /// HID report identifier.
    report_id: u8,
    /// HID report payload bytes.
    data: Vec<u8>,
}

/// Touch contact snapshot used by touch-state reads.
#[derive(Debug, Clone)]
struct RawTouchContact {
    /// Backend contact identifier.
    contact_id: u32,
    /// Touch phase for this contact.
    phase: InputTouchContactPhase,
    /// Contact x coordinate.
    x: f64,
    /// Contact y coordinate.
    y: f64,
    /// Contact pressure in normalized units.
    pressure: f64,
    /// Contact major radius.
    radius_x: f64,
    /// Contact minor radius.
    radius_y: f64,
    /// Contact tilt around x axis.
    tilt_x: f64,
    /// Contact tilt around y axis.
    tilt_y: f64,
}

/// Pen snapshot used by pointer-state reads.
#[derive(Debug, Clone, Copy)]
pub(super) struct RawPenStateSnapshot {
    /// Pen x coordinate in backend units.
    pub(super) x: f64,
    /// Pen y coordinate in backend units.
    pub(super) y: f64,
    /// Pen pressure in normalized units.
    pub(super) pressure: f64,
    /// Pen tilt around x axis.
    pub(super) tilt_x: f64,
    /// Pen tilt around y axis.
    pub(super) tilt_y: f64,
    /// Whether the pen tip is currently in contact.
    pub(super) in_contact: bool,
    /// Whether the pen is currently in range.
    pub(super) in_range: bool,
}

/// Touch snapshot packet captured from one touch message.
#[derive(Debug, Clone)]
struct RawTouchPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Stable runtime source device identifier.
    device_id: String,
    /// Active contacts for this snapshot.
    contacts: Vec<RawTouchContact>,
}

/// In-memory queues used by the raw input worker thread.
#[derive(Debug)]
struct RawInputQueues {
    /// Pending keyboard packets.
    keyboard: VecDeque<RawInputPacket>,
    /// Pending mouse packets.
    mouse: VecDeque<RawInputPacket>,
    /// Pending monitor packets.
    monitor: VecDeque<RawMonitorPacket>,
    /// Pending raw-hid packets.
    hid: VecDeque<RawHidPacket>,
    /// Pending touch snapshots.
    touch: VecDeque<RawTouchPacket>,
    /// Current pressed state for pointer button bits.
    mouse_buttons: u32,
    /// Current pressed state for virtual-key indices.
    key_down: [bool; 256],
    /// Current toggled caps-lock state.
    caps_lock_on: bool,
    /// Current toggled num-lock state.
    num_lock_on: bool,
    /// Current toggled scroll-lock state.
    scroll_lock_on: bool,
    /// Stable monitor identifiers keyed by raw device handle.
    monitor_device_ids: HashMap<isize, String>,
    /// Stable monitor kinds keyed by raw device handle.
    monitor_device_kinds: HashMap<isize, InputDeviceKind>,
    /// Active per-device stream counts keyed by runtime id.
    active_input_streams: HashMap<String, usize>,
    /// Monotonic raw-hid sequence counters keyed by runtime id.
    hid_sequences: HashMap<String, u64>,
    /// Monotonic touch sequence counters keyed by runtime id.
    touch_sequences: HashMap<String, u64>,
    /// Active touch contacts keyed by runtime device and contact ids.
    active_touch_contacts: HashMap<String, HashMap<u32, RawTouchContact>>,
}

impl RawInputQueues {
    /// Build empty raw-input queues.
    fn new() -> Self {
        Self {
            keyboard: VecDeque::new(),
            mouse: VecDeque::new(),
            monitor: VecDeque::new(),
            hid: VecDeque::new(),
            touch: VecDeque::new(),
            mouse_buttons: 0,
            key_down: [false; 256],
            caps_lock_on: query_toggle_key_state(VK_CAPITAL),
            num_lock_on: query_toggle_key_state(VK_NUMLOCK),
            scroll_lock_on: query_toggle_key_state(VK_SCROLL),
            monitor_device_ids: HashMap::new(),
            monitor_device_kinds: HashMap::new(),
            active_input_streams: HashMap::new(),
            hid_sequences: HashMap::new(),
            touch_sequences: HashMap::new(),
            active_touch_contacts: HashMap::new(),
        }
    }
}

/// Shared state between the raw worker and binding call sites.
#[derive(Debug)]
struct RawInputState {
    /// Queued events for all raw streams.
    queues: Mutex<RawInputQueues>,
    /// Wait primitive used by blocking read calls.
    wake: Condvar,
}

impl RawInputState {
    /// Build one empty synchronized raw-input state.
    fn new() -> Self {
        Self {
            queues: Mutex::new(RawInputQueues::new()),
            wake: Condvar::new(),
        }
    }
}

/// Process-global windows raw-input service.
#[derive(Debug)]
pub(crate) struct WindowsRawInputService {
    /// Registered raw-input runtimes keyed by worker id.
    runtimes: Mutex<ProcessSubscriberRegistry<WorkerId, WindowsRawInputRuntimeState>>,
    /// Worker-thread control payload.
    worker: Mutex<Option<RawInputWorker>>,
    /// Liveness state for the raw-input message worker.
    worker_running: AtomicBool,
}

impl Service for WindowsRawInputService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

/// Raw-input worker-thread control payload.
#[derive(Debug)]
struct RawInputWorker {
    /// Native thread identifier used for quit signaling.
    thread_id: u32,
    /// Join handle for deterministic worker teardown.
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl WindowsRawInputService {
    /// Create one process-global windows raw-input service.
    fn new() -> Self {
        Self {
            runtimes: Mutex::new(ProcessSubscriberRegistry::default()),
            worker: Mutex::new(None),
            worker_running: AtomicBool::new(false),
        }
    }

    /// Register one worker-local raw-input runtime.
    fn register_runtime(
        &self,
        worker_id: WorkerId,
        runtime_state: &Arc<WindowsRawInputRuntimeState>,
    ) {
        let mut runtimes = self.runtimes.lock();
        runtimes.register(worker_id, runtime_state);
    }

    /// Unregister one worker-local raw-input runtime.
    fn unregister_runtime(&self, worker_id: WorkerId) {
        let should_shutdown = {
            let mut runtimes = self.runtimes.lock();
            runtimes.unregister(worker_id);
            runtimes.is_empty()
        };

        if should_shutdown {
            self.shutdown_worker();
        }
    }

    /// Return one snapshot of the live raw-input runtimes.
    fn runtime_states_snapshot(&self) -> Vec<Arc<WindowsRawInputRuntimeState>> {
        let mut runtimes = self.runtimes.lock();

        runtimes.snapshot()
    }

    /// Return whether the worker thread is currently running.
    fn is_worker_running(&self) -> bool {
        self.worker_running.load(Ordering::Acquire)
    }

    /// Ensure one live worker thread is available.
    fn ensure_worker(
        self: &Arc<Self>,
        binding: &BindingCallContext,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let mut worker = self.worker.lock();

        // restart the worker after unexpected exits
        let requires_spawn = match worker.as_ref() {
            Some(_) => !self.is_worker_running(),
            None => true,
        };

        if requires_spawn {
            let next_worker = spawn_raw_input_worker(binding, self).map_err(|error| {
                service_error(operation, format!("raw input unavailable: {error}"))
            })?;
            *worker = Some(next_worker);
        }

        Ok(())
    }

    /// Shut down one active worker thread when present.
    fn shutdown_worker(&self) {
        let worker = self.worker.lock().take();
        let Some(worker) = worker else {
            return;
        };

        let thread_id = worker.thread_id;
        if thread_id != 0 {
            unsafe {
                PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
            }
        }

        if let Some(handle) = worker.handle.lock().take() {
            let _ = handle.join();
        }
    }
}

/// Worker-owned mutable state for windows raw-input streams.
#[derive(Debug)]
pub(crate) struct WindowsRawInputRuntimeState {
    /// Runtime-configured queue capacity for keyboard and mouse packets.
    raw_input_queue_limit: AtomicUsize,
    /// Runtime-configured queue capacity for monitor packets.
    raw_monitor_queue_limit: AtomicUsize,
    /// Runtime-configured queue capacity for raw-hid packets.
    raw_hid_queue_limit: AtomicUsize,
    /// Runtime-configured queue capacity for touch snapshots.
    raw_touch_queue_limit: AtomicUsize,
    /// Shared queue state for this worker-local runtime.
    state: Arc<RawInputState>,
    /// Whether runtime finalizers were already registered.
    finalizer_registered: AtomicBool,
    /// Whether this runtime was already attached to the shared raw-input service.
    service_registered: AtomicBool,
    /// Cached raw gamepad decoder state keyed by runtime device identifier.
    gamepad_decoder_cache: Mutex<HashMap<String, RawGamepadDecoderEntry>>,
}

impl WindowsRawInputRuntimeState {
    /// Build one worker-owned raw-input state payload.
    pub(crate) fn new(_worker_id: WorkerId) -> Self {
        Self {
            raw_input_queue_limit: AtomicUsize::new(RAW_INPUT_QUEUE_LIMIT),
            raw_monitor_queue_limit: AtomicUsize::new(RAW_MONITOR_QUEUE_LIMIT),
            raw_hid_queue_limit: AtomicUsize::new(RAW_HID_QUEUE_LIMIT),
            raw_touch_queue_limit: AtomicUsize::new(RAW_TOUCH_QUEUE_LIMIT),
            state: Arc::new(RawInputState::new()),
            finalizer_registered: AtomicBool::new(false),
            service_registered: AtomicBool::new(false),
            gamepad_decoder_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for WindowsRawInputRuntimeState {
    /// Build one default raw-input runtime state for tests and fallback construction.
    fn default() -> Self {
        Self::new(WorkerId(0))
    }
}

/// Return one worker-owned raw-input mutable state.
pub(super) fn windows_raw_input_runtime_state(
    binding: &BindingCallContext,
) -> Arc<WindowsRawInputRuntimeState> {
    binding
        .worker()
        .platform_state
        .input
        .windows_raw_input_runtime_state(binding)
}

/// Return one shared windows raw-input service.
pub(crate) fn windows_raw_input_service(
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsRawInputService>> {
    WindowsRawInputService::global(|| Ok(WindowsRawInputService::new())).map_err(|error| {
        core_platform::io_operation_error(
            operation,
            None,
            format!("failed to initialize windows raw input service: {error}"),
        )
    })
}

/// Return the configured keyboard and mouse queue capacity.
fn raw_input_queue_limit(runtime_state: &WindowsRawInputRuntimeState) -> usize {
    runtime_state
        .raw_input_queue_limit
        .load(Ordering::Relaxed)
        .max(1)
}

/// Return the configured monitor queue capacity.
fn raw_monitor_queue_limit(runtime_state: &WindowsRawInputRuntimeState) -> usize {
    runtime_state
        .raw_monitor_queue_limit
        .load(Ordering::Relaxed)
        .max(1)
}

/// Return the configured raw-hid queue capacity.
fn raw_hid_queue_limit(runtime_state: &WindowsRawInputRuntimeState) -> usize {
    runtime_state
        .raw_hid_queue_limit
        .load(Ordering::Relaxed)
        .max(1)
}

/// Return the configured touch queue capacity.
fn raw_touch_queue_limit(runtime_state: &WindowsRawInputRuntimeState) -> usize {
    runtime_state
        .raw_touch_queue_limit
        .load(Ordering::Relaxed)
        .max(1)
}

/// Apply raw-input queue limits from one binding context.
fn configure_raw_queue_limits(
    runtime_state: &WindowsRawInputRuntimeState,
    binding: &BindingCallContext,
) {
    runtime_state.raw_input_queue_limit.store(
        windows_core::windows_raw_input_queue_capacity(binding),
        Ordering::Relaxed,
    );
    runtime_state.raw_monitor_queue_limit.store(
        windows_core::windows_raw_monitor_queue_capacity(binding),
        Ordering::Relaxed,
    );
    runtime_state.raw_hid_queue_limit.store(
        windows_core::windows_raw_hid_queue_capacity(binding),
        Ordering::Relaxed,
    );
    runtime_state.raw_touch_queue_limit.store(
        windows_core::windows_raw_touch_queue_capacity(binding),
        Ordering::Relaxed,
    );
}

/// Queue selector for keyboard, mouse, and monitor streams.
#[derive(Clone, Copy)]
enum RawQueueKind {
    /// Keyboard packet queue.
    Keyboard,
    /// Mouse packet queue.
    Mouse,
}

/// Cached parser and report metadata for one opened raw gamepad stream.
#[derive(Debug)]
struct RawGamepadDecoderEntry {
    /// Open hid handle used for input-report probes.
    handle: HANDLE,
    /// Cached hid preparsed metadata used for usage decoding.
    preparsed: PHIDP_PREPARSED_DATA,
    /// Input report size in bytes.
    report_size: u16,
    /// Candidate report identifiers to probe.
    report_ids: Vec<u8>,
    /// Stable mapping classification derived from descriptor capabilities.
    mapping: InputGamepadMappingType,
}

/// Read one monotonic timestamp from the shared runtime clock domain.
fn now_timestamp_ns() -> u64 {
    core_platform::monotonic_now_ns()
}

/// Query one current pointer position snapshot.
fn current_pointer_position() -> (f64, f64) {
    let mut point = POINT { x: 0, y: 0 };
    let status = unsafe { GetCursorPos(&mut point) };
    if status == 0 {
        return (0.0, 0.0);
    }

    (f64::from(point.x), f64::from(point.y))
}

/// Query one lock-key toggle state from the host.
fn query_toggle_key_state(virtual_key: usize) -> bool {
    let key_code = virtual_key as i32;
    let key_state = unsafe { GetKeyState(key_code) as u16 };
    (key_state & 0x0001) != 0
}

/// Return whether one key code maps to one left or right modifier key.
fn is_side_modifier_key(virtual_key: usize) -> bool {
    matches!(
        virtual_key,
        VK_LSHIFT | VK_RSHIFT | VK_LCONTROL | VK_RCONTROL | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN
    )
}

/// Update lock-key toggle state from one keyboard press edge.
fn update_lock_key_state(
    queues: &mut RawInputQueues,
    virtual_key: usize,
    is_pressed: bool,
    was_pressed: bool,
) {
    // toggle only on the leading press edge
    if !is_pressed || was_pressed {
        return;
    }

    match virtual_key {
        VK_CAPITAL => {
            queues.caps_lock_on = !queues.caps_lock_on;
        }
        VK_NUMLOCK => {
            queues.num_lock_on = !queues.num_lock_on;
        }
        VK_SCROLL => {
            queues.scroll_lock_on = !queues.scroll_lock_on;
        }
        _ => {}
    }
}

/// Build CONTROL_KEY_STATE-compatible modifiers from current raw keyboard state.
fn control_key_state_from_queues(queues: &RawInputQueues) -> u32 {
    let mut modifiers = 0u32;

    // synthesize modifier-pressed bits from side-specific key states
    if queues.key_down[VK_SHIFT] || queues.key_down[VK_LSHIFT] || queues.key_down[VK_RSHIFT] {
        modifiers |= SHIFT_PRESSED;
    }
    if queues.key_down[VK_LCONTROL] {
        modifiers |= LEFT_CTRL_PRESSED;
    }
    if queues.key_down[VK_RCONTROL] {
        modifiers |= RIGHT_CTRL_PRESSED;
    }
    if queues.key_down[VK_CONTROL] && !queues.key_down[VK_LCONTROL] && !queues.key_down[VK_RCONTROL]
    {
        modifiers |= LEFT_CTRL_PRESSED;
    }
    if queues.key_down[VK_LMENU] {
        modifiers |= LEFT_ALT_PRESSED;
    }
    if queues.key_down[VK_RMENU] {
        modifiers |= RIGHT_ALT_PRESSED;
    }
    if queues.key_down[VK_MENU] && !queues.key_down[VK_LMENU] && !queues.key_down[VK_RMENU] {
        modifiers |= LEFT_ALT_PRESSED;
    }

    // synthesize lock bits from tracked toggle state
    if queues.caps_lock_on {
        modifiers |= CAPSLOCK_ON;
    }
    if queues.num_lock_on {
        modifiers |= NUMLOCK_ON;
    }
    if queues.scroll_lock_on {
        modifiers |= SCROLLLOCK_ON;
    }

    // synthesize enhanced-key bit when either windows key is held
    if queues.key_down[VK_LWIN] || queues.key_down[VK_RWIN] {
        modifiers |= ENHANCED_KEY;
    }

    modifiers
}

/// Return one stable monitor device id from one raw device handle.
fn monitor_device_id(raw_device: isize) -> String {
    // use the backend device name when available for stable ids across reconnects
    if let Some(name) = raw_device_name(raw_device) {
        return format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{name}");
    }

    // otherwise use one descriptor fingerprint before falling back to volatile handles
    if let Some(fingerprint) = raw_device_fingerprint(raw_device) {
        return format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{fingerprint}");
    }

    format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{:x}", raw_device as usize)
}

/// Return one stable fingerprint from one raw device descriptor payload.
fn raw_device_fingerprint(raw_device: isize) -> Option<String> {
    let handle = raw_device as HANDLE;
    let mut info = RID_DEVICE_INFO {
        cbSize: mem::size_of::<RID_DEVICE_INFO>() as u32,
        dwType: 0,
        Anonymous: unsafe { mem::zeroed() },
    };
    let mut size = mem::size_of::<RID_DEVICE_INFO>() as u32;
    let status = unsafe {
        GetRawInputDeviceInfoW(
            handle,
            RIDI_DEVICEINFO,
            &mut info as *mut _ as *mut c_void,
            &mut size,
        )
    };
    if status == u32::MAX {
        return None;
    }

    if info.dwType == RIM_TYPEKEYBOARD {
        let keyboard = unsafe { info.Anonymous.keyboard };
        return Some(format!("keyboard:k{}", keyboard.dwNumberOfKeysTotal));
    }

    if info.dwType == RIM_TYPEMOUSE {
        let mouse = unsafe { info.Anonymous.mouse };
        return Some(format!(
            "mouse:b{}:h{}",
            mouse.dwNumberOfButtons, mouse.fHasHorizontalWheel
        ));
    }

    if info.dwType == RIM_TYPEHID {
        let hid = unsafe { info.Anonymous.hid };
        return Some(format!(
            "hid:{:04x}:{:04x}:{:04x}:{:04x}",
            saturating_u16(hid.dwVendorId),
            saturating_u16(hid.dwProductId),
            hid.usUsagePage,
            hid.usUsage
        ));
    }

    None
}

/// Return one runtime device kind from one raw device handle.
fn raw_device_kind(raw_device: isize) -> InputDeviceKind {
    // query per-device RID metadata
    let handle = raw_device as HANDLE;
    let mut info = RID_DEVICE_INFO {
        cbSize: mem::size_of::<RID_DEVICE_INFO>() as u32,
        dwType: 0,
        Anonymous: unsafe { mem::zeroed() },
    };
    let mut size = mem::size_of::<RID_DEVICE_INFO>() as u32;
    let status = unsafe {
        GetRawInputDeviceInfoW(
            handle,
            RIDI_DEVICEINFO,
            &mut info as *mut _ as *mut c_void,
            &mut size,
        )
    };
    if status == u32::MAX {
        return InputDeviceKind::Raw;
    }

    if info.dwType == RIM_TYPEKEYBOARD {
        return InputDeviceKind::Keyboard;
    }
    if info.dwType == RIM_TYPEMOUSE {
        return InputDeviceKind::Mouse;
    }
    if info.dwType == RIM_TYPEHID {
        let hid = unsafe { info.Anonymous.hid };
        return classify_hid_device_kind(hid.usUsagePage, hid.usUsage);
    }

    InputDeviceKind::Raw
}

/// Return one lowercased backend raw-device name string.
fn raw_device_name(raw_device: isize) -> Option<String> {
    // query UTF-16 device name size from user32
    let handle = raw_device as HANDLE;
    let mut code_units = 0u32;
    let status = unsafe {
        GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, ptr::null_mut(), &mut code_units)
    };
    if status == u32::MAX || code_units == 0 {
        return None;
    }

    // read UTF-16 device name and strip one optional trailing nul unit
    let mut buffer = vec![0u16; code_units as usize];
    let status = unsafe {
        GetRawInputDeviceInfoW(
            handle,
            RIDI_DEVICENAME,
            buffer.as_mut_ptr().cast(),
            &mut code_units,
        )
    };
    if status == u32::MAX || code_units == 0 {
        return None;
    }

    let limit = usize::min(buffer.len(), code_units as usize);
    let name_len = buffer[..limit]
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(limit);
    if name_len == 0 {
        return None;
    }

    Some(String::from_utf16_lossy(&buffer[..name_len]).to_ascii_lowercase())
}

/// Saturate one u32 count into u16.
fn saturating_u16(value: u32) -> u16 {
    value.min(u32::from(u16::MAX)) as u16
}

/// Classify one HID usage tuple into one runtime device kind.
fn classify_hid_device_kind(usage_page: u16, usage: u16) -> InputDeviceKind {
    if usage_page == HID_USAGE_PAGE_GENERIC {
        if usage == HID_USAGE_GENERIC_MOUSE {
            return InputDeviceKind::Mouse;
        }

        if usage == HID_USAGE_GENERIC_KEYBOARD {
            return InputDeviceKind::Keyboard;
        }

        if usage == HID_USAGE_GENERIC_JOYSTICK || usage == HID_USAGE_GENERIC_GAMEPAD {
            return InputDeviceKind::Gamepad;
        }

        return InputDeviceKind::Raw;
    }

    if usage_page == HID_USAGE_PAGE_DIGITIZER {
        if usage == HID_USAGE_DIGITIZER_TOUCH_SCREEN
            || usage == HID_USAGE_DIGITIZER_TOUCH_PAD
            || usage == HID_USAGE_DIGITIZER_FINGER
        {
            return InputDeviceKind::Touch;
        }

        if usage == HID_USAGE_DIGITIZER_PEN {
            return InputDeviceKind::Pen;
        }

        return InputDeviceKind::Raw;
    }

    InputDeviceKind::Raw
}

/// Resolve one sensor kind from one HID sensor usage value.
fn sensor_kind_from_usage(usage_page: u16, usage: u16) -> Option<InputSensorKind> {
    if usage_page != HID_USAGE_PAGE_SENSOR {
        return None;
    }

    if usage == HID_USAGE_SENSOR_ACCELEROMETER_3D {
        return Some(InputSensorKind::Accelerometer);
    }

    if usage == HID_USAGE_SENSOR_GYROMETER_3D {
        return Some(InputSensorKind::Gyroscope);
    }

    if usage == HID_USAGE_SENSOR_MAGNETOMETER_3D {
        return Some(InputSensorKind::Magnetometer);
    }

    if usage == HID_USAGE_SENSOR_GRAVITY_VECTOR {
        return Some(InputSensorKind::Gravity);
    }

    if usage == HID_USAGE_SENSOR_LINEAR_ACCELERATION {
        return Some(InputSensorKind::LinearAcceleration);
    }

    if usage == HID_USAGE_SENSOR_DEVICE_ORIENTATION {
        return Some(InputSensorKind::Orientation);
    }

    None
}

/// Probed hid descriptor metadata used to refine runtime capability fields.
#[derive(Debug, Clone)]
struct HidDescriptorProbe {
    /// Input report byte length.
    input_report_bytes: u16,
    /// Output report byte length.
    output_report_bytes: u16,
    /// Feature report byte length.
    feature_report_bytes: u16,
    /// Logical input button count.
    input_button_count: u16,
    /// Logical input value count.
    input_value_count: u16,
    /// Flattened input value-capability spans.
    input_value_capabilities: Vec<RawHidValueCapability>,
    /// Flattened input button-capability spans.
    input_button_capabilities: Vec<RawHidButtonCapability>,
}

/// Return one hid value-capability usage range.
fn hid_value_usage_range(capability: &HIDP_VALUE_CAPS) -> (u16, u16) {
    if capability.IsRange != 0 {
        let range = unsafe { capability.Anonymous.Range };
        return (range.UsageMin, range.UsageMax);
    }

    let not_range = unsafe { capability.Anonymous.NotRange };
    (not_range.Usage, not_range.Usage)
}

/// Return one hid button-capability usage range.
fn hid_button_usage_range(capability: &HIDP_BUTTON_CAPS) -> (u16, u16) {
    if capability.IsRange != 0 {
        let range = unsafe { capability.Anonymous.Range };
        return (range.UsageMin, range.UsageMax);
    }

    let not_range = unsafe { capability.Anonymous.NotRange };
    (not_range.Usage, not_range.Usage)
}

/// Return one normalized logical span for one hid value-capability entry.
fn hid_logical_span(capability: &RawHidValueCapability) -> f64 {
    let min = capability.logical_min as f64;
    let max = capability.logical_max as f64;
    (max - min).abs()
}

/// Return one inclusive usage-count for one hid usage span.
fn hid_usage_span_count(usage_min: u16, usage_max: u16) -> u32 {
    u32::from(usage_max.saturating_sub(usage_min).saturating_add(1))
}

/// Return whether one usage is one gamepad axis usage.
fn is_gamepad_axis_usage(usage: u16) -> bool {
    matches!(
        usage,
        HID_USAGE_GENERIC_X
            | HID_USAGE_GENERIC_Y
            | HID_USAGE_GENERIC_Z
            | HID_USAGE_GENERIC_RX
            | HID_USAGE_GENERIC_RY
            | HID_USAGE_GENERIC_RZ
            | HID_USAGE_GENERIC_SLIDER
            | HID_USAGE_GENERIC_DIAL
            | HID_USAGE_GENERIC_WHEEL
            | HID_USAGE_GENERIC_HAT_SWITCH
    )
}

/// Return one flattened count for one hid button page.
fn hid_button_count_for_page(capabilities: &[RawHidButtonCapability], usage_page: u16) -> u16 {
    let mut count = 0u32;
    for capability in capabilities {
        if capability.usage_page != usage_page {
            continue;
        }

        count = count.saturating_add(hid_usage_span_count(
            capability.usage_min,
            capability.usage_max,
        ));
    }

    saturating_u16(count)
}

/// Return one flattened count for gamepad axis usages.
fn hid_gamepad_axis_count(capabilities: &[RawHidValueCapability]) -> u16 {
    let mut count = 0u32;
    for capability in capabilities {
        if capability.usage_page != HID_USAGE_PAGE_GENERIC {
            continue;
        }

        for usage in capability.usage_min..=capability.usage_max {
            if is_gamepad_axis_usage(usage) {
                count = count.saturating_add(1);
            }
        }
    }

    saturating_u16(count)
}

/// Return whether one hid parser status indicates success.
fn hid_status_is_success(status: i32) -> bool {
    status == HIDP_STATUS_SUCCESS
}

/// Execute one callback with hid preparsed data and always release the parser buffer.
fn with_hid_preparsed_data<R>(
    handle: HANDLE,
    operation: &'static str,
    callback: impl FnOnce(PHIDP_PREPARSED_DATA) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    // resolve preparsed data from the hid handle
    let mut preparsed: PHIDP_PREPARSED_DATA = 0;
    let status = unsafe { HidD_GetPreparsedData(handle, &mut preparsed) };
    if status == 0 || preparsed == 0 {
        return Err(service_error(
            operation,
            "hid preparsed data is unavailable for this device",
        ));
    }

    // run caller logic before releasing the preparsed buffer
    let result = callback(preparsed);
    let _ = unsafe { HidD_FreePreparsedData(preparsed) };
    result
}

/// Open one hid path for descriptor probing with read-write or read-only fallback.
fn open_hid_path_for_probe(path: &str) -> Option<HANDLE> {
    let wide = normalize_hid_device_path(path).ok()?;

    // prefer read-write access so output-report capability probing remains available
    let read_write_handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if read_write_handle != 0 && read_write_handle != INVALID_HANDLE_VALUE {
        return Some(read_write_handle);
    }

    // fall back to read-only access when read-write is denied
    let read_only_handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if read_only_handle == 0 || read_only_handle == INVALID_HANDLE_VALUE {
        return None;
    }

    Some(read_only_handle)
}

/// Probe hid report and control metadata from one raw path.
fn probe_hid_descriptor(path: &str) -> Option<HidDescriptorProbe> {
    let handle = open_hid_path_for_probe(path)?;

    // resolve hid parser capabilities and value spans
    let probe = with_hid_preparsed_data(handle, "destack.input.device.list", |preparsed| {
        let mut caps = unsafe { mem::zeroed::<HIDP_CAPS>() };
        let status = unsafe { HidP_GetCaps(preparsed, &mut caps) };
        if !hid_status_is_success(status) {
            return Err(service_error(
                "destack.input.device.list",
                "HidP_GetCaps failed for hid descriptor probe",
            ));
        }

        // query value caps and flatten usage spans
        let mut value_cap_count = caps.NumberInputValueCaps;
        let mut value_caps =
            vec![unsafe { mem::zeroed::<HIDP_VALUE_CAPS>() }; value_cap_count as usize];
        let mut input_value_capabilities = Vec::new();
        let mut input_value_count = 0u32;
        if value_cap_count > 0 {
            let status = unsafe {
                HidP_GetValueCaps(
                    HidP_Input,
                    value_caps.as_mut_ptr(),
                    &mut value_cap_count,
                    preparsed,
                )
            };
            if hid_status_is_success(status) {
                for value_cap in value_caps.into_iter().take(value_cap_count as usize) {
                    let (usage_min, usage_max) = hid_value_usage_range(&value_cap);
                    let flattened = RawHidValueCapability {
                        usage_page: value_cap.UsagePage,
                        usage_min,
                        usage_max,
                        link_collection: value_cap.LinkCollection,
                        report_id: value_cap.ReportID,
                        logical_min: value_cap.LogicalMin,
                        logical_max: value_cap.LogicalMax,
                    };

                    let flattened_count = usage_max.saturating_sub(usage_min).saturating_add(1);
                    input_value_count =
                        input_value_count.saturating_add(u32::from(flattened_count));
                    input_value_capabilities.push(flattened);
                }
            }
        }

        // query button caps and derive one flattened button usage count
        let mut button_cap_count = caps.NumberInputButtonCaps;
        let mut button_caps =
            vec![unsafe { mem::zeroed::<HIDP_BUTTON_CAPS>() }; button_cap_count as usize];
        let mut input_button_count = 0u32;
        let mut input_button_capabilities = Vec::new();
        if button_cap_count > 0 {
            let status = unsafe {
                HidP_GetButtonCaps(
                    HidP_Input,
                    button_caps.as_mut_ptr(),
                    &mut button_cap_count,
                    preparsed,
                )
            };
            if hid_status_is_success(status) {
                for button_cap in button_caps.into_iter().take(button_cap_count as usize) {
                    let (usage_min, usage_max) = hid_button_usage_range(&button_cap);
                    let flattened_count = usage_max.saturating_sub(usage_min).saturating_add(1);
                    input_button_count =
                        input_button_count.saturating_add(u32::from(flattened_count));
                    input_button_capabilities.push(RawHidButtonCapability {
                        usage_page: button_cap.UsagePage,
                        usage_min,
                        usage_max,
                        link_collection: button_cap.LinkCollection,
                        report_id: button_cap.ReportID,
                    });
                }
            }
        }

        Ok(HidDescriptorProbe {
            input_report_bytes: caps.InputReportByteLength,
            output_report_bytes: caps.OutputReportByteLength,
            feature_report_bytes: caps.FeatureReportByteLength,
            input_button_count: saturating_u16(input_button_count),
            input_value_count: saturating_u16(input_value_count),
            input_value_capabilities,
            input_button_capabilities,
        })
    })
    .ok();

    unsafe {
        CloseHandle(handle);
    }

    probe
}

/// Return one raw-input descriptor for one device-list entry.
fn descriptor_from_device_entry(entry: RAWINPUTDEVICELIST) -> Option<RawInputDeviceDescriptor> {
    let raw_device = entry.hDevice;
    let id = monitor_device_id(raw_device);
    let raw_path = raw_device_name(raw_device);
    let name = raw_path.clone().unwrap_or_else(|| id.clone());

    // query per-device RID metadata
    let mut info = RID_DEVICE_INFO {
        cbSize: mem::size_of::<RID_DEVICE_INFO>() as u32,
        dwType: 0,
        Anonymous: unsafe { mem::zeroed() },
    };
    let mut size = mem::size_of::<RID_DEVICE_INFO>() as u32;
    let status = unsafe {
        GetRawInputDeviceInfoW(
            entry.hDevice,
            RIDI_DEVICEINFO,
            &mut info as *mut _ as *mut c_void,
            &mut size,
        )
    };
    if status == u32::MAX {
        return None;
    }

    // map RID metadata into runtime descriptor fields
    let kind: InputDeviceKind;
    let mut vendor_id = 0u16;
    let mut product_id = 0u16;
    let usage_page: u16;
    let usage: u16;
    let mut report_size = 0u16;
    let mut output_report_size = 0u16;
    let mut feature_report_size = 0u16;
    let mut key_count = 0u16;
    let mut button_count = 0u16;
    let mut axis_count = 0u16;
    let mut supports_rumble = false;
    let mut supports_battery = false;
    let mut supports_light = false;
    let mut supports_raw_hid = false;
    let mut value_capabilities = Vec::new();
    let mut button_capabilities = Vec::new();

    // populate keyboard metadata from RID keyboard descriptors
    if info.dwType == RIM_TYPEKEYBOARD {
        let keyboard = unsafe { info.Anonymous.keyboard };
        kind = InputDeviceKind::Keyboard;
        usage_page = HID_USAGE_PAGE_GENERIC;
        usage = HID_USAGE_GENERIC_KEYBOARD;
        key_count = if keyboard.dwNumberOfKeysTotal > 0 {
            saturating_u16(keyboard.dwNumberOfKeysTotal)
        } else {
            255
        };
    }
    // populate mouse metadata from RID mouse descriptors
    else if info.dwType == RIM_TYPEMOUSE {
        let mouse = unsafe { info.Anonymous.mouse };
        kind = InputDeviceKind::Mouse;
        usage_page = HID_USAGE_PAGE_GENERIC;
        usage = HID_USAGE_GENERIC_MOUSE;
        button_count = if mouse.dwNumberOfButtons > 0 {
            saturating_u16(mouse.dwNumberOfButtons)
        } else {
            5
        };
        axis_count = if mouse.fHasHorizontalWheel != 0 { 4 } else { 3 };
    }
    // populate hid metadata and parser-derived capabilities from RID hid descriptors
    else if info.dwType == RIM_TYPEHID {
        let hid = unsafe { info.Anonymous.hid };
        kind = classify_hid_device_kind(hid.usUsagePage, hid.usUsage);
        vendor_id = saturating_u16(hid.dwVendorId);
        product_id = saturating_u16(hid.dwProductId);
        usage_page = hid.usUsagePage;
        usage = hid.usUsage;
        supports_raw_hid = true;

        if let Some(path) = raw_path.as_deref()
            && let Some(probe) = probe_hid_descriptor(path)
        {
            report_size = probe.input_report_bytes;
            output_report_size = probe.output_report_bytes;
            feature_report_size = probe.feature_report_bytes;
            value_capabilities = probe.input_value_capabilities;
            button_capabilities = probe.input_button_capabilities;

            if kind == InputDeviceKind::Keyboard {
                key_count = probe.input_button_count.max(255);
            } else if kind == InputDeviceKind::Mouse {
                button_count = probe.input_button_count.max(5);
                axis_count = probe.input_value_count.max(2);
            } else if kind == InputDeviceKind::Touch || kind == InputDeviceKind::Pen {
                button_count = probe.input_button_count.max(1);
                axis_count = probe.input_value_count.max(2);
            } else if kind == InputDeviceKind::Gamepad {
                let probed_button_count =
                    hid_button_count_for_page(&button_capabilities, HID_USAGE_PAGE_BUTTON);
                let probed_axis_count = hid_gamepad_axis_count(&value_capabilities);
                button_count = probed_button_count.max(10);
                axis_count = probed_axis_count.max(4);
                supports_rumble = output_report_size > 0;
                supports_battery = feature_report_size > 0;
                supports_light = vendor_id == SONY_VENDOR_ID
                    && (output_report_size == SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES
                        || output_report_size == SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES);
            } else if kind == InputDeviceKind::Raw {
                button_count = probe.input_button_count;
                axis_count = probe.input_value_count;
            }
        }
    } else {
        return None;
    }

    let has_sensor_capabilities = value_capabilities.iter().any(|capability| {
        if capability.usage_page != HID_USAGE_PAGE_SENSOR {
            return false;
        }

        for usage in capability.usage_min..=capability.usage_max {
            if sensor_kind_from_usage(HID_USAGE_PAGE_SENSOR, usage).is_some() {
                return true;
            }
        }

        false
    });

    Some(RawInputDeviceDescriptor {
        id: id.clone(),
        instance_id: id.clone(),
        hardware_id: id,
        name,
        raw_path,
        kind,
        vendor_id,
        product_id,
        usage_page,
        usage,
        report_size,
        output_report_size,
        feature_report_size,
        key_count,
        button_count,
        axis_count,
        supports_text: false,
        supports_rumble,
        supports_battery,
        supports_light,
        supports_pointer_grab: kind == InputDeviceKind::Mouse,
        supports_raw_hid,
        supports_sensors: has_sensor_capabilities
            || sensor_kind_from_usage(usage_page, usage).is_some(),
        supports_player_index: false,
        value_capabilities,
        button_capabilities,
    })
}

/// Enumerate raw-input devices from user32 and map into runtime descriptors.
fn enumerate_raw_input_devices(
    operation: &'static str,
) -> RuntimeResult<Vec<RawInputDeviceDescriptor>> {
    // query device-list length
    let mut count = 0u32;
    let status = unsafe {
        GetRawInputDeviceList(
            ptr::null_mut(),
            &mut count,
            mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        )
    };
    if status == u32::MAX {
        let code = core_platform::last_error_code();
        return Err(core_platform::io_error_with_code(operation, code));
    }
    if count == 0 {
        return Ok(Vec::new());
    }

    // read raw device-list entries with one retry for races
    let mut retries = 0u32;
    loop {
        let mut entries = Vec::with_capacity(count as usize);
        entries.resize_with(count as usize, || unsafe {
            mem::zeroed::<RAWINPUTDEVICELIST>()
        });
        let mut next_count = count;
        let status = unsafe {
            GetRawInputDeviceList(
                entries.as_mut_ptr(),
                &mut next_count,
                mem::size_of::<RAWINPUTDEVICELIST>() as u32,
            )
        };
        if status == u32::MAX {
            let code = core_platform::last_error_code();
            if retries < 1 && next_count > count {
                retries += 1;
                count = next_count;
                continue;
            }

            return Err(core_platform::io_error_with_code(operation, code));
        }

        // project entries into descriptors and sort for deterministic ordering
        let mut devices = Vec::new();
        for entry in entries.into_iter().take(status as usize) {
            if let Some(device) = descriptor_from_device_entry(entry) {
                devices.push(device);
            }
        }
        devices.sort_by(|left, right| left.id.cmp(&right.id));
        return Ok(devices);
    }
}

/// List raw-input devices for windows input list calls.
pub(super) fn list_raw_input_devices(
    operation: &'static str,
) -> RuntimeResult<Vec<RawInputDeviceDescriptor>> {
    enumerate_raw_input_devices(operation)
}

/// Resolve one raw-input device descriptor by runtime identifier.
pub(super) fn resolve_raw_input_device(
    id: &str,
    operation: &'static str,
) -> RuntimeResult<Option<RawInputDeviceDescriptor>> {
    let id_lower = id.to_ascii_lowercase();
    let devices = enumerate_raw_input_devices(operation)?;
    Ok(devices
        .into_iter()
        .find(|device| device.id.eq_ignore_ascii_case(&id_lower)))
}

/// Return one stable axis code for one usage tuple.
fn hid_axis_code(usage_page: u16, usage: u16) -> u32 {
    if usage_page == HID_USAGE_PAGE_GENERIC {
        return u32::from(usage);
    }

    (u32::from(usage_page) << 16) | u32::from(usage)
}

/// Return one stable button code for one usage tuple.
fn hid_button_code(usage_page: u16, usage: u16) -> u32 {
    if usage_page == HID_USAGE_PAGE_BUTTON {
        return u32::from(usage);
    }

    (u32::from(usage_page) << 16) | u32::from(usage)
}

/// Return one axis capability row from one hid value-capability span and usage.
fn axis_info_from_hid_value_capability(
    capability: &RawHidValueCapability,
    usage: u16,
) -> InputAxisMetadata {
    let minimum = capability.logical_min as f64;
    let maximum = capability.logical_max as f64;
    let span = (maximum - minimum).abs();
    let resolution = if span > 0.0 { 1.0 / span } else { 0.0 };
    InputAxisMetadata {
        code: hid_axis_code(capability.usage_page, usage),
        minimum,
        maximum,
        flat: 0.0,
        fuzz: 0.0,
        resolution,
    }
}

/// Build axis capability rows from one raw-input descriptor.
fn axis_infos_for_raw_input_device(device: &RawInputDeviceDescriptor) -> Vec<InputAxisMetadata> {
    // derive axis metadata from hid parser capabilities when available
    let mut axes = Vec::new();
    let mut seen_codes = HashSet::new();
    for capability in &device.value_capabilities {
        let include_capability = if device.kind == InputDeviceKind::Gamepad {
            capability.usage_page == HID_USAGE_PAGE_GENERIC
        } else if device.supports_sensors {
            capability.usage_page == HID_USAGE_PAGE_SENSOR
        } else {
            capability.usage_page == HID_USAGE_PAGE_GENERIC
        };
        if !include_capability {
            continue;
        }

        for usage in capability.usage_min..=capability.usage_max {
            if device.kind == InputDeviceKind::Gamepad && !is_gamepad_axis_usage(usage) {
                continue;
            }

            let code = hid_axis_code(capability.usage_page, usage);
            if !seen_codes.insert(code) {
                continue;
            }

            axes.push(axis_info_from_hid_value_capability(capability, usage));
        }
    }
    if !axes.is_empty() {
        return axes;
    }

    // fall back to count-derived axis rows when no hid parser metadata is available
    let mut fallback_axes = Vec::new();
    for code in 0..u32::from(device.axis_count) {
        fallback_axes.push(InputAxisMetadata {
            code,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        });
    }

    fallback_axes
}

/// Build button capability rows from one raw-input descriptor.
fn button_infos_for_raw_input_device(
    device: &RawInputDeviceDescriptor,
) -> Vec<InputButtonMetadata> {
    // keep keyboard button layout aligned to virtual-key code space
    if device.kind == InputDeviceKind::Keyboard {
        let mut buttons = Vec::new();
        for code in 0..u32::from(device.key_count) {
            buttons.push(InputButtonMetadata {
                code,
                analog: false,
            });
        }

        return buttons;
    }

    // derive button metadata from hid parser capabilities when available
    let mut buttons = Vec::new();
    let mut seen_codes = HashSet::new();
    for capability in &device.button_capabilities {
        for usage in capability.usage_min..=capability.usage_max {
            let code = hid_button_code(capability.usage_page, usage);
            if !seen_codes.insert(code) {
                continue;
            }

            buttons.push(InputButtonMetadata {
                code,
                analog: false,
            });
        }
    }
    if !buttons.is_empty() {
        return buttons;
    }

    // fall back to count-derived button rows when hid parser metadata is unavailable
    let mut fallback_buttons = Vec::new();
    for code in 0..u32::from(device.button_count) {
        fallback_buttons.push(InputButtonMetadata {
            code,
            analog: false,
        });
    }

    fallback_buttons
}

/// Build one runtime capabilities payload from one raw-input descriptor.
pub(super) fn capabilities_for_raw_input_device(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
) -> InputDeviceCapabilities {
    let mut kinds = Vec::new();
    if device.kind == InputDeviceKind::Keyboard {
        kinds.push(InputDeviceCapabilityKind::Keyboard);
    }
    if device.kind == InputDeviceKind::Mouse {
        kinds.push(InputDeviceCapabilityKind::Pointer);
    }
    if device.kind == InputDeviceKind::Touch {
        kinds.push(InputDeviceCapabilityKind::Touch);
    }
    if device.kind == InputDeviceKind::Pen {
        kinds.push(InputDeviceCapabilityKind::Pen);
        kinds.push(InputDeviceCapabilityKind::Pointer);
    }
    if device.kind == InputDeviceKind::Gamepad {
        kinds.push(InputDeviceCapabilityKind::Gamepad);
    }
    if device.supports_sensors {
        kinds.push(InputDeviceCapabilityKind::Sensor);
    }
    if device.supports_rumble {
        kinds.push(InputDeviceCapabilityKind::Haptics);
    }
    if device.supports_text {
        kinds.push(InputDeviceCapabilityKind::TextInput);
    }
    let axes = axis_infos_for_raw_input_device(device);
    let buttons = button_infos_for_raw_input_device(device);
    let axis_from_backend = !device.value_capabilities.is_empty() && !axes.is_empty();
    let axis_from_count = !axis_from_backend && !axes.is_empty();
    let button_from_backend = device.kind != InputDeviceKind::Keyboard
        && !device.button_capabilities.is_empty()
        && !buttons.is_empty();
    let button_from_count = !button_from_backend && !buttons.is_empty();
    let has_backend_metadata = axis_from_backend || button_from_backend;
    let has_count_metadata = axis_from_count || button_from_count;
    let metadata_origin = if has_backend_metadata && has_count_metadata {
        InputCapabilityMetadataOrigin::Mixed
    } else if has_backend_metadata {
        InputCapabilityMetadataOrigin::BackendDescriptor
    } else if has_count_metadata {
        InputCapabilityMetadataOrigin::CountDerived
    } else {
        InputCapabilityMetadataOrigin::DeviceSummary
    };
    let axis_metadata_fidelity = if axis_from_backend {
        InputCapabilityMetadataFidelity::Full
    } else {
        InputCapabilityMetadataFidelity::Minimal
    };
    let button_metadata_fidelity = if button_from_backend {
        InputCapabilityMetadataFidelity::Full
    } else if device.kind == InputDeviceKind::Keyboard && !buttons.is_empty() {
        InputCapabilityMetadataFidelity::Partial
    } else {
        InputCapabilityMetadataFidelity::Minimal
    };

    InputDeviceCapabilities {
        kinds: binding.store_array(kinds),
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        metadata_origin,
        axis_metadata_fidelity,
        button_metadata_fidelity,
        supports_relative_pointer: device.kind == InputDeviceKind::Mouse,
        supports_pointer_grab: device.kind == InputDeviceKind::Mouse,
        supports_pointer_capture: false,
        supports_pointer_warp: device.kind == InputDeviceKind::Mouse,
        supports_text_input: device.supports_text,
        supports_composition: false,
        supports_rumble: device.supports_rumble,
        supports_trigger_rumble: false,
        supports_sensors: device.supports_sensors,
        supports_battery_state: device.supports_battery,
        supports_light_control: device.supports_light,
        supports_raw_hid: device.supports_raw_hid,
        supports_player_index: device.supports_player_index,
    }
}

/// Resolve one monitor device id for a raw device-change message.
fn resolve_monitor_device_id(
    kind: u32,
    raw_device: isize,
    monitor_device_ids: &mut HashMap<isize, String>,
) -> String {
    if kind == GIDC_ARRIVAL {
        let device_id = monitor_device_id(raw_device);
        monitor_device_ids.insert(raw_device, device_id.clone());
        return device_id;
    }

    if let Some(device_id) = monitor_device_ids.remove(&raw_device) {
        return device_id;
    }

    monitor_device_id(raw_device)
}

/// Normalize one raw mouse packet into runtime pointer-motion payload.
fn normalize_raw_mouse_motion(mouse: &windows_sys::Win32::UI::Input::RAWMOUSE) -> (f64, f64, u32) {
    let is_absolute = (mouse.usFlags & RAW_MOUSE_MOVE_ABSOLUTE) != 0;
    if is_absolute {
        let x = (mouse.lLastX as f64 / RAW_MOUSE_COORDINATE_MAX).clamp(0.0, 1.0);
        let y = (mouse.lLastY as f64 / RAW_MOUSE_COORDINATE_MAX).clamp(0.0, 1.0);
        let code = u32::from(mouse.usFlags & (RAW_MOUSE_MOVE_ABSOLUTE | RAW_MOUSE_VIRTUAL_DESKTOP));
        (x, y, code)
    } else {
        (mouse.lLastX as f64, mouse.lLastY as f64, 0)
    }
}

/// Register one runtime teardown finalizer for the raw-input service.
fn register_runtime_finalizer(
    binding: &BindingCallContext,
    service: &Arc<WindowsRawInputService>,
    runtime_state: &Arc<WindowsRawInputRuntimeState>,
) {
    if runtime_state
        .finalizer_registered
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let worker_id = binding.worker().id;
    let service = Arc::clone(service);
    let runtime_state = Arc::clone(runtime_state);
    binding.worker().finalizers.register(move || {
        service.unregister_runtime(worker_id);
        clear_raw_gamepad_decoder_cache(&runtime_state);
    });
}

/// Return the raw gamepad decoder cache used by polling snapshots.
fn raw_gamepad_decoder_cache(
    runtime_state: &WindowsRawInputRuntimeState,
) -> &Mutex<HashMap<String, RawGamepadDecoderEntry>> {
    &runtime_state.gamepad_decoder_cache
}

/// Release all cached raw gamepad decoder entries.
fn clear_raw_gamepad_decoder_cache(runtime_state: &WindowsRawInputRuntimeState) {
    let mut cache = raw_gamepad_decoder_cache(runtime_state).lock();
    let entries = cache.drain().map(|(_, entry)| entry).collect::<Vec<_>>();
    drop(cache);

    for entry in entries {
        release_raw_gamepad_decoder_entry(entry);
    }
}

/// Release one cached raw gamepad decoder entry.
fn release_raw_gamepad_decoder_entry(entry: RawGamepadDecoderEntry) {
    if entry.preparsed != 0 {
        let _ = unsafe { HidD_FreePreparsedData(entry.preparsed) };
    }

    if entry.handle != 0 && entry.handle != INVALID_HANDLE_VALUE {
        unsafe {
            CloseHandle(entry.handle);
        }
    }
}

/// Remove one cached raw gamepad decoder entry for one runtime device identifier.
fn remove_raw_gamepad_decoder_entry(runtime_state: &WindowsRawInputRuntimeState, device_id: &str) {
    let mut cache = raw_gamepad_decoder_cache(runtime_state).lock();
    let Some(entry) = cache.remove(device_id) else {
        return;
    };

    release_raw_gamepad_decoder_entry(entry);
}

/// Return whether one value-capability table includes one usage.
fn value_usage_supported_by_capabilities(
    capabilities: &[RawHidValueCapability],
    usage_page: u16,
    usage: u16,
) -> bool {
    for capability in capabilities {
        if capability.usage_page != usage_page {
            continue;
        }

        if usage >= capability.usage_min && usage <= capability.usage_max {
            return true;
        }
    }

    false
}

/// Return whether one button-capability table includes one usage.
fn button_usage_supported_by_capabilities(
    capabilities: &[RawHidButtonCapability],
    usage_page: u16,
    usage: u16,
) -> bool {
    for capability in capabilities {
        if capability.usage_page != usage_page {
            continue;
        }

        if usage >= capability.usage_min && usage <= capability.usage_max {
            return true;
        }
    }

    false
}

/// Classify one stable gamepad mapping profile from descriptor capabilities.
fn classify_gamepad_mapping_from_capabilities(
    device: &RawInputDeviceDescriptor,
) -> InputGamepadMappingType {
    let has_face_buttons = button_usage_supported_by_capabilities(
        &device.button_capabilities,
        HID_USAGE_PAGE_BUTTON,
        1,
    ) && button_usage_supported_by_capabilities(
        &device.button_capabilities,
        HID_USAGE_PAGE_BUTTON,
        2,
    ) && button_usage_supported_by_capabilities(
        &device.button_capabilities,
        HID_USAGE_PAGE_BUTTON,
        3,
    ) && button_usage_supported_by_capabilities(
        &device.button_capabilities,
        HID_USAGE_PAGE_BUTTON,
        4,
    );

    let has_left_stick = value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_X,
    ) && value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_Y,
    );
    let has_right_stick = (value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_RX,
    ) && value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_RY,
    )) || (value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_Z,
    ) && value_usage_supported_by_capabilities(
        &device.value_capabilities,
        HID_USAGE_PAGE_GENERIC,
        HID_USAGE_GENERIC_RZ,
    ));

    if has_face_buttons && has_left_stick && has_right_stick {
        return InputGamepadMappingType::Standard;
    }

    InputGamepadMappingType::None
}

/// Build one deterministic report-id probe list from descriptor capabilities.
fn gamepad_report_id_candidates(device: &RawInputDeviceDescriptor) -> Vec<u8> {
    let mut report_ids = HashSet::new();
    for capability in &device.button_capabilities {
        if capability.report_id != 0 {
            report_ids.insert(capability.report_id);
        }
    }

    for capability in &device.value_capabilities {
        if capability.report_id != 0 {
            report_ids.insert(capability.report_id);
        }
    }

    let mut report_ids = report_ids.into_iter().collect::<Vec<_>>();
    report_ids.sort_unstable();
    if !report_ids.contains(&0) {
        report_ids.push(0);
    }

    report_ids
}

/// Open one cached raw gamepad decoder entry from one descriptor.
fn open_raw_gamepad_decoder_entry(
    device: &RawInputDeviceDescriptor,
    operation: &'static str,
) -> RuntimeResult<RawGamepadDecoderEntry> {
    let handle = open_hid_device_handle(device, FILE_GENERIC_READ, operation)?;
    let mut preparsed: PHIDP_PREPARSED_DATA = 0;
    let status = unsafe { HidD_GetPreparsedData(handle, &mut preparsed) };
    if status == 0 || preparsed == 0 {
        unsafe {
            CloseHandle(handle);
        }

        return Err(core_platform::io_error("HidD_GetPreparsedData"));
    }

    Ok(RawGamepadDecoderEntry {
        handle,
        preparsed,
        report_size: device.report_size,
        report_ids: gamepad_report_id_candidates(device),
        mapping: classify_gamepad_mapping_from_capabilities(device),
    })
}

/// Build one io-would-block runtime error for empty raw queues.
fn io_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Build one raw service io runtime error.
fn service_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Push one raw input packet with bounded queue growth.
fn push_input_packet(
    runtime_state: &WindowsRawInputRuntimeState,
    queue: &mut VecDeque<RawInputPacket>,
    packet: RawInputPacket,
) {
    let queue_limit = raw_input_queue_limit(runtime_state);
    if queue.len() >= queue_limit {
        queue.pop_front();

        // coalesce one overflow marker in the queue tail when drops occur
        if let Some(overflow) = queue.back_mut()
            && overflow.kind == windows_core::WindowsInputEventKind::Device
            && overflow.action == InputEventAction::Cancel
            && overflow.code == RAW_INPUT_OVERFLOW_CODE
        {
            overflow.value = overflow.value.saturating_add(1);
            queue.push_back(packet);
            return;
        }

        queue.push_back(RawInputPacket {
            timestamp_ns: packet.timestamp_ns,
            device_id: packet.device_id.clone(),
            kind: windows_core::WindowsInputEventKind::Device,
            action: InputEventAction::Cancel,
            code: RAW_INPUT_OVERFLOW_CODE,
            scan_code: RAW_INPUT_OVERFLOW_CODE,
            value: 1,
            x: 0.0,
            y: 0.0,
            wheel_x: 0.0,
            wheel_y: 0.0,
            buttons: 0,
            modifiers: 0,
            repeat: false,
        });

        // keep queue bounded after inserting one new overflow marker
        if queue.len() >= queue_limit {
            queue.pop_front();
        }
    }

    queue.push_back(packet);
}

/// Push one monitor packet with bounded queue growth.
fn push_monitor_packet(
    runtime_state: &WindowsRawInputRuntimeState,
    queue: &mut VecDeque<RawMonitorPacket>,
    packet: RawMonitorPacket,
) {
    let queue_limit = raw_monitor_queue_limit(runtime_state);
    if queue.len() >= queue_limit {
        queue.pop_front();

        // coalesce one overflow marker in the queue tail when drops occur
        if let Some(overflow) = queue.back_mut()
            && overflow.action == InputEventAction::Cancel
            && overflow.code == RAW_MONITOR_OVERFLOW_CODE
        {
            overflow.value = overflow.value.saturating_add(1);
            queue.push_back(packet);
            return;
        }

        queue.push_back(RawMonitorPacket {
            timestamp_ns: packet.timestamp_ns,
            action: InputEventAction::Cancel,
            device_id: WINDOWS_INPUT_RAW_MONITOR_ID.to_string(),
            device_kind: InputDeviceKind::Raw,
            code: RAW_MONITOR_OVERFLOW_CODE,
            value: 1,
        });

        // keep queue bounded after inserting one new overflow marker
        if queue.len() >= queue_limit {
            queue.pop_front();
        }
    }

    queue.push_back(packet);
}

/// Push one raw-hid packet with bounded queue growth.
fn push_hid_packet(
    runtime_state: &WindowsRawInputRuntimeState,
    queue: &mut VecDeque<RawHidPacket>,
    packet: RawHidPacket,
) {
    if queue.len() >= raw_hid_queue_limit(runtime_state) {
        queue.pop_front();
    }

    queue.push_back(packet);
}

/// Push one touch snapshot with bounded queue growth.
fn push_touch_packet(
    runtime_state: &WindowsRawInputRuntimeState,
    queue: &mut VecDeque<RawTouchPacket>,
    packet: RawTouchPacket,
) {
    if queue.len() >= raw_touch_queue_limit(runtime_state) {
        queue.pop_front();
    }

    queue.push_back(packet);
}

/// Register keyboard and mouse raw-input devices for one window target.
fn ensure_raw_input_registration(hwnd: HWND) -> RuntimeResult<()> {
    // register keyboard, mouse, touch, and sensor lanes for input-sink delivery
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_MOUSE,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_JOYSTICK,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_GAMEPAD,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_DIGITIZER,
            usUsage: 0,
            dwFlags: RIDEV_PAGEONLY | RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_SENSOR,
            usUsage: 0,
            dwFlags: RIDEV_PAGEONLY | RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        },
    ];
    let status = unsafe {
        RegisterRawInputDevices(
            devices.as_ptr(),
            devices.len() as u32,
            mem::size_of::<RAWINPUTDEVICE>() as u32,
        )
    };
    if status == 0 {
        return Err(service_error(
            "destack.input.device.open",
            "RegisterRawInputDevices failed",
        ));
    }

    // register touch messages when supported, but keep raw input alive on hosts without touch window APIs
    let _ = unsafe { RegisterTouchWindow(hwnd, TWF_WANTPALM) };

    Ok(())
}

/// Spawn the raw-input worker and wait for successful initialization.
fn spawn_raw_input_worker(
    _binding: &BindingCallContext,
    service: &Arc<WindowsRawInputService>,
) -> Result<RawInputWorker, String> {
    // spawn the message-thread worker and wait for readiness
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
    let thread_service = Arc::clone(service);
    let handle = start_with_policy(
        "destack-input-raw",
        "destack.input.device.open",
        ExecutionPolicy::process(ExecutionMode::Loop),
        move || {
            raw_input_thread_main(thread_service, ready_tx);
        },
    )
    .map_err(|error| error.to_string())?;

    // require worker readiness before serving any binding calls
    let ready = ready_rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| "raw input thread startup timed out".to_string())?;
    let thread_id = match ready {
        Ok(thread_id) => thread_id,
        Err(error) => {
            let _ = handle.join();
            return Err(error);
        }
    };

    let worker = RawInputWorker {
        thread_id,
        handle: Mutex::new(Some(handle)),
    };

    Ok(worker)
}

/// Run the raw-input message-thread main loop.
fn raw_input_thread_main(
    service: Arc<WindowsRawInputService>,
    ready_tx: mpsc::Sender<Result<u32, String>>,
) {
    // resolve module instance and register a message-only window class
    let instance = unsafe { GetModuleHandleW(ptr::null()) } as HINSTANCE;
    let window_class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(raw_input_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: ptr::null(),
        lpszClassName: CLASS_NAME.as_ptr(),
    };

    let class_atom = unsafe { RegisterClassW(&window_class) };
    if class_atom == 0 {
        let error_code = core_platform::last_error_code() as u32;
        if error_code != ERROR_CLASS_ALREADY_EXISTS {
            let _ = ready_tx.send(Err("RegisterClassW failed".to_string()));
            return;
        }
    }

    // create one callback context and a message-only window for raw input delivery
    let mut window_context = RawInputWindowContext {
        service: Arc::clone(&service),
    };
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            0,
            instance,
            (&mut window_context as *mut RawInputWindowContext).cast(),
        )
    };
    if hwnd == 0 {
        let _ = ready_tx.send(Err("CreateWindowExW failed".to_string()));
        return;
    }

    // bind raw keyboard and mouse usages to this worker window
    if let Err(error) = ensure_raw_input_registration(hwnd) {
        unsafe {
            DestroyWindow(hwnd);
        }

        let _ = ready_tx.send(Err(error.message()));
        return;
    }

    // report readiness before entering the message loop
    service.worker_running.store(true, Ordering::Release);
    let thread_id = unsafe { GetCurrentThreadId() };
    let _ = ready_tx.send(Ok(thread_id));

    // run the host-owned blocking ingress loop until shutdown
    let loop_result: Result<(), String> = {
        run_windows_ingress_loop();
        Ok(())
    };

    // release the worker window on exit
    unsafe {
        DestroyWindow(hwnd);
    }

    // mark worker exit and wake readers so they can surface restartable failures
    service.worker_running.store(false, Ordering::Release);

    for runtime_state in service.runtime_states_snapshot() {
        runtime_state.state.wake.notify_all();
    }

    // crash fast when the host integration contract is unexpectedly broken
    if let Err(error) = loop_result {
        panic!("raw input message loop failed: {error:?}");
    }
}

/// Dispatch raw input and device-change window messages.
unsafe extern "system" fn raw_input_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => {
            // install one callback context pointer for subsequent messages
            let create = lparam as *const CREATESTRUCTW;
            if create.is_null() {
                return 0;
            }

            let context = unsafe { (*create).lpCreateParams } as isize;
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, context);
            }
            1
        }
        WM_INPUT => {
            let Some(context) = (unsafe { raw_input_window_context(hwnd) }) else {
                return 0;
            };

            unsafe {
                handle_raw_input_message(&(*context).service, lparam);
            }
            0
        }
        WM_INPUT_DEVICE_CHANGE => {
            let Some(context) = (unsafe { raw_input_window_context(hwnd) }) else {
                return 0;
            };

            unsafe {
                handle_raw_device_change_message(&(*context).service, wparam as u32, lparam);
            }
            0
        }
        WM_TOUCH => {
            let Some(context) = (unsafe { raw_input_window_context(hwnd) }) else {
                let _ = unsafe { CloseTouchInputHandle(lparam) };
                return 0;
            };

            unsafe {
                handle_touch_message(&(*context).service, wparam, lparam);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Worker-side context passed to the message-only raw-input window.
#[derive(Debug)]
struct RawInputWindowContext {
    /// Process-global raw-input service.
    service: Arc<WindowsRawInputService>,
}

/// Resolve one window callback context from one message-only window.
unsafe fn raw_input_window_context(hwnd: HWND) -> Option<*mut RawInputWindowContext> {
    let context = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RawInputWindowContext };
    if context.is_null() {
        return None;
    }

    Some(context)
}

/// Handle one raw device-change notification.
fn handle_raw_device_change_message(
    service: &WindowsRawInputService,
    kind: u32,
    raw_device: isize,
) {
    // map Win32 device-change kinds into runtime monitor actions
    let action = if kind == GIDC_ARRIVAL {
        InputEventAction::Connect
    } else if kind == GIDC_REMOVAL {
        InputEventAction::Disconnect
    } else {
        return;
    };

    // fan out one monitor packet into every live runtime queue
    let timestamp_ns = now_timestamp_ns();
    for runtime_state in service.runtime_states_snapshot() {
        let mut queues = runtime_state.state.queues.lock();
        let device_kind = if kind == GIDC_ARRIVAL {
            let kind = raw_device_kind(raw_device);
            queues.monitor_device_kinds.insert(raw_device, kind);
            kind
        } else {
            queues
                .monitor_device_kinds
                .remove(&raw_device)
                .unwrap_or_else(|| raw_device_kind(raw_device))
        };
        let device_id = resolve_monitor_device_id(kind, raw_device, &mut queues.monitor_device_ids);

        push_monitor_packet(
            &runtime_state,
            &mut queues.monitor,
            RawMonitorPacket {
                timestamp_ns,
                action,
                device_id,
                device_kind,
                code: kind,
                value: if action == InputEventAction::Connect {
                    1
                } else {
                    0
                },
            },
        );
        runtime_state.state.wake.notify_all();
    }
}

/// Map Win32 touch flags into one stable touch phase.
fn touch_phase_from_flags(flags: u32) -> InputTouchContactPhase {
    if (flags & TOUCHEVENTF_DOWN) != 0 {
        return InputTouchContactPhase::Begin;
    }

    if (flags & TOUCHEVENTF_UP) != 0 {
        return InputTouchContactPhase::End;
    }

    if (flags & TOUCHEVENTF_MOVE) != 0 {
        return InputTouchContactPhase::Move;
    }

    InputTouchContactPhase::Move
}

/// Convert one touch coordinate from one hundredth-pixel unit into pixels.
fn touch_coordinate_from_raw(value: i32) -> f64 {
    value as f64 / 100.0
}

/// Handle one WM_TOUCH payload and enqueue per-device touch snapshots.
fn handle_touch_message(service: &WindowsRawInputService, wparam: usize, lparam: isize) {
    // decode touch-packet count from the low word of wparam
    let touch_count = (wparam & 0xffffusize) as u32;
    if touch_count == 0 {
        let _ = unsafe { CloseTouchInputHandle(lparam) };
        return;
    }

    // read touch payloads from user32 before mutating queue state
    let mut touches = Vec::new();
    touches.resize_with(touch_count as usize, || unsafe {
        std::mem::zeroed::<TOUCHINPUT>()
    });
    let status = unsafe {
        GetTouchInputInfo(
            lparam,
            touch_count,
            touches.as_mut_ptr(),
            std::mem::size_of::<TOUCHINPUT>() as i32,
        )
    };
    let _ = unsafe { CloseTouchInputHandle(lparam) };
    if status == 0 {
        return;
    }

    // update per-device contact maps and publish one snapshot per touched device
    let timestamp_ns = now_timestamp_ns();
    for runtime_state in service.runtime_states_snapshot() {
        let mut queues = runtime_state.state.queues.lock();
        let mut touched_devices = HashSet::new();

        for touch in &touches {
            let device_id = monitor_device_id(touch.hSource);
            let has_active_stream = queues
                .active_input_streams
                .get(&device_id)
                .copied()
                .unwrap_or(0)
                > 0;
            if !has_active_stream {
                continue;
            }

            let phase = touch_phase_from_flags(touch.dwFlags);
            let pressure = if phase == InputTouchContactPhase::End {
                0.0
            } else {
                1.0
            };
            let radius_x = if (touch.dwMask & TOUCHINPUTMASKF_CONTACTAREA) != 0 {
                touch.cxContact as f64 / 200.0
            } else {
                0.0
            };
            let radius_y = if (touch.dwMask & TOUCHINPUTMASKF_CONTACTAREA) != 0 {
                touch.cyContact as f64 / 200.0
            } else {
                0.0
            };
            let entry = RawTouchContact {
                contact_id: touch.dwID,
                phase,
                x: touch_coordinate_from_raw(touch.x),
                y: touch_coordinate_from_raw(touch.y),
                pressure,
                radius_x,
                radius_y,
                tilt_x: 0.0,
                tilt_y: 0.0,
            };

            let contacts = queues
                .active_touch_contacts
                .entry(device_id.clone())
                .or_default();
            if phase == InputTouchContactPhase::End {
                contacts.remove(&entry.contact_id);
            } else {
                contacts.insert(entry.contact_id, entry);
            }

            touched_devices.insert(device_id);
        }

        for device_id in touched_devices {
            let Some(contacts) = queues.active_touch_contacts.get(&device_id) else {
                continue;
            };
            let mut snapshot_contacts = contacts.values().cloned().collect::<Vec<_>>();
            snapshot_contacts.sort_by_key(|contact| contact.contact_id);
            let sequence = queues.touch_sequences.entry(device_id.clone()).or_insert(1);
            let packet = RawTouchPacket {
                timestamp_ns,
                device_id: device_id.clone(),
                contacts: snapshot_contacts,
            };
            *sequence = sequence.saturating_add(1);
            push_touch_packet(&runtime_state, &mut queues.touch, packet);
        }

        runtime_state.state.wake.notify_all();
    }
}

/// Handle one raw keyboard or mouse input message payload.
fn handle_raw_input_message(service: &WindowsRawInputService, raw_input_handle: isize) {
    // query payload size and allocate one temporary raw-input buffer
    let mut size = 0u32;
    let header_size = mem::size_of::<RAWINPUTHEADER>() as u32;
    let query_status = unsafe {
        GetRawInputData(
            raw_input_handle,
            RID_INPUT,
            ptr::null_mut(),
            &mut size,
            header_size,
        )
    };
    if query_status == u32::MAX || size == 0 {
        return;
    }

    // read raw-input bytes into the temporary buffer
    let mut buffer = vec![0u8; size as usize];
    let read_status = unsafe {
        GetRawInputData(
            raw_input_handle,
            RID_INPUT,
            buffer.as_mut_ptr().cast(),
            &mut size,
            header_size,
        )
    };
    if read_status == u32::MAX || read_status != size {
        return;
    }
    if (size as usize) < mem::size_of::<RAWINPUT>() {
        return;
    }

    // parse payload and enqueue normalized packets into every subscribed runtime
    let raw = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<RAWINPUT>()) };
    let timestamp = now_timestamp_ns();
    let source_device_id = monitor_device_id(raw.header.hDevice);

    for runtime_state in service.runtime_states_snapshot() {
        let mut queues = runtime_state.state.queues.lock();
        let has_active_stream = queues
            .active_input_streams
            .get(&source_device_id)
            .copied()
            .unwrap_or(0)
            > 0;
        if !has_active_stream {
            continue;
        }

        if raw.header.dwType == RIM_TYPEKEYBOARD {
            let keyboard = unsafe { raw.data.keyboard };
            let key_code = keyboard.VKey as usize;
            if key_code >= queues.key_down.len() {
                continue;
            }

            let is_break = (u32::from(keyboard.Flags) & RI_KEY_BREAK) != 0;
            let is_pressed = !is_break;
            let was_pressed = queues.key_down[key_code];
            queues.key_down[key_code] = is_pressed;
            if !is_side_modifier_key(key_code) {
                update_lock_key_state(&mut queues, key_code, is_pressed, was_pressed);
            }

            let is_repeat = is_pressed && was_pressed;
            let action = if is_repeat {
                InputEventAction::Repeat
            } else if is_pressed {
                InputEventAction::Press
            } else {
                InputEventAction::Release
            };
            let modifiers = control_key_state_from_queues(&queues);

            push_input_packet(
                &runtime_state,
                &mut queues.keyboard,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    device_id: source_device_id.clone(),
                    kind: windows_core::WindowsInputEventKind::Key,
                    action,
                    code: keyboard.VKey as u32,
                    scan_code: keyboard.MakeCode as u32,
                    value: if is_repeat {
                        2
                    } else if is_pressed {
                        1
                    } else {
                        0
                    },
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    buttons: 0,
                    modifiers,
                    repeat: is_repeat,
                },
            );
            runtime_state.state.wake.notify_all();
            continue;
        }

        if raw.header.dwType == RIM_TYPEMOUSE {
            let mouse = unsafe { raw.data.mouse };
            let is_absolute = (mouse.usFlags & RAW_MOUSE_MOVE_ABSOLUTE) != 0;
            let button_flags = unsafe { u32::from(mouse.Anonymous.Anonymous.usButtonFlags) };
            let button_data = unsafe { mouse.Anonymous.Anonymous.usButtonData };
            let modifiers = control_key_state_from_queues(&queues);
            let (pointer_x, pointer_y) = current_pointer_position();
            let motion_buttons = queues.mouse_buttons;
            let mut next_mouse_buttons = queues.mouse_buttons;

            if mouse.lLastX != 0 || mouse.lLastY != 0 || is_absolute {
                let (x, y, code) = normalize_raw_mouse_motion(&mouse);

                push_input_packet(
                    &runtime_state,
                    &mut queues.mouse,
                    RawInputPacket {
                        timestamp_ns: timestamp,
                        device_id: source_device_id.clone(),
                        kind: windows_core::WindowsInputEventKind::PointerMotion,
                        action: InputEventAction::Move,
                        code,
                        scan_code: code,
                        value: if is_absolute { 1 } else { 0 },
                        x,
                        y,
                        wheel_x: 0.0,
                        wheel_y: 0.0,
                        buttons: motion_buttons,
                        modifiers,
                        repeat: false,
                    },
                );
            }

            if button_flags & RI_MOUSE_WHEEL != 0 {
                let delta = i16::from_ne_bytes(button_data.to_ne_bytes()) as f64;
                push_input_packet(
                    &runtime_state,
                    &mut queues.mouse,
                    RawInputPacket {
                        timestamp_ns: timestamp,
                        device_id: source_device_id.clone(),
                        kind: windows_core::WindowsInputEventKind::Scroll,
                        action: InputEventAction::Scroll,
                        code: RI_MOUSE_WHEEL,
                        scan_code: RI_MOUSE_WHEEL,
                        value: delta as i64,
                        x: pointer_x,
                        y: pointer_y,
                        wheel_x: 0.0,
                        wheel_y: delta,
                        buttons: motion_buttons,
                        modifiers,
                        repeat: false,
                    },
                );
            }

            if button_flags & RI_MOUSE_HWHEEL != 0 {
                let delta = i16::from_ne_bytes(button_data.to_ne_bytes()) as f64;
                push_input_packet(
                    &runtime_state,
                    &mut queues.mouse,
                    RawInputPacket {
                        timestamp_ns: timestamp,
                        device_id: source_device_id.clone(),
                        kind: windows_core::WindowsInputEventKind::Scroll,
                        action: InputEventAction::Scroll,
                        code: RI_MOUSE_HWHEEL,
                        scan_code: RI_MOUSE_HWHEEL,
                        value: delta as i64,
                        x: pointer_x,
                        y: pointer_y,
                        wheel_x: delta,
                        wheel_y: 0.0,
                        buttons: motion_buttons,
                        modifiers,
                        repeat: false,
                    },
                );
            }

            push_mouse_button_events(
                &runtime_state,
                &mut queues.mouse,
                &source_device_id,
                &mut next_mouse_buttons,
                button_flags,
                timestamp,
                pointer_x,
                pointer_y,
                modifiers,
            );
            queues.mouse_buttons = next_mouse_buttons;
            runtime_state.state.wake.notify_all();
            continue;
        }

        if raw.header.dwType != RIM_TYPEHID {
            continue;
        }

        let hid = unsafe { raw.data.hid };
        let report_size = hid.dwSizeHid as usize;
        let report_count = hid.dwCount as usize;
        if report_size == 0 || report_count == 0 {
            continue;
        }

        let data_start = unsafe { raw.data.hid.bRawData.as_ptr() as usize };
        let buffer_start = buffer.as_ptr() as usize;
        if data_start < buffer_start {
            continue;
        }

        let data_offset = data_start - buffer_start;
        let total_size = report_size.saturating_mul(report_count);
        if data_offset > buffer.len() || total_size > buffer.len().saturating_sub(data_offset) {
            continue;
        }

        let data = &buffer[data_offset..data_offset + total_size];
        for report in data.chunks(report_size) {
            if report.is_empty() {
                continue;
            }

            let sequence = {
                let next = queues
                    .hid_sequences
                    .entry(source_device_id.clone())
                    .or_insert(1);
                let sequence = *next;
                *next = next.saturating_add(1);
                sequence
            };

            let report_id = report[0];
            push_hid_packet(
                &runtime_state,
                &mut queues.hid,
                RawHidPacket {
                    timestamp_ns: timestamp,
                    sequence,
                    device_id: source_device_id.clone(),
                    report_id,
                    data: report.to_vec(),
                },
            );
        }

        runtime_state.state.wake.notify_all();
    }
}

/// Push mouse button transitions described by one raw flag word.
fn push_mouse_button_events(
    runtime_state: &WindowsRawInputRuntimeState,
    queue: &mut VecDeque<RawInputPacket>,
    device_id: &str,
    buttons_state: &mut u32,
    button_flags: u32,
    timestamp: u64,
    pointer_x: f64,
    pointer_y: f64,
    modifiers: u32,
) {
    // map Win32 button flag pairs to stable runtime button codes
    let button_cases = [
        (RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP, 0u32),
        (RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP, 1u32),
        (RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_3_UP, 2u32),
        (RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP, 3u32),
        (RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP, 4u32),
    ];

    for (down_flag, up_flag, code) in button_cases {
        if button_flags & down_flag != 0 {
            *buttons_state |= 1u32 << code;

            // enqueue button-press packet
            push_input_packet(
                runtime_state,
                queue,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    device_id: device_id.to_string(),
                    kind: windows_core::WindowsInputEventKind::PointerButton,
                    action: InputEventAction::Press,
                    code,
                    scan_code: code,
                    value: 1,
                    x: pointer_x,
                    y: pointer_y,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    buttons: *buttons_state,
                    modifiers,
                    repeat: false,
                },
            );
        }

        if button_flags & up_flag != 0 {
            *buttons_state &= !(1u32 << code);

            // enqueue button-release packet
            push_input_packet(
                runtime_state,
                queue,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    device_id: device_id.to_string(),
                    kind: windows_core::WindowsInputEventKind::PointerButton,
                    action: InputEventAction::Release,
                    code,
                    scan_code: code,
                    value: 0,
                    x: pointer_x,
                    y: pointer_y,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    buttons: *buttons_state,
                    modifiers,
                    repeat: false,
                },
            );
        }
    }
}

/// Pop one queued packet, optionally blocking for the next event.
fn pop_input_event_for_device(
    binding: &BindingCallContext,
    queue_kind: RawQueueKind,
    device_id: &str,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<RawInputPacket> {
    // ensure the singleton raw service is initialized
    let runtime_state = windows_raw_input_runtime_state(binding);
    let service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();

    loop {
        // select one queue and pop the next packet for this specific device
        let queue = match queue_kind {
            RawQueueKind::Keyboard => &mut queues.keyboard,
            RawQueueKind::Mouse => &mut queues.mouse,
        };
        let event = queue
            .iter()
            .position(|event| event.device_id == device_id)
            .and_then(|index| queue.remove(index));
        if let Some(event) = event {
            return Ok(event);
        }

        // return would-block immediately for nonblocking callers
        if nonblocking {
            return Err(io_would_block(operation, "input queue is empty"));
        }

        // otherwise wait until the worker enqueues the next packet or exits
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        runtime_state.state.wake.wait(&mut queues);
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Pop one queued monitor packet, optionally blocking for the next event.
fn pop_monitor_event(
    binding: &BindingCallContext,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<RawMonitorPacket> {
    // ensure the singleton raw service is initialized
    let runtime_state = windows_raw_input_runtime_state(binding);
    let service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();

    loop {
        // pop one monitor packet when available
        if let Some(event) = queues.monitor.pop_front() {
            return Ok(event);
        }

        // return would-block immediately for nonblocking callers
        if nonblocking {
            return Err(io_would_block(operation, "input queue is empty"));
        }

        // otherwise wait until the worker enqueues the next packet or exits
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        runtime_state.state.wake.wait(&mut queues);
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Pop one queued raw-hid packet for one device, optionally blocking.
fn pop_raw_hid_packet_for_device(
    binding: &BindingCallContext,
    device_id: &str,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<RawHidPacket> {
    let runtime_state = windows_raw_input_runtime_state(binding);
    let service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();

    loop {
        let packet = queues
            .hid
            .iter()
            .position(|packet| packet.device_id == device_id)
            .and_then(|index| queues.hid.remove(index));
        if let Some(packet) = packet {
            return Ok(packet);
        }

        if nonblocking {
            return Err(io_would_block(operation, "input queue is empty"));
        }

        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        runtime_state.state.wake.wait(&mut queues);
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Pop one queued raw-hid packet for one device with one bounded timeout.
fn pop_raw_hid_packet_for_device_with_timeout(
    binding: &BindingCallContext,
    device_id: &str,
    timeoutns: u64,
    operation: &'static str,
) -> RuntimeResult<RawHidPacket> {
    if timeoutns == 0 {
        return pop_raw_hid_packet_for_device(binding, device_id, true, operation);
    }

    let runtime_state = windows_raw_input_runtime_state(binding);
    let service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();
    let deadline = Instant::now().checked_add(Duration::from_nanos(timeoutns));

    loop {
        let packet = queues
            .hid
            .iter()
            .position(|packet| packet.device_id == device_id)
            .and_then(|index| queues.hid.remove(index));
        if let Some(packet) = packet {
            return Ok(packet);
        }

        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        if let Some(deadline) = deadline {
            let now = Instant::now();
            let Some(remaining) = deadline.checked_duration_since(now) else {
                return Err(io_would_block(operation, "input queue is empty"));
            };
            if remaining.is_zero() {
                return Err(io_would_block(operation, "input queue is empty"));
            }

            runtime_state.state.wake.wait_for(&mut queues, remaining);
        } else {
            runtime_state.state.wake.wait(&mut queues);
        }

        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Pop one queued touch snapshot for one device, optionally blocking.
fn pop_touch_packet_for_device(
    binding: &BindingCallContext,
    device_id: &str,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<RawTouchPacket> {
    let runtime_state = windows_raw_input_runtime_state(binding);
    let service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();

    loop {
        let packet = queues
            .touch
            .iter()
            .position(|packet| packet.device_id == device_id)
            .and_then(|index| queues.touch.remove(index));
        if let Some(packet) = packet {
            return Ok(packet);
        }

        if nonblocking {
            return Err(io_would_block(operation, "input queue is empty"));
        }

        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        runtime_state.state.wake.wait(&mut queues);
        if !service.is_worker_running() {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Return the singleton raw service or map startup errors.
fn ensure_raw_service(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsRawInputService>> {
    let runtime_state = windows_raw_input_runtime_state(binding);
    configure_raw_queue_limits(&runtime_state, binding);

    let service = binding
        .worker()
        .platform_state
        .input
        .windows_raw_input_service(operation)?;

    ensure_runtime_registration(binding, &service, &runtime_state);
    service.ensure_worker(binding, operation)?;

    Ok(service)
}

/// Register one runtime with the shared raw-input service once.
fn ensure_runtime_registration(
    binding: &BindingCallContext,
    service: &Arc<WindowsRawInputService>,
    runtime_state: &Arc<WindowsRawInputRuntimeState>,
) {
    if runtime_state
        .service_registered
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    register_runtime_finalizer(binding, service, runtime_state);
    service.register_runtime(binding.worker().id, runtime_state);
}

/// Ensure the raw service is initialized for one binding operation.
pub(super) fn ensure_service(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = ensure_raw_service(binding, operation)?;
    Ok(())
}

/// Register one opened per-device stream for queue filtering.
pub(super) fn register_input_stream(
    binding: &BindingCallContext,
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsRawInputRuntimeState>> {
    // ensure service startup before mutating shared stream counts
    let runtime_state = windows_raw_input_runtime_state(binding);
    let _service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();
    let current = queues
        .active_input_streams
        .get(device_id)
        .copied()
        .unwrap_or(0);
    if current > 0 {
        return Err(io_would_block(
            operation,
            "input device stream is already open",
        ));
    }

    queues.active_input_streams.insert(device_id.to_string(), 1);
    drop(queues);

    Ok(runtime_state)
}

/// Release one opened per-device stream for queue filtering.
pub(super) fn release_input_stream(
    runtime_state: &Arc<WindowsRawInputRuntimeState>,
    device_id: &str,
) {
    // decrement and remove per-device stream counters
    let mut queues = runtime_state.state.queues.lock();
    let Some(current) = queues.active_input_streams.get(device_id).copied() else {
        return;
    };

    if current <= 1 {
        queues.active_input_streams.remove(device_id);
        queues.hid_sequences.remove(device_id);
        queues.touch_sequences.remove(device_id);
        queues.active_touch_contacts.remove(device_id);
        queues.hid.retain(|packet| packet.device_id != device_id);
        queues.touch.retain(|packet| packet.device_id != device_id);
    } else {
        queues
            .active_input_streams
            .insert(device_id.to_string(), current - 1);
        return;
    }

    // release any cached gamepad decoder resources for fully closed streams
    remove_raw_gamepad_decoder_entry(runtime_state, device_id);
}

/// Read one event for one opened raw-input device.
pub(super) fn read_device_event(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // dispatch keyboard and mouse streams through dedicated queues
    if device.kind == InputDeviceKind::Keyboard || device.kind == InputDeviceKind::Mouse {
        let queue_kind = if device.kind == InputDeviceKind::Keyboard {
            RawQueueKind::Keyboard
        } else {
            RawQueueKind::Mouse
        };
        let event =
            pop_input_event_for_device(binding, queue_kind, &device.id, nonblocking, operation)?;
        let mut payload = windows_core::empty_event_payload(binding);
        match event.kind {
            windows_core::WindowsInputEventKind::Key => {
                payload.key = InputKeyEventPayload {
                    action: event.action,
                    backend_code: event.code,
                    backend_scan_code: event.scan_code,
                    backend_value: event.value,
                    modifiers: event.modifiers,
                    repeat: event.repeat,
                };
            }
            windows_core::WindowsInputEventKind::PointerMotion => {
                payload.pointer_motion = InputPointerMotionEventPayload {
                    x: event.x,
                    y: event.y,
                    buttons: event.buttons,
                    modifiers: event.modifiers,
                };
            }
            windows_core::WindowsInputEventKind::PointerButton => {
                payload.pointer_button = InputPointerButtonEventPayload {
                    action: event.action,
                    backend_code: event.code,
                    backend_value: event.value,
                    x: event.x,
                    y: event.y,
                    modifiers: event.modifiers,
                };
            }
            windows_core::WindowsInputEventKind::Scroll => {
                payload.scroll = InputScrollEventPayload {
                    wheel_x: event.wheel_x,
                    wheel_y: event.wheel_y,
                    x: event.x,
                    y: event.y,
                    modifiers: event.modifiers,
                };
            }
            windows_core::WindowsInputEventKind::Device => {
                payload.device = InputDeviceEventPayload {
                    action: event.action,
                    backend_code: event.code,
                    backend_value: event.value,
                };
            }
            _ => {}
        }

        return Ok(windows_core::build_input_event(
            binding,
            event.kind,
            event.timestamp_ns,
            0,
            &event.device_id,
            payload,
        ));
    }

    // map touch snapshots into one touch event payload
    if matches!(device.kind, InputDeviceKind::Touch | InputDeviceKind::Pen) {
        let touch = pop_touch_packet_for_device(binding, &device.id, nonblocking, operation)?;
        let mut payload = windows_core::empty_event_payload(binding);
        if let Some(contact) = touch.contacts.first() {
            let action = if contact.phase == InputTouchContactPhase::Begin {
                InputEventAction::Press
            } else if contact.phase == InputTouchContactPhase::End {
                InputEventAction::Release
            } else if contact.phase == InputTouchContactPhase::Cancel {
                InputEventAction::Cancel
            } else {
                InputEventAction::Move
            };
            payload.touch.action = action;
            payload.touch.contact_id = contact.contact_id;
            payload.touch.x = contact.x;
            payload.touch.y = contact.y;
            payload.touch.pressure = contact.pressure;
        } else {
            payload.touch.action = InputEventAction::Cancel;
        }

        return Ok(windows_core::build_input_event(
            binding,
            windows_core::WindowsInputEventKind::Touch,
            touch.timestamp_ns,
            0,
            &touch.device_id,
            payload,
        ));
    }

    // map sensor-capable streams into one sensor event payload
    if let Some(sensor_kind) = sensor_kind_for_device(device) {
        let sample = read_sensor_sample(binding, device, sensor_kind, nonblocking, operation)?;
        let mut payload = windows_core::empty_event_payload(binding);
        payload.sensor.action = InputEventAction::Axis;
        payload.sensor.backend_code = sensor_kind as u32;
        payload.sensor.backend_value = i64::from(sample.flags);
        payload.sensor.x = sample.x;
        payload.sensor.y = sample.y;
        payload.sensor.z = sample.z;
        return Ok(windows_core::build_input_event(
            binding,
            windows_core::WindowsInputEventKind::Sensor,
            sample.timestamp_ns,
            0,
            &device.id,
            payload,
        ));
    }

    // map generic raw-hid packets into one device event payload
    let packet = pop_raw_hid_packet_for_device(binding, &device.id, nonblocking, operation)?;
    let mut payload = windows_core::empty_event_payload(binding);
    payload.device.action = InputEventAction::Move;
    payload.device.backend_code = u32::from(packet.report_id);
    payload.device.backend_value = packet.data.len() as i64;
    Ok(windows_core::build_input_event(
        binding,
        windows_core::WindowsInputEventKind::Device,
        packet.timestamp_ns,
        0,
        &packet.device_id,
        payload,
    ))
}

/// Map one action to one monitor event kind.
/// Build one monitor event payload for raw-input monitor deltas.
fn build_raw_monitor_event(
    binding: &BindingCallContext,
    timestamp_ns: u64,
    sequence: u64,
    device_id: &str,
    device_kind: InputDeviceKind,
    action: InputEventAction,
) -> InputMonitorEvent {
    let connected = !matches!(action, InputEventAction::Disconnect);
    let metadata = InputMonitorEventMetadata {
        timestamp_ns,
        sequence,
        device_id: binding.store_string(device_id),
        device_kind,
        connected,
    };

    match action {
        InputEventAction::Connect => {
            InputMonitorEvent::InputMonitorConnectEvent(InputMonitorConnectEvent {
                kind: binding.store_string("connect"),
                metadata,
            })
        }
        InputEventAction::Disconnect => {
            InputMonitorEvent::InputMonitorDisconnectEvent(InputMonitorDisconnectEvent {
                kind: binding.store_string("disconnect"),
                metadata,
            })
        }
        _ => InputMonitorEvent::InputMonitorChangeEvent(InputMonitorChangeEvent {
            kind: binding.store_string("change"),
            metadata,
        }),
    }
}

/// Read one monitor event from the raw queue.
pub(super) fn read_monitor_event(
    binding: &BindingCallContext,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputMonitorEvent> {
    // pop one monitor packet and map queue state
    let event = pop_monitor_event(binding, nonblocking, operation)?;

    // map monitor packet into one runtime monitor payload
    Ok(build_raw_monitor_event(
        binding,
        event.timestamp_ns,
        0,
        &event.device_id,
        event.device_kind,
        event.action,
    ))
}

/// Resolve one optional sensor lane from one raw-input device descriptor.
pub(super) fn sensor_kinds_for_device(device: &RawInputDeviceDescriptor) -> Vec<InputSensorKind> {
    // preserve stable sensor-kind ordering across hosts and runs
    // canonical sensor ordering for deterministic output
    const SENSOR_KIND_ORDER: [InputSensorKind; 6] = [
        InputSensorKind::Accelerometer,
        InputSensorKind::Gyroscope,
        InputSensorKind::Magnetometer,
        InputSensorKind::Gravity,
        InputSensorKind::LinearAcceleration,
        InputSensorKind::Orientation,
    ];

    // probe supported kinds from hid sensor usage capabilities
    let mut supported = HashSet::new();
    for capability in &device.value_capabilities {
        if capability.usage_page != HID_USAGE_PAGE_SENSOR {
            continue;
        }

        for usage in capability.usage_min..=capability.usage_max {
            let Some(kind) = sensor_kind_from_usage(HID_USAGE_PAGE_SENSOR, usage) else {
                continue;
            };
            supported.insert(kind);
        }
    }

    // fall back to top-level hid usage classification when capability metadata is absent
    if supported.is_empty()
        && let Some(kind) = sensor_kind_from_usage(device.usage_page, device.usage)
    {
        supported.insert(kind);
    }

    // emit kinds in deterministic canonical order
    let mut kinds = Vec::new();
    for kind in SENSOR_KIND_ORDER {
        if supported.contains(&kind) {
            kinds.push(kind);
        }
    }

    kinds
}

/// Resolve one optional default sensor lane from one raw-input device descriptor.
pub(super) fn sensor_kind_for_device(device: &RawInputDeviceDescriptor) -> Option<InputSensorKind> {
    sensor_kinds_for_device(device).into_iter().next()
}

/// Return one backend-derived sensor resolution estimate for one raw-input device.
fn sensor_resolution_for_device(device: &RawInputDeviceDescriptor) -> f64 {
    let mut best_resolution = 0.0;
    for capability in &device.value_capabilities {
        if capability.usage_page != HID_USAGE_PAGE_SENSOR {
            continue;
        }

        let span = hid_logical_span(capability);
        if span <= 0.0 {
            continue;
        }

        let resolution = 1.0 / span;
        if best_resolution == 0.0 || resolution < best_resolution {
            best_resolution = resolution;
        }
    }

    best_resolution
}

/// Build sensor-info payload rows from one raw-input descriptor.
pub(super) fn sensor_infos_for_device(
    device: &RawInputDeviceDescriptor,
) -> Vec<InputSensorDescriptor> {
    let kinds = sensor_kinds_for_device(device);
    if kinds.is_empty() {
        return Vec::new();
    }

    let resolution = sensor_resolution_for_device(device);
    let mut infos = Vec::with_capacity(kinds.len());
    for kind in kinds {
        infos.push(InputSensorDescriptor {
            kind,
            min_sample_rate_hz: 0.0,
            max_sample_rate_hz: 0.0,
            resolution,
            supports_wake: device.feature_report_size > 0,
        });
    }

    infos
}

/// Normalize one Win32 raw-input device path into one CreateFileW path.
fn normalize_hid_device_path(path: &str) -> RuntimeResult<Vec<u16>> {
    if path.starts_with("\\\\?\\") {
        return core_platform::wide_from_str("path", path);
    }

    if let Some(stripped) = path.strip_prefix("\\??\\") {
        let normalized = format!("\\\\?\\{stripped}");
        return core_platform::wide_from_str("path", &normalized);
    }

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.rawhid.open")).boxed())
}

/// Open one HID device file handle for report operations.
fn open_hid_device_handle(
    device: &RawInputDeviceDescriptor,
    access: u32,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let Some(path) = device.raw_path.as_deref() else {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    };
    let wide = normalize_hid_device_path(path)?;

    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if handle == 0 || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        let code = core_platform::last_error_code() as u32;
        if code == ERROR_FILE_NOT_FOUND {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                Some(code as i32),
                Some(operation.to_string()),
                None,
                "hid device not found".to_string(),
            ))
            .boxed());
        }
        if code == ERROR_ACCESS_DENIED {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoPermissionDenied),
                None,
                Some(code as i32),
                Some(operation.to_string()),
                None,
                "hid device open denied".to_string(),
            ))
            .boxed());
        }
        return Err(core_platform::io_error_with_code(
            "CreateFileW",
            code as i32,
        ));
    }

    Ok(handle)
}

/// One raw gamepad usage-value sample decoded from hid parser calls.
#[derive(Debug, Clone, Copy)]
struct RawGamepadUsageSample {
    /// Raw usage value from hid parser state.
    raw: u32,
    /// Logical minimum for this usage.
    logical_min: i32,
    /// Logical maximum for this usage.
    logical_max: i32,
}

/// One decoded gamepad state snapshot from one hid report payload.
#[derive(Debug, Clone)]
struct DecodedRawGamepadState {
    /// Standardized left and right stick axis values.
    axes: [f64; STANDARD_GAMEPAD_AXIS_COUNT],
    /// Standardized gamepad button values.
    buttons: [InputGamepadButtonState; STANDARD_GAMEPAD_BUTTON_COUNT],
    /// Mapping classification for this payload shape.
    mapping: InputGamepadMappingType,
}

/// Return one neutral gamepad state snapshot for unsupported or empty payloads.
fn neutral_gamepad_state() -> DecodedRawGamepadState {
    DecodedRawGamepadState {
        axes: [0.0; STANDARD_GAMEPAD_AXIS_COUNT],
        buttons: [InputGamepadButtonState {
            pressed: false,
            touched: false,
            value: 0.0,
        }; STANDARD_GAMEPAD_BUTTON_COUNT],
        mapping: InputGamepadMappingType::None,
    }
}

/// Return one latest queued raw-hid packet for one device without draining queue state.
fn latest_raw_hid_packet_for_device(
    binding: &BindingCallContext,
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<Option<RawHidPacket>> {
    let runtime_state = windows_raw_input_runtime_state(binding);
    let _service = ensure_raw_service(binding, operation)?;
    let queues = runtime_state.state.queues.lock();
    Ok(queues
        .hid
        .iter()
        .rev()
        .find(|packet| packet.device_id == device_id)
        .cloned())
}

/// Read one current hid input report through one already-open hid handle.
fn read_input_report_with_handle(
    handle: HANDLE,
    report_size: u16,
    device_id: &str,
    report_id_candidates: &[u8],
) -> Option<RawHidPacket> {
    let report_bytes = usize::from(report_size.max(1)).max(64);
    if report_bytes == 0 {
        return None;
    }

    let mut report_ids = if report_id_candidates.is_empty() {
        vec![0]
    } else {
        report_id_candidates.to_vec()
    };
    if !report_ids.contains(&0) {
        report_ids.push(0);
    }

    for report_id in report_ids {
        let mut report = vec![0u8; report_bytes];
        report[0] = report_id;
        let status =
            unsafe { HidD_GetInputReport(handle, report.as_mut_ptr().cast(), report.len() as u32) };
        if status == 0 {
            continue;
        }

        let decoded_report_id = if report[0] != 0 { report[0] } else { report_id };
        return Some(RawHidPacket {
            timestamp_ns: now_timestamp_ns(),
            sequence: 0,
            device_id: device_id.to_string(),
            report_id: decoded_report_id,
            data: report,
        });
    }

    None
}

/// Normalize one hid value into one centered [-1, 1] range.
fn normalize_hid_centered_value(value: u32, logical_min: i32, logical_max: i32) -> f64 {
    if logical_max <= logical_min {
        return 0.0;
    }

    let minimum = logical_min as f64;
    let maximum = logical_max as f64;
    let sample = if logical_min < 0 {
        (value as i32) as f64
    } else {
        value as f64
    };
    let sample = sample.clamp(minimum, maximum);
    let unit = (sample - minimum) / (maximum - minimum);
    (unit * 2.0 - 1.0).clamp(-1.0, 1.0)
}

/// Normalize one hid value into one unsigned [0, 1] range.
fn normalize_hid_unsigned_value(value: u32, logical_min: i32, logical_max: i32) -> f64 {
    if logical_max <= logical_min {
        return 0.0;
    }

    let minimum = logical_min as f64;
    let maximum = logical_max as f64;
    let sample = if logical_min < 0 {
        (value as i32) as f64
    } else {
        value as f64
    };
    let sample = sample.clamp(minimum, maximum);
    ((sample - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
}

/// Map one hid button usage to one standard gamepad button index.
fn standard_button_index_for_hid_usage(usage: u16) -> Option<usize> {
    match usage {
        1 => Some(0),
        2 => Some(1),
        3 => Some(2),
        4 => Some(3),
        5 => Some(4),
        6 => Some(5),
        7 => Some(6),
        8 => Some(7),
        9 => Some(8),
        10 => Some(9),
        11 => Some(10),
        12 => Some(11),
        13 => Some(12),
        14 => Some(13),
        15 => Some(14),
        16 => Some(15),
        17 => Some(16),
        _ => None,
    }
}

/// Apply one hat-switch sample to standard gamepad dpad button bits.
fn apply_hat_switch_to_dpad(pressed: &mut [bool; STANDARD_GAMEPAD_BUTTON_COUNT], hat_value: u32) {
    pressed[12] = false;
    pressed[13] = false;
    pressed[14] = false;
    pressed[15] = false;

    match hat_value {
        0 => {
            pressed[12] = true;
        }
        1 => {
            pressed[12] = true;
            pressed[15] = true;
        }
        2 => {
            pressed[15] = true;
        }
        3 => {
            pressed[13] = true;
            pressed[15] = true;
        }
        4 => {
            pressed[13] = true;
        }
        5 => {
            pressed[13] = true;
            pressed[14] = true;
        }
        6 => {
            pressed[14] = true;
        }
        7 => {
            pressed[12] = true;
            pressed[14] = true;
        }
        _ => {}
    }
}

/// Decode one gamepad payload from one hid packet with one parser context.
fn decode_raw_gamepad_packet(
    device: &RawInputDeviceDescriptor,
    preparsed: PHIDP_PREPARSED_DATA,
    packet: &RawHidPacket,
    mapping: InputGamepadMappingType,
) -> DecodedRawGamepadState {
    let mut pressed_buttons = [false; STANDARD_GAMEPAD_BUTTON_COUNT];

    // collect all active button usages from hid button pages
    let mut link_collections = HashSet::new();
    for capability in &device.button_capabilities {
        if capability.usage_page != HID_USAGE_PAGE_BUTTON {
            continue;
        }

        if capability.report_id != 0
            && packet.report_id != 0
            && capability.report_id != packet.report_id
        {
            continue;
        }

        link_collections.insert(capability.link_collection);
    }

    for link_collection in link_collections {
        let max_length =
            unsafe { HidP_MaxUsageListLength(HidP_Input, HID_USAGE_PAGE_BUTTON, preparsed) };
        if max_length == 0 {
            continue;
        }

        let mut usages = vec![0u16; max_length as usize];
        let mut usage_length = max_length;
        let status = unsafe {
            HidP_GetUsages(
                HidP_Input,
                HID_USAGE_PAGE_BUTTON,
                link_collection,
                usages.as_mut_ptr(),
                &mut usage_length,
                preparsed,
                packet.data.as_ptr().cast_mut().cast(),
                packet.data.len() as u32,
            )
        };
        if !hid_status_is_success(status) {
            continue;
        }

        for usage in usages.into_iter().take(usage_length as usize) {
            let Some(index) = standard_button_index_for_hid_usage(usage) else {
                continue;
            };
            pressed_buttons[index] = true;
        }
    }

    // read generic-desktop value usages for sticks, hat-switch, and trigger lanes
    let mut samples = HashMap::<u16, RawGamepadUsageSample>::new();
    for capability in &device.value_capabilities {
        if capability.usage_page != HID_USAGE_PAGE_GENERIC {
            continue;
        }

        if capability.report_id != 0
            && packet.report_id != 0
            && capability.report_id != packet.report_id
        {
            continue;
        }

        for usage in capability.usage_min..=capability.usage_max {
            if !is_gamepad_axis_usage(usage)
                && usage != HID_USAGE_GENERIC_DPAD_UP
                && usage != HID_USAGE_GENERIC_DPAD_DOWN
                && usage != HID_USAGE_GENERIC_DPAD_LEFT
                && usage != HID_USAGE_GENERIC_DPAD_RIGHT
                && usage != HID_USAGE_GENERIC_SYSTEM_MAIN_MENU
            {
                continue;
            }

            let mut raw_value = 0u32;
            let status = unsafe {
                HidP_GetUsageValue(
                    HidP_Input,
                    capability.usage_page,
                    capability.link_collection,
                    usage,
                    &mut raw_value,
                    preparsed,
                    packet.data.as_ptr().cast(),
                    packet.data.len() as u32,
                )
            };
            if !hid_status_is_success(status) {
                continue;
            }

            samples.insert(
                usage,
                RawGamepadUsageSample {
                    raw: raw_value,
                    logical_min: capability.logical_min,
                    logical_max: capability.logical_max,
                },
            );
        }
    }

    // map generic-desktop digital dpad usages onto standard dpad indices
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_DPAD_UP) {
        pressed_buttons[12] = sample.raw != 0;
    }
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_DPAD_DOWN) {
        pressed_buttons[13] = sample.raw != 0;
    }
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_DPAD_LEFT) {
        pressed_buttons[14] = sample.raw != 0;
    }
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_DPAD_RIGHT) {
        pressed_buttons[15] = sample.raw != 0;
    }
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_SYSTEM_MAIN_MENU) {
        pressed_buttons[16] = sample.raw != 0;
    }
    if let Some(sample) = samples.get(&HID_USAGE_GENERIC_HAT_SWITCH) {
        apply_hat_switch_to_dpad(&mut pressed_buttons, sample.raw);
    }

    // map left-stick and right-stick samples into standardized axis slots
    let left_x = samples
        .get(&HID_USAGE_GENERIC_X)
        .map(|sample| {
            normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .unwrap_or(0.0);
    let left_y = samples
        .get(&HID_USAGE_GENERIC_Y)
        .map(|sample| {
            normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .unwrap_or(0.0);
    let right_x = samples
        .get(&HID_USAGE_GENERIC_RX)
        .map(|sample| {
            normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .or_else(|| {
            samples.get(&HID_USAGE_GENERIC_Z).map(|sample| {
                normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
            })
        })
        .unwrap_or(0.0);
    let right_y = samples
        .get(&HID_USAGE_GENERIC_RY)
        .map(|sample| {
            normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .or_else(|| {
            samples.get(&HID_USAGE_GENERIC_RZ).map(|sample| {
                normalize_hid_centered_value(sample.raw, sample.logical_min, sample.logical_max)
            })
        })
        .unwrap_or(0.0);

    // map trigger lanes from z and rz usages when available
    let left_trigger = samples
        .get(&HID_USAGE_GENERIC_Z)
        .map(|sample| {
            normalize_hid_unsigned_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .or_else(|| {
            samples.get(&HID_USAGE_GENERIC_SLIDER).map(|sample| {
                normalize_hid_unsigned_value(sample.raw, sample.logical_min, sample.logical_max)
            })
        })
        .unwrap_or(0.0);
    let right_trigger = samples
        .get(&HID_USAGE_GENERIC_RZ)
        .map(|sample| {
            normalize_hid_unsigned_value(sample.raw, sample.logical_min, sample.logical_max)
        })
        .or_else(|| {
            samples.get(&HID_USAGE_GENERIC_DIAL).map(|sample| {
                normalize_hid_unsigned_value(sample.raw, sample.logical_min, sample.logical_max)
            })
        })
        .unwrap_or(0.0);
    pressed_buttons[6] = pressed_buttons[6] || left_trigger > 0.5;
    pressed_buttons[7] = pressed_buttons[7] || right_trigger > 0.5;

    // project normalized buttons into runtime snapshot payload order
    let mut buttons = [InputGamepadButtonState {
        pressed: false,
        touched: false,
        value: 0.0,
    }; STANDARD_GAMEPAD_BUTTON_COUNT];
    for (index, button) in buttons.iter_mut().enumerate() {
        let value = if index == 6 {
            left_trigger
        } else if index == 7 {
            right_trigger
        } else if pressed_buttons[index] {
            1.0
        } else {
            0.0
        };
        *button = InputGamepadButtonState {
            pressed: pressed_buttons[index],
            touched: pressed_buttons[index],
            value,
        };
    }

    DecodedRawGamepadState {
        axes: [left_x, left_y, right_x, right_y],
        buttons,
        mapping,
    }
}

/// Decode one gamepad snapshot using one cached parser and report-decoder entry.
fn decode_cached_raw_gamepad_state(
    runtime_state: &WindowsRawInputRuntimeState,
    device: &RawInputDeviceDescriptor,
    latest_packet: Option<RawHidPacket>,
    operation: &'static str,
) -> RuntimeResult<(u64, DecodedRawGamepadState)> {
    let mut cache = raw_gamepad_decoder_cache(runtime_state).lock();
    if !cache.contains_key(&device.id) {
        let entry = open_raw_gamepad_decoder_entry(device, operation)?;
        cache.insert(device.id.clone(), entry);
    }

    let Some(entry) = cache.get(&device.id) else {
        return Err(service_error(
            operation,
            "missing cached raw gamepad decoder state",
        ));
    };

    let packet = if let Some(packet) = latest_packet {
        Some(packet)
    } else {
        read_input_report_with_handle(
            entry.handle,
            entry.report_size,
            &device.id,
            &entry.report_ids,
        )
    };
    let Some(packet) = packet else {
        return Ok((
            now_timestamp_ns(),
            DecodedRawGamepadState {
                mapping: entry.mapping,
                ..neutral_gamepad_state()
            },
        ));
    };

    let decoded = decode_raw_gamepad_packet(device, entry.preparsed, &packet, entry.mapping);
    Ok((packet.timestamp_ns, decoded))
}

/// Read one gamepad state snapshot for one raw-input gamepad descriptor.
pub(super) fn gamepad_state_for_raw_input_device(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // reject non-gamepad descriptors for gamepad state operations
    if device.kind != InputDeviceKind::Gamepad {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // capture the latest queued hid packet before probing direct report state
    let runtime_state = windows_raw_input_runtime_state(binding);
    let latest_packet = latest_raw_hid_packet_for_device(binding, &device.id, operation)?;

    // decode one packet using the persistent decoder cache
    let (timestamp_ns, decoded) =
        decode_cached_raw_gamepad_state(&runtime_state, device, latest_packet, operation)?;

    let battery = if device.supports_battery {
        InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::Unknown,
            level: 0.0,
        }
    } else {
        InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::NotPresent,
            level: 0.0,
        }
    };

    Ok(InputGamepadState {
        timestamp_ns,
        connected: true,
        mapping: decoded.mapping,
        connection_type: InputGamepadConnectionType::Unknown,
        player_index: u8::MAX,
        battery,
        supports_rumble: device.supports_rumble,
        supports_trigger_rumble: false,
        axes: binding.store_array_copy(&decoded.axes),
        buttons: binding.store_array_copy(&decoded.buttons),
        touches: binding.store_array(Vec::new()),
    })
}

/// Build one sony dualshock 4 usb light-control report payload.
fn build_sony_dualshock4_light_report(red: u8, green: u8, blue: u8) -> Vec<u8> {
    let mut report = vec![0u8; usize::from(SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES) - 1];
    report[0] = 0x07;
    report[5] = red;
    report[6] = green;
    report[7] = blue;
    report
}

/// Build one sony dualsense usb light-control report payload.
fn build_sony_dualsense_light_report(red: u8, green: u8, blue: u8) -> Vec<u8> {
    let mut report = vec![0u8; usize::from(SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES) - 1];
    report[1] = 0x04;
    report[44] = red;
    report[45] = green;
    report[46] = blue;
    report
}

/// Set one gamepad light color for one raw-input gamepad descriptor.
pub(super) fn set_gamepad_light_for_raw_input_device(
    device: &RawInputDeviceDescriptor,
    red: u8,
    green: u8,
    blue: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject unsupported descriptors before issuing host output reports
    if device.kind != InputDeviceKind::Gamepad || !device.supports_light {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    if device.vendor_id != SONY_VENDOR_ID {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // route sony usb report formats by negotiated output-report length
    if device.output_report_size == SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES {
        let report = build_sony_dualshock4_light_report(red, green, blue);
        let _ = write_output_report(
            device,
            SONY_DUALSHOCK4_USB_EFFECTS_REPORT_ID,
            &report,
            operation,
        )?;
        return Ok(());
    }

    if device.output_report_size == SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES {
        let report = build_sony_dualsense_light_report(red, green, blue);
        let _ = write_output_report(
            device,
            SONY_DUALSENSE_USB_EFFECTS_REPORT_ID,
            &report,
            operation,
        )?;
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Read one raw-hid report for one opened device descriptor.
pub(super) fn read_raw_hid_report_with_timeout(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    maxbytes: u32,
    timeoutns: u64,
    operation: &'static str,
) -> RuntimeResult<InputRawHidReport> {
    let packet =
        pop_raw_hid_packet_for_device_with_timeout(binding, &device.id, timeoutns, operation)?;

    let payload = if packet.report_id != 0 && packet.data.len() > 1 {
        packet.data[1..].to_vec()
    } else {
        packet.data.clone()
    };
    let limit = usize::min(payload.len(), maxbytes as usize);
    let payload = payload[..limit].to_vec();

    Ok(InputRawHidReport {
        timestamp_ns: packet.timestamp_ns,
        sequence: packet.sequence,
        report_id: packet.report_id,
        data: binding.store_slice(payload),
    })
}

/// Read one raw-hid report for one opened device descriptor.
pub(super) fn read_raw_hid_report(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    maxbytes: u32,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputRawHidReport> {
    let packet = pop_raw_hid_packet_for_device(binding, &device.id, nonblocking, operation)?;

    let payload = if packet.report_id != 0 && packet.data.len() > 1 {
        packet.data[1..].to_vec()
    } else {
        packet.data.clone()
    };
    let limit = usize::min(payload.len(), maxbytes as usize);
    let payload = payload[..limit].to_vec();

    Ok(InputRawHidReport {
        timestamp_ns: packet.timestamp_ns,
        sequence: packet.sequence,
        report_id: packet.report_id,
        data: binding.store_slice(payload),
    })
}

/// Read one current touch snapshot from active contact state.
pub(super) fn read_touch_state_snapshot(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    operation: &'static str,
) -> RuntimeResult<InputTouchState> {
    // resolve the raw service and inspect current active contacts
    let runtime_state = windows_raw_input_runtime_state(binding);
    let _service = ensure_raw_service(binding, operation)?;
    let mut queues = runtime_state.state.queues.lock();

    // snapshot active contacts for this device in stable contact-id order
    let contacts = queues
        .active_touch_contacts
        .get(&device.id)
        .cloned()
        .unwrap_or_default();
    let mut contacts = contacts.values().cloned().collect::<Vec<_>>();
    contacts.sort_by_key(|contact| contact.contact_id);

    // allocate one sequence number for this snapshot read
    let next_sequence = queues.touch_sequences.entry(device.id.clone()).or_insert(1);
    let sequence = *next_sequence;
    *next_sequence = next_sequence.saturating_add(1);

    // project contact payloads into runtime touch-state rows
    let mut projected_contacts = Vec::with_capacity(contacts.len());
    for contact in contacts {
        projected_contacts.push(InputTouchContactState {
            contact_id: contact.contact_id,
            phase: contact.phase,
            x: contact.x,
            y: contact.y,
            pressure: contact.pressure,
            radius_x: contact.radius_x,
            radius_y: contact.radius_y,
            tilt_x: contact.tilt_x,
            tilt_y: contact.tilt_y,
        });
    }

    Ok(InputTouchState {
        timestamp_ns: now_timestamp_ns(),
        sequence,
        device_id: binding.store_string(&device.id),
        contacts: binding.store_array(projected_contacts),
    })
}

/// Read one current pen snapshot from active touch-contact state.
pub(super) fn read_pen_state(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    operation: &'static str,
) -> RuntimeResult<Option<RawPenStateSnapshot>> {
    // reject non-pen descriptors for pen-state reads
    if device.kind != InputDeviceKind::Pen {
        return Ok(None);
    }

    // resolve the raw service and inspect current active contacts
    let runtime_state = windows_raw_input_runtime_state(binding);
    let _service = ensure_raw_service(binding, operation)?;
    let queues = runtime_state.state.queues.lock();
    let Some(contacts) = queues.active_touch_contacts.get(&device.id) else {
        return Ok(Some(RawPenStateSnapshot {
            x: 0.0,
            y: 0.0,
            pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            in_contact: false,
            in_range: false,
        }));
    };

    // select one stable primary contact for pointer-state projection
    let contact = contacts.values().min_by_key(|contact| contact.contact_id);
    let Some(contact) = contact else {
        return Ok(Some(RawPenStateSnapshot {
            x: 0.0,
            y: 0.0,
            pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            in_contact: false,
            in_range: false,
        }));
    };

    Ok(Some(RawPenStateSnapshot {
        x: contact.x,
        y: contact.y,
        pressure: contact.pressure,
        tilt_x: contact.tilt_x,
        tilt_y: contact.tilt_y,
        in_contact: true,
        in_range: true,
    }))
}

/// Decode one sensor sample from one raw-hid packet payload.
fn decode_sensor_sample_from_packet(
    packet: RawHidPacket,
    sensor_kind: InputSensorKind,
    operation: &'static str,
) -> RuntimeResult<InputSensorSample> {
    let payload = if packet.report_id != 0 && packet.data.len() > 1 {
        &packet.data[1..]
    } else {
        packet.data.as_slice()
    };

    if payload.len() >= 16 {
        let x = f32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as f64;
        let y = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]) as f64;
        let z = f32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]) as f64;
        let w = f32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]) as f64;
        return Ok(InputSensorSample {
            kind: sensor_kind,
            timestamp_ns: packet.timestamp_ns,
            x,
            y,
            z,
            w,
            flags: payload.len() as u32,
        });
    }

    if payload.len() >= 6 {
        let x = i16::from_le_bytes([payload[0], payload[1]]) as f64;
        let y = i16::from_le_bytes([payload[2], payload[3]]) as f64;
        let z = i16::from_le_bytes([payload[4], payload[5]]) as f64;
        return Ok(InputSensorSample {
            kind: sensor_kind,
            timestamp_ns: packet.timestamp_ns,
            x,
            y,
            z,
            w: 0.0,
            flags: payload.len() as u32,
        });
    }

    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        "sensor packet payload is too short".to_string(),
    ))
    .boxed())
}

/// Read one sensor sample from one sensor-capable raw-hid stream.
pub(super) fn read_sensor_sample(
    binding: &BindingCallContext,
    device: &RawInputDeviceDescriptor,
    sensor_kind: InputSensorKind,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputSensorSample> {
    let packet = pop_raw_hid_packet_for_device(binding, &device.id, nonblocking, operation)?;
    decode_sensor_sample_from_packet(packet, sensor_kind, operation)
}

/// Read one hid feature report from one raw-input device.
pub(super) fn get_feature_report(
    device: &RawInputDeviceDescriptor,
    report_id: u8,
    maxbytes: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let handle = open_hid_device_handle(device, FILE_GENERIC_READ | FILE_GENERIC_WRITE, operation)?;
    let mut report = vec![0u8; maxbytes as usize + 1];
    report[0] = report_id;

    let status =
        unsafe { HidD_GetFeature(handle, report.as_mut_ptr().cast(), report.len() as u32) };
    unsafe {
        CloseHandle(handle);
    }
    if status == 0 {
        return Err(core_platform::io_error("HidD_GetFeature"));
    }

    if report_id != 0 {
        return Ok(report[1..].to_vec());
    }
    Ok(report)
}

/// Write one hid feature report to one raw-input device.
pub(super) fn set_feature_report(
    device: &RawInputDeviceDescriptor,
    report_id: u8,
    data: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let handle = open_hid_device_handle(device, FILE_GENERIC_READ | FILE_GENERIC_WRITE, operation)?;
    let mut report = Vec::with_capacity(data.len() + 1);
    report.push(report_id);
    report.extend_from_slice(data);

    let status = unsafe { HidD_SetFeature(handle, report.as_ptr().cast(), report.len() as u32) };
    unsafe {
        CloseHandle(handle);
    }
    if status == 0 {
        return Err(core_platform::io_error("HidD_SetFeature"));
    }

    Ok(())
}

/// Write one hid output report to one raw-input device.
pub(super) fn write_output_report(
    device: &RawInputDeviceDescriptor,
    report_id: u8,
    data: &[u8],
    operation: &'static str,
) -> RuntimeResult<u32> {
    let handle = open_hid_device_handle(device, FILE_GENERIC_WRITE | FILE_GENERIC_READ, operation)?;
    let mut report = Vec::with_capacity(data.len() + 1);
    report.push(report_id);
    report.extend_from_slice(data);

    let status =
        unsafe { HidD_SetOutputReport(handle, report.as_ptr().cast(), report.len() as u32) };
    unsafe {
        CloseHandle(handle);
    }
    if status == 0 {
        return Err(core_platform::io_error("HidD_SetOutputReport"));
    }

    Ok(data.len() as u32)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use super::{
        GIDC_REMOVAL, RAW_MOUSE_MOVE_ABSOLUTE, RI_MOUSE_BUTTON_1_DOWN, VK_CAPITAL, VK_LSHIFT,
        VK_LWIN, VK_RCONTROL, VK_RMENU,
    };
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::input::host::windows::raw::{
        CAPSLOCK_ON, ENHANCED_KEY, HID_USAGE_GENERIC_GAMEPAD, HID_USAGE_GENERIC_RX,
        HID_USAGE_GENERIC_RY, HID_USAGE_GENERIC_X, HID_USAGE_GENERIC_Y, HID_USAGE_PAGE_BUTTON,
        HID_USAGE_PAGE_GENERIC, HID_USAGE_PAGE_SENSOR, HID_USAGE_SENSOR_ACCELEROMETER_3D,
        HID_USAGE_SENSOR_GYROMETER_3D, NUMLOCK_ON, RAW_INPUT_OVERFLOW_CODE, RAW_INPUT_QUEUE_LIMIT,
        RAW_MONITOR_OVERFLOW_CODE, RAW_MONITOR_QUEUE_LIMIT, RIGHT_ALT_PRESSED, RIGHT_CTRL_PRESSED,
        RawHidButtonCapability, RawHidPacket, RawHidValueCapability, RawInputDeviceDescriptor,
        RawInputPacket, RawInputQueues, RawMonitorPacket, SCROLLLOCK_ON, SHIFT_PRESSED,
        SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES, SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES,
        STANDARD_GAMEPAD_BUTTON_COUNT, WINDOWS_INPUT_MONITOR_ID_PREFIX,
        WindowsRawInputRuntimeState, apply_hat_switch_to_dpad, build_sony_dualsense_light_report,
        build_sony_dualshock4_light_report, classify_gamepad_mapping_from_capabilities,
        control_key_state_from_queues, decode_sensor_sample_from_packet,
        gamepad_report_id_candidates, monitor_device_id, normalize_raw_mouse_motion,
        push_input_packet, push_monitor_packet, push_mouse_button_events,
        resolve_monitor_device_id, sensor_kind_from_usage, sensor_kinds_for_device,
        standard_button_index_for_hid_usage, update_lock_key_state,
    };
    use crate::platform::input::{
        InputDeviceKind, InputEventAction, InputGamepadMappingType, InputSensorKind,
    };

    /// Drop the oldest keyboard packet when the queue reaches its bounded capacity.
    #[test]
    fn test_push_input_packet_drops_oldest_when_full() {
        let runtime_state = WindowsRawInputRuntimeState::default();
        let mut queue = VecDeque::new();
        for index in 0..=RAW_INPUT_QUEUE_LIMIT {
            push_input_packet(
                &runtime_state,
                &mut queue,
                RawInputPacket {
                    timestamp_ns: index as u64,
                    device_id: "raw:device:test".to_string(),
                    kind: windows_core::WindowsInputEventKind::Key,
                    action: InputEventAction::Press,
                    code: index as u32,
                    scan_code: index as u32,
                    value: 1,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    buttons: 0,
                    modifiers: 0,
                    repeat: false,
                },
            );
        }

        assert_eq!(queue.len(), RAW_INPUT_QUEUE_LIMIT);
        let overflow_event = queue.iter().find(|event| {
            event.kind == windows_core::WindowsInputEventKind::Device
                && event.action == InputEventAction::Cancel
                && event.code == RAW_INPUT_OVERFLOW_CODE
        });
        assert!(
            overflow_event.is_some(),
            "queue should contain one overflow marker when drops occur"
        );
    }

    /// Drop the oldest monitor packet when the monitor queue reaches capacity.
    #[test]
    fn test_push_monitor_packet_drops_oldest_when_full() {
        let runtime_state = WindowsRawInputRuntimeState::default();
        let mut queue = VecDeque::new();
        for index in 0..=RAW_MONITOR_QUEUE_LIMIT {
            push_monitor_packet(
                &runtime_state,
                &mut queue,
                RawMonitorPacket {
                    timestamp_ns: index as u64,
                    action: InputEventAction::Connect,
                    device_id: format!("device:{index}"),
                    device_kind: InputDeviceKind::Raw,
                    code: index as u32,
                    value: 1,
                },
            );
        }

        assert_eq!(queue.len(), RAW_MONITOR_QUEUE_LIMIT);
        let overflow_event = queue.iter().find(|event| {
            event.action == InputEventAction::Cancel && event.code == RAW_MONITOR_OVERFLOW_CODE
        });
        assert!(
            overflow_event.is_some(),
            "queue should contain one monitor overflow marker when drops occur"
        );
    }

    /// Return prefixed monitor identifiers for raw-device handle probes.
    #[test]
    fn test_monitor_device_id_falls_back_when_name_query_fails() {
        let id = monitor_device_id(0);
        assert!(
            id.starts_with(WINDOWS_INPUT_MONITOR_ID_PREFIX),
            "monitor ids should include the monitor prefix"
        );
    }

    /// Reuse cached monitor identifiers for removal events.
    #[test]
    fn test_resolve_monitor_device_id_uses_cached_identifier_on_removal() {
        let mut cache = HashMap::new();
        cache.insert(7, "raw:device:stable7".to_string());

        let id = resolve_monitor_device_id(GIDC_REMOVAL, 7, &mut cache);
        assert_eq!(id, "raw:device:stable7");
        assert!(!cache.contains_key(&7));
    }

    /// Normalize absolute raw mouse coordinates into unit-space payload.
    #[test]
    fn test_normalize_raw_mouse_motion_normalizes_absolute_coordinates() {
        let mut mouse: windows_sys::Win32::UI::Input::RAWMOUSE = unsafe { std::mem::zeroed() };
        mouse.usFlags = RAW_MOUSE_MOVE_ABSOLUTE;
        mouse.lLastX = 32768;
        mouse.lLastY = 65535;

        let (x, y, code) = normalize_raw_mouse_motion(&mouse);
        assert!(x > 0.49 && x < 0.51);
        assert_eq!(y, 1.0);
        assert_eq!(code, u32::from(RAW_MOUSE_MOVE_ABSOLUTE));
    }

    /// Keep pointer-button packets aligned with the sampled pointer position.
    #[test]
    fn test_push_mouse_button_events_preserves_pointer_coordinates() {
        let runtime_state = WindowsRawInputRuntimeState::default();
        let mut queue = VecDeque::new();
        let mut buttons = 0u32;
        push_mouse_button_events(
            &runtime_state,
            &mut queue,
            "raw:device:test",
            &mut buttons,
            RI_MOUSE_BUTTON_1_DOWN,
            7u64,
            123.0,
            456.0,
            0,
        );

        assert_eq!(queue.len(), 1, "one packet should be queued");
        let packet = queue.pop_front().expect("one packet should exist");
        assert_eq!(
            packet.kind,
            windows_core::WindowsInputEventKind::PointerButton
        );
        assert_eq!(packet.x, 123.0);
        assert_eq!(packet.y, 456.0);
    }

    /// Build control-key-state bits from side-specific modifier keys and lock toggles.
    #[test]
    fn test_control_key_state_from_queues_sets_expected_bits() {
        let mut queues = RawInputQueues::new();
        queues.key_down[VK_LSHIFT] = true;
        queues.key_down[VK_RCONTROL] = true;
        queues.key_down[VK_RMENU] = true;
        queues.key_down[VK_LWIN] = true;
        queues.caps_lock_on = true;
        queues.num_lock_on = true;
        queues.scroll_lock_on = true;

        let modifiers = control_key_state_from_queues(&queues);
        assert_eq!(modifiers & SHIFT_PRESSED, SHIFT_PRESSED);
        assert_eq!(modifiers & RIGHT_CTRL_PRESSED, RIGHT_CTRL_PRESSED);
        assert_eq!(modifiers & RIGHT_ALT_PRESSED, RIGHT_ALT_PRESSED);
        assert_eq!(modifiers & CAPSLOCK_ON, CAPSLOCK_ON);
        assert_eq!(modifiers & NUMLOCK_ON, NUMLOCK_ON);
        assert_eq!(modifiers & SCROLLLOCK_ON, SCROLLLOCK_ON);
        assert_eq!(modifiers & ENHANCED_KEY, ENHANCED_KEY);
    }

    /// Toggle lock keys only on leading press edges.
    #[test]
    fn test_update_lock_key_state_toggles_only_on_press_edge() {
        let mut queues = RawInputQueues::new();
        queues.caps_lock_on = false;

        update_lock_key_state(&mut queues, VK_CAPITAL, true, false);
        assert!(queues.caps_lock_on);

        update_lock_key_state(&mut queues, VK_CAPITAL, true, true);
        assert!(queues.caps_lock_on);

        update_lock_key_state(&mut queues, VK_CAPITAL, false, true);
        assert!(queues.caps_lock_on);

        update_lock_key_state(&mut queues, VK_CAPITAL, true, false);
        assert!(!queues.caps_lock_on);
    }

    /// Reject unknown HID sensor usages instead of coercing them into accelerometer.
    #[test]
    fn test_sensor_kind_from_usage_rejects_unknown_sensor_usage() {
        let kind = sensor_kind_from_usage(HID_USAGE_PAGE_SENSOR, 0xffff);
        assert_eq!(kind, None);
    }

    /// Reject malformed sensor payloads with explicit invalid-data errors.
    #[test]
    fn test_decode_sensor_sample_from_packet_rejects_short_payloads() {
        let packet = RawHidPacket {
            timestamp_ns: 1,
            sequence: 1,
            device_id: "raw:device:test".to_string(),
            report_id: 0,
            data: vec![1, 2, 3, 4, 5],
        };

        let error = decode_sensor_sample_from_packet(
            packet,
            InputSensorKind::Accelerometer,
            "destack.input.sensor.tryRead",
        )
        .expect_err("short payloads should be rejected");
        let code = error.platform_error().map(|platform| platform.code);
        assert_eq!(code, Some(PlatformErrorCode::IoInvalidData));
    }

    /// Build one synthetic raw gamepad descriptor for capability-based tests.
    fn synthetic_gamepad_descriptor() -> RawInputDeviceDescriptor {
        RawInputDeviceDescriptor {
            id: "raw:gamepad:test".to_string(),
            instance_id: "raw:gamepad:test".to_string(),
            hardware_id: "raw:gamepad:test".to_string(),
            name: "synthetic gamepad".to_string(),
            raw_path: None,
            kind: InputDeviceKind::Gamepad,
            vendor_id: 0,
            product_id: 0,
            usage_page: HID_USAGE_PAGE_GENERIC,
            usage: HID_USAGE_GENERIC_GAMEPAD,
            report_size: 64,
            output_report_size: 0,
            feature_report_size: 0,
            key_count: 0,
            button_count: 17,
            axis_count: 4,
            supports_text: false,
            supports_rumble: false,
            supports_battery: false,
            supports_light: false,
            supports_pointer_grab: false,
            supports_raw_hid: true,
            supports_sensors: false,
            supports_player_index: false,
            value_capabilities: vec![
                RawHidValueCapability {
                    usage_page: HID_USAGE_PAGE_GENERIC,
                    usage_min: HID_USAGE_GENERIC_X,
                    usage_max: HID_USAGE_GENERIC_X,
                    link_collection: 0,
                    report_id: 1,
                    logical_min: -32768,
                    logical_max: 32767,
                },
                RawHidValueCapability {
                    usage_page: HID_USAGE_PAGE_GENERIC,
                    usage_min: HID_USAGE_GENERIC_Y,
                    usage_max: HID_USAGE_GENERIC_Y,
                    link_collection: 0,
                    report_id: 1,
                    logical_min: -32768,
                    logical_max: 32767,
                },
                RawHidValueCapability {
                    usage_page: HID_USAGE_PAGE_GENERIC,
                    usage_min: HID_USAGE_GENERIC_RX,
                    usage_max: HID_USAGE_GENERIC_RX,
                    link_collection: 0,
                    report_id: 1,
                    logical_min: -32768,
                    logical_max: 32767,
                },
                RawHidValueCapability {
                    usage_page: HID_USAGE_PAGE_GENERIC,
                    usage_min: HID_USAGE_GENERIC_RY,
                    usage_max: HID_USAGE_GENERIC_RY,
                    link_collection: 0,
                    report_id: 1,
                    logical_min: -32768,
                    logical_max: 32767,
                },
            ],
            button_capabilities: vec![RawHidButtonCapability {
                usage_page: HID_USAGE_PAGE_BUTTON,
                usage_min: 1,
                usage_max: 10,
                link_collection: 0,
                report_id: 1,
            }],
        }
    }

    /// Classify mapping from capabilities without depending on transient button state.
    #[test]
    fn test_classify_gamepad_mapping_from_capabilities_is_stable() {
        let descriptor = synthetic_gamepad_descriptor();
        let mapping = classify_gamepad_mapping_from_capabilities(&descriptor);
        assert_eq!(mapping, InputGamepadMappingType::Standard);
    }

    /// Derive report-id candidates from hid capability metadata with zero fallback.
    #[test]
    fn test_gamepad_report_id_candidates_include_zero_and_sorted_ids() {
        let descriptor = synthetic_gamepad_descriptor();
        let report_ids = gamepad_report_id_candidates(&descriptor);
        assert_eq!(report_ids, vec![1, 0]);
    }

    /// Derive multiple sensor kinds from hid sensor capability spans.
    #[test]
    fn test_sensor_kinds_for_device_derives_multiple_sensor_lanes() {
        let mut descriptor = synthetic_gamepad_descriptor();
        descriptor.kind = InputDeviceKind::Raw;
        descriptor.usage_page = HID_USAGE_PAGE_SENSOR;
        descriptor.usage = HID_USAGE_SENSOR_ACCELEROMETER_3D;
        descriptor.supports_sensors = true;
        descriptor.value_capabilities = vec![
            RawHidValueCapability {
                usage_page: HID_USAGE_PAGE_SENSOR,
                usage_min: HID_USAGE_SENSOR_ACCELEROMETER_3D,
                usage_max: HID_USAGE_SENSOR_ACCELEROMETER_3D,
                link_collection: 0,
                report_id: 2,
                logical_min: -2048,
                logical_max: 2048,
            },
            RawHidValueCapability {
                usage_page: HID_USAGE_PAGE_SENSOR,
                usage_min: HID_USAGE_SENSOR_GYROMETER_3D,
                usage_max: HID_USAGE_SENSOR_GYROMETER_3D,
                link_collection: 0,
                report_id: 2,
                logical_min: -2048,
                logical_max: 2048,
            },
        ];
        descriptor.button_capabilities.clear();

        let kinds = sensor_kinds_for_device(&descriptor);
        assert_eq!(
            kinds,
            vec![InputSensorKind::Accelerometer, InputSensorKind::Gyroscope]
        );
    }

    /// Map canonical hid button usages to standard gamepad button indices.
    #[test]
    fn test_standard_button_index_for_hid_usage_maps_expected_order() {
        assert_eq!(standard_button_index_for_hid_usage(1), Some(0));
        assert_eq!(standard_button_index_for_hid_usage(6), Some(5));
        assert_eq!(standard_button_index_for_hid_usage(17), Some(16));
        assert_eq!(standard_button_index_for_hid_usage(0), None);
    }

    /// Map hat-switch diagonal samples to expected dpad button bits.
    #[test]
    fn test_apply_hat_switch_to_dpad_maps_diagonal_values() {
        let mut pressed = [false; STANDARD_GAMEPAD_BUTTON_COUNT];
        apply_hat_switch_to_dpad(&mut pressed, 7);
        assert!(pressed[12], "hat value 7 should set dpad up");
        assert!(pressed[14], "hat value 7 should set dpad left");
        assert!(!pressed[13], "hat value 7 should clear dpad down");
        assert!(!pressed[15], "hat value 7 should clear dpad right");
    }

    /// Encode dualshock 4 usb light packets using the expected payload offsets.
    #[test]
    fn test_build_sony_dualshock4_light_report_writes_rgb_slots() {
        let report = build_sony_dualshock4_light_report(1, 2, 3);
        assert_eq!(
            report.len(),
            usize::from(SONY_DUALSHOCK4_USB_EFFECTS_REPORT_BYTES) - 1
        );
        assert_eq!(report[0], 0x07);
        assert_eq!(report[5], 1);
        assert_eq!(report[6], 2);
        assert_eq!(report[7], 3);
    }

    /// Encode dualsense usb light packets using the expected payload offsets.
    #[test]
    fn test_build_sony_dualsense_light_report_writes_rgb_slots() {
        let report = build_sony_dualsense_light_report(4, 5, 6);
        assert_eq!(
            report.len(),
            usize::from(SONY_DUALSENSE_USB_EFFECTS_REPORT_BYTES) - 1
        );
        assert_eq!(report[1], 0x04);
        assert_eq!(report[44], 4);
        assert_eq!(report[45], 5);
        assert_eq!(report[46], 6);
    }
}

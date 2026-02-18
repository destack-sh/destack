use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, mpsc};
use std::time::Duration;
use std::{mem, ptr, thread};

use parking_lot::{Condvar, Mutex};
use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT,
    RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICENAME, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
    RegisterRawInputDevices,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GIDC_ARRIVAL, GIDC_REMOVAL,
    GetMessageW, HWND_MESSAGE, MSG, PostQuitMessage, RI_KEY_BREAK, RI_MOUSE_BUTTON_1_DOWN,
    RI_MOUSE_BUTTON_1_UP, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP, RI_MOUSE_BUTTON_3_DOWN,
    RI_MOUSE_BUTTON_3_UP, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP, RI_MOUSE_BUTTON_5_DOWN,
    RI_MOUSE_BUTTON_5_UP, RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, RegisterClassW, TranslateMessage,
    WM_DESTROY, WM_INPUT, WM_INPUT_DEVICE_CHANGE, WNDCLASSW,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputEvent, InputEventAction, InputEventKind};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Stable identifier for the raw keyboard pseudo-device.
const WINDOWS_INPUT_RAW_KEYBOARD_ID: &str = "raw:keyboard";
/// Stable identifier for the raw mouse pseudo-device.
const WINDOWS_INPUT_RAW_MOUSE_ID: &str = "raw:mouse";
/// Prefix for monitor event device identifiers derived from raw device handles.
const WINDOWS_INPUT_MONITOR_ID_PREFIX: &str = "raw:device:";
/// Pseudo-device identifier used for monitor queue overflow notifications.
const WINDOWS_INPUT_RAW_MONITOR_ID: &str = "raw:monitor";
/// Empty text payload used for non-text events.
const WINDOWS_INPUT_EMPTY_TEXT: &str = "";
/// Maximum queued keyboard or mouse packets before oldest-drop backpressure.
const RAW_INPUT_QUEUE_LIMIT: usize = 8192;
/// Maximum queued monitor packets before oldest-drop backpressure.
const RAW_MONITOR_QUEUE_LIMIT: usize = 1024;
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
/// Generic HID mouse usage.
const HID_USAGE_GENERIC_MOUSE: u16 = 2;
/// Generic HID keyboard usage.
const HID_USAGE_GENERIC_KEYBOARD: u16 = 6;

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

/// Raw input packet captured from keyboard or mouse streams.
#[derive(Debug, Clone)]
struct RawInputPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Runtime event kind.
    kind: InputEventKind,
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
    /// Backend event code.
    code: u32,
    /// Scalar payload value.
    value: i64,
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
}

impl RawInputQueues {
    /// Build empty raw-input queues.
    fn new() -> Self {
        Self {
            keyboard: VecDeque::new(),
            mouse: VecDeque::new(),
            monitor: VecDeque::new(),
            key_down: [false; 256],
            caps_lock_on: query_toggle_key_state(VK_CAPITAL),
            num_lock_on: query_toggle_key_state(VK_NUMLOCK),
            scroll_lock_on: query_toggle_key_state(VK_SCROLL),
            monitor_device_ids: HashMap::new(),
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

/// Initialized raw-input worker service.
#[derive(Debug, Clone)]
struct RawInputService {
    /// Shared worker state.
    state: std::sync::Arc<RawInputState>,
}

/// Queue selector for keyboard, mouse, and monitor streams.
#[derive(Clone, Copy)]
enum RawQueueKind {
    /// Keyboard packet queue.
    Keyboard,
    /// Mouse packet queue.
    Mouse,
    /// Monitor packet queue.
    Monitor,
}

/// Shared pointer used by the window procedure to enqueue events.
static RAW_INPUT_STATE: OnceLock<std::sync::Arc<RawInputState>> = OnceLock::new();
/// Singleton service slot with restart support when the worker exits.
static RAW_INPUT_SERVICE: OnceLock<Mutex<Option<RawInputService>>> = OnceLock::new();
/// Liveness state for the raw-input message worker.
static RAW_INPUT_WORKER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Return the host performance-counter frequency.
fn performance_counter_frequency() -> u64 {
    static PERFORMANCE_COUNTER_FREQUENCY: OnceLock<u64> = OnceLock::new();
    *PERFORMANCE_COUNTER_FREQUENCY.get_or_init(|| {
        let mut frequency = 0i64;
        let status = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if status == 0 || frequency <= 0 {
            return 0;
        }

        frequency as u64
    })
}

/// Read one monotonic timestamp from QueryPerformanceCounter.
fn now_timestamp_ns() -> u64 {
    let frequency = performance_counter_frequency();
    if frequency == 0 {
        return 0;
    }

    let mut counter = 0i64;
    let status = unsafe { QueryPerformanceCounter(&mut counter) };
    if status == 0 || counter < 0 {
        return 0;
    }

    ((counter as u128).saturating_mul(1_000_000_000u128) / u128::from(frequency)) as u64
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
    // query UTF-16 device name size from user32
    let handle = raw_device as HANDLE;
    let mut code_units = 0u32;
    let status = unsafe {
        GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, ptr::null_mut(), &mut code_units)
    };
    if status == u32::MAX || code_units == 0 {
        return format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{:x}", raw_device as usize);
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
        return format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{:x}", raw_device as usize);
    }

    let limit = usize::min(buffer.len(), code_units as usize);
    let name_len = buffer[..limit]
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(limit);
    if name_len == 0 {
        return format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{:x}", raw_device as usize);
    }

    let name = String::from_utf16_lossy(&buffer[..name_len]).to_ascii_lowercase();
    format!("{WINDOWS_INPUT_MONITOR_ID_PREFIX}{name}")
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

/// Return the raw service slot used for lazy startup and restart.
fn raw_service_slot() -> &'static Mutex<Option<RawInputService>> {
    RAW_INPUT_SERVICE.get_or_init(|| Mutex::new(None))
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
fn push_input_packet(queue: &mut VecDeque<RawInputPacket>, packet: RawInputPacket) {
    if queue.len() >= RAW_INPUT_QUEUE_LIMIT {
        queue.pop_front();

        // coalesce one overflow marker in the queue tail when drops occur
        if let Some(overflow) = queue.back_mut() {
            if overflow.kind == InputEventKind::Device
                && overflow.action == InputEventAction::Cancel
                && overflow.code == RAW_INPUT_OVERFLOW_CODE
            {
                overflow.value = overflow.value.saturating_add(1);
                queue.push_back(packet);
                return;
            }
        }

        queue.push_back(RawInputPacket {
            timestamp_ns: packet.timestamp_ns,
            kind: InputEventKind::Device,
            action: InputEventAction::Cancel,
            code: RAW_INPUT_OVERFLOW_CODE,
            scan_code: RAW_INPUT_OVERFLOW_CODE,
            value: 1,
            x: 0.0,
            y: 0.0,
            wheel_x: 0.0,
            wheel_y: 0.0,
            modifiers: 0,
            repeat: false,
        });

        // keep queue bounded after inserting one new overflow marker
        if queue.len() >= RAW_INPUT_QUEUE_LIMIT {
            queue.pop_front();
        }
    }

    queue.push_back(packet);
}

/// Push one monitor packet with bounded queue growth.
fn push_monitor_packet(queue: &mut VecDeque<RawMonitorPacket>, packet: RawMonitorPacket) {
    if queue.len() >= RAW_MONITOR_QUEUE_LIMIT {
        queue.pop_front();

        // coalesce one overflow marker in the queue tail when drops occur
        if let Some(overflow) = queue.back_mut() {
            if overflow.action == InputEventAction::Cancel
                && overflow.code == RAW_MONITOR_OVERFLOW_CODE
            {
                overflow.value = overflow.value.saturating_add(1);
                queue.push_back(packet);
                return;
            }
        }

        queue.push_back(RawMonitorPacket {
            timestamp_ns: packet.timestamp_ns,
            action: InputEventAction::Cancel,
            device_id: WINDOWS_INPUT_RAW_MONITOR_ID.to_string(),
            code: RAW_MONITOR_OVERFLOW_CODE,
            value: 1,
        });

        // keep queue bounded after inserting one new overflow marker
        if queue.len() >= RAW_MONITOR_QUEUE_LIMIT {
            queue.pop_front();
        }
    }

    queue.push_back(packet);
}

/// Register keyboard and mouse raw-input devices for one window target.
fn ensure_raw_input_registration(hwnd: HWND) -> RuntimeResult<()> {
    // register keyboard and mouse usages for input-sink delivery
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

    Ok(())
}

/// Spawn the raw-input worker and wait for successful initialization.
fn spawn_raw_input_service() -> Result<RawInputService, String> {
    // reuse existing shared state so restarted workers keep servicing the same queues
    let state = if let Some(state) = RAW_INPUT_STATE.get() {
        state.clone()
    } else {
        let state = std::sync::Arc::new(RawInputState::new());
        let _ = RAW_INPUT_STATE.set(state.clone());
        state
    };

    // spawn the message-thread worker and wait for readiness
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let thread_state = state.clone();
    thread::Builder::new()
        .name("destack-input-raw".to_string())
        .spawn(move || {
            raw_input_thread_main(thread_state, ready_tx);
        })
        .map_err(|error| format!("failed to spawn raw input thread: {error}"))?;

    // require worker readiness before serving any binding calls
    let ready = ready_rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| "raw input thread startup timed out".to_string())?;
    ready?;

    Ok(RawInputService { state })
}

/// Run the raw-input message-thread main loop.
fn raw_input_thread_main(
    state: std::sync::Arc<RawInputState>,
    ready_tx: mpsc::Sender<Result<(), String>>,
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

    // create the message-only window used for raw input delivery
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
            ptr::null(),
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

    // keep one reference alive in this thread for event push paths
    let _thread_state = state;

    // report readiness before entering the message loop
    RAW_INPUT_WORKER_RUNNING.store(true, Ordering::Release);
    let _ = ready_tx.send(Ok(()));

    // pump the Windows message queue until shutdown
    let mut message = MSG {
        hwnd: 0,
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
    };
    loop {
        let status = unsafe { GetMessageW(&mut message, 0, 0, 0) };
        if status == 0 {
            break;
        }
        if status < 0 {
            break;
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    // release the worker window on exit
    unsafe {
        DestroyWindow(hwnd);
    }

    // mark worker exit and wake readers so they can surface restartable failures
    RAW_INPUT_WORKER_RUNNING.store(false, Ordering::Release);
    if let Some(state) = RAW_INPUT_STATE.get() {
        state.wake.notify_all();
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
        WM_INPUT => {
            handle_raw_input_message(lparam as isize);
            0
        }
        WM_INPUT_DEVICE_CHANGE => {
            handle_raw_device_change_message(wparam as u32, lparam as isize);
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

/// Handle one raw device-change notification.
fn handle_raw_device_change_message(kind: u32, raw_device: isize) {
    // resolve shared state that receives monitor packets
    let Some(state) = RAW_INPUT_STATE.get() else {
        return;
    };

    // map Win32 device-change kinds into runtime monitor actions
    let action = if kind == GIDC_ARRIVAL {
        InputEventAction::Connect
    } else if kind == GIDC_REMOVAL {
        InputEventAction::Disconnect
    } else {
        return;
    };

    // enqueue one monitor packet and wake blocked readers
    let mut queues = state.queues.lock();
    let device_id = resolve_monitor_device_id(kind, raw_device, &mut queues.monitor_device_ids);
    push_monitor_packet(
        &mut queues.monitor,
        RawMonitorPacket {
            timestamp_ns: now_timestamp_ns(),
            action,
            device_id,
            code: kind,
            value: if action == InputEventAction::Connect {
                1
            } else {
                0
            },
        },
    );
    state.wake.notify_all();
}

/// Handle one raw keyboard or mouse input message payload.
fn handle_raw_input_message(raw_input_handle: isize) {
    // resolve shared state that receives input packets
    let Some(state) = RAW_INPUT_STATE.get() else {
        return;
    };

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

    // parse payload and enqueue normalized packets
    let raw = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<RAWINPUT>()) };
    let mut queues = state.queues.lock();
    let timestamp = now_timestamp_ns();

    if raw.header.dwType == RIM_TYPEKEYBOARD {
        // decode keyboard press or release semantics
        let keyboard = unsafe { raw.data.keyboard };
        let key_code = keyboard.VKey as usize;
        if key_code >= queues.key_down.len() {
            return;
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

        // enqueue normalized keyboard packet
        push_input_packet(
            &mut queues.keyboard,
            RawInputPacket {
                timestamp_ns: timestamp,
                kind: InputEventKind::Key,
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
                modifiers,
                repeat: is_repeat,
            },
        );
    } else if raw.header.dwType == RIM_TYPEMOUSE {
        // decode mouse movement, wheel, and button payloads
        let mouse = unsafe { raw.data.mouse };
        let is_absolute = (mouse.usFlags & RAW_MOUSE_MOVE_ABSOLUTE) != 0;
        let button_flags = unsafe { u32::from(mouse.Anonymous.Anonymous.usButtonFlags) };
        let button_data = unsafe { mouse.Anonymous.Anonymous.usButtonData };
        let modifiers = control_key_state_from_queues(&queues);

        if mouse.lLastX != 0 || mouse.lLastY != 0 || is_absolute {
            // normalize absolute coordinates and preserve relative deltas
            let (x, y, code) = normalize_raw_mouse_motion(&mouse);

            // enqueue pointer movement
            push_input_packet(
                &mut queues.mouse,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    kind: InputEventKind::PointerMotion,
                    action: InputEventAction::Move,
                    code,
                    scan_code: code,
                    value: if is_absolute { 1 } else { 0 },
                    x,
                    y,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    modifiers,
                    repeat: false,
                },
            );
        }

        if button_flags & RI_MOUSE_WHEEL != 0 {
            // enqueue vertical wheel delta
            let delta = i16::from_ne_bytes(button_data.to_ne_bytes()) as f64;
            push_input_packet(
                &mut queues.mouse,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    kind: InputEventKind::Scroll,
                    action: InputEventAction::Scroll,
                    code: RI_MOUSE_WHEEL,
                    scan_code: RI_MOUSE_WHEEL,
                    value: delta as i64,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: delta,
                    modifiers,
                    repeat: false,
                },
            );
        }

        if button_flags & RI_MOUSE_HWHEEL != 0 {
            // enqueue horizontal wheel delta
            let delta = i16::from_ne_bytes(button_data.to_ne_bytes()) as f64;
            push_input_packet(
                &mut queues.mouse,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    kind: InputEventKind::Scroll,
                    action: InputEventAction::Scroll,
                    code: RI_MOUSE_HWHEEL,
                    scan_code: RI_MOUSE_HWHEEL,
                    value: delta as i64,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: delta,
                    wheel_y: 0.0,
                    modifiers,
                    repeat: false,
                },
            );
        }

        // enqueue pressed or released button packets
        push_mouse_button_events(&mut queues.mouse, button_flags, timestamp, modifiers);
    }

    // wake blocked readers after packet enqueue
    state.wake.notify_all();
}

/// Push mouse button transitions described by one raw flag word.
fn push_mouse_button_events(
    queue: &mut VecDeque<RawInputPacket>,
    button_flags: u32,
    timestamp: u64,
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
            // enqueue button-press packet
            push_input_packet(
                queue,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    kind: InputEventKind::PointerButton,
                    action: InputEventAction::Press,
                    code,
                    scan_code: code,
                    value: 1,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    modifiers,
                    repeat: false,
                },
            );
        }

        if button_flags & up_flag != 0 {
            // enqueue button-release packet
            push_input_packet(
                queue,
                RawInputPacket {
                    timestamp_ns: timestamp,
                    kind: InputEventKind::PointerButton,
                    action: InputEventAction::Release,
                    code,
                    scan_code: code,
                    value: 0,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    modifiers,
                    repeat: false,
                },
            );
        }
    }
}

/// Pop one queued packet, optionally blocking for the next event.
fn pop_queue_event(
    queue_kind: RawQueueKind,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<QueueEvent> {
    // ensure the singleton raw service is initialized
    let service = ensure_raw_service(operation)?;
    let mut queues = service.state.queues.lock();

    loop {
        // select one queue and pop the next packet if available
        let event = match queue_kind {
            RawQueueKind::Keyboard => queues.keyboard.pop_front().map(QueueEvent::Input),
            RawQueueKind::Mouse => queues.mouse.pop_front().map(QueueEvent::Input),
            RawQueueKind::Monitor => queues.monitor.pop_front().map(QueueEvent::Monitor),
        };
        if let Some(event) = event {
            return Ok(event);
        }

        // return would-block immediately for nonblocking callers
        if nonblocking {
            return Err(io_would_block(operation, "input queue is empty"));
        }

        // otherwise wait until the worker enqueues the next packet or exits
        if !RAW_INPUT_WORKER_RUNNING.load(Ordering::Acquire) {
            return Err(service_error(operation, "raw input worker stopped"));
        }

        service.state.wake.wait(&mut queues);
        if !RAW_INPUT_WORKER_RUNNING.load(Ordering::Acquire) {
            return Err(service_error(operation, "raw input worker stopped"));
        }
    }
}

/// Tagged event payload returned from queue pop operations.
enum QueueEvent {
    /// Keyboard or mouse event packet.
    Input(RawInputPacket),
    /// Device monitor event packet.
    Monitor(RawMonitorPacket),
}

/// Return the singleton raw service or map startup errors.
fn ensure_raw_service(operation: &'static str) -> RuntimeResult<RawInputService> {
    let mut slot = raw_service_slot().lock();

    // respawn the service when no worker exists or the previous worker has exited
    let requires_spawn = match slot.as_ref() {
        Some(_) => !RAW_INPUT_WORKER_RUNNING.load(Ordering::Acquire),
        None => true,
    };
    if requires_spawn {
        let service = spawn_raw_input_service()
            .map_err(|error| service_error(operation, format!("raw input unavailable: {error}")))?;
        *slot = Some(service);
    }

    match slot.as_ref() {
        Some(service) => Ok(service.clone()),
        None => Err(service_error(
            operation,
            "raw input unavailable: missing service state",
        )),
    }
}

/// Ensure the raw service is initialized for one binding operation.
pub(super) fn ensure_service(operation: &'static str) -> RuntimeResult<()> {
    let _ = ensure_raw_service(operation)?;
    Ok(())
}

/// Read one keyboard event from the raw queue.
pub(super) fn read_keyboard_event(
    context: &RuntimeCallContext,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // pop one keyboard packet and validate queue kind
    let event = pop_queue_event(RawQueueKind::Keyboard, nonblocking, operation)?;
    let QueueEvent::Input(event) = event else {
        return Err(service_error(operation, "internal keyboard queue mismatch"));
    };

    // map raw keyboard packet into one runtime event payload
    Ok(InputEvent {
        kind: event.kind,
        timestamp_ns: event.timestamp_ns,
        sequence: 0,
        device_id: context.store_string(WINDOWS_INPUT_RAW_KEYBOARD_ID),
        action: event.action,
        code: event.code,
        scan_code: event.scan_code,
        value: event.value,
        x: event.x,
        y: event.y,
        wheel_x: event.wheel_x,
        wheel_y: event.wheel_y,
        modifiers: event.modifiers,
        repeat: event.repeat,
        text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
    })
}

/// Read one mouse event from the raw queue.
pub(super) fn read_mouse_event(
    context: &RuntimeCallContext,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // pop one mouse packet and validate queue kind
    let event = pop_queue_event(RawQueueKind::Mouse, nonblocking, operation)?;
    let QueueEvent::Input(event) = event else {
        return Err(service_error(operation, "internal mouse queue mismatch"));
    };

    // map raw mouse packet into one runtime event payload
    Ok(InputEvent {
        kind: event.kind,
        timestamp_ns: event.timestamp_ns,
        sequence: 0,
        device_id: context.store_string(WINDOWS_INPUT_RAW_MOUSE_ID),
        action: event.action,
        code: event.code,
        scan_code: event.scan_code,
        value: event.value,
        x: event.x,
        y: event.y,
        wheel_x: event.wheel_x,
        wheel_y: event.wheel_y,
        modifiers: event.modifiers,
        repeat: event.repeat,
        text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
    })
}

/// Read one monitor event from the raw queue.
pub(super) fn read_monitor_event(
    context: &RuntimeCallContext,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    // pop one monitor packet and validate queue kind
    let event = pop_queue_event(RawQueueKind::Monitor, nonblocking, operation)?;
    let QueueEvent::Monitor(event) = event else {
        return Err(service_error(operation, "internal monitor queue mismatch"));
    };

    // map monitor packet into one runtime device event payload
    Ok(InputEvent {
        kind: InputEventKind::Device,
        timestamp_ns: event.timestamp_ns,
        sequence: 0,
        device_id: context.store_string(&event.device_id),
        action: event.action,
        code: event.code,
        scan_code: event.code,
        value: event.value,
        x: 0.0,
        y: 0.0,
        wheel_x: 0.0,
        wheel_y: 0.0,
        modifiers: 0,
        repeat: false,
        text: context.store_string(WINDOWS_INPUT_EMPTY_TEXT),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drop the oldest keyboard packet when the queue reaches its bounded capacity.
    #[test]
    fn test_push_input_packet_drops_oldest_when_full() {
        let mut queue = VecDeque::new();
        for index in 0..=RAW_INPUT_QUEUE_LIMIT {
            push_input_packet(
                &mut queue,
                RawInputPacket {
                    timestamp_ns: index as u64,
                    kind: InputEventKind::Key,
                    action: InputEventAction::Press,
                    code: index as u32,
                    scan_code: index as u32,
                    value: 1,
                    x: 0.0,
                    y: 0.0,
                    wheel_x: 0.0,
                    wheel_y: 0.0,
                    modifiers: 0,
                    repeat: false,
                },
            );
        }

        assert_eq!(queue.len(), RAW_INPUT_QUEUE_LIMIT);
        let overflow_event = queue.iter().find(|event| {
            event.kind == InputEventKind::Device
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
        let mut queue = VecDeque::new();
        for index in 0..=RAW_MONITOR_QUEUE_LIMIT {
            push_monitor_packet(
                &mut queue,
                RawMonitorPacket {
                    timestamp_ns: index as u64,
                    action: InputEventAction::Connect,
                    device_id: format!("device:{index}"),
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

    /// Fall back to handle-derived monitor ids when Win32 name queries fail.
    #[test]
    fn test_monitor_device_id_falls_back_when_name_query_fails() {
        let id = monitor_device_id(0);
        assert_eq!(id, "raw:device:0");
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
}

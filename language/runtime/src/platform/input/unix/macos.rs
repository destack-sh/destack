use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;

use parking_lot::{Condvar, Mutex};

use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputEvent, InputEventAction, InputEventKind, InputReadMode};
use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Stable runtime identifier for macOS global session input.
pub(super) const MACOS_INPUT_SESSION_ID: &str = "macos:session";
/// Alias accepted for macOS global session input.
pub(super) const MACOS_INPUT_SESSION_ALIAS: &str = "session";
/// Alias accepted for macOS event-tap style global input.
pub(super) const MACOS_INPUT_EVENT_TAP_ALIAS: &str = "eventtap";
/// Display name for macOS global session input.
pub(super) const MACOS_INPUT_SESSION_NAME: &str = "macos global input session";
/// Reported key count for the event-tap macOS session backend.
pub(super) const MACOS_SESSION_KEY_COUNT: u16 = 128;
/// Reported button count for the event-tap macOS session backend.
pub(super) const MACOS_SESSION_BUTTON_COUNT: u16 = 5;
/// Reported axis count for the event-tap macOS session backend.
pub(super) const MACOS_SESSION_AXIS_COUNT: u16 = 2;
/// Maximum queued packets per subscription queue before oldest-drop.
const MACOS_EVENT_QUEUE_LIMIT: usize = 8192;

/// CoreGraphics event type for one left mouse button down packet.
const KCG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
/// CoreGraphics event type for one left mouse button up packet.
const KCG_EVENT_LEFT_MOUSE_UP: u32 = 2;
/// CoreGraphics event type for one right mouse button down packet.
const KCG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
/// CoreGraphics event type for one right mouse button up packet.
const KCG_EVENT_RIGHT_MOUSE_UP: u32 = 4;
/// CoreGraphics event type for one mouse move packet.
const KCG_EVENT_MOUSE_MOVED: u32 = 5;
/// CoreGraphics event type for one left mouse drag packet.
const KCG_EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
/// CoreGraphics event type for one right mouse drag packet.
const KCG_EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
/// CoreGraphics event type for one key down packet.
const KCG_EVENT_KEY_DOWN: u32 = 10;
/// CoreGraphics event type for one key up packet.
const KCG_EVENT_KEY_UP: u32 = 11;
/// CoreGraphics event type for one modifier-flags change packet.
const KCG_EVENT_FLAGS_CHANGED: u32 = 12;
/// CoreGraphics event type for one scroll wheel packet.
const KCG_EVENT_SCROLL_WHEEL: u32 = 22;
/// CoreGraphics event type for one other mouse button down packet.
const KCG_EVENT_OTHER_MOUSE_DOWN: u32 = 25;
/// CoreGraphics event type for one other mouse button up packet.
const KCG_EVENT_OTHER_MOUSE_UP: u32 = 26;
/// CoreGraphics event type for one other mouse drag packet.
const KCG_EVENT_OTHER_MOUSE_DRAGGED: u32 = 27;
/// CoreGraphics event type for one tap disabled by timeout packet.
const KCG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xffff_fffe;
/// CoreGraphics event type for one tap disabled by user-input packet.
const KCG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xffff_ffff;

/// CoreGraphics field id for mouse button number.
const KCG_MOUSE_EVENT_BUTTON_NUMBER: i32 = 3;
/// CoreGraphics field id for keyboard autorepeat.
const KCG_KEYBOARD_EVENT_AUTOREPEAT: i32 = 8;
/// CoreGraphics field id for keyboard keycode.
const KCG_KEYBOARD_EVENT_KEYCODE: i32 = 9;
/// CoreGraphics field id for vertical wheel delta.
const KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS1: i32 = 11;
/// CoreGraphics field id for horizontal wheel delta.
const KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS2: i32 = 12;

/// CoreGraphics session-wide event tap location.
const KCG_SESSION_EVENT_TAP: u32 = 1;
/// CoreGraphics insertion point for new event taps.
const KCG_HEAD_INSERT_EVENT_TAP: u32 = 0;
/// CoreGraphics listen-only event tap option.
const KCG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

/// CoreFoundation run-loop source order for event taps.
const KCF_RUN_LOOP_SOURCE_ORDER: libc::c_long = 0;

/// CoreGraphics point payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGPoint {
    /// X coordinate.
    x: f64,
    /// Y coordinate.
    y: f64,
}

/// CoreGraphics event reference type.
type CGEventRef = *mut libc::c_void;
/// CoreGraphics event-tap proxy reference type.
type CGEventTapProxy = *mut libc::c_void;
/// CoreGraphics event mask type.
type CGEventMask = u64;
/// CoreFoundation opaque reference type.
type CFTypeRef = *const libc::c_void;
/// CoreFoundation allocator reference type.
type CFAllocatorRef = *const libc::c_void;
/// CoreFoundation string reference type.
type CFStringRef = *const libc::c_void;
/// CoreFoundation run loop reference type.
type CFRunLoopRef = *mut libc::c_void;
/// CoreFoundation run-loop source reference type.
type CFRunLoopSourceRef = *mut libc::c_void;
/// CoreFoundation mach-port reference type.
type CFMachPortRef = *mut libc::c_void;
/// CoreGraphics event-tap callback function type.
type CGEventTapCallBack =
    Option<unsafe extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut libc::c_void) -> CGEventRef>;

// link ApplicationServices symbols used for global input event taps
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Create one global event tap for one event mask.
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut libc::c_void,
    ) -> CFMachPortRef;
    /// Enable or disable one global event tap.
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    /// Return pointer location from one CoreGraphics event object.
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    /// Return one integer field from one CoreGraphics event object.
    fn CGEventGetIntegerValueField(event: CGEventRef, field: i32) -> i64;
    /// Return one modifier-flags bitset from one CoreGraphics event object.
    fn CGEventGetFlags(event: CGEventRef) -> u64;
}

// link CoreFoundation symbols used for run-loop integration
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Create one run-loop source from one mach port.
    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: libc::c_long,
    ) -> CFRunLoopSourceRef;
    /// Return the current thread run loop.
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    /// Add one source to one run loop with one mode.
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    /// Run one CoreFoundation run loop until it is stopped.
    fn CFRunLoopRun();
    /// Release one CoreFoundation object reference.
    fn CFRelease(value: CFTypeRef);
    /// Default run-loop mode used by event sources.
    static kCFRunLoopDefaultMode: CFStringRef;
}

/// Queued packet captured from one CoreGraphics event tap callback.
#[derive(Debug, Clone)]
struct MacosTapPacket {
    /// Event timestamp in monotonic nanoseconds.
    timestamp_ns: u64,
    /// Event kind selector.
    kind: InputEventKind,
    /// Event action selector.
    action: InputEventAction,
    /// Backend event code.
    code: u32,
    /// Backend scan code.
    scan_code: u32,
    /// Scalar event value.
    value: i64,
    /// Pointer x coordinate when applicable.
    x: f64,
    /// Pointer y coordinate when applicable.
    y: f64,
    /// Horizontal wheel delta.
    wheel_x: f64,
    /// Vertical wheel delta.
    wheel_y: f64,
    /// Modifier flags bitset.
    modifiers: u32,
    /// Whether this packet is one repeat event.
    repeat: bool,
}

/// Event-tap worker startup state.
#[derive(Debug, Clone)]
enum MacosTapStartupState {
    /// Worker setup is pending.
    Pending,
    /// Worker setup succeeded.
    Ready,
    /// Worker setup failed with one human-readable reason.
    Failed(String),
}

/// Shared in-memory queues for one global event tap service.
#[derive(Debug)]
struct MacosTapQueues {
    /// Startup lifecycle state for the worker thread.
    startup: MacosTapStartupState,
    /// Next subscription id allocator.
    next_subscription_id: u64,
    /// Per-subscription event queues.
    subscriptions: HashMap<u64, VecDeque<MacosTapPacket>>,
}

impl MacosTapQueues {
    /// Build one empty event-tap queue state.
    fn new() -> Self {
        Self {
            startup: MacosTapStartupState::Pending,
            next_subscription_id: 1,
            subscriptions: HashMap::new(),
        }
    }
}

/// Shared synchronization state for the macOS event tap service.
#[derive(Debug)]
struct MacosTapState {
    /// Protected queue payloads and startup metadata.
    queues: Mutex<MacosTapQueues>,
    /// Wake primitive used by blocking readers.
    wake: Condvar,
}

impl MacosTapState {
    /// Build one empty synchronized event-tap state.
    fn new() -> Self {
        Self {
            queues: Mutex::new(MacosTapQueues::new()),
            wake: Condvar::new(),
        }
    }
}

/// Cached macOS session state for one opened binding.
#[derive(Debug)]
pub(super) struct MacosInputState {
    /// Optional subscription id allocated for this handle.
    subscription_id: Option<u64>,
}

impl MacosInputState {
    /// Build one empty macOS session state.
    pub(super) fn new() -> Self {
        Self {
            subscription_id: None,
        }
    }

    /// Return the current subscription id when one is active.
    fn subscription_id(&self) -> Option<u64> {
        self.subscription_id
    }

    /// Store one subscription id for this state.
    fn set_subscription_id(&mut self, subscription_id: u64) {
        self.subscription_id = Some(subscription_id);
    }

    /// Take and clear the current subscription id.
    fn take_subscription_id(&mut self) -> Option<u64> {
        self.subscription_id.take()
    }
}

/// Global event-tap service state.
static MACOS_TAP_STATE: OnceLock<Arc<MacosTapState>> = OnceLock::new();
/// Worker bootstrap slot for one event-tap service.
static MACOS_TAP_WORKER: OnceLock<Result<(), String>> = OnceLock::new();
/// Global event-tap port pointer for callback-side re-enable handling.
static MACOS_TAP_PORT: AtomicUsize = AtomicUsize::new(0);

/// Return one shared event-tap state instance.
fn macos_tap_state() -> &'static Arc<MacosTapState> {
    MACOS_TAP_STATE.get_or_init(|| Arc::new(MacosTapState::new()))
}

/// Return one event-mask bit for one CoreGraphics event type.
const fn cg_event_mask_bit(event_type: u32) -> CGEventMask {
    1u64 << event_type
}

/// Return one standardized event mask for supported macOS input events.
const fn supported_event_mask() -> CGEventMask {
    cg_event_mask_bit(KCG_EVENT_LEFT_MOUSE_DOWN)
        | cg_event_mask_bit(KCG_EVENT_LEFT_MOUSE_UP)
        | cg_event_mask_bit(KCG_EVENT_RIGHT_MOUSE_DOWN)
        | cg_event_mask_bit(KCG_EVENT_RIGHT_MOUSE_UP)
        | cg_event_mask_bit(KCG_EVENT_MOUSE_MOVED)
        | cg_event_mask_bit(KCG_EVENT_LEFT_MOUSE_DRAGGED)
        | cg_event_mask_bit(KCG_EVENT_RIGHT_MOUSE_DRAGGED)
        | cg_event_mask_bit(KCG_EVENT_KEY_DOWN)
        | cg_event_mask_bit(KCG_EVENT_KEY_UP)
        | cg_event_mask_bit(KCG_EVENT_FLAGS_CHANGED)
        | cg_event_mask_bit(KCG_EVENT_SCROLL_WHEEL)
        | cg_event_mask_bit(KCG_EVENT_OTHER_MOUSE_DOWN)
        | cg_event_mask_bit(KCG_EVENT_OTHER_MOUSE_UP)
        | cg_event_mask_bit(KCG_EVENT_OTHER_MOUSE_DRAGGED)
}

/// Return whether one identifier targets the macOS session backend.
pub(super) fn is_macos_session_identifier(id: &str, id_lower: &str) -> bool {
    id == MACOS_INPUT_SESSION_ID
        || id_lower == MACOS_INPUT_SESSION_ALIAS
        || id_lower == MACOS_INPUT_EVENT_TAP_ALIAS
}

/// Ensure the background event-tap worker is started and ready.
fn ensure_tap_service_ready(operation: &'static str) -> RuntimeResult<()> {
    // spawn the worker once for this process
    let start_result = MACOS_TAP_WORKER.get_or_init(|| {
        let state = Arc::clone(macos_tap_state());
        let builder = thread::Builder::new().name("destack-input-macos-tap".to_string());
        match builder.spawn(move || run_event_tap_worker(state)) {
            Ok(_) => Ok(()),
            Err(error) => Err(format!("failed to spawn macos input worker: {error}")),
        }
    });
    if let Err(error) = start_result {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some(operation.to_string()),
            None,
            error.clone(),
        ))
        .boxed());
    }

    // wait until worker reports ready or failed startup state
    let state = macos_tap_state();
    let mut queues = state.queues.lock();
    loop {
        match &queues.startup {
            MacosTapStartupState::Ready => return Ok(()),
            MacosTapStartupState::Failed(error) => {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoPermissionDenied),
                    None,
                    None,
                    Some(operation.to_string()),
                    None,
                    error.clone(),
                ))
                .boxed());
            }
            MacosTapStartupState::Pending => {
                state.wake.wait(&mut queues);
            }
        }
    }
}

/// Register one subscriber queue for one opened input handle.
fn register_subscription(operation: &'static str) -> RuntimeResult<u64> {
    ensure_tap_service_ready(operation)?;

    let state = macos_tap_state();
    let mut queues = state.queues.lock();
    let subscription_id = queues.next_subscription_id;
    queues.next_subscription_id = queues.next_subscription_id.saturating_add(1);
    queues
        .subscriptions
        .insert(subscription_id, VecDeque::with_capacity(64));

    Ok(subscription_id)
}

/// Remove one subscriber queue when one handle is closed.
pub(super) fn unregister_subscription(subscription_id: u64) {
    let state = macos_tap_state();
    let mut queues = state.queues.lock();
    queues.subscriptions.remove(&subscription_id);
}

/// Pop one queued packet for one subscription without blocking.
fn try_pop_subscription_event(subscription_id: u64) -> RuntimeResult<Option<MacosTapPacket>> {
    let state = macos_tap_state();
    let mut queues = state.queues.lock();

    match queues.startup {
        MacosTapStartupState::Failed(ref error) => {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoPermissionDenied),
                None,
                None,
                Some("destack.input.event.read".to_string()),
                None,
                error.clone(),
            ))
            .boxed());
        }
        MacosTapStartupState::Pending | MacosTapStartupState::Ready => {}
    }

    let Some(queue) = queues.subscriptions.get_mut(&subscription_id) else {
        return Ok(None);
    };

    Ok(queue.pop_front())
}

/// Pop one queued packet for one subscription and wait when empty.
fn wait_pop_subscription_event(subscription_id: u64) -> RuntimeResult<Option<MacosTapPacket>> {
    let state = macos_tap_state();
    let mut queues = state.queues.lock();

    loop {
        // reject failed worker state with explicit permission-like error
        if let MacosTapStartupState::Failed(ref error) = queues.startup {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoPermissionDenied),
                None,
                None,
                Some("destack.input.event.read".to_string()),
                None,
                error.clone(),
            ))
            .boxed());
        }

        // return one queued packet when available
        let Some(queue) = queues.subscriptions.get_mut(&subscription_id) else {
            return Ok(None);
        };
        if let Some(packet) = queue.pop_front() {
            return Ok(Some(packet));
        }

        // wait for callback-enqueued packets
        state.wake.wait(&mut queues);
    }
}

/// Convert one queued packet into one runtime input event payload.
fn packet_to_input_event(context: &RuntimeCallContext, packet: MacosTapPacket) -> InputEvent {
    InputEvent {
        kind: packet.kind,
        timestamp_ns: packet.timestamp_ns,
        sequence: 0,
        device_id: context.store_string(MACOS_INPUT_SESSION_ID),
        action: packet.action,
        code: packet.code,
        scan_code: packet.scan_code,
        value: packet.value,
        x: packet.x,
        y: packet.y,
        wheel_x: packet.wheel_x,
        wheel_y: packet.wheel_y,
        modifiers: packet.modifiers,
        repeat: packet.repeat,
        text: context.store_string(input_core::UNIX_INPUT_EMPTY_TEXT),
    }
}

/// Resolve or allocate one subscription id for one opened handle.
fn resolve_subscription_id(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let subscription = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<input_core::UnixInputBinding>())?;
            if binding.backend != input_core::UnixInputBackend::Platform {
                return None;
            }

            let state = binding.macos_state.as_mut()?;
            if let Some(subscription_id) = state.subscription_id() {
                return Some(Ok(subscription_id));
            }

            match register_subscription(operation) {
                Ok(subscription_id) => {
                    state.set_subscription_id(subscription_id);
                    Some(Ok(subscription_id))
                }
                Err(error) => Some(Err(error)),
            }
        });

    match subscription {
        Some(Some(Ok(subscription_id))) => Ok(subscription_id),
        Some(Some(Err(error))) => Err(error),
        Some(None) | None => Err(input_core::input_not_found(operation, handle)),
    }
}

/// Poll one event-tap-backed macOS input event.
pub(super) fn read_macos_session_event(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    _read_mode: InputReadMode,
) -> RuntimeResult<InputEvent> {
    let subscription_id = resolve_subscription_id(context, handle, "destack.input.event.read")?;

    if nonblocking {
        let Some(packet) = try_pop_subscription_event(subscription_id)? else {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                Some(libc::EWOULDBLOCK),
                Some("poll".to_string()),
                None,
                "input queue is empty",
            ))
            .boxed());
        };
        return Ok(packet_to_input_event(context, packet));
    }

    let Some(packet) = wait_pop_subscription_event(subscription_id)? else {
        return Err(input_core::input_not_found(
            "destack.input.event.read",
            handle,
        ));
    };
    Ok(packet_to_input_event(context, packet))
}

/// Release one macOS session subscription for one input handle.
pub(super) fn release_macos_session_subscription(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
) {
    let subscription = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<input_core::UnixInputBinding>())?;
            if binding.backend != input_core::UnixInputBackend::Platform {
                return None;
            }

            let state = binding.macos_state.as_mut()?;
            Some(state.take_subscription_id())
        });

    if let Some(subscription_id) = subscription.flatten().flatten() {
        unregister_subscription(subscription_id);
    }
}

/// Run the global CoreGraphics event-tap worker loop.
fn run_event_tap_worker(state: Arc<MacosTapState>) {
    // create one listen-only event tap for supported event types
    let tap = unsafe {
        CGEventTapCreate(
            KCG_SESSION_EVENT_TAP,
            KCG_HEAD_INSERT_EVENT_TAP,
            KCG_EVENT_TAP_OPTION_LISTEN_ONLY,
            supported_event_mask(),
            Some(event_tap_callback),
            std::ptr::null_mut(),
        )
    };
    if tap.is_null() {
        set_startup_failed(
            &state,
            "macos input event tap unavailable: accessibility permission is required".to_string(),
        );
        return;
    }
    MACOS_TAP_PORT.store(tap as usize, Ordering::Release);

    // create one run-loop source for the event tap
    let source =
        unsafe { CFMachPortCreateRunLoopSource(std::ptr::null(), tap, KCF_RUN_LOOP_SOURCE_ORDER) };
    if source.is_null() {
        MACOS_TAP_PORT.store(0, Ordering::Release);
        unsafe {
            CFRelease(tap.cast::<libc::c_void>());
        }
        set_startup_failed(
            &state,
            "macos input run-loop source creation failed".to_string(),
        );
        return;
    }

    // attach and enable the event tap in the worker run loop
    let run_loop = unsafe { CFRunLoopGetCurrent() };
    unsafe {
        CFRunLoopAddSource(run_loop, source, kCFRunLoopDefaultMode);
        CGEventTapEnable(tap, true);
    }
    set_startup_ready(&state);

    // run the event loop until process shutdown
    unsafe {
        CFRunLoopRun();
        MACOS_TAP_PORT.store(0, Ordering::Release);
        CFRelease(source.cast::<libc::c_void>());
        CFRelease(tap.cast::<libc::c_void>());
    }
}

/// Mark event-tap startup as ready and wake waiters.
fn set_startup_ready(state: &MacosTapState) {
    let mut queues = state.queues.lock();
    queues.startup = MacosTapStartupState::Ready;
    state.wake.notify_all();
}

/// Mark event-tap startup as failed and wake waiters.
fn set_startup_failed(state: &MacosTapState, message: String) {
    let mut queues = state.queues.lock();
    queues.startup = MacosTapStartupState::Failed(message);
    state.wake.notify_all();
}

/// Enqueue one packet to all active subscriber queues.
fn enqueue_packet_to_subscribers(packet: MacosTapPacket) {
    let state = macos_tap_state();
    let mut queues = state.queues.lock();
    for queue in queues.subscriptions.values_mut() {
        if queue.len() >= MACOS_EVENT_QUEUE_LIMIT {
            queue.pop_front();
        }
        queue.push_back(packet.clone());
    }
    state.wake.notify_all();
}

/// Map one CoreGraphics event into one queued packet when supported.
fn map_tap_event(event_type: u32, event: CGEventRef) -> Option<MacosTapPacket> {
    let timestamp_ns = input_core::monotonic_timestamp_ns();
    let point = unsafe { CGEventGetLocation(event) };
    let modifiers = unsafe { CGEventGetFlags(event) as u32 };

    match event_type {
        KCG_EVENT_KEY_DOWN | KCG_EVENT_KEY_UP => {
            let key_code =
                unsafe { CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_KEYCODE) };
            if key_code < 0 {
                return None;
            }

            let is_key_down = event_type == KCG_EVENT_KEY_DOWN;
            let is_repeat = is_key_down
                && unsafe { CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_AUTOREPEAT) }
                    != 0;
            let action = if !is_key_down {
                InputEventAction::Release
            } else if is_repeat {
                InputEventAction::Repeat
            } else {
                InputEventAction::Press
            };
            let value = if !is_key_down {
                0
            } else if is_repeat {
                2
            } else {
                1
            };

            Some(MacosTapPacket {
                timestamp_ns,
                kind: InputEventKind::Key,
                action,
                code: key_code as u32,
                scan_code: key_code as u32,
                value,
                x: point.x,
                y: point.y,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers,
                repeat: is_repeat,
            })
        }
        KCG_EVENT_LEFT_MOUSE_DOWN
        | KCG_EVENT_LEFT_MOUSE_UP
        | KCG_EVENT_RIGHT_MOUSE_DOWN
        | KCG_EVENT_RIGHT_MOUSE_UP
        | KCG_EVENT_OTHER_MOUSE_DOWN
        | KCG_EVENT_OTHER_MOUSE_UP => {
            let action = if event_type == KCG_EVENT_LEFT_MOUSE_UP
                || event_type == KCG_EVENT_RIGHT_MOUSE_UP
                || event_type == KCG_EVENT_OTHER_MOUSE_UP
            {
                InputEventAction::Release
            } else {
                InputEventAction::Press
            };

            let button = match event_type {
                KCG_EVENT_LEFT_MOUSE_DOWN | KCG_EVENT_LEFT_MOUSE_UP => 0,
                KCG_EVENT_RIGHT_MOUSE_DOWN | KCG_EVENT_RIGHT_MOUSE_UP => 1,
                _ => {
                    let raw_button = unsafe {
                        CGEventGetIntegerValueField(event, KCG_MOUSE_EVENT_BUTTON_NUMBER)
                    };
                    if raw_button < 0 { 0 } else { raw_button as u32 }
                }
            };

            Some(MacosTapPacket {
                timestamp_ns,
                kind: InputEventKind::PointerButton,
                action,
                code: button,
                scan_code: button,
                value: if action == InputEventAction::Press {
                    1
                } else {
                    0
                },
                x: point.x,
                y: point.y,
                wheel_x: 0.0,
                wheel_y: 0.0,
                modifiers,
                repeat: false,
            })
        }
        KCG_EVENT_MOUSE_MOVED
        | KCG_EVENT_LEFT_MOUSE_DRAGGED
        | KCG_EVENT_RIGHT_MOUSE_DRAGGED
        | KCG_EVENT_OTHER_MOUSE_DRAGGED => Some(MacosTapPacket {
            timestamp_ns,
            kind: InputEventKind::PointerMotion,
            action: InputEventAction::Move,
            code: event_type,
            scan_code: event_type,
            value: 0,
            x: point.x,
            y: point.y,
            wheel_x: 0.0,
            wheel_y: 0.0,
            modifiers,
            repeat: false,
        }),
        KCG_EVENT_SCROLL_WHEEL => {
            let wheel_y =
                unsafe { CGEventGetIntegerValueField(event, KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS1) };
            let wheel_x =
                unsafe { CGEventGetIntegerValueField(event, KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS2) };
            let value = if wheel_y != 0 { wheel_y } else { wheel_x };

            Some(MacosTapPacket {
                timestamp_ns,
                kind: InputEventKind::Scroll,
                action: InputEventAction::Scroll,
                code: event_type,
                scan_code: event_type,
                value,
                x: point.x,
                y: point.y,
                wheel_x: wheel_x as f64,
                wheel_y: wheel_y as f64,
                modifiers,
                repeat: false,
            })
        }
        KCG_EVENT_FLAGS_CHANGED => Some(MacosTapPacket {
            timestamp_ns,
            kind: InputEventKind::Device,
            action: InputEventAction::Move,
            code: event_type,
            scan_code: event_type,
            value: modifiers as i64,
            x: point.x,
            y: point.y,
            wheel_x: 0.0,
            wheel_y: 0.0,
            modifiers,
            repeat: false,
        }),
        _ => None,
    }
}

/// Receive one event-tap callback and enqueue supported event packets.
unsafe extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    _user_info: *mut libc::c_void,
) -> CGEventRef {
    // drop disabled notifications and pass through original event
    if event_type == KCG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == KCG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        let tap = MACOS_TAP_PORT.load(Ordering::Acquire) as CFMachPortRef;
        if !tap.is_null() {
            unsafe {
                CGEventTapEnable(tap, true);
            }
        }
        return event;
    }

    // map supported events and enqueue for subscribers
    if let Some(packet) = map_tap_event(event_type, event) {
        enqueue_packet_to_subscribers(packet);
    }

    event
}

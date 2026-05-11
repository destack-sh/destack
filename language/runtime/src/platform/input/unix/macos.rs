use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use parking_lot::{Condvar, Mutex};

use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputCoordinateSpace, InputDeviceCapabilities,
    InputDeviceCapabilityKind, InputEvent, InputEventAction, InputKeyEventPayload,
    InputKeyLocation, InputKeyboardState, InputModifierState, InputPointerButtonEventPayload,
    InputPointerMotionEventPayload, InputPointerState, InputPointerType, InputReadMode,
    InputScrollEventPayload, InputWheelDeltaMode,
};
use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, resource};
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy, start_with_policy};

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

/// CoreGraphics keycode for left command.
const KCG_KEYCODE_LEFT_COMMAND: u32 = 55;
/// CoreGraphics keycode for right command.
const KCG_KEYCODE_RIGHT_COMMAND: u32 = 54;
/// CoreGraphics keycode for left shift.
const KCG_KEYCODE_LEFT_SHIFT: u32 = 56;
/// CoreGraphics keycode for right shift.
const KCG_KEYCODE_RIGHT_SHIFT: u32 = 60;
/// CoreGraphics keycode for left option.
const KCG_KEYCODE_LEFT_OPTION: u32 = 58;
/// CoreGraphics keycode for right option.
const KCG_KEYCODE_RIGHT_OPTION: u32 = 61;
/// CoreGraphics keycode for left control.
const KCG_KEYCODE_LEFT_CONTROL: u32 = 59;
/// CoreGraphics keycode for right control.
const KCG_KEYCODE_RIGHT_CONTROL: u32 = 62;
/// CoreGraphics keycode for caps lock.
const KCG_KEYCODE_CAPS_LOCK: u32 = 57;
/// CoreGraphics keycode for function modifier.
const KCG_KEYCODE_FUNCTION: u32 = 63;

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

/// CoreGraphics modifier flag for caps-lock state.
const KCG_EVENT_FLAG_MASK_CAPS_LOCK: u64 = 1 << 16;
/// CoreGraphics modifier flag for shift state.
const KCG_EVENT_FLAG_MASK_SHIFT: u64 = 1 << 17;
/// CoreGraphics modifier flag for control state.
const KCG_EVENT_FLAG_MASK_CONTROL: u64 = 1 << 18;
/// CoreGraphics modifier flag for option state.
const KCG_EVENT_FLAG_MASK_OPTION: u64 = 1 << 19;
/// CoreGraphics modifier flag for command state.
const KCG_EVENT_FLAG_MASK_COMMAND: u64 = 1 << 20;
/// CoreGraphics modifier flag for function-key state.
const KCG_EVENT_FLAG_MASK_FUNCTION: u64 = 1 << 23;

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
/// CoreGraphics event-source reference type.
type CGEventSourceRef = *const libc::c_void;
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
    /// Return one modifier-flags bitset for one event-source state.
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
    /// Return whether one key is currently pressed for one event-source state.
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
    /// Return whether one mouse button is currently pressed for one event-source state.
    fn CGEventSourceButtonState(state_id: i32, button: u32) -> bool;
    /// Warp one global cursor position to one absolute display-space point.
    fn CGWarpMouseCursorPosition(new_cursor_position: CGPoint) -> i32;
    /// Create one synthetic event from one event source.
    fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
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
    kind: input_core::UnixInputEventKind,
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
    /// Current pointer-button bitset.
    buttons: u32,
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
    /// Current pressed pointer-button bitset.
    pointer_buttons: u32,
}

impl MacosTapQueues {
    /// Build one empty event-tap queue state.
    fn new() -> Self {
        Self {
            startup: MacosTapStartupState::Pending,
            next_subscription_id: 1,
            subscriptions: HashMap::new(),
            pointer_buttons: 0,
        }
    }
}

/// Shared synchronization state for the macOS event tap service.
#[derive(Debug)]
pub(crate) struct MacosTapState {
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
/// CoreGraphics state id for combined session input state.
const KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE: i32 = 0;
/// Runtime modifier bit for shift.
const MODIFIER_SHIFT: u32 = 1 << 0;
/// Runtime modifier bit for control.
const MODIFIER_CONTROL: u32 = 1 << 1;
/// Runtime modifier bit for alt.
const MODIFIER_ALT: u32 = 1 << 2;
/// Runtime modifier bit for meta.
const MODIFIER_META: u32 = 1 << 3;
/// Runtime modifier bit for caps lock.
const MODIFIER_CAPS_LOCK: u32 = 1 << 4;
/// Runtime pointer button bit for left.
const POINTER_BUTTON_LEFT: u32 = 1u32 << 0;
/// Runtime pointer button bit for right.
const POINTER_BUTTON_RIGHT: u32 = 1u32 << 1;
/// Runtime pointer button bit for middle.
const POINTER_BUTTON_MIDDLE: u32 = 1u32 << 2;
/// Runtime pointer button bit for x1.
const POINTER_BUTTON_X1: u32 = 1u32 << 3;
/// Runtime pointer button bit for x2.
const POINTER_BUTTON_X2: u32 = 1u32 << 4;

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
        start_with_policy(
            "destack-input-macos-tap",
            "destack.input.event.open",
            ExecutionPolicy::process(ExecutionMode::Loop),
            move || run_event_tap_worker(state),
        )
        .map(|_handle| ())
        .map_err(|error| error.to_string())
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
fn packet_to_input_event(binding: &BindingCallContext, packet: MacosTapPacket) -> InputEvent {
    let mut payload = input_core::empty_unix_event_payload(binding);
    match packet.kind {
        input_core::UnixInputEventKind::Key => {
            payload.key = InputKeyEventPayload {
                action: packet.action,
                key: None,
                code: None,
                location: InputKeyLocation::Standard,
                backend_code: packet.code,
                backend_scan_code: packet.scan_code,
                backend_value: packet.value,
                backend_modifiers: packet.modifiers,
                modifier_state: modifier_state_from_runtime_modifiers(packet.modifiers),
                repeat: packet.repeat,
                is_composing: false,
            };
        }
        input_core::UnixInputEventKind::PointerMotion => {
            payload.pointer_motion = InputPointerMotionEventPayload {
                pointer_id: 0,
                pointer_type: InputPointerType::Mouse,
                is_primary: true,
                coordinate_space: InputCoordinateSpace::GlobalLogical,
                position_x: packet.x,
                position_y: packet.y,
                delta_x: 0.0,
                delta_y: 0.0,
                buttons: packet.buttons,
                backend_buttons: packet.buttons,
                modifier_state: modifier_state_from_runtime_modifiers(packet.modifiers),
                contact: None,
                pen: None,
                coalesced_samples: binding.store_slice(Vec::new()),
                predicted_samples: binding.store_slice(Vec::new()),
            };
        }
        input_core::UnixInputEventKind::PointerButton => {
            payload.pointer_button = InputPointerButtonEventPayload {
                action: packet.action,
                pointer_id: 0,
                pointer_type: InputPointerType::Mouse,
                is_primary: true,
                button: packet.code as i16,
                buttons: packet.buttons,
                backend_code: packet.code,
                backend_value: packet.value,
                position_x: packet.x,
                position_y: packet.y,
                coordinate_space: InputCoordinateSpace::GlobalLogical,
                modifier_state: modifier_state_from_runtime_modifiers(packet.modifiers),
                contact: None,
                pen: None,
            };
        }
        input_core::UnixInputEventKind::Scroll => {
            payload.scroll = InputScrollEventPayload {
                pointer_id: Some(0),
                pointer_type: Some(InputPointerType::Mouse),
                delta_mode: InputWheelDeltaMode::Pixel,
                delta_x: packet.wheel_x,
                delta_y: packet.wheel_y,
                position_x: packet.x,
                position_y: packet.y,
                coordinate_space: InputCoordinateSpace::GlobalLogical,
                buttons: packet.buttons,
                modifier_state: modifier_state_from_runtime_modifiers(packet.modifiers),
            };
        }
        _ => {}
    }

    input_core::build_unix_input_event(
        binding,
        packet.kind,
        packet.timestamp_ns,
        0,
        MACOS_INPUT_SESSION_ID,
        payload,
    )
}

/// Return one CoreGraphics flag-mask bit for one modifier-key keycode.
fn modifier_flag_mask_for_keycode(key_code: u32) -> Option<u64> {
    match key_code {
        KCG_KEYCODE_LEFT_SHIFT | KCG_KEYCODE_RIGHT_SHIFT => Some(KCG_EVENT_FLAG_MASK_SHIFT),
        KCG_KEYCODE_LEFT_CONTROL | KCG_KEYCODE_RIGHT_CONTROL => Some(KCG_EVENT_FLAG_MASK_CONTROL),
        KCG_KEYCODE_LEFT_OPTION | KCG_KEYCODE_RIGHT_OPTION => Some(KCG_EVENT_FLAG_MASK_OPTION),
        KCG_KEYCODE_LEFT_COMMAND | KCG_KEYCODE_RIGHT_COMMAND => Some(KCG_EVENT_FLAG_MASK_COMMAND),
        KCG_KEYCODE_CAPS_LOCK => Some(KCG_EVENT_FLAG_MASK_CAPS_LOCK),
        KCG_KEYCODE_FUNCTION => Some(KCG_EVENT_FLAG_MASK_FUNCTION),
        _ => None,
    }
}

/// Decode one flags-changed packet into key-action semantics.
fn flags_changed_action_and_value(
    key_code: u32,
    modifiers: u64,
) -> Option<(InputEventAction, i64)> {
    let mask = modifier_flag_mask_for_keycode(key_code)?;
    if (modifiers & mask) != 0 {
        return Some((InputEventAction::Press, 1));
    }

    Some((InputEventAction::Release, 0))
}

/// Return one capability payload for the macOS global-session backend.
pub(super) fn query_macos_session_capabilities(
    binding: &BindingCallContext,
) -> InputDeviceCapabilities {
    // expose keyboard and pointer lanes from the global event-tap stream
    let kinds = vec![
        InputDeviceCapabilityKind::Keyboard,
        InputDeviceCapabilityKind::Pointer,
    ];

    // expose pointer axes and wheel lanes for cursor and scroll semantics
    let axes = vec![
        InputAxisMetadata {
            code: 0,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: 1,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS1 as u32,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: KCG_SCROLL_WHEEL_EVENT_DELTA_AXIS2 as u32,
            minimum: 0.0,
            maximum: 0.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
    ];

    // expose primary pointer-button lanes used by the session event tap
    let buttons = vec![
        InputButtonMetadata {
            code: 0,
            analog: false,
        },
        InputButtonMetadata {
            code: 1,
            analog: false,
        },
        InputButtonMetadata {
            code: 2,
            analog: false,
        },
        InputButtonMetadata {
            code: 3,
            analog: false,
        },
        InputButtonMetadata {
            code: 4,
            analog: false,
        },
    ];

    InputDeviceCapabilities {
        kinds: binding.store_array(kinds),
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        metadata_origin: InputCapabilityMetadataOrigin::Mixed,
        axis_metadata_fidelity: InputCapabilityMetadataFidelity::Partial,
        button_metadata_fidelity: InputCapabilityMetadataFidelity::Partial,
        supports_relative_pointer: true,
        supports_pointer_grab: false,
        supports_pointer_capture: false,
        supports_pointer_warp: true,
        supports_text_input: false,
        supports_composition: false,
        supports_edit_intents: false,
        supports_rumble: false,
        supports_trigger_rumble: false,
        supports_sensors: false,
        supports_battery_state: false,
        supports_light_control: false,
        supports_raw_hid: false,
        supports_player_index: false,
        supports_pointer_coalescing: false,
        supports_pointer_prediction: false,
        supports_pen_hover_distance: false,
        supports_pen_orientation_angles: false,
    }
}

/// Decode runtime modifier bits from one CoreGraphics modifier-flag payload.
fn runtime_modifiers_from_cg_flags(flags: u64) -> u32 {
    let mut modifiers = 0u32;
    if (flags & KCG_EVENT_FLAG_MASK_SHIFT) != 0 {
        modifiers |= MODIFIER_SHIFT;
    }
    if (flags & KCG_EVENT_FLAG_MASK_CONTROL) != 0 {
        modifiers |= MODIFIER_CONTROL;
    }
    if (flags & KCG_EVENT_FLAG_MASK_OPTION) != 0 {
        modifiers |= MODIFIER_ALT;
    }
    if (flags & KCG_EVENT_FLAG_MASK_COMMAND) != 0 {
        modifiers |= MODIFIER_META;
    }
    if (flags & KCG_EVENT_FLAG_MASK_CAPS_LOCK) != 0 {
        modifiers |= MODIFIER_CAPS_LOCK;
    }

    modifiers
}

/// Build one normalized modifier-state payload from runtime modifier bits.
fn modifier_state_from_runtime_modifiers(modifiers: u32) -> InputModifierState {
    InputModifierState {
        is_alt: (modifiers & MODIFIER_ALT) != 0,
        is_alt_graph: false,
        is_caps_lock: (modifiers & MODIFIER_CAPS_LOCK) != 0,
        is_control: (modifiers & MODIFIER_CONTROL) != 0,
        is_fn: false,
        is_fn_lock: false,
        is_meta: (modifiers & MODIFIER_META) != 0,
        is_num_lock: false,
        is_scroll_lock: false,
        is_shift: (modifiers & MODIFIER_SHIFT) != 0,
        is_symbol: false,
        is_symbol_lock: false,
    }
}

/// Build one stable pointer-button bitset from CoreGraphics button state.
fn pointer_buttons_from_event_source_state() -> u32 {
    let mut buttons = 0u32;
    if unsafe { CGEventSourceButtonState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, 0) } {
        buttons |= POINTER_BUTTON_LEFT;
    }
    if unsafe { CGEventSourceButtonState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, 1) } {
        buttons |= POINTER_BUTTON_RIGHT;
    }
    if unsafe { CGEventSourceButtonState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, 2) } {
        buttons |= POINTER_BUTTON_MIDDLE;
    }
    if unsafe { CGEventSourceButtonState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, 3) } {
        buttons |= POINTER_BUTTON_X1;
    }
    if unsafe { CGEventSourceButtonState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, 4) } {
        buttons |= POINTER_BUTTON_X2;
    }

    buttons
}

/// Return one current cursor position from one synthetic CoreGraphics event.
fn current_pointer_position(operation: &'static str) -> RuntimeResult<(f64, f64)> {
    let event = unsafe { CGEventCreate(std::ptr::null::<libc::c_void>()) };
    if event.is_null() {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some(operation.to_string()),
            None,
            "failed to sample current macos pointer position".to_string(),
        ))
        .boxed());
    }

    let point = unsafe { CGEventGetLocation(event) };
    unsafe {
        CFRelease(event.cast::<libc::c_void>());
    }

    Ok((point.x, point.y))
}

/// Read one host keyboard snapshot from macOS global event-source state.
pub(super) fn keyboard_state_snapshot(
    binding: &BindingCallContext,
    sequence: u64,
    device_id: &str,
) -> RuntimeResult<InputKeyboardState> {
    let mut pressed_codes = Vec::new();
    for code in 0u16..=255u16 {
        if unsafe { CGEventSourceKeyState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE, code) } {
            pressed_codes.push(code as u32);
        }
    }

    let flags = unsafe { CGEventSourceFlagsState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE) };
    let modifiers = runtime_modifiers_from_cg_flags(flags);

    Ok(InputKeyboardState {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence,
        device_id: binding.store_string(device_id),
        backend_modifiers: modifiers,
        modifier_state: modifier_state_from_runtime_modifiers(modifiers),
        pressed_codes: binding.store_array(Vec::new()),
        pressed_keys: binding.store_array(Vec::new()),
        pressed_backend_codes: binding.store_array(pressed_codes.clone()),
        pressed_scan_codes: binding.store_array(pressed_codes),
        layout: None,
        is_composing: false,
    })
}

/// Read one host pointer snapshot from macOS global event-source state.
pub(super) fn pointer_state_snapshot(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<InputPointerState> {
    let (x, y) = current_pointer_position(operation)?;
    let flags = unsafe { CGEventSourceFlagsState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE) };
    let modifiers = runtime_modifiers_from_cg_flags(flags);
    let buttons = pointer_buttons_from_event_source_state();

    Ok(InputPointerState {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence: 0,
        device_id: binding.store_string(MACOS_INPUT_SESSION_ID),
        pointer_id: 0,
        pointer_type: InputPointerType::Mouse,
        is_primary: true,
        coordinate_space: InputCoordinateSpace::GlobalLogical,
        x,
        y,
        delta_x: 0.0,
        delta_y: 0.0,
        buttons,
        modifier_state: modifier_state_from_runtime_modifiers(modifiers),
        pen: None,
        contact: None,
    })
}

/// Warp one global macOS cursor to one absolute display-space position.
pub(super) fn warp_pointer_position(x: f64, y: f64, operation: &'static str) -> RuntimeResult<()> {
    // validate finite warp coordinates before host calls
    if !x.is_finite() || !y.is_finite() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "x",
            "x and y must be finite",
        ))
        .boxed());
    }

    // apply one global cursor warp through coregraphics
    let status = unsafe {
        CGWarpMouseCursorPosition(CGPoint {
            x: x.round(),
            y: y.round(),
        })
    };
    if status != 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoPermissionDenied),
            None,
            Some(status),
            Some(operation.to_string()),
            None,
            "failed to warp macos pointer position".to_string(),
        ))
        .boxed());
    }

    Ok(())
}

/// Resolve or allocate one subscription id for one opened handle.
fn resolve_subscription_id(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let subscription = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<input_core::UnixInputBinding>())?;
            if resolved_binding.backend != input_core::UnixInputBackend::Platform {
                return None;
            }

            let state = resolved_binding.macos_state.as_mut()?;
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    _read_mode: InputReadMode,
) -> RuntimeResult<InputEvent> {
    let subscription_id = resolve_subscription_id(binding, handle, "destack.input.event.read")?;

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
        return Ok(packet_to_input_event(binding, packet));
    }

    let Some(packet) = wait_pop_subscription_event(subscription_id)? else {
        return Err(input_core::input_not_found(
            "destack.input.event.read",
            handle,
        ));
    };
    Ok(packet_to_input_event(binding, packet))
}

/// Release one macOS session subscription for one input handle.
pub(super) fn release_macos_session_subscription(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) {
    let subscription = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputDevice {
                return None;
            }

            if entry.label.as_deref() != Some(input_core::INPUT_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<input_core::UnixInputBinding>())?;
            if resolved_binding.backend != input_core::UnixInputBackend::Platform {
                return None;
            }

            let state = resolved_binding.macos_state.as_mut()?;
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
fn enqueue_packet_to_subscribers(queues: &mut MacosTapQueues, packet: &MacosTapPacket) {
    for queue in queues.subscriptions.values_mut() {
        if queue.len() >= MACOS_EVENT_QUEUE_LIMIT {
            queue.pop_front();
        }
        queue.push_back(packet.clone());
    }
}

/// Return one pointer-button mask for one backend pointer button code.
fn pointer_button_mask(code: u32) -> u32 {
    if code >= 32 {
        return 0;
    }

    1u32 << code
}

/// Update and stamp pointer-button state for one queued packet.
fn stamp_pointer_button_state(queues: &mut MacosTapQueues, packet: &mut MacosTapPacket) {
    // update persistent pressed state from pointer-button transitions
    if packet.kind == input_core::UnixInputEventKind::PointerButton {
        let mask = pointer_button_mask(packet.code);
        if packet.action == InputEventAction::Press {
            queues.pointer_buttons |= mask;
        } else if packet.action == InputEventAction::Release {
            queues.pointer_buttons &= !mask;
        }

        packet.buttons = queues.pointer_buttons;
        return;
    }

    // stamp current pressed-state context onto pointer motion and scroll packets
    if packet.kind == input_core::UnixInputEventKind::PointerMotion
        || packet.kind == input_core::UnixInputEventKind::Scroll
    {
        // keep packet button state synchronized with host state snapshots
        let buttons = pointer_buttons_from_event_source_state();
        queues.pointer_buttons = buttons;
        packet.buttons = buttons;
    }
}

/// Map one CoreGraphics event into one queued packet when supported.
fn map_tap_event(event_type: u32, event: CGEventRef) -> Option<MacosTapPacket> {
    let timestamp_ns = input_core::monotonic_timestamp_ns();
    let point = unsafe { CGEventGetLocation(event) };
    let raw_flags = unsafe { CGEventGetFlags(event) };
    let modifiers = runtime_modifiers_from_cg_flags(raw_flags);

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
                kind: input_core::UnixInputEventKind::Key,
                action,
                code: key_code as u32,
                scan_code: key_code as u32,
                value,
                x: point.x,
                y: point.y,
                wheel_x: 0.0,
                wheel_y: 0.0,
                buttons: 0,
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
                kind: input_core::UnixInputEventKind::PointerButton,
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
                buttons: 0,
                modifiers,
                repeat: false,
            })
        }
        KCG_EVENT_MOUSE_MOVED
        | KCG_EVENT_LEFT_MOUSE_DRAGGED
        | KCG_EVENT_RIGHT_MOUSE_DRAGGED
        | KCG_EVENT_OTHER_MOUSE_DRAGGED => Some(MacosTapPacket {
            timestamp_ns,
            kind: input_core::UnixInputEventKind::PointerMotion,
            action: InputEventAction::Move,
            code: event_type,
            scan_code: event_type,
            value: 0,
            x: point.x,
            y: point.y,
            wheel_x: 0.0,
            wheel_y: 0.0,
            buttons: 0,
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
                kind: input_core::UnixInputEventKind::Scroll,
                action: InputEventAction::Scroll,
                code: event_type,
                scan_code: event_type,
                value,
                x: point.x,
                y: point.y,
                wheel_x: wheel_x as f64,
                wheel_y: wheel_y as f64,
                buttons: 0,
                modifiers,
                repeat: false,
            })
        }
        KCG_EVENT_FLAGS_CHANGED => {
            let key_code =
                unsafe { CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_KEYCODE) };
            if key_code < 0 {
                return None;
            }

            let key_code = key_code as u32;
            let (action, value) = flags_changed_action_and_value(key_code, raw_flags)?;

            Some(MacosTapPacket {
                timestamp_ns,
                kind: input_core::UnixInputEventKind::Key,
                action,
                code: key_code,
                scan_code: key_code,
                value,
                x: point.x,
                y: point.y,
                wheel_x: 0.0,
                wheel_y: 0.0,
                buttons: 0,
                modifiers,
                repeat: false,
            })
        }
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
    if let Some(mut packet) = map_tap_event(event_type, event) {
        let state = macos_tap_state();
        let mut queues = state.queues.lock();
        stamp_pointer_button_state(&mut queues, &mut packet);
        enqueue_packet_to_subscribers(&mut queues, &packet);
        state.wake.notify_all();
    }

    event
}

#[cfg(test)]
mod tests {
    use crate::platform::input::InputEventAction;

    use super::{
        KCG_EVENT_FLAG_MASK_CAPS_LOCK, KCG_EVENT_FLAG_MASK_COMMAND, KCG_EVENT_FLAG_MASK_CONTROL,
        KCG_EVENT_FLAG_MASK_OPTION, KCG_EVENT_FLAG_MASK_SHIFT, KCG_KEYCODE_LEFT_CONTROL,
        KCG_KEYCODE_LEFT_SHIFT, KCG_KEYCODE_RIGHT_SHIFT, MODIFIER_ALT, MODIFIER_CAPS_LOCK,
        MODIFIER_CONTROL, MODIFIER_META, MODIFIER_SHIFT, flags_changed_action_and_value,
        modifier_flag_mask_for_keycode, runtime_modifiers_from_cg_flags,
    };

    /// Map shift keycodes to the shift modifier flag.
    #[test]
    fn test_modifier_flag_mask_for_keycode_maps_shift_variants() {
        assert_eq!(
            modifier_flag_mask_for_keycode(KCG_KEYCODE_LEFT_SHIFT),
            Some(KCG_EVENT_FLAG_MASK_SHIFT)
        );
        assert_eq!(
            modifier_flag_mask_for_keycode(KCG_KEYCODE_RIGHT_SHIFT),
            Some(KCG_EVENT_FLAG_MASK_SHIFT)
        );
    }

    /// Decode flags-changed packets into press and release actions.
    #[test]
    fn test_flags_changed_action_and_value_maps_press_and_release() {
        let press =
            flags_changed_action_and_value(KCG_KEYCODE_LEFT_CONTROL, KCG_EVENT_FLAG_MASK_CONTROL);
        assert_eq!(press, Some((InputEventAction::Press, 1)));

        let release = flags_changed_action_and_value(KCG_KEYCODE_LEFT_CONTROL, 0);
        assert_eq!(release, Some((InputEventAction::Release, 0)));
    }

    /// Reject unknown keycodes in flags-changed mapping.
    #[test]
    fn test_flags_changed_action_and_value_rejects_unknown_keycode() {
        assert_eq!(flags_changed_action_and_value(999, 0), None);
    }

    /// Normalize coregraphics modifier flags into runtime modifier bits.
    #[test]
    fn test_runtime_modifiers_from_cg_flags_normalizes_expected_bits() {
        let flags = KCG_EVENT_FLAG_MASK_SHIFT
            | KCG_EVENT_FLAG_MASK_CONTROL
            | KCG_EVENT_FLAG_MASK_OPTION
            | KCG_EVENT_FLAG_MASK_COMMAND
            | KCG_EVENT_FLAG_MASK_CAPS_LOCK;
        let modifiers = runtime_modifiers_from_cg_flags(flags);

        assert_eq!(modifiers & MODIFIER_SHIFT, MODIFIER_SHIFT);
        assert_eq!(modifiers & MODIFIER_CONTROL, MODIFIER_CONTROL);
        assert_eq!(modifiers & MODIFIER_ALT, MODIFIER_ALT);
        assert_eq!(modifiers & MODIFIER_META, MODIFIER_META);
        assert_eq!(modifiers & MODIFIER_CAPS_LOCK, MODIFIER_CAPS_LOCK);
    }
}

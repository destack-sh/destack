use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;
use std::path::Path;

use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputCompositionEventPayload, InputDeviceCapabilities,
    InputDeviceCapabilityKind, InputDeviceDescriptor, InputDeviceEventPayload, InputDeviceKind,
    InputEvent, InputEventAction, InputEventKind, InputGamepadBatteryState,
    InputGamepadBatteryStatus, InputGamepadButtonState, InputGamepadConnectionType,
    InputGamepadEventPayload, InputGamepadMappingType, InputGamepadState, InputGamepadTouchState,
    InputHapticEffectParameters, InputHapticEffectType, InputKeyEventPayload, InputKeyboardState,
    InputPenState, InputPointerButtonEventPayload, InputPointerMotionEventPayload,
    InputPointerState, InputScrollEventPayload, InputSensorDescriptor, InputSensorEventPayload,
    InputSensorKind, InputSensorSample, InputTextEventPayload, InputTouchContactPhase,
    InputTouchContactState, InputTouchEventPayload, InputTouchState,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Linux input-device directory path.
pub(super) const INPUT_DEVICE_DIRECTORY: &str = "/dev/input";
/// Prefix for Linux evdev event nodes.
pub(super) const INPUT_EVENT_PREFIX: &str = "event";
/// Linux hidraw root directory path.
const INPUT_HIDRAW_DIRECTORY: &str = "/dev";
/// Prefix for Linux hidraw nodes.
const INPUT_HIDRAW_PREFIX: &str = "hidraw";
/// Prefix for Linux runtime evdev identifiers.
const LINUX_EVDEV_ID_PREFIX: &str = "linux:evdev:";
/// Prefix for Linux runtime hidraw identifiers.
const LINUX_HIDRAW_ID_PREFIX: &str = "linux:hidraw:";
/// Linux EV_SYN event kind.
const EV_SYN: u16 = 0x00;
/// Linux EV_KEY event kind.
const EV_KEY: u16 = 0x01;
/// Linux EV_REL event kind.
const EV_REL: u16 = 0x02;
/// Linux EV_ABS event kind.
const EV_ABS: u16 = 0x03;
/// Linux EV_FF event kind.
const EV_FF: u16 = 0x15;
/// Linux REL_X event code.
const REL_X: u16 = 0x00;
/// Linux REL_Y event code.
const REL_Y: u16 = 0x01;
/// Linux REL_WHEEL event code.
const REL_WHEEL: u16 = 0x08;
/// Linux REL_HWHEEL event code.
const REL_HWHEEL: u16 = 0x06;
/// Linux ABS_X event code.
const ABS_X: u16 = 0x00;
/// Linux ABS_Y event code.
const ABS_Y: u16 = 0x01;
/// Linux ABS_Z event code.
const ABS_Z: u16 = 0x02;
/// Linux ABS_RX event code.
const ABS_RX: u16 = 0x03;
/// Linux ABS_RY event code.
const ABS_RY: u16 = 0x04;
/// Linux ABS_RZ event code.
const ABS_RZ: u16 = 0x05;
/// Linux ABS_THROTTLE event code.
const ABS_THROTTLE: u16 = 0x06;
/// Linux ABS_RUDDER event code.
const ABS_RUDDER: u16 = 0x07;
/// Linux ABS_WHEEL event code.
const ABS_WHEEL: u16 = 0x08;
/// Linux ABS_GAS event code.
const ABS_GAS: u16 = 0x09;
/// Linux ABS_BRAKE event code.
const ABS_BRAKE: u16 = 0x0a;
/// Linux ABS_HAT0X event code.
const ABS_HAT0X: u16 = 0x10;
/// Linux ABS_HAT0Y event code.
const ABS_HAT0Y: u16 = 0x11;
/// Linux ABS_HAT1X event code.
const ABS_HAT1X: u16 = 0x12;
/// Linux ABS_HAT1Y event code.
const ABS_HAT1Y: u16 = 0x13;
/// Linux ABS_HAT2X event code.
const ABS_HAT2X: u16 = 0x14;
/// Linux ABS_HAT2Y event code.
const ABS_HAT2Y: u16 = 0x15;
/// Linux ABS_HAT3X event code.
const ABS_HAT3X: u16 = 0x16;
/// Linux ABS_HAT3Y event code.
const ABS_HAT3Y: u16 = 0x17;
/// Linux ABS_PRESSURE event code.
const ABS_PRESSURE: u16 = 0x18;
/// Linux ABS_TILT_X event code.
const ABS_TILT_X: u16 = 0x1a;
/// Linux ABS_TILT_Y event code.
const ABS_TILT_Y: u16 = 0x1b;
/// Linux ABS_MT_SLOT event code.
const ABS_MT_SLOT: u16 = 0x2f;
/// Linux ABS_MT_TRACKING_ID event code.
const ABS_MT_TRACKING_ID: u16 = 0x39;
/// Linux ABS_MT_POSITION_X event code.
const ABS_MT_POSITION_X: u16 = 0x35;
/// Linux ABS_MT_POSITION_Y event code.
const ABS_MT_POSITION_Y: u16 = 0x36;
/// Linux ABS_MT_PRESSURE event code.
const ABS_MT_PRESSURE: u16 = 0x3a;
/// Linux KEY_A key-bit index.
const KEY_A: usize = 30;
/// Linux KEY_LEFTSHIFT key-bit index.
const KEY_LEFTSHIFT: u16 = 42;
/// Linux KEY_RIGHTSHIFT key-bit index.
const KEY_RIGHTSHIFT: u16 = 54;
/// Linux KEY_LEFTCTRL key-bit index.
const KEY_LEFTCTRL: u16 = 29;
/// Linux KEY_RIGHTCTRL key-bit index.
const KEY_RIGHTCTRL: u16 = 97;
/// Linux KEY_LEFTALT key-bit index.
const KEY_LEFTALT: u16 = 56;
/// Linux KEY_RIGHTALT key-bit index.
const KEY_RIGHTALT: u16 = 100;
/// Linux KEY_LEFTMETA key-bit index.
const KEY_LEFTMETA: u16 = 125;
/// Linux KEY_RIGHTMETA key-bit index.
const KEY_RIGHTMETA: u16 = 126;
/// Linux KEY_CAPSLOCK key-bit index.
const KEY_CAPSLOCK: u16 = 58;
/// Linux KEY_NUMLOCK key-bit index.
const KEY_NUMLOCK: u16 = 69;
/// Linux KEY_SCROLLLOCK key-bit index.
const KEY_SCROLLLOCK: u16 = 70;
/// Linux BTN_MOUSE_LEFT key-bit index.
const BTN_MOUSE_LEFT: usize = 0x110;
/// Linux BTN_TOUCH key-bit index.
const BTN_TOUCH: usize = 0x14a;
/// Linux BTN_STYLUS key-bit index.
const BTN_STYLUS: usize = 0x14b;
/// Linux BTN_SOUTH key-bit index.
const BTN_SOUTH: usize = 0x130;
/// Linux BTN_EAST key-bit index.
const BTN_EAST: usize = 0x131;
/// Linux BTN_NORTH key-bit index.
const BTN_NORTH: usize = 0x133;
/// Linux BTN_WEST key-bit index.
const BTN_WEST: usize = 0x134;
/// Linux BTN_TL key-bit index.
const BTN_TL: usize = 0x136;
/// Linux BTN_TR key-bit index.
const BTN_TR: usize = 0x137;
/// Linux BTN_TL2 key-bit index.
const BTN_TL2: usize = 0x138;
/// Linux BTN_TR2 key-bit index.
const BTN_TR2: usize = 0x139;
/// Linux BTN_SELECT key-bit index.
const BTN_SELECT: usize = 0x13a;
/// Linux BTN_START key-bit index.
const BTN_START: usize = 0x13b;
/// Linux BTN_MODE key-bit index.
const BTN_MODE: usize = 0x13c;
/// Linux BTN_THUMBL key-bit index.
const BTN_THUMBL: usize = 0x13d;
/// Linux BTN_THUMBR key-bit index.
const BTN_THUMBR: usize = 0x13e;
/// Linux BTN_DPAD_UP key-bit index.
const BTN_DPAD_UP: usize = 0x220;
/// Linux BTN_DPAD_DOWN key-bit index.
const BTN_DPAD_DOWN: usize = 0x221;
/// Linux BTN_DPAD_LEFT key-bit index.
const BTN_DPAD_LEFT: usize = 0x222;
/// Linux BTN_DPAD_RIGHT key-bit index.
const BTN_DPAD_RIGHT: usize = 0x223;
/// Linux BTN_GAMEPAD key-bit index.
const BTN_GAMEPAD: usize = 0x130;
/// Linux BTN_MISC range start.
const BTN_MISC_START: usize = 0x100;
/// Linux BTN_MISC range end.
const BTN_MISC_END: usize = 0x109;
/// Linux BTN_MOUSE range start.
const BTN_MOUSE_START: usize = 0x110;
/// Linux BTN_MOUSE range end.
const BTN_MOUSE_END: usize = 0x11f;
/// Linux BTN_JOYSTICK range start.
const BTN_JOYSTICK_START: usize = 0x120;
/// Linux BTN_JOYSTICK range end.
const BTN_JOYSTICK_END: usize = 0x12f;
/// Linux BTN_GAMEPAD range start.
const BTN_GAMEPAD_START: usize = 0x130;
/// Linux BTN_GAMEPAD range end.
const BTN_GAMEPAD_END: usize = 0x13f;
/// Linux BTN_DIGI range start.
const BTN_DIGI_START: usize = 0x140;
/// Linux BTN_DIGI range end.
const BTN_DIGI_END: usize = 0x14f;
/// Linux BTN_WHEEL range start.
const BTN_WHEEL_START: usize = 0x150;
/// Linux BTN_WHEEL range end.
const BTN_WHEEL_END: usize = 0x15f;
/// Linux BTN_TRIGGER_HAPPY range start.
const BTN_TRIGGER_HAPPY_START: usize = 0x2c0;
/// Linux BTN_TRIGGER_HAPPY range end.
const BTN_TRIGGER_HAPPY_END: usize = 0x2e7;
/// Buffer size for Linux event capability bitsets.
const MAX_EVENT_BITS: usize = 64;
/// Buffer size for Linux key capability bitsets.
const MAX_KEY_BITS: usize = 256;
/// Buffer size for Linux relative-axis capability bitsets.
const MAX_REL_BITS: usize = 64;
/// Buffer size for Linux absolute-axis capability bitsets.
const MAX_ABS_BITS: usize = 64;
/// Buffer size for Linux force-feedback capability bitsets.
const MAX_FF_BITS: usize = 16;
/// Linux force-feedback rumble effect code.
const FF_RUMBLE: usize = 0x50;
/// Linux EVIOCGID ioctl request number.
const EVIOCGID_REQUEST: libc::c_ulong =
    ior_request(b'E', 0x02, std::mem::size_of::<LinuxInputId>());
/// Linux EVIOCGRAB ioctl request number.
const EVIOCGRAB_REQUEST: libc::c_ulong =
    iow_request(b'E', 0x90, std::mem::size_of::<libc::c_int>());
/// Linux EVIOCSFF ioctl request number.
const EVIOCSFF_REQUEST: libc::c_ulong =
    iow_request(b'E', 0x80, std::mem::size_of::<LinuxFfEffect>());
/// Linux EVIOCRMFF ioctl request number.
const EVIOCRMFF_REQUEST: libc::c_ulong =
    iow_request(b'E', 0x81, std::mem::size_of::<libc::c_int>());
/// Linux force-feedback event-value for start.
const FF_EVENT_START: i32 = 1;
/// Linux force-feedback event-value for stop.
const FF_EVENT_STOP: i32 = 0;
/// Modifier bit: one or more shift keys are active.
const MODIFIER_SHIFT: u32 = 1 << 0;
/// Modifier bit: one or more control keys are active.
const MODIFIER_CONTROL: u32 = 1 << 1;
/// Modifier bit: one or more alt keys are active.
const MODIFIER_ALT: u32 = 1 << 2;
/// Modifier bit: one or more meta keys are active.
const MODIFIER_META: u32 = 1 << 3;
/// Modifier bit: caps-lock is toggled on.
const MODIFIER_CAPS_LOCK: u32 = 1 << 4;
/// Modifier bit: num-lock is toggled on.
const MODIFIER_NUM_LOCK: u32 = 1 << 5;
/// Modifier bit: scroll-lock is toggled on.
const MODIFIER_SCROLL_LOCK: u32 = 1 << 6;

/// Linux input device identity returned by EVIOCGID.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxInputId {
    /// Input bus kind.
    bustype: u16,
    /// Device vendor id.
    vendor: u16,
    /// Device product id.
    product: u16,
    /// Device version id.
    version: u16,
}

/// Linux hidraw device info payload returned by HIDIOCGRAWINFO.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxHidrawDevInfo {
    /// Bus type identifier.
    bustype: u32,
    /// Device vendor id.
    vendor: u16,
    /// Device product id.
    product: u16,
}

/// Linux absolute-axis payload returned by EVIOCGABS.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxInputAbsInfo {
    /// Current axis value.
    value: i32,
    /// Minimum axis value.
    minimum: i32,
    /// Maximum axis value.
    maximum: i32,
    /// Axis fuzz threshold.
    fuzz: i32,
    /// Axis flat threshold.
    flat: i32,
    /// Axis resolution.
    resolution: i32,
}

/// Linux input event payload returned by evdev reads.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct LinuxInputEvent {
    /// Event timestamp.
    time: libc::timeval,
    /// Event kind.
    kind: u16,
    /// Event code.
    code: u16,
    /// Event value.
    value: i32,
}

/// Linux force-feedback trigger payload.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFfTrigger {
    /// Trigger button code.
    button: u16,
    /// Trigger interval.
    interval: u16,
}

/// Linux force-feedback replay payload.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFfReplay {
    /// Replay duration in milliseconds.
    length: u16,
    /// Replay delay in milliseconds.
    delay: u16,
}

/// Linux force-feedback rumble payload.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFfRumbleEffect {
    /// Strong-motor magnitude.
    strong_magnitude: u16,
    /// Weak-motor magnitude.
    weak_magnitude: u16,
}

/// Linux force-feedback effect payload.
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxFfEffect {
    /// Force-feedback effect type.
    type_: u16,
    /// Kernel-managed effect id.
    id: i16,
    /// Direction value for directional effects.
    direction: u16,
    /// Trigger payload.
    trigger: LinuxFfTrigger,
    /// Replay payload.
    replay: LinuxFfReplay,
    /// Union payload bytes for effect-specific parameters.
    #[cfg(target_pointer_width = "64")]
    payload: [u64; 4],
    /// Union payload bytes for effect-specific parameters.
    #[cfg(target_pointer_width = "32")]
    payload: [u32; 7],
}

/// Device metadata collected from Linux evdev descriptors.
#[derive(Clone)]
struct InputDeviceMetadata {
    /// Host-visible device name.
    name: String,
    /// Stable per-instance identifier when available.
    instance_id: String,
    /// Stable hardware identifier when available.
    hardware_id: String,
    /// Classified runtime device kind.
    kind: InputDeviceKind,
    /// Device vendor id.
    vendor_id: u16,
    /// Device product id.
    product_id: u16,
    /// Number of logical keys when reported by the backend.
    key_count: u16,
    /// Number of logical buttons when reported by the backend.
    button_count: u16,
    /// Number of logical axes when reported by the backend.
    axis_count: u16,
    /// Whether this endpoint supports force-feedback rumble.
    supports_rumble: bool,
    /// Whether this endpoint supports raw-hid report lanes.
    supports_raw_hid: bool,
    /// Host transport name for this endpoint.
    transport: &'static str,
}

/// Build one Linux ioctl request number for read-only payloads.
const fn ior_request(type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    ioc_request(2, type_byte, number, size)
}

/// Build one Linux ioctl request number for write payloads.
const fn iow_request(type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    ioc_request(1, type_byte, number, size)
}

/// Build one Linux ioctl request number from direction, type, number, and size.
const fn ioc_request(direction: u8, type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    /// Shift for ioctl number bits.
    const IOC_NR_SHIFT: u64 = 0;
    /// Shift for ioctl type bits.
    const IOC_TYPE_SHIFT: u64 = IOC_NR_SHIFT + 8;
    /// Shift for ioctl size bits.
    const IOC_SIZE_SHIFT: u64 = IOC_TYPE_SHIFT + 8;
    /// Shift for ioctl direction bits.
    const IOC_DIR_SHIFT: u64 = IOC_SIZE_SHIFT + 14;

    (((direction as u64) << IOC_DIR_SHIFT)
        | ((type_byte as u64) << IOC_TYPE_SHIFT)
        | ((number as u64) << IOC_NR_SHIFT)
        | ((size as u64) << IOC_SIZE_SHIFT)) as libc::c_ulong
}

/// Build EVIOCGNAME request number for one buffer length.
fn eviocgname_request(length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x06, length)
}

/// Build EVIOCGPHYS request number for one buffer length.
fn eviocgphys_request(length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x07, length)
}

/// Build EVIOCGUNIQ request number for one buffer length.
fn eviocguniq_request(length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x08, length)
}

/// Build EVIOCGBIT request number for one event kind and buffer length.
fn eviocgbit_request(event: u16, length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x20 + event as u8, length)
}

/// Build EVIOCGABS request number for one absolute-axis code.
fn eviocgabs_request(code: u16) -> libc::c_ulong {
    ior_request(
        b'E',
        0x40 + code as u8,
        std::mem::size_of::<LinuxInputAbsInfo>(),
    )
}

/// Build EVIOCGKEY request number for one key-state bitset length.
fn eviocgkey_request(length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x18, length)
}

/// Build HIDIOCGRAWNAME request number for one buffer length.
fn hidiocgrawname_request(length: usize) -> libc::c_ulong {
    ior_request(b'H', 0x04, length)
}

/// Build HIDIOCGRAWINFO request number.
fn hidiocgrawinfo_request() -> libc::c_ulong {
    ior_request(b'H', 0x03, std::mem::size_of::<LinuxHidrawDevInfo>())
}

/// Return whether one Linux node name matches `<prefix><digits>`.
fn is_linux_node_name(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    if suffix.is_empty() {
        return false;
    }

    suffix.as_bytes().iter().all(|byte| byte.is_ascii_digit())
}

/// Return whether one Linux absolute path matches `<directory>/<prefix><digits>`.
fn is_linux_node_path(path: &str, directory: &str, prefix: &str) -> bool {
    let node_path = Path::new(path);
    if node_path.parent() != Some(Path::new(directory)) {
        return false;
    }

    let Some(name) = node_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    is_linux_node_name(name, prefix)
}

/// Normalize one Linux input id into one canonical evdev or hidraw path.
pub(super) fn normalize_input_path(id: &str) -> RuntimeResult<String> {
    if id.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id cannot be empty",
        ))
        .boxed());
    }

    if id.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed());
    }

    if is_linux_node_path(id, INPUT_DEVICE_DIRECTORY, INPUT_EVENT_PREFIX) {
        return Ok(id.to_string());
    }
    if is_linux_node_path(id, INPUT_HIDRAW_DIRECTORY, INPUT_HIDRAW_PREFIX) {
        return Ok(id.to_string());
    }

    if is_linux_node_name(id, INPUT_EVENT_PREFIX) {
        return Ok(format!("{INPUT_DEVICE_DIRECTORY}/{id}"));
    }
    if is_linux_node_name(id, INPUT_HIDRAW_PREFIX) {
        return Ok(format!("{INPUT_HIDRAW_DIRECTORY}/{id}"));
    }

    // resolve stable runtime identifiers by scanning current device metadata
    if id.starts_with(LINUX_EVDEV_ID_PREFIX) || id.starts_with(LINUX_HIDRAW_ID_PREFIX) {
        let resolved = resolve_runtime_device_id_path(id)?;
        if let Some(path) = resolved {
            return Ok(path);
        }
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one runtime input id, /dev/input/event path, /dev/hidraw path, or node name",
    ))
    .boxed())
}

/// Resolve one stable runtime Linux device identifier into one openable path.
fn resolve_runtime_device_id_path(id: &str) -> RuntimeResult<Option<String>> {
    // scan evdev endpoints and match one runtime identifier
    let evdev_paths = list_linux_device_paths()?;
    for path in evdev_paths {
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_EVENT_PREFIX);
        let metadata = query_device_metadata(&path, fallback_name);
        if linux_runtime_device_id(path.as_str(), &metadata, false) == id {
            return Ok(Some(path));
        }
    }

    // scan hidraw endpoints and match one runtime identifier
    let hidraw_paths = list_linux_hidraw_paths()?;
    for path in hidraw_paths {
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_HIDRAW_PREFIX);
        let metadata = query_hidraw_metadata(&path, fallback_name);
        if linux_runtime_device_id(path.as_str(), &metadata, true) == id {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

/// Build one stable runtime identifier for one Linux input endpoint.
fn linux_runtime_device_id(path: &str, metadata: &InputDeviceMetadata, is_hidraw: bool) -> String {
    let prefix = if is_hidraw {
        LINUX_HIDRAW_ID_PREFIX
    } else {
        LINUX_EVDEV_ID_PREFIX
    };

    // prefer backend instance identifiers because they survive path renumbering
    if !metadata.instance_id.is_empty() && metadata.instance_id != path {
        return format!("{prefix}instance:{}", metadata.instance_id);
    }

    // otherwise use backend hardware topology identifiers when available
    if !metadata.hardware_id.is_empty() && metadata.hardware_id != path {
        return format!("{prefix}hardware:{}", metadata.hardware_id);
    }

    // use vendor and product fingerprints only when hardware identifiers are unavailable
    if metadata.vendor_id != 0 || metadata.product_id != 0 {
        return format!(
            "{prefix}fingerprint:{:04x}:{:04x}:{}:{path}",
            metadata.vendor_id,
            metadata.product_id,
            metadata.name.to_ascii_lowercase(),
        );
    }

    // fall back to path identifiers on hosts that expose no stable metadata
    format!("{prefix}path:{path}")
}

/// Return whether one identifier is one Linux hidraw runtime id.
pub(super) fn is_linux_hidraw_runtime_id(id: &str) -> bool {
    id.starts_with(LINUX_HIDRAW_ID_PREFIX)
}

/// Build one runtime identifier for one Linux device path.
pub(super) fn linux_runtime_device_id_for_path(path: &str) -> String {
    if is_linux_node_path(path, INPUT_DEVICE_DIRECTORY, INPUT_EVENT_PREFIX) {
        let fallback_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_EVENT_PREFIX);
        let metadata = query_device_metadata(path, fallback_name);
        return linux_runtime_device_id(path, &metadata, false);
    }

    if is_linux_node_path(path, INPUT_HIDRAW_DIRECTORY, INPUT_HIDRAW_PREFIX) {
        let fallback_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_HIDRAW_PREFIX);
        let metadata = query_hidraw_metadata(path, fallback_name);
        return linux_runtime_device_id(path, &metadata, true);
    }

    format!("{LINUX_EVDEV_ID_PREFIX}path:{path}")
}

/// Enumerate Linux evdev devices and map them into runtime metadata.
pub(super) fn list_linux_devices(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    let evdev_paths = list_linux_device_paths()?;
    let hidraw_paths = list_linux_hidraw_paths()?;
    let mut devices = Vec::with_capacity(evdev_paths.len() + hidraw_paths.len());
    for path in evdev_paths {
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_EVENT_PREFIX);
        let metadata = query_device_metadata(&path, fallback_name);
        let runtime_id = linux_runtime_device_id(path.as_str(), &metadata, false);

        devices.push(InputDeviceDescriptor {
            id: binding.store_string(&runtime_id),
            instance_id: binding.store_string(&metadata.instance_id),
            hardware_id: binding.store_string(&metadata.hardware_id),
            name: binding.store_string(&metadata.name),
            transport: binding.store_string(metadata.transport),
            kind: metadata.kind,
            vendor_id: metadata.vendor_id,
            product_id: metadata.product_id,
            key_count: metadata.key_count,
            button_count: metadata.button_count,
            axis_count: metadata.axis_count,
            connected: true,
            supports_exclusive_grab: true,
            supports_raw: true,
            supports_text: false,
            supports_rumble: metadata.supports_rumble,
            supports_battery: false,
            supports_light: false,
            supports_raw_hid: metadata.supports_raw_hid,
            is_virtual: false,
            is_system: true,
        });
    }

    for path in hidraw_paths {
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_HIDRAW_PREFIX);
        let metadata = query_hidraw_metadata(&path, fallback_name);
        let runtime_id = linux_runtime_device_id(path.as_str(), &metadata, true);

        devices.push(InputDeviceDescriptor {
            id: binding.store_string(&runtime_id),
            instance_id: binding.store_string(&metadata.instance_id),
            hardware_id: binding.store_string(&metadata.hardware_id),
            name: binding.store_string(&metadata.name),
            transport: binding.store_string(metadata.transport),
            kind: metadata.kind,
            vendor_id: metadata.vendor_id,
            product_id: metadata.product_id,
            key_count: metadata.key_count,
            button_count: metadata.button_count,
            axis_count: metadata.axis_count,
            connected: true,
            supports_exclusive_grab: false,
            supports_raw: true,
            supports_text: false,
            supports_rumble: metadata.supports_rumble,
            supports_battery: false,
            supports_light: false,
            supports_raw_hid: metadata.supports_raw_hid,
            is_virtual: false,
            is_system: true,
        });
    }

    devices.sort_unstable_by(|left, right| {
        let left_id = unsafe { left.id.as_str() }.unwrap_or_default();
        let right_id = unsafe { right.id.as_str() }.unwrap_or_default();
        left_id.cmp(right_id)
    });

    Ok(devices)
}

/// Read one Linux evdev event from one descriptor.
pub(super) fn read_linux_event(
    binding: &BindingCallContext,
    descriptor: RawFd,
    nonblocking: bool,
    device_id: &str,
    device_kind: InputDeviceKind,
    modifiers: u32,
    pointer_buttons: u32,
) -> RuntimeResult<(InputEvent, u32, u32)> {
    let mut modifiers_state = modifiers;
    let mut pointer_buttons_state = pointer_buttons;

    loop {
        // probe readiness in nonblocking mode before issuing reads
        if nonblocking {
            let mut pollfd = libc::pollfd {
                fd: descriptor,
                events: libc::POLLIN,
                revents: 0,
            };

            let poll_status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, 0) };
            if poll_status < 0 {
                return Err(core_platform::io_error("poll", None));
            }
            if poll_status == 0 {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoWouldBlock),
                    None,
                    Some(libc::EWOULDBLOCK),
                    Some("poll".to_string()),
                    None,
                    "input queue is empty",
                ))
                .boxed());
            }
        }

        // read one full evdev packet, handling partial reads and retryable errors
        let mut event = MaybeUninit::<LinuxInputEvent>::uninit();
        let total_bytes = std::mem::size_of::<LinuxInputEvent>();
        let mut read_offset = 0usize;
        while read_offset < total_bytes {
            let read_ptr = unsafe { (event.as_mut_ptr() as *mut u8).add(read_offset) };
            let read_len = total_bytes - read_offset;
            let read_status =
                unsafe { libc::read(descriptor, read_ptr as *mut libc::c_void, read_len) };
            if read_status == 0 {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoNotFound),
                    None,
                    None,
                    Some("read".to_string()),
                    None,
                    "input device reached end of stream",
                ))
                .boxed());
            }
            if read_status < 0 {
                let errno = core_platform::get_errno();
                if errno == libc::EINTR {
                    continue;
                }
                if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                    if nonblocking {
                        return Err(RuntimeError::from(PlatformError::io_with(
                            Some(PlatformErrorCode::IoWouldBlock),
                            None,
                            Some(errno),
                            Some("read".to_string()),
                            None,
                            "input queue is empty",
                        ))
                        .boxed());
                    }

                    input_core::wait_for_readable_descriptor(descriptor)?;
                    continue;
                }

                return Err(core_platform::io_error("read", None));
            }

            read_offset += read_status as usize;
        }

        // filter framing and unsupported packets from the semantic stream
        let event = unsafe { event.assume_init() };
        if should_skip_linux_event(&event) {
            continue;
        }

        // update stream-local modifier state before the mapped event is emitted
        modifiers_state = update_linux_modifiers(modifiers_state, &event);

        // update stream-local pointer button state before mapping motion payloads
        pointer_buttons_state = update_linux_pointer_buttons(pointer_buttons_state, &event);

        // map one normalized event payload and return the updated modifier bitset
        let mapped_event = map_linux_event(
            binding,
            event,
            device_id,
            device_kind,
            modifiers_state,
            pointer_buttons_state,
        );
        return Ok((mapped_event, modifiers_state, pointer_buttons_state));
    }
}

/// Return whether one Linux evdev packet should be skipped from semantic streams.
pub(super) fn should_skip_linux_event(event: &LinuxInputEvent) -> bool {
    if event.kind == EV_SYN {
        return true;
    }

    !(event.kind == EV_KEY || event.kind == EV_REL || event.kind == EV_ABS)
}

/// Set EVIOCGRAB on one Linux evdev descriptor.
pub(super) fn set_linux_grab(descriptor: RawFd, enable: bool) -> RuntimeResult<()> {
    let grab_value = if enable { 1 } else { 0 } as libc::c_int;
    let status = unsafe { libc::ioctl(descriptor, EVIOCGRAB_REQUEST, grab_value) };
    if status < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOTTY {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.event.setExclusiveGrab",
            ))
            .boxed());
        }

        return Err(core_platform::io_error("ioctl(EVIOCGRAB)", None));
    }

    Ok(())
}

/// Read one EVIOCGKEY state bitset from one Linux input descriptor.
fn read_key_state_bits(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<[u8; MAX_KEY_BITS]> {
    let mut key_bits = [0u8; MAX_KEY_BITS];
    let status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgkey_request(key_bits.len()),
            key_bits.as_mut_ptr(),
        )
    };
    if status < 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(core_platform::get_errno()),
            Some(operation.to_string()),
            None,
            "failed to query current key state".to_string(),
        ))
        .boxed());
    }

    Ok(key_bits)
}

/// Read one EVIOCGABS axis payload from one Linux input descriptor.
fn read_abs_axis_info(descriptor: RawFd, axis: u16) -> Option<LinuxInputAbsInfo> {
    let mut abs_info = MaybeUninit::<LinuxInputAbsInfo>::uninit();
    let status = unsafe { libc::ioctl(descriptor, eviocgabs_request(axis), abs_info.as_mut_ptr()) };
    if status < 0 {
        return None;
    }

    Some(unsafe { abs_info.assume_init() })
}

/// Normalize one signed absolute axis value to `[-1, 1]`.
fn normalize_signed_axis(value: i32, minimum: i32, maximum: i32) -> f64 {
    if maximum <= minimum {
        return 0.0;
    }

    let clamped = value.clamp(minimum, maximum);
    let min = minimum as f64;
    let max = maximum as f64;
    ((clamped as f64 - min) / (max - min) * 2.0 - 1.0).clamp(-1.0, 1.0)
}

/// Normalize one unsigned absolute axis value to `[0, 1]`.
fn normalize_unsigned_axis(value: i32, minimum: i32, maximum: i32) -> f64 {
    if maximum <= minimum {
        return 0.0;
    }

    let clamped = value.clamp(minimum, maximum);
    let min = minimum as f64;
    let max = maximum as f64;
    ((clamped as f64 - min) / (max - min)).clamp(0.0, 1.0)
}

/// Build runtime modifier bits from one EVIOCGKEY bitset.
fn modifiers_from_key_bits(key_bits: &[u8]) -> u32 {
    let mut modifiers = 0u32;

    if bit_is_set(key_bits, KEY_LEFTSHIFT as usize) || bit_is_set(key_bits, KEY_RIGHTSHIFT as usize)
    {
        modifiers |= MODIFIER_SHIFT;
    }
    if bit_is_set(key_bits, KEY_LEFTCTRL as usize) || bit_is_set(key_bits, KEY_RIGHTCTRL as usize) {
        modifiers |= MODIFIER_CONTROL;
    }
    if bit_is_set(key_bits, KEY_LEFTALT as usize) || bit_is_set(key_bits, KEY_RIGHTALT as usize) {
        modifiers |= MODIFIER_ALT;
    }
    if bit_is_set(key_bits, KEY_LEFTMETA as usize) || bit_is_set(key_bits, KEY_RIGHTMETA as usize) {
        modifiers |= MODIFIER_META;
    }
    if bit_is_set(key_bits, KEY_CAPSLOCK as usize) {
        modifiers |= MODIFIER_CAPS_LOCK;
    }
    if bit_is_set(key_bits, KEY_NUMLOCK as usize) {
        modifiers |= MODIFIER_NUM_LOCK;
    }
    if bit_is_set(key_bits, KEY_SCROLLLOCK as usize) {
        modifiers |= MODIFIER_SCROLL_LOCK;
    }

    modifiers
}

/// Build stable pointer-button bits from one EVIOCGKEY bitset.
fn pointer_buttons_from_key_bits(key_bits: &[u8]) -> u32 {
    let mut buttons = 0u32;

    if bit_is_set(key_bits, BTN_MOUSE_LEFT) {
        buttons |= 1u32 << 0;
    }
    if bit_is_set(key_bits, BTN_MOUSE_LEFT + 1) {
        buttons |= 1u32 << 1;
    }
    if bit_is_set(key_bits, BTN_MOUSE_LEFT + 2) {
        buttons |= 1u32 << 2;
    }
    if bit_is_set(key_bits, BTN_MOUSE_LEFT + 3) {
        buttons |= 1u32 << 3;
    }
    if bit_is_set(key_bits, BTN_MOUSE_LEFT + 4) {
        buttons |= 1u32 << 4;
    }

    buttons
}

/// Query one keyboard snapshot from one Linux input descriptor.
pub(super) fn keyboard_state_snapshot(
    binding: &BindingCallContext,
    descriptor: RawFd,
    sequence: u64,
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<InputKeyboardState> {
    // load currently pressed key bits from the kernel
    let key_bits = read_key_state_bits(descriptor, operation)?;

    // project logical key codes excluding button namespaces
    let mut pressed_codes = Vec::new();
    for code in 0..(key_bits.len() * 8) {
        if !bit_is_set(&key_bits, code) || is_button_code(code) {
            continue;
        }
        pressed_codes.push(code as u32);
    }

    // derive current modifier flags from pressed key bits
    let modifiers = modifiers_from_key_bits(&key_bits);

    Ok(InputKeyboardState {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence,
        device_id: binding.store_string(device_id),
        modifiers,
        pressed_codes: binding.store_array(pressed_codes.clone()),
        pressed_scan_codes: binding.store_array(pressed_codes),
    })
}

/// Query one pointer snapshot from one Linux input descriptor.
pub(super) fn pointer_state_snapshot(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<InputPointerState> {
    // load currently pressed key bits from the kernel
    let key_bits = read_key_state_bits(descriptor, operation)?;

    // derive current modifier and pointer-button state
    let modifiers = modifiers_from_key_bits(&key_bits);
    let buttons = pointer_buttons_from_key_bits(&key_bits);

    // query absolute pointer coordinates when the device reports them
    let x = read_abs_axis_info(descriptor, ABS_X)
        .map(|axis| axis.value as f64)
        .unwrap_or(0.0);
    let y = read_abs_axis_info(descriptor, ABS_Y)
        .map(|axis| axis.value as f64)
        .unwrap_or(0.0);

    // query pen pressure and tilt when supported by the descriptor
    let pressure_axis = read_abs_axis_info(descriptor, ABS_PRESSURE)
        .or_else(|| read_abs_axis_info(descriptor, ABS_MT_PRESSURE));
    let pressure = pressure_axis
        .map(|axis| normalize_unsigned_axis(axis.value, axis.minimum, axis.maximum))
        .unwrap_or(0.0);
    let tilt_x = read_abs_axis_info(descriptor, ABS_TILT_X)
        .map(|axis| normalize_signed_axis(axis.value, axis.minimum, axis.maximum))
        .unwrap_or(0.0);
    let tilt_y = read_abs_axis_info(descriptor, ABS_TILT_Y)
        .map(|axis| normalize_signed_axis(axis.value, axis.minimum, axis.maximum))
        .unwrap_or(0.0);

    // derive contact and in-range state from key and pressure lanes
    let has_pen_data = pressure_axis.is_some()
        || tilt_x != 0.0
        || tilt_y != 0.0
        || bit_is_set(&key_bits, BTN_STYLUS);
    let in_contact = bit_is_set(&key_bits, BTN_TOUCH) || pressure > 0.0 || buttons != 0;
    let in_range = if has_pen_data {
        bit_is_set(&key_bits, BTN_STYLUS) || in_contact
    } else {
        true
    };

    let pen = if has_pen_data {
        Some(InputPenState {
            pressure,
            tangential_pressure: 0.0,
            tilt_x,
            tilt_y,
            twist: 0.0,
            in_contact,
            in_range,
        })
    } else {
        None
    };

    Ok(InputPointerState {
        x,
        y,
        buttons,
        modifiers,
        pen,
    })
}

/// Build one standard gamepad-button state payload.
fn standard_gamepad_button_state(pressed: bool, value: f64) -> InputGamepadButtonState {
    InputGamepadButtonState {
        pressed,
        touched: pressed || value > 0.0,
        value,
    }
}

/// Read one standard gamepad axis value from one Linux absolute axis code.
fn read_gamepad_axis(descriptor: RawFd, axis: u16) -> f64 {
    let Some(info) = read_abs_axis_info(descriptor, axis) else {
        return 0.0;
    };

    normalize_signed_axis(info.value, info.minimum, info.maximum)
}

/// Read one gamepad trigger value from one Linux absolute axis code.
fn read_gamepad_trigger(descriptor: RawFd, axis: u16) -> f64 {
    let Some(info) = read_abs_axis_info(descriptor, axis) else {
        return 0.0;
    };

    normalize_unsigned_axis(info.value, info.minimum, info.maximum)
}

/// Query one gamepad snapshot from one Linux input descriptor.
pub(super) fn gamepad_state_snapshot(
    binding: &BindingCallContext,
    descriptor: RawFd,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // load currently pressed key bits from the kernel
    let key_bits = read_key_state_bits(descriptor, operation)?;

    // project standard gamepad axes from Linux absolute axis lanes
    let axes = vec![
        read_gamepad_axis(descriptor, ABS_X),
        read_gamepad_axis(descriptor, ABS_Y),
        read_gamepad_axis(descriptor, ABS_RX),
        read_gamepad_axis(descriptor, ABS_RY),
    ];

    // project standard gamepad button table from key and trigger lanes
    let left_trigger_axis = read_gamepad_trigger(descriptor, ABS_Z);
    let right_trigger_axis = read_gamepad_trigger(descriptor, ABS_RZ);
    let left_trigger_pressed = bit_is_set(&key_bits, BTN_TL2);
    let right_trigger_pressed = bit_is_set(&key_bits, BTN_TR2);
    let left_trigger_value = if left_trigger_pressed {
        1.0
    } else {
        left_trigger_axis
    };
    let right_trigger_value = if right_trigger_pressed {
        1.0
    } else {
        right_trigger_axis
    };

    let mut buttons = vec![
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_SOUTH),
            if bit_is_set(&key_bits, BTN_SOUTH) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_EAST),
            if bit_is_set(&key_bits, BTN_EAST) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_WEST),
            if bit_is_set(&key_bits, BTN_WEST) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_NORTH),
            if bit_is_set(&key_bits, BTN_NORTH) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_TL),
            if bit_is_set(&key_bits, BTN_TL) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_TR),
            if bit_is_set(&key_bits, BTN_TR) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(left_trigger_pressed, left_trigger_value),
        standard_gamepad_button_state(right_trigger_pressed, right_trigger_value),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_SELECT),
            if bit_is_set(&key_bits, BTN_SELECT) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_START),
            if bit_is_set(&key_bits, BTN_START) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_THUMBL),
            if bit_is_set(&key_bits, BTN_THUMBL) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_THUMBR),
            if bit_is_set(&key_bits, BTN_THUMBR) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_DPAD_UP),
            if bit_is_set(&key_bits, BTN_DPAD_UP) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_DPAD_DOWN),
            if bit_is_set(&key_bits, BTN_DPAD_DOWN) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_DPAD_LEFT),
            if bit_is_set(&key_bits, BTN_DPAD_LEFT) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_DPAD_RIGHT),
            if bit_is_set(&key_bits, BTN_DPAD_RIGHT) {
                1.0
            } else {
                0.0
            },
        ),
        standard_gamepad_button_state(
            bit_is_set(&key_bits, BTN_MODE),
            if bit_is_set(&key_bits, BTN_MODE) {
                1.0
            } else {
                0.0
            },
        ),
    ];

    // include hat-axis dpad fallback when key-button lanes are absent
    let hat_x = read_abs_axis_info(descriptor, ABS_HAT0X)
        .map(|axis| axis.value)
        .unwrap_or(0);
    let hat_y = read_abs_axis_info(descriptor, ABS_HAT0Y)
        .map(|axis| axis.value)
        .unwrap_or(0);
    if hat_y < 0 {
        buttons[12] = standard_gamepad_button_state(true, 1.0);
    } else if hat_y > 0 {
        buttons[13] = standard_gamepad_button_state(true, 1.0);
    }
    if hat_x < 0 {
        buttons[14] = standard_gamepad_button_state(true, 1.0);
    } else if hat_x > 0 {
        buttons[15] = standard_gamepad_button_state(true, 1.0);
    }

    Ok(InputGamepadState {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        connected: true,
        mapping: InputGamepadMappingType::Standard,
        connection_type: InputGamepadConnectionType::Wired,
        player_index,
        battery: InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::Unknown,
            level: 0.0,
        },
        supports_rumble: supports_linux_rumble(descriptor),
        supports_trigger_rumble: false,
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        touches: binding.store_array(Vec::<InputGamepadTouchState>::new()),
    })
}

/// Query one touch snapshot from one Linux input descriptor.
pub(super) fn touch_state_snapshot(
    binding: &BindingCallContext,
    descriptor: RawFd,
    sequence: u64,
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<InputTouchState> {
    // load currently pressed key bits from the kernel
    let key_bits = read_key_state_bits(descriptor, operation)?;

    // derive in-contact state from BTN_TOUCH and pressure lanes
    let pressure_axis = read_abs_axis_info(descriptor, ABS_MT_PRESSURE)
        .or_else(|| read_abs_axis_info(descriptor, ABS_PRESSURE));
    let pressure = pressure_axis
        .map(|axis| normalize_unsigned_axis(axis.value, axis.minimum, axis.maximum))
        .unwrap_or(0.0);
    let in_contact = bit_is_set(&key_bits, BTN_TOUCH) || pressure > 0.0;

    // project one primary touch contact when the device is currently in contact
    let mut contacts = Vec::new();
    if in_contact {
        let contact_id = read_abs_axis_info(descriptor, ABS_MT_TRACKING_ID)
            .map(|axis| axis.value.max(0) as u32)
            .unwrap_or(0);
        let x = read_abs_axis_info(descriptor, ABS_MT_POSITION_X)
            .or_else(|| read_abs_axis_info(descriptor, ABS_X))
            .map(|axis| axis.value as f64)
            .unwrap_or(0.0);
        let y = read_abs_axis_info(descriptor, ABS_MT_POSITION_Y)
            .or_else(|| read_abs_axis_info(descriptor, ABS_Y))
            .map(|axis| axis.value as f64)
            .unwrap_or(0.0);

        contacts.push(InputTouchContactState {
            contact_id,
            phase: InputTouchContactPhase::Move,
            x,
            y,
            pressure,
            radius_x: 0.0,
            radius_y: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
        });
    }

    Ok(InputTouchState {
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence,
        device_id: binding.store_string(device_id),
        contacts: binding.store_array(contacts),
    })
}

/// Return whether one Linux input descriptor reports force-feedback rumble support.
pub(super) fn supports_linux_rumble(descriptor: RawFd) -> bool {
    // ensure the descriptor reports EV_FF before querying FF capabilities
    let mut event_bits = [0u8; MAX_EVENT_BITS];
    let event_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(0, event_bits.len()),
            event_bits.as_mut_ptr(),
        )
    };
    if event_status < 0 || !bit_is_set(&event_bits, EV_FF as usize) {
        return false;
    }

    // inspect force-feedback capabilities for FF_RUMBLE support
    let mut ff_bits = [0u8; MAX_FF_BITS];
    let ff_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(EV_FF, ff_bits.len()),
            ff_bits.as_mut_ptr(),
        )
    };
    if ff_status < 0 {
        return false;
    }

    bit_is_set(&ff_bits, FF_RUMBLE)
}

/// Return one axis tuple for one supported sensor kind.
fn sensor_axes_for_kind(kind: InputSensorKind) -> Option<(u16, u16, u16)> {
    match kind {
        InputSensorKind::Accelerometer
        | InputSensorKind::Gravity
        | InputSensorKind::LinearAcceleration => Some((ABS_X, ABS_Y, ABS_Z)),
        InputSensorKind::Gyroscope | InputSensorKind::Orientation => Some((ABS_RX, ABS_RY, ABS_RZ)),
        InputSensorKind::Magnetometer
        | InputSensorKind::Barometer
        | InputSensorKind::AmbientLight
        | InputSensorKind::Proximity
        | InputSensorKind::StepCounter => None,
    }
}

/// Return whether one absolute axis is available on one descriptor.
fn has_absolute_axis(descriptor: RawFd, axis: u16) -> bool {
    read_abs_axis_info(descriptor, axis).is_some()
}

/// Return supported sensor kinds for one Linux input descriptor.
fn sensor_kinds_for_descriptor(
    descriptor: RawFd,
    device_kind: InputDeviceKind,
) -> Vec<InputSensorKind> {
    // keep sensor lanes constrained to raw descriptors to avoid conflating gamepad axes
    if device_kind != InputDeviceKind::Raw {
        return Vec::new();
    }

    // detect accelerometer-like axis triples
    let has_acceleration_axes = has_absolute_axis(descriptor, ABS_X)
        && has_absolute_axis(descriptor, ABS_Y)
        && has_absolute_axis(descriptor, ABS_Z);

    // detect gyroscope-like axis triples
    let has_gyroscope_axes = has_absolute_axis(descriptor, ABS_RX)
        && has_absolute_axis(descriptor, ABS_RY)
        && has_absolute_axis(descriptor, ABS_RZ);

    // project supported kinds from detected axis families
    let mut kinds = Vec::new();
    if has_acceleration_axes {
        kinds.push(InputSensorKind::Accelerometer);
        kinds.push(InputSensorKind::Gravity);
        kinds.push(InputSensorKind::LinearAcceleration);
    }
    if has_gyroscope_axes {
        kinds.push(InputSensorKind::Gyroscope);
        kinds.push(InputSensorKind::Orientation);
    }

    kinds
}

/// Query Linux sensor capability metadata from one descriptor.
pub(super) fn linux_sensor_infos(
    descriptor: RawFd,
    device_kind: InputDeviceKind,
) -> Vec<InputSensorDescriptor> {
    // query supported sensor kinds from descriptor axis topology
    let kinds = sensor_kinds_for_descriptor(descriptor, device_kind);
    let mut infos = Vec::with_capacity(kinds.len());

    for kind in kinds {
        let Some((axis_x, axis_y, axis_z)) = sensor_axes_for_kind(kind) else {
            continue;
        };

        // aggregate one representative resolution estimate from axis metadata
        let mut resolution_sum = 0.0f64;
        let mut resolution_count = 0usize;
        for axis in [axis_x, axis_y, axis_z] {
            if let Some(info) = read_abs_axis_info(descriptor, axis)
                && info.resolution > 0
            {
                resolution_sum += f64::from(info.resolution);
                resolution_count = resolution_count.saturating_add(1);
            }
        }
        let resolution = if resolution_count > 0 {
            resolution_sum / resolution_count as f64
        } else {
            0.0
        };

        infos.push(InputSensorDescriptor {
            kind,
            min_sample_rate_hz: 0.0,
            max_sample_rate_hz: 0.0,
            resolution,
            supports_wake: false,
        });
    }

    infos
}

/// Build one io-would-block sensor error.
fn sensor_would_block(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        Some(libc::EWOULDBLOCK),
        Some(operation.to_string()),
        None,
        "sensor queue is empty".to_string(),
    ))
    .boxed()
}

/// Wait for one descriptor to become readable for sensor sampling.
fn wait_for_sensor_readable(
    descriptor: RawFd,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // choose one nonblocking or blocking poll timeout
    let timeout_ms = if nonblocking { 0 } else { -1 };
    let mut pollfd = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };

    loop {
        let status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms) };
        if status > 0 {
            return Ok(());
        }
        if status == 0 {
            return Err(sensor_would_block(operation));
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(core_platform::io_error("poll", None));
    }
}

/// Consume one raw evdev packet from one descriptor.
fn consume_linux_event_packet(descriptor: RawFd, operation: &'static str) -> RuntimeResult<()> {
    let mut event = MaybeUninit::<LinuxInputEvent>::uninit();
    let total_bytes = std::mem::size_of::<LinuxInputEvent>();
    let mut read_offset = 0usize;

    while read_offset < total_bytes {
        let read_ptr = unsafe { (event.as_mut_ptr() as *mut u8).add(read_offset) };
        let read_len = total_bytes - read_offset;
        let read_status =
            unsafe { libc::read(descriptor, read_ptr.cast::<libc::c_void>(), read_len) };
        if read_status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                None,
                Some("read".to_string()),
                None,
                "input device reached end of stream".to_string(),
            ))
            .boxed());
        }
        if read_status < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                return Err(sensor_would_block(operation));
            }

            return Err(core_platform::io_error("read", None));
        }

        read_offset += read_status as usize;
    }

    Ok(())
}

/// Read one Linux sensor sample from absolute-axis state.
pub(super) fn read_linux_sensor_sample(
    descriptor: RawFd,
    device_kind: InputDeviceKind,
    kind: InputSensorKind,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputSensorSample> {
    // verify sensor-kind support before issuing reads
    let supported = sensor_kinds_for_descriptor(descriptor, device_kind);
    if !supported.contains(&kind) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "kind",
            "sensor kind is not supported by this device",
        ))
        .boxed());
    }

    // wait for one readable packet to preserve stream semantics
    wait_for_sensor_readable(descriptor, nonblocking, operation)?;

    // consume one packet to advance the descriptor stream
    if let Err(error) = consume_linux_event_packet(descriptor, operation) {
        if nonblocking && is_io_would_block_error(&error) {
            return Err(sensor_would_block(operation));
        }

        return Err(error);
    }

    // load sampled axes from current absolute-axis state
    let Some((axis_x, axis_y, axis_z)) = sensor_axes_for_kind(kind) else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "kind",
            "sensor kind is not supported by this backend",
        ))
        .boxed());
    };
    let axis_info_x = read_abs_axis_info(descriptor, axis_x)
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())?;
    let axis_info_y = read_abs_axis_info(descriptor, axis_y)
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())?;
    let axis_info_z = read_abs_axis_info(descriptor, axis_z)
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())?;

    // project one sample payload with backend-native units
    let w = if kind == InputSensorKind::Orientation {
        1.0
    } else {
        0.0
    };
    Ok(InputSensorSample {
        kind,
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        x: f64::from(axis_info_x.value),
        y: f64::from(axis_info_y.value),
        z: f64::from(axis_info_z.value),
        w,
        flags: 0,
    })
}

/// Return haptics effect kinds supported by one Linux descriptor.
pub(super) fn linux_haptics_effects(descriptor: RawFd) -> Vec<InputHapticEffectType> {
    if !supports_linux_rumble(descriptor) {
        return Vec::new();
    }

    vec![InputHapticEffectType::DualRumble]
}

/// Return one runtime-clamped rumble magnitude.
fn clamped_rumble_magnitude(value: f64) -> u16 {
    if !value.is_finite() {
        return 0;
    }

    let normalized = value.clamp(0.0, 1.0);
    (normalized * f64::from(u16::MAX)).round() as u16
}

/// Build one Linux force-feedback effect payload for rumble.
fn rumble_effect_payload(effect_id: i16, params: InputHapticEffectParameters) -> LinuxFfEffect {
    let mut effect = LinuxFfEffect {
        type_: FF_RUMBLE as u16,
        id: effect_id,
        direction: 0,
        trigger: LinuxFfTrigger {
            button: 0,
            interval: 0,
        },
        replay: LinuxFfReplay {
            length: params.duration_ms.min(u64::from(u16::MAX)) as u16,
            delay: params.start_delay_ms.min(u64::from(u16::MAX)) as u16,
        },
        #[cfg(target_pointer_width = "64")]
        payload: [0u64; 4],
        #[cfg(target_pointer_width = "32")]
        payload: [0u32; 7],
    };

    let rumble = LinuxFfRumbleEffect {
        strong_magnitude: clamped_rumble_magnitude(params.strong_magnitude),
        weak_magnitude: clamped_rumble_magnitude(params.weak_magnitude),
    };
    unsafe {
        std::ptr::write(
            effect.payload.as_mut_ptr().cast::<LinuxFfRumbleEffect>(),
            rumble,
        );
    }

    effect
}

/// Write one force-feedback start or stop event to one descriptor.
fn write_rumble_event(descriptor: RawFd, effect_id: i16, value: i32) -> RuntimeResult<()> {
    let mut event = LinuxInputEvent {
        time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        kind: EV_FF,
        code: effect_id.max(0) as u16,
        value,
    };

    let mut write_offset = 0usize;
    let total_bytes = std::mem::size_of::<LinuxInputEvent>();
    while write_offset < total_bytes {
        let write_ptr = unsafe {
            (&mut event as *mut LinuxInputEvent)
                .cast::<u8>()
                .add(write_offset)
        };
        let write_len = total_bytes - write_offset;
        let write_status =
            unsafe { libc::write(descriptor, write_ptr.cast::<libc::c_void>(), write_len) };
        if write_status < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }

            return Err(core_platform::io_error("write", None));
        }
        if write_status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                None,
                Some("write".to_string()),
                None,
                "haptics endpoint reached end of stream".to_string(),
            ))
            .boxed());
        }

        write_offset += write_status as usize;
    }

    Ok(())
}

/// Upload and play one Linux rumble effect and return the active effect id.
pub(super) fn play_linux_rumble(
    descriptor: RawFd,
    params: InputHapticEffectParameters,
    operation: &'static str,
) -> RuntimeResult<i16> {
    // upload one rumble effect to the kernel ff table
    let mut effect = rumble_effect_payload(-1, params);
    let status = unsafe { libc::ioctl(descriptor, EVIOCSFF_REQUEST, &mut effect) };
    if status < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOSYS || errno == libc::ENOTTY {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        return Err(core_platform::io_error("ioctl(EVIOCSFF)", None));
    }

    // start playback for the uploaded effect id
    write_rumble_event(descriptor, effect.id, FF_EVENT_START)?;
    Ok(effect.id)
}

/// Stop one active Linux rumble effect and remove it from the kernel table.
pub(super) fn stop_linux_rumble(
    descriptor: RawFd,
    effect_id: i16,
    operation: &'static str,
) -> RuntimeResult<()> {
    // ignore invalid ids because no active effect is present
    if effect_id < 0 {
        return Ok(());
    }

    // stop active playback before erasing kernel state
    write_rumble_event(descriptor, effect_id, FF_EVENT_STOP)?;

    // erase one kernel-side effect slot
    let effect_id_raw = libc::c_int::from(effect_id);
    let status = unsafe { libc::ioctl(descriptor, EVIOCRMFF_REQUEST, effect_id_raw) };
    if status < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOSYS || errno == libc::ENOTTY {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        return Err(core_platform::io_error("ioctl(EVIOCRMFF)", None));
    }

    Ok(())
}

/// Return whether one runtime error is one io-would-block platform error.
fn is_io_would_block_error(error: &RuntimeError) -> bool {
    error.platform_error().map(|platform| platform.code) == Some(PlatformErrorCode::IoWouldBlock)
}

/// Return whether one bit is set in one packed bitset.
fn bit_is_set(bits: &[u8], index: usize) -> bool {
    let byte_index = index / 8;
    if byte_index >= bits.len() {
        return false;
    }

    let bit_mask = 1u8 << (index % 8);
    (bits[byte_index] & bit_mask) != 0
}

/// Classify one raw Linux event into one runtime event kind.
fn input_event_kind(raw_kind: u16, code: u16, device_kind: InputDeviceKind) -> InputEventKind {
    match raw_kind {
        EV_KEY => {
            if code as usize == BTN_TOUCH {
                InputEventKind::Touch
            } else if (BTN_MOUSE_LEFT..=(BTN_MOUSE_LEFT + 4)).contains(&(code as usize)) {
                InputEventKind::PointerButton
            } else if is_gamepad_button_code(code as usize)
                || (device_kind == InputDeviceKind::Gamepad && is_button_code(code as usize))
            {
                InputEventKind::Gamepad
            } else {
                InputEventKind::Key
            }
        }
        EV_REL => {
            if code == REL_WHEEL || code == REL_HWHEEL {
                InputEventKind::Scroll
            } else {
                InputEventKind::PointerMotion
            }
        }
        EV_ABS => {
            if is_touch_absolute_code(code) {
                InputEventKind::Touch
            } else if device_kind == InputDeviceKind::Raw && is_sensor_absolute_code(code) {
                InputEventKind::Sensor
            } else if is_gamepad_absolute_code(code) || device_kind == InputDeviceKind::Gamepad {
                InputEventKind::Gamepad
            } else if code == ABS_X || code == ABS_Y {
                InputEventKind::PointerMotion
            } else {
                InputEventKind::Touch
            }
        }
        EV_SYN => InputEventKind::Device,
        _ => InputEventKind::Device,
    }
}

/// Return whether one EV_KEY code is one gamepad-button namespace code.
fn is_gamepad_button_code(code: usize) -> bool {
    (BTN_JOYSTICK_START..=BTN_JOYSTICK_END).contains(&code)
        || (BTN_GAMEPAD_START..=BTN_GAMEPAD_END).contains(&code)
        || (BTN_TRIGGER_HAPPY_START..=BTN_TRIGGER_HAPPY_END).contains(&code)
}

/// Return whether one EV_ABS code belongs to one gamepad axis namespace.
fn is_gamepad_absolute_code(code: u16) -> bool {
    matches!(
        code,
        ABS_X
            | ABS_Y
            | ABS_Z
            | ABS_RX
            | ABS_RY
            | ABS_RZ
            | ABS_THROTTLE
            | ABS_RUDDER
            | ABS_WHEEL
            | ABS_GAS
            | ABS_BRAKE
            | ABS_HAT0X
            | ABS_HAT0Y
            | ABS_HAT1X
            | ABS_HAT1Y
            | ABS_HAT2X
            | ABS_HAT2Y
            | ABS_HAT3X
            | ABS_HAT3Y
    )
}

/// Return whether one EV_ABS code belongs to one sensor-axis namespace.
fn is_sensor_absolute_code(code: u16) -> bool {
    matches!(code, ABS_X | ABS_Y | ABS_Z | ABS_RX | ABS_RY | ABS_RZ)
}

/// Return whether one EV_ABS code belongs to one multitouch namespace.
fn is_touch_absolute_code(code: u16) -> bool {
    matches!(code, ABS_MT_SLOT | ABS_MT_POSITION_X | ABS_MT_POSITION_Y)
}

/// Map one Linux evdev payload into one runtime input event.
fn map_linux_event(
    binding: &BindingCallContext,
    raw: LinuxInputEvent,
    device_id: &str,
    device_kind: InputDeviceKind,
    modifiers: u32,
    pointer_buttons: u32,
) -> InputEvent {
    let kind = input_event_kind(raw.kind, raw.code, device_kind);

    let mut x = 0.0_f64;
    let mut y = 0.0_f64;
    if raw.code == REL_X || raw.code == ABS_X {
        x = raw.value as f64;
    } else if raw.code == REL_Y || raw.code == ABS_Y {
        y = raw.value as f64;
    }

    let action = match raw.kind {
        EV_KEY => {
            if raw.value == 0 {
                InputEventAction::Release
            } else if raw.value == 2 {
                InputEventAction::Repeat
            } else {
                InputEventAction::Press
            }
        }
        EV_REL => {
            if raw.code == REL_WHEEL || raw.code == REL_HWHEEL {
                InputEventAction::Scroll
            } else {
                InputEventAction::Move
            }
        }
        EV_ABS => {
            if kind == InputEventKind::Gamepad || kind == InputEventKind::Sensor {
                InputEventAction::Axis
            } else {
                InputEventAction::Move
            }
        }
        _ => InputEventAction::Move,
    };

    let wheel_x = if raw.code == REL_HWHEEL {
        raw.value as f64
    } else {
        0.0
    };
    let wheel_y = if raw.code == REL_WHEEL {
        raw.value as f64
    } else {
        0.0
    };

    let mut payload = input_core::empty_unix_event_payload(binding);
    match kind {
        InputEventKind::Key => {
            payload.key = InputKeyEventPayload {
                action,
                backend_code: raw.code as u32,
                backend_scan_code: raw.code as u32,
                backend_value: raw.value as i64,
                modifiers,
                repeat: raw.value == 2,
            };
        }
        InputEventKind::PointerMotion => {
            payload.pointer_motion = InputPointerMotionEventPayload {
                x,
                y,
                buttons: pointer_buttons,
                modifiers,
            };
        }
        InputEventKind::PointerButton => {
            payload.pointer_button = InputPointerButtonEventPayload {
                action,
                backend_code: raw.code as u32,
                backend_value: raw.value as i64,
                x,
                y,
                modifiers,
            };
        }
        InputEventKind::Scroll => {
            payload.scroll = InputScrollEventPayload {
                wheel_x,
                wheel_y,
                x,
                y,
                modifiers,
            };
        }
        InputEventKind::Touch => {
            payload.touch = InputTouchEventPayload {
                action,
                contact_id: raw.code as u32,
                x,
                y,
                pressure: raw.value as f64,
            };
        }
        InputEventKind::Gamepad => {
            payload.gamepad = InputGamepadEventPayload {
                action,
                backend_code: raw.code as u32,
                backend_value: raw.value as i64,
            };
        }
        InputEventKind::Text => {
            payload.text = InputTextEventPayload {
                text: binding.store_string(input_core::UNIX_INPUT_EMPTY_TEXT),
            };
        }
        InputEventKind::Device => {
            payload.device = InputDeviceEventPayload {
                action,
                backend_code: raw.code as u32,
                backend_value: raw.value as i64,
            };
        }
        InputEventKind::Sensor => {
            let sensor_x = if raw.code == ABS_X || raw.code == ABS_RX {
                raw.value as f64
            } else {
                0.0
            };
            let sensor_y = if raw.code == ABS_Y || raw.code == ABS_RY {
                raw.value as f64
            } else {
                0.0
            };
            let sensor_z = if raw.code == ABS_Z || raw.code == ABS_RZ {
                raw.value as f64
            } else {
                0.0
            };
            payload.sensor = InputSensorEventPayload {
                action,
                backend_code: raw.code as u32,
                backend_value: raw.value as i64,
                x: sensor_x,
                y: sensor_y,
                z: sensor_z,
            };
        }
        InputEventKind::Composition => {
            payload.composition = InputCompositionEventPayload {
                action,
                text: binding.store_string(input_core::UNIX_INPUT_EMPTY_TEXT),
                selection_start: 0,
                selection_end: 0,
            };
        }
    }

    input_core::build_unix_input_event(
        binding,
        kind,
        linux_event_timestamp_ns(&raw),
        0,
        device_id,
        payload,
    )
}

/// Convert one kernel timeval stamp into one nanosecond timestamp.
fn linux_event_timestamp_ns(event: &LinuxInputEvent) -> u64 {
    if event.time.tv_sec < 0 || event.time.tv_usec < 0 {
        return input_core::monotonic_timestamp_ns();
    }

    let seconds = event.time.tv_sec as u128;
    let micros = event.time.tv_usec as u128;
    seconds
        .saturating_mul(1_000_000_000u128)
        .saturating_add(micros.saturating_mul(1_000u128))
        .min(u128::from(u64::MAX)) as u64
}

/// Map one Linux pointer button code into one runtime pointer-button bit mask.
fn pointer_button_mask(code: u16) -> Option<u32> {
    let code = code as usize;
    if !(BTN_MOUSE_LEFT..=(BTN_MOUSE_LEFT + 4)).contains(&code) {
        return None;
    }

    Some(1u32 << (code - BTN_MOUSE_LEFT))
}

/// Update Linux pointer-button bitset from one evdev key packet.
fn update_linux_pointer_buttons(current: u32, event: &LinuxInputEvent) -> u32 {
    if event.kind != EV_KEY {
        return current;
    }

    let Some(mask) = pointer_button_mask(event.code) else {
        return current;
    };

    if event.value == 0 {
        return current & !mask;
    }

    current | mask
}

/// Update Linux modifier state from one evdev key packet.
fn update_linux_modifiers(current: u32, event: &LinuxInputEvent) -> u32 {
    // ignore non-key packets because they carry no modifier transitions
    if event.kind != EV_KEY {
        return current;
    }

    // normalize one active state for key press and repeat transitions
    let is_active = event.value != 0;

    // apply hold-style modifier transitions
    match event.code {
        KEY_LEFTSHIFT | KEY_RIGHTSHIFT => {
            return update_modifier_bit(current, MODIFIER_SHIFT, is_active);
        }
        KEY_LEFTCTRL | KEY_RIGHTCTRL => {
            return update_modifier_bit(current, MODIFIER_CONTROL, is_active);
        }
        KEY_LEFTALT | KEY_RIGHTALT => {
            return update_modifier_bit(current, MODIFIER_ALT, is_active);
        }
        KEY_LEFTMETA | KEY_RIGHTMETA => {
            return update_modifier_bit(current, MODIFIER_META, is_active);
        }
        _ => {}
    }

    // apply lock-key toggles on press edges only
    if event.value == 1 {
        return match event.code {
            KEY_CAPSLOCK => current ^ MODIFIER_CAPS_LOCK,
            KEY_NUMLOCK => current ^ MODIFIER_NUM_LOCK,
            KEY_SCROLLLOCK => current ^ MODIFIER_SCROLL_LOCK,
            _ => current,
        };
    }

    current
}

/// Set or clear one modifier bit.
fn update_modifier_bit(current: u32, bit: u32, is_active: bool) -> u32 {
    if is_active {
        current | bit
    } else {
        current & !bit
    }
}

/// Classify one Linux device kind from descriptor capabilities.
fn detect_device_kind(name: &str, descriptor: RawFd) -> InputDeviceKind {
    let name_kind = detect_device_kind_from_name(name);
    if name_kind != InputDeviceKind::Raw {
        return name_kind;
    }

    let mut event_bits = [0u8; MAX_EVENT_BITS];
    let event_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(0, event_bits.len()),
            event_bits.as_mut_ptr(),
        )
    };
    if event_status < 0 {
        return InputDeviceKind::Raw;
    }

    let has_relative = bit_is_set(&event_bits, EV_REL as usize);
    let has_absolute = bit_is_set(&event_bits, EV_ABS as usize);
    let has_keys = bit_is_set(&event_bits, EV_KEY as usize);
    if !has_keys {
        return InputDeviceKind::Raw;
    }

    let mut key_bits = [0u8; MAX_KEY_BITS];
    let key_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(EV_KEY, key_bits.len()),
            key_bits.as_mut_ptr(),
        )
    };
    if key_status < 0 {
        if has_relative {
            return InputDeviceKind::Mouse;
        }
        if has_absolute {
            return InputDeviceKind::Touch;
        }

        return InputDeviceKind::Raw;
    }

    if bit_is_set(&key_bits, BTN_STYLUS) {
        return InputDeviceKind::Pen;
    }
    if bit_is_set(&key_bits, BTN_GAMEPAD) {
        return InputDeviceKind::Gamepad;
    }
    if bit_is_set(&key_bits, BTN_MOUSE_LEFT) || has_relative {
        return InputDeviceKind::Mouse;
    }
    if bit_is_set(&key_bits, BTN_TOUCH) || has_absolute {
        return InputDeviceKind::Touch;
    }
    if bit_is_set(&key_bits, KEY_A) {
        return InputDeviceKind::Keyboard;
    }

    InputDeviceKind::Raw
}

/// Classify one Linux device kind from one human-readable device name.
fn detect_device_kind_from_name(name: &str) -> InputDeviceKind {
    let name_lower = name.to_lowercase();
    if name_lower.contains("keyboard") {
        return InputDeviceKind::Keyboard;
    }
    if name_lower.contains("mouse") || name_lower.contains("trackpad") {
        return InputDeviceKind::Mouse;
    }
    if name_lower.contains("touch") {
        return InputDeviceKind::Touch;
    }
    if name_lower.contains("gamepad") || name_lower.contains("controller") {
        return InputDeviceKind::Gamepad;
    }
    if name_lower.contains("stylus") || name_lower.contains("pen") {
        return InputDeviceKind::Pen;
    }

    InputDeviceKind::Raw
}

/// Return the number of set bits in one capability bitset.
fn count_set_bits(bits: &[u8]) -> usize {
    bits.iter().map(|byte| byte.count_ones() as usize).sum()
}

/// Return whether one Linux EV_KEY code belongs to one button namespace.
fn is_button_code(code: usize) -> bool {
    (BTN_MISC_START..=BTN_MISC_END).contains(&code)
        || (BTN_MOUSE_START..=BTN_MOUSE_END).contains(&code)
        || (BTN_JOYSTICK_START..=BTN_JOYSTICK_END).contains(&code)
        || (BTN_GAMEPAD_START..=BTN_GAMEPAD_END).contains(&code)
        || (BTN_DIGI_START..=BTN_DIGI_END).contains(&code)
        || (BTN_WHEEL_START..=BTN_WHEEL_END).contains(&code)
        || (BTN_TRIGGER_HAPPY_START..=BTN_TRIGGER_HAPPY_END).contains(&code)
}

/// Query Linux key, button, and axis counts from one input descriptor.
fn query_device_counts(descriptor: RawFd) -> (u16, u16, u16) {
    // load event capabilities to decide which capability tables exist
    let mut event_bits = [0u8; MAX_EVENT_BITS];
    let event_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(0, event_bits.len()),
            event_bits.as_mut_ptr(),
        )
    };
    if event_status < 0 {
        return (0, 0, 0);
    }

    // split EV_KEY capabilities into logical keys and logical buttons
    let mut key_count = 0usize;
    let mut button_count = 0usize;
    let mut key_bits = [0u8; MAX_KEY_BITS];
    let key_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(EV_KEY, key_bits.len()),
            key_bits.as_mut_ptr(),
        )
    };
    if key_status >= 0 {
        for code in 0..(key_bits.len() * 8) {
            if !bit_is_set(&key_bits, code) {
                continue;
            }

            if is_button_code(code) {
                button_count = button_count.saturating_add(1);
            } else {
                key_count = key_count.saturating_add(1);
            }
        }
    }

    // accumulate relative and absolute axes when reported by the backend
    let mut axis_count = 0usize;
    if bit_is_set(&event_bits, EV_REL as usize) {
        let mut rel_bits = [0u8; MAX_REL_BITS];
        let rel_status = unsafe {
            libc::ioctl(
                descriptor,
                eviocgbit_request(EV_REL, rel_bits.len()),
                rel_bits.as_mut_ptr(),
            )
        };
        if rel_status >= 0 {
            axis_count = axis_count.saturating_add(count_set_bits(&rel_bits));
        }
    }
    if bit_is_set(&event_bits, EV_ABS as usize) {
        let mut abs_bits = [0u8; MAX_ABS_BITS];
        let abs_status = unsafe {
            libc::ioctl(
                descriptor,
                eviocgbit_request(EV_ABS, abs_bits.len()),
                abs_bits.as_mut_ptr(),
            )
        };
        if abs_status >= 0 {
            axis_count = axis_count.saturating_add(count_set_bits(&abs_bits));
        }
    }

    (
        key_count.min(u16::MAX as usize) as u16,
        button_count.min(u16::MAX as usize) as u16,
        axis_count.min(u16::MAX as usize) as u16,
    )
}

/// Query one Linux device kind from one event-node path.
pub(super) fn linux_device_kind_for_path(path: &str) -> InputDeviceKind {
    let fallback_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(INPUT_EVENT_PREFIX);
    query_device_metadata(path, fallback_name).kind
}

/// Query one backend-derived capabilities payload for one opened Linux descriptor.
pub(super) fn query_linux_capabilities(
    binding: &BindingCallContext,
    descriptor: RawFd,
    device_kind: InputDeviceKind,
    _supports_exclusive_grab: bool,
    supports_text: bool,
    supports_rumble: bool,
    supports_battery: bool,
    supports_light: bool,
    supports_raw_hid: bool,
) -> InputDeviceCapabilities {
    // query event tables to derive feature booleans and capability payload vectors
    let mut event_bits = [0u8; MAX_EVENT_BITS];
    let event_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgbit_request(0, event_bits.len()),
            event_bits.as_mut_ptr(),
        )
    };
    let has_event_bits = event_status >= 0;
    let has_relative = has_event_bits && bit_is_set(&event_bits, EV_REL as usize);
    let has_absolute = has_event_bits && bit_is_set(&event_bits, EV_ABS as usize);
    let has_key = has_event_bits && bit_is_set(&event_bits, EV_KEY as usize);
    let sensor_infos = linux_sensor_infos(descriptor, device_kind);
    let has_sensors = !sensor_infos.is_empty();

    // build capability kinds from device identity and supported feature lanes
    let mut kinds = Vec::new();
    match device_kind {
        InputDeviceKind::Keyboard => kinds.push(InputDeviceCapabilityKind::Keyboard),
        InputDeviceKind::Mouse => kinds.push(InputDeviceCapabilityKind::Pointer),
        InputDeviceKind::Touch => {
            kinds.push(InputDeviceCapabilityKind::Touch);
            kinds.push(InputDeviceCapabilityKind::Pointer);
        }
        InputDeviceKind::Pen => {
            kinds.push(InputDeviceCapabilityKind::Pen);
            kinds.push(InputDeviceCapabilityKind::Pointer);
        }
        InputDeviceKind::Gamepad => kinds.push(InputDeviceCapabilityKind::Gamepad),
        InputDeviceKind::Raw => {}
    }
    if supports_text {
        kinds.push(InputDeviceCapabilityKind::TextInput);
    }
    if supports_rumble {
        kinds.push(InputDeviceCapabilityKind::Haptics);
    }
    if has_sensors && !kinds.contains(&InputDeviceCapabilityKind::Sensor) {
        kinds.push(InputDeviceCapabilityKind::Sensor);
    }

    // collect axis tables from EV_REL and EV_ABS capabilities
    let mut axes = Vec::new();
    if has_relative {
        let mut rel_bits = [0u8; MAX_REL_BITS];
        let rel_status = unsafe {
            libc::ioctl(
                descriptor,
                eviocgbit_request(EV_REL, rel_bits.len()),
                rel_bits.as_mut_ptr(),
            )
        };
        if rel_status >= 0 {
            for code in 0..(rel_bits.len() * 8) {
                if !bit_is_set(&rel_bits, code) {
                    continue;
                }

                axes.push(InputAxisMetadata {
                    code: code as u32,
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                });
            }
        }
    }
    if has_absolute {
        let mut abs_bits = [0u8; MAX_ABS_BITS];
        let abs_status = unsafe {
            libc::ioctl(
                descriptor,
                eviocgbit_request(EV_ABS, abs_bits.len()),
                abs_bits.as_mut_ptr(),
            )
        };
        if abs_status >= 0 {
            for code in 0..(abs_bits.len() * 8) {
                if !bit_is_set(&abs_bits, code) {
                    continue;
                }

                let axis_code = code as u16;
                let mut abs_info = MaybeUninit::<LinuxInputAbsInfo>::uninit();
                let abs_info_status = unsafe {
                    libc::ioctl(
                        descriptor,
                        eviocgabs_request(axis_code),
                        abs_info.as_mut_ptr(),
                    )
                };
                if abs_info_status >= 0 {
                    let abs_info = unsafe { abs_info.assume_init() };
                    axes.push(InputAxisMetadata {
                        code: u32::from(axis_code),
                        minimum: f64::from(abs_info.minimum),
                        maximum: f64::from(abs_info.maximum),
                        flat: f64::from(abs_info.flat),
                        fuzz: f64::from(abs_info.fuzz),
                        resolution: f64::from(abs_info.resolution),
                    });
                    continue;
                }

                axes.push(InputAxisMetadata {
                    code: u32::from(axis_code),
                    minimum: 0.0,
                    maximum: 0.0,
                    flat: 0.0,
                    fuzz: 0.0,
                    resolution: 0.0,
                });
            }
        }
    }

    // collect button tables from EV_KEY capabilities
    let mut buttons = Vec::new();
    if has_key {
        let mut key_bits = [0u8; MAX_KEY_BITS];
        let key_status = unsafe {
            libc::ioctl(
                descriptor,
                eviocgbit_request(EV_KEY, key_bits.len()),
                key_bits.as_mut_ptr(),
            )
        };
        if key_status >= 0 {
            for code in 0..(key_bits.len() * 8) {
                if !bit_is_set(&key_bits, code) || !is_button_code(code) {
                    continue;
                }

                buttons.push(InputButtonMetadata {
                    code: code as u32,
                    analog: false,
                });
            }
        }
    }

    let axis_metadata_fidelity = if axes.is_empty() {
        InputCapabilityMetadataFidelity::Minimal
    } else if has_absolute {
        InputCapabilityMetadataFidelity::Partial
    } else {
        InputCapabilityMetadataFidelity::Full
    };
    let button_metadata_fidelity = if buttons.is_empty() {
        InputCapabilityMetadataFidelity::Minimal
    } else {
        InputCapabilityMetadataFidelity::Full
    };

    InputDeviceCapabilities {
        kinds: binding.store_array(kinds),
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        metadata_origin: InputCapabilityMetadataOrigin::BackendDescriptor,
        axis_metadata_fidelity,
        button_metadata_fidelity,
        supports_relative_pointer: has_relative
            && matches!(device_kind, InputDeviceKind::Mouse | InputDeviceKind::Pen),
        supports_pointer_grab: false,
        supports_pointer_capture: false,
        supports_pointer_warp: false,
        supports_text_input: supports_text,
        supports_composition: false,
        supports_rumble,
        supports_trigger_rumble: false,
        supports_sensors: has_sensors,
        supports_battery_state: supports_battery,
        supports_light_control: supports_light,
        supports_raw_hid,
        supports_player_index: device_kind == InputDeviceKind::Gamepad,
    }
}

/// Query Linux evdev metadata for one device path.
fn query_device_metadata(path: &str, fallback_name: &str) -> InputDeviceMetadata {
    let mut metadata = InputDeviceMetadata {
        name: fallback_name.to_string(),
        instance_id: path.to_string(),
        hardware_id: path.to_string(),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
        key_count: 0,
        button_count: 0,
        axis_count: 0,
        supports_rumble: false,
        supports_raw_hid: false,
        transport: "evdev",
    };

    let Ok(path_cstring) = CString::new(path) else {
        return metadata;
    };

    let descriptor = unsafe {
        libc::open(
            path_cstring.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return metadata;
    }

    let mut name_bytes = [0u8; 256];
    let name_status = unsafe {
        libc::ioctl(
            descriptor,
            eviocgname_request(name_bytes.len()),
            name_bytes.as_mut_ptr(),
        )
    };
    if name_status > 0 {
        let raw_len = name_status as usize;
        let trunc_len = raw_len.min(name_bytes.len());
        let zero_index = name_bytes[..trunc_len]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(trunc_len);
        if zero_index > 0 {
            let name = String::from_utf8_lossy(&name_bytes[..zero_index]).to_string();
            if !name.is_empty() {
                metadata.name = name;
            }
        }
    }

    if let Some(instance_id) = query_device_string(descriptor, eviocguniq_request(256))
        && !instance_id.is_empty()
    {
        metadata.instance_id = instance_id;
    }
    if let Some(hardware_id) = query_device_string(descriptor, eviocgphys_request(256))
        && !hardware_id.is_empty()
    {
        metadata.hardware_id = hardware_id;
    }

    let mut input_id = MaybeUninit::<LinuxInputId>::uninit();
    let id_status = unsafe { libc::ioctl(descriptor, EVIOCGID_REQUEST, input_id.as_mut_ptr()) };
    if id_status == 0 {
        let input_id = unsafe { input_id.assume_init() };
        metadata.vendor_id = input_id.vendor;
        metadata.product_id = input_id.product;
    }

    metadata.kind = detect_device_kind(&metadata.name, descriptor);
    let (key_count, button_count, axis_count) = query_device_counts(descriptor);
    metadata.key_count = key_count;
    metadata.button_count = button_count;
    metadata.axis_count = axis_count;
    metadata.supports_rumble = supports_linux_rumble(descriptor);
    metadata.supports_raw_hid = false;

    unsafe {
        libc::close(descriptor);
    }

    metadata
}

/// Query Linux hidraw metadata for one device path.
fn query_hidraw_metadata(path: &str, fallback_name: &str) -> InputDeviceMetadata {
    let mut metadata = InputDeviceMetadata {
        name: fallback_name.to_string(),
        instance_id: path.to_string(),
        hardware_id: path.to_string(),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
        key_count: 0,
        button_count: 0,
        axis_count: 0,
        supports_rumble: false,
        supports_raw_hid: true,
        transport: "hidraw",
    };

    let Ok(path_cstring) = CString::new(path) else {
        return metadata;
    };

    let descriptor = unsafe {
        libc::open(
            path_cstring.as_ptr(),
            libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    let descriptor = if descriptor >= 0 {
        descriptor
    } else {
        let errno = core_platform::get_errno();
        if errno != libc::EACCES && errno != libc::EPERM {
            return metadata;
        }

        unsafe {
            libc::open(
                path_cstring.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        }
    };
    if descriptor < 0 {
        return metadata;
    }

    if let Some(name) = query_hidraw_string(descriptor, hidiocgrawname_request(256))
        && !name.is_empty()
    {
        metadata.name = name;
    }

    let mut hidraw_info = MaybeUninit::<LinuxHidrawDevInfo>::uninit();
    let info_status = unsafe {
        libc::ioctl(
            descriptor,
            hidiocgrawinfo_request(),
            hidraw_info.as_mut_ptr(),
        )
    };
    if info_status == 0 {
        let hidraw_info = unsafe { hidraw_info.assume_init() };
        metadata.vendor_id = hidraw_info.vendor;
        metadata.product_id = hidraw_info.product;
        metadata.hardware_id = format!(
            "hid:{:04x}:{:04x}:{:04x}",
            hidraw_info.bustype, hidraw_info.vendor, hidraw_info.product
        );
    }

    metadata.kind = detect_device_kind_from_name(&metadata.name);
    metadata.supports_raw_hid = true;

    unsafe {
        libc::close(descriptor);
    }

    metadata
}

/// Query one null-terminated UTF-8 metadata string from one evdev ioctl.
fn query_device_string(descriptor: RawFd, request: libc::c_ulong) -> Option<String> {
    let mut buffer = [0u8; 256];
    let status = unsafe { libc::ioctl(descriptor, request, buffer.as_mut_ptr()) };
    if status <= 0 {
        return None;
    }

    let raw_len = status as usize;
    let trunc_len = raw_len.min(buffer.len());
    let zero_index = buffer[..trunc_len]
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(trunc_len);
    if zero_index == 0 {
        return None;
    }

    Some(String::from_utf8_lossy(&buffer[..zero_index]).to_string())
}

/// Query one null-terminated UTF-8 metadata string from one hidraw ioctl.
fn query_hidraw_string(descriptor: RawFd, request: libc::c_ulong) -> Option<String> {
    let mut buffer = [0u8; 256];
    let status = unsafe { libc::ioctl(descriptor, request, buffer.as_mut_ptr()) };
    if status <= 0 {
        return None;
    }

    let raw_len = status as usize;
    let trunc_len = raw_len.min(buffer.len());
    let zero_index = buffer[..trunc_len]
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(trunc_len);
    if zero_index == 0 {
        return None;
    }

    Some(String::from_utf8_lossy(&buffer[..zero_index]).to_string())
}

/// List Linux `/dev/input` event node paths.
fn list_linux_device_paths() -> RuntimeResult<Vec<String>> {
    let entries = match fs::read_dir(INPUT_DEVICE_DIRECTORY) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(Vec::new());
            }

            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                error.raw_os_error(),
                Some("read_dir".to_string()),
                Some(INPUT_DEVICE_DIRECTORY.to_string()),
                format!("failed to read {INPUT_DEVICE_DIRECTORY}: {error}"),
            ))
            .boxed());
        }
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            RuntimeError::from(PlatformError::io_with(
                None,
                None,
                error.raw_os_error(),
                Some("read_dir".to_string()),
                Some(INPUT_DEVICE_DIRECTORY.to_string()),
                format!("failed to read directory entry: {error}"),
            ))
            .boxed()
        })?;

        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(INPUT_EVENT_PREFIX) {
            continue;
        }

        let path = format!("{INPUT_DEVICE_DIRECTORY}/{name}");
        if Path::new(&path).exists() {
            paths.push(path);
        }
    }

    paths.sort_unstable();
    Ok(paths)
}

/// List Linux `/dev/hidraw*` node paths.
fn list_linux_hidraw_paths() -> RuntimeResult<Vec<String>> {
    let entries = match fs::read_dir(INPUT_HIDRAW_DIRECTORY) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(Vec::new());
            }

            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                error.raw_os_error(),
                Some("read_dir".to_string()),
                Some(INPUT_HIDRAW_DIRECTORY.to_string()),
                format!("failed to read {INPUT_HIDRAW_DIRECTORY}: {error}"),
            ))
            .boxed());
        }
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            RuntimeError::from(PlatformError::io_with(
                None,
                None,
                error.raw_os_error(),
                Some("read_dir".to_string()),
                Some(INPUT_HIDRAW_DIRECTORY.to_string()),
                format!("failed to read directory entry: {error}"),
            ))
            .boxed()
        })?;

        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !is_linux_node_name(name, INPUT_HIDRAW_PREFIX) {
            continue;
        }

        let path = format!("{INPUT_HIDRAW_DIRECTORY}/{name}");
        if Path::new(&path).exists() {
            paths.push(path);
        }
    }

    paths.sort_unstable();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use crate::platform::input::host::unix::linux::{
        ABS_RY, ABS_Z, BTN_GAMEPAD_START, BTN_MOUSE_START, BTN_TRIGGER_HAPPY_END, EV_ABS, EV_KEY,
        EV_REL, EV_SYN, KEY_CAPSLOCK, KEY_LEFTSHIFT, LinuxInputEvent, MODIFIER_CAPS_LOCK,
        MODIFIER_SHIFT, REL_WHEEL, input_event_kind, is_button_code, map_linux_event,
        should_skip_linux_event, update_linux_modifiers,
    };
    use crate::platform::input::{InputDeviceKind, InputEvent, InputEventAction, InputEventKind};
    use crate::tests::runtime::TestRuntime;

    /// Map Linux wheel events with scroll action semantics.
    #[test]
    fn test_map_linux_event_sets_scroll_action_for_wheel_codes() {
        let runtime = TestRuntime::deterministic_random();
        runtime.with_native_call_context(|context| {
            let raw = LinuxInputEvent {
                time: libc::timeval {
                    tv_sec: 1,
                    tv_usec: 0,
                },
                kind: EV_REL,
                code: REL_WHEEL,
                value: 1,
            };

            let event = map_linux_event(
                context,
                raw,
                "/dev/input/event0",
                InputDeviceKind::Mouse,
                0,
                0,
            );

            match event {
                InputEvent::InputScrollEvent(event) => {
                    assert_eq!(event.payload.wheel_y, 1.0);
                    assert_eq!(event.payload.wheel_x, 0.0);
                }
                _ => {
                    panic!("expected scroll event");
                }
            }
        });
    }

    /// Skip Linux EV_SYN packets from semantic input streams.
    #[test]
    fn test_should_skip_linux_event_for_sync_packets() {
        let event = LinuxInputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind: EV_SYN,
            code: 0,
            value: 0,
        };
        assert!(should_skip_linux_event(&event));
    }

    /// Keep Linux key packets in semantic input streams.
    #[test]
    fn test_should_skip_linux_event_for_key_packets() {
        let event = LinuxInputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind: EV_KEY,
            code: 30,
            value: 1,
        };
        assert!(!should_skip_linux_event(&event));
    }

    /// Classify EV_KEY codes by BTN namespace ranges.
    #[test]
    fn test_is_button_code_uses_btn_ranges_only() {
        assert!(is_button_code(BTN_MOUSE_START));
        assert!(is_button_code(BTN_GAMEPAD_START));
        assert!(is_button_code(BTN_TRIGGER_HAPPY_END));

        // KEY_OK is one non-button EV_KEY code above 0x100
        assert!(!is_button_code(0x160));
    }

    /// Track hold-style modifier bits across press and release transitions.
    #[test]
    fn test_update_linux_modifiers_tracks_hold_keys() {
        let mut modifiers = 0u32;
        let mut event = LinuxInputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind: EV_KEY,
            code: KEY_LEFTSHIFT,
            value: 1,
        };

        // shift press sets the shift bit
        modifiers = update_linux_modifiers(modifiers, &event);
        assert_eq!(modifiers & MODIFIER_SHIFT, MODIFIER_SHIFT);

        // shift release clears the shift bit
        event.value = 0;
        modifiers = update_linux_modifiers(modifiers, &event);
        assert_eq!(modifiers & MODIFIER_SHIFT, 0);
    }

    /// Toggle lock-style modifier bits on press edges only.
    #[test]
    fn test_update_linux_modifiers_toggles_lock_bits_on_press() {
        let mut modifiers = 0u32;
        let mut event = LinuxInputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind: EV_KEY,
            code: KEY_CAPSLOCK,
            value: 1,
        };

        // first press toggles caps-lock on
        modifiers = update_linux_modifiers(modifiers, &event);
        assert_eq!(modifiers & MODIFIER_CAPS_LOCK, MODIFIER_CAPS_LOCK);

        // release keeps lock state unchanged
        event.value = 0;
        modifiers = update_linux_modifiers(modifiers, &event);
        assert_eq!(modifiers & MODIFIER_CAPS_LOCK, MODIFIER_CAPS_LOCK);

        // second press toggles caps-lock off
        event.value = 1;
        modifiers = update_linux_modifiers(modifiers, &event);
        assert_eq!(modifiers & MODIFIER_CAPS_LOCK, 0);
    }

    /// Classify raw absolute-axis sensor lanes as sensor events.
    #[test]
    fn test_input_event_kind_maps_raw_sensor_axes_to_sensor_kind() {
        assert_eq!(
            input_event_kind(EV_ABS, ABS_Z, InputDeviceKind::Raw),
            InputEventKind::Sensor
        );
        assert_eq!(
            input_event_kind(EV_ABS, ABS_RY, InputDeviceKind::Raw),
            InputEventKind::Sensor
        );
    }

    /// Map sensor-axis packets into sensor payload coordinates.
    #[test]
    fn test_map_linux_event_sets_sensor_axis_payload() {
        let runtime = TestRuntime::deterministic_random();
        runtime.with_native_call_context(|context| {
            let raw = LinuxInputEvent {
                time: libc::timeval {
                    tv_sec: 1,
                    tv_usec: 0,
                },
                kind: EV_ABS,
                code: ABS_Z,
                value: 7,
            };

            let event = map_linux_event(
                context,
                raw,
                "/dev/input/event0",
                InputDeviceKind::Raw,
                0,
                0,
            );

            match event {
                InputEvent::InputSensorEvent(event) => {
                    assert_eq!(event.payload.action, InputEventAction::Axis);
                    assert_eq!(event.payload.x, 0.0);
                    assert_eq!(event.payload.y, 0.0);
                    assert_eq!(event.payload.z, 7.0);
                }
                _ => {
                    panic!("expected sensor event");
                }
            }
        });
    }
}

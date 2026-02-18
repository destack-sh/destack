use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;
use std::path::Path;

use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputDeviceInfo, InputDeviceKind, InputEvent, InputEventAction, InputEventKind,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Linux input-device directory path.
pub(super) const INPUT_DEVICE_DIRECTORY: &str = "/dev/input";
/// Prefix for Linux evdev event nodes.
pub(super) const INPUT_EVENT_PREFIX: &str = "event";
/// Linux EV_SYN event kind.
const EV_SYN: u16 = 0x00;
/// Linux EV_KEY event kind.
const EV_KEY: u16 = 0x01;
/// Linux EV_REL event kind.
const EV_REL: u16 = 0x02;
/// Linux EV_ABS event kind.
const EV_ABS: u16 = 0x03;
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
/// Linux EVIOCGID ioctl request number.
const EVIOCGID_REQUEST: libc::c_ulong =
    ior_request(b'E', 0x02, std::mem::size_of::<LinuxInputId>());
/// Linux EVIOCGRAB ioctl request number.
const EVIOCGRAB_REQUEST: libc::c_ulong =
    iow_request(b'E', 0x90, std::mem::size_of::<libc::c_int>());
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

/// Device metadata collected from Linux evdev descriptors.
#[derive(Clone)]
struct InputDeviceMetadata {
    /// Host-visible device name.
    name: String,
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

/// Build EVIOCGBIT request number for one event kind and buffer length.
fn eviocgbit_request(event: u16, length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x20 + event as u8, length)
}

/// Normalize one Linux input id into one absolute evdev path.
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

    if id.starts_with("/dev/input/event") {
        return Ok(id.to_string());
    }

    if id.starts_with(INPUT_EVENT_PREFIX) {
        return Ok(format!("{INPUT_DEVICE_DIRECTORY}/{id}"));
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one /dev/input/event path or event node name",
    ))
    .boxed())
}

/// Enumerate Linux evdev devices and map them into runtime metadata.
pub(super) fn list_linux_devices(
    context: &RuntimeCallContext,
) -> RuntimeResult<Vec<InputDeviceInfo>> {
    let paths = list_linux_device_paths()?;
    let mut devices = Vec::with_capacity(paths.len());
    for path in paths {
        let fallback_name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(INPUT_EVENT_PREFIX);
        let metadata = query_device_metadata(&path, fallback_name);

        devices.push(InputDeviceInfo {
            id: context.store_string(&path),
            name: context.store_string(&metadata.name),
            kind: metadata.kind,
            vendor_id: metadata.vendor_id,
            product_id: metadata.product_id,
            key_count: metadata.key_count,
            button_count: metadata.button_count,
            axis_count: metadata.axis_count,
            connected: true,
            supports_grab: true,
            supports_raw: true,
            supports_text: false,
            supports_rumble: false,
        });
    }

    Ok(devices)
}

/// Read one Linux evdev event from one descriptor.
pub(super) fn read_linux_event(
    context: &RuntimeCallContext,
    descriptor: RawFd,
    nonblocking: bool,
    device_id: &str,
    modifiers: u32,
) -> RuntimeResult<(InputEvent, u32)> {
    let mut modifiers_state = modifiers;

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

        // map one normalized event payload and return the updated modifier bitset
        let mapped_event = map_linux_event(context, event, device_id, modifiers_state);
        return Ok((mapped_event, modifiers_state));
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
                "destack.input.event.setGrab",
            ))
            .boxed());
        }

        return Err(core_platform::io_error("ioctl(EVIOCGRAB)", None));
    }

    Ok(())
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
fn input_event_kind(raw_kind: u16, code: u16) -> InputEventKind {
    match raw_kind {
        EV_KEY => {
            if code as usize == BTN_TOUCH {
                InputEventKind::Touch
            } else if (BTN_MOUSE_LEFT..=(BTN_MOUSE_LEFT + 2)).contains(&(code as usize)) {
                InputEventKind::PointerButton
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
            if code == ABS_X || code == ABS_Y {
                InputEventKind::PointerMotion
            } else {
                InputEventKind::Touch
            }
        }
        EV_SYN => InputEventKind::Device,
        _ => InputEventKind::Device,
    }
}

/// Map one Linux evdev payload into one runtime input event.
fn map_linux_event(
    context: &RuntimeCallContext,
    raw: LinuxInputEvent,
    device_id: &str,
    modifiers: u32,
) -> InputEvent {
    let kind = input_event_kind(raw.kind, raw.code);

    let mut x = 0.0_f64;
    let mut y = 0.0_f64;
    if raw.code == REL_X || raw.code == ABS_X {
        x = raw.value as f64;
    } else if raw.code == REL_Y || raw.code == ABS_Y {
        y = raw.value as f64;
    }

    InputEvent {
        kind,
        timestamp_ns: input_core::monotonic_timestamp_ns(),
        sequence: 0,
        device_id: context.store_string(device_id),
        action: match raw.kind {
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
            EV_ABS => InputEventAction::Move,
            _ => InputEventAction::Move,
        },
        code: raw.code as u32,
        scan_code: raw.code as u32,
        value: raw.value as i64,
        x,
        y,
        wheel_x: if raw.code == REL_HWHEEL {
            raw.value as f64
        } else {
            0.0
        },
        wheel_y: if raw.code == REL_WHEEL {
            raw.value as f64
        } else {
            0.0
        },
        modifiers,
        repeat: raw.value == 2,
        text: context.store_string(input_core::UNIX_INPUT_EMPTY_TEXT),
    }
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
fn classify_device_kind(name: &str, descriptor: RawFd) -> InputDeviceKind {
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

/// Query Linux evdev metadata for one device path.
fn query_device_metadata(path: &str, fallback_name: &str) -> InputDeviceMetadata {
    let mut metadata = InputDeviceMetadata {
        name: fallback_name.to_string(),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
        key_count: 0,
        button_count: 0,
        axis_count: 0,
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

    let mut input_id = MaybeUninit::<LinuxInputId>::uninit();
    let id_status = unsafe { libc::ioctl(descriptor, EVIOCGID_REQUEST, input_id.as_mut_ptr()) };
    if id_status == 0 {
        let input_id = unsafe { input_id.assume_init() };
        metadata.vendor_id = input_id.vendor;
        metadata.product_id = input_id.product;
    }

    metadata.kind = classify_device_kind(&metadata.name, descriptor);
    let (key_count, button_count, axis_count) = query_device_counts(descriptor);
    metadata.key_count = key_count;
    metadata.button_count = button_count;
    metadata.axis_count = axis_count;

    unsafe {
        libc::close(descriptor);
    }

    metadata
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

#[cfg(test)]
mod tests {
    use super::*;
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

            let event = map_linux_event(context, raw, "/dev/input/event0", 0);
            assert_eq!(event.kind, InputEventKind::Scroll);
            assert_eq!(event.action, InputEventAction::Scroll);
            assert_eq!(event.wheel_y, 1.0);
            assert_eq!(event.wheel_x, 0.0);
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
}

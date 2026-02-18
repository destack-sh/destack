use std::ffi::CString;
use std::os::unix::io::RawFd;

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::mem::MaybeUninit;
#[cfg(target_os = "linux")]
use std::path::Path;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputDeviceInfo, InputDeviceKind, InputEvent, InputEventKind};
use crate::platform::resource::{ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::RuntimeCallContext;

pub(super) const INPUT_RESOURCE_LABEL: &str = "input.device";
pub(super) const UNIX_INPUT_TTY_PATH: &str = "/dev/tty";
const UNIX_INPUT_TTY_ID: &str = "tty:stdin";
const UNIX_INPUT_TTY_ALIAS: &str = "tty";
const UNIX_INPUT_STDIN_ALIAS: &str = "stdin";
const UNIX_INPUT_TTY_NAME: &str = "unix terminal input";

#[cfg(target_os = "linux")]
const INPUT_DEVICE_DIRECTORY: &str = "/dev/input";
#[cfg(target_os = "linux")]
const INPUT_EVENT_PREFIX: &str = "event";
#[cfg(target_os = "linux")]
const EV_SYN: u16 = 0x00;
#[cfg(target_os = "linux")]
const EV_KEY: u16 = 0x01;
#[cfg(target_os = "linux")]
const EV_REL: u16 = 0x02;
#[cfg(target_os = "linux")]
const EV_ABS: u16 = 0x03;
#[cfg(target_os = "linux")]
const REL_X: u16 = 0x00;
#[cfg(target_os = "linux")]
const REL_Y: u16 = 0x01;
#[cfg(target_os = "linux")]
const REL_WHEEL: u16 = 0x08;
#[cfg(target_os = "linux")]
const REL_HWHEEL: u16 = 0x06;
#[cfg(target_os = "linux")]
const ABS_X: u16 = 0x00;
#[cfg(target_os = "linux")]
const ABS_Y: u16 = 0x01;
#[cfg(target_os = "linux")]
const KEY_A: usize = 30;
#[cfg(target_os = "linux")]
const BTN_MOUSE_LEFT: usize = 0x110;
#[cfg(target_os = "linux")]
const BTN_TOUCH: usize = 0x14a;
#[cfg(target_os = "linux")]
const BTN_STYLUS: usize = 0x14b;
#[cfg(target_os = "linux")]
const BTN_GAMEPAD: usize = 0x130;
#[cfg(target_os = "linux")]
const MAX_EVENT_BITS: usize = 64;
#[cfg(target_os = "linux")]
const MAX_KEY_BITS: usize = 256;
#[cfg(target_os = "linux")]
const EVIOCGID_REQUEST: libc::c_ulong =
    ior_request(b'E', 0x02, std::mem::size_of::<LinuxInputId>());
#[cfg(target_os = "linux")]
const EVIOCGRAB_REQUEST: libc::c_ulong =
    iow_request(b'E', 0x90, std::mem::size_of::<libc::c_int>());

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxInputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxInputEvent {
    time: libc::timeval,
    kind: u16,
    code: u16,
    value: i32,
}

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct InputDeviceMetadata {
    name: String,
    kind: InputDeviceKind,
    vendor_id: u16,
    product_id: u16,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum UnixInputBackend {
    #[cfg(target_os = "linux")]
    LinuxEvdev,
    UnixTerminal,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UnixInputBinding {
    pub(super) backend: UnixInputBackend,
}

#[derive(Debug, Clone)]
pub(super) struct UnixInputOpenSpec {
    pub(super) path: String,
    pub(super) backend: UnixInputBackend,
}

#[derive(Debug)]
pub(super) struct InputDeviceFinalizer {
    pub(super) fd: RawFd,
}

impl ResourceFinalizer for InputDeviceFinalizer {
    /// Close one input device descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[cfg(target_os = "linux")]
const fn ior_request(type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    ioc_request(2, type_byte, number, size)
}

#[cfg(target_os = "linux")]
const fn iow_request(type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    ioc_request(1, type_byte, number, size)
}

#[cfg(target_os = "linux")]
const fn ioc_request(direction: u8, type_byte: u8, number: u8, size: usize) -> libc::c_ulong {
    const IOC_NR_SHIFT: u64 = 0;
    const IOC_TYPE_SHIFT: u64 = IOC_NR_SHIFT + 8;
    const IOC_SIZE_SHIFT: u64 = IOC_TYPE_SHIFT + 8;
    const IOC_DIR_SHIFT: u64 = IOC_SIZE_SHIFT + 14;

    (((direction as u64) << IOC_DIR_SHIFT)
        | ((type_byte as u64) << IOC_TYPE_SHIFT)
        | ((number as u64) << IOC_NR_SHIFT)
        | ((size as u64) << IOC_SIZE_SHIFT)) as libc::c_ulong
}

#[cfg(target_os = "linux")]
fn eviocgname_request(length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x06, length)
}

#[cfg(target_os = "linux")]
fn eviocgbit_request(event: u16, length: usize) -> libc::c_ulong {
    ior_request(b'E', 0x20 + event as u8, length)
}

pub(super) fn input_not_found(
    operation: &'static str,
    handle: resource::InputDeviceHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input device handle {} not found", handle.0.0),
    ))
    .boxed()
}

pub(super) fn resolve_unix_input_binding(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<(RawFd, UnixInputBackend)> {
    let binding = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let descriptor = entry.fd()?;
        let backend = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())
            .map(|binding| binding.backend)
            .unwrap_or({
                #[cfg(target_os = "linux")]
                {
                    UnixInputBackend::LinuxEvdev
                }
                #[cfg(not(target_os = "linux"))]
                {
                    UnixInputBackend::UnixTerminal
                }
            });

        Some((descriptor, backend))
    });

    match binding.flatten() {
        Some(binding) => Ok(binding),
        None => Err(input_not_found(operation, handle)),
    }
}

#[cfg(target_os = "linux")]
fn normalize_input_path(id: &str) -> RuntimeResult<String> {
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

pub(super) fn normalize_unix_input_spec(id: &str) -> RuntimeResult<UnixInputOpenSpec> {
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

    let id_lower = id.to_ascii_lowercase();
    if id == UNIX_INPUT_TTY_PATH
        || id == UNIX_INPUT_TTY_ID
        || id_lower == UNIX_INPUT_TTY_ALIAS
        || id_lower == UNIX_INPUT_STDIN_ALIAS
    {
        return Ok(UnixInputOpenSpec {
            path: UNIX_INPUT_TTY_PATH.to_string(),
            backend: UnixInputBackend::UnixTerminal,
        });
    }

    #[cfg(target_os = "linux")]
    {
        let path = normalize_input_path(id)?;
        return Ok(UnixInputOpenSpec {
            path,
            backend: UnixInputBackend::LinuxEvdev,
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id must be one terminal alias (tty:stdin, tty, stdin, /dev/tty)",
        ))
        .boxed())
    }
}

pub(super) fn list_unix_devices(
    context: &RuntimeCallContext,
) -> RuntimeResult<Vec<InputDeviceInfo>> {
    #[cfg(target_os = "linux")]
    {
        let devices = list_linux_devices(context)?;
        if !devices.is_empty() {
            return Ok(devices);
        }
    }

    let tty_device = list_terminal_device(context)?;
    Ok(tty_device.into_iter().collect())
}

#[cfg(target_os = "linux")]
fn list_linux_devices(context: &RuntimeCallContext) -> RuntimeResult<Vec<InputDeviceInfo>> {
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
            connected: true,
        });
    }

    Ok(devices)
}

pub(super) fn open_input_descriptor(path: &str) -> RuntimeResult<RawFd> {
    let path_cstring = CString::new(path).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed()
    })?;

    let descriptor = unsafe { libc::open(path_cstring.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        return Err(core_platform::io_error("open", Some(path)));
    }

    Ok(descriptor)
}

pub(super) fn read_unix_event(
    descriptor: RawFd,
    backend: UnixInputBackend,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
) -> RuntimeResult<InputEvent> {
    match backend {
        #[cfg(target_os = "linux")]
        UnixInputBackend::LinuxEvdev => read_linux_event(descriptor, handle, nonblocking),
        UnixInputBackend::UnixTerminal => read_terminal_event(descriptor, handle, nonblocking),
    }
}

#[cfg(target_os = "linux")]
fn read_linux_event(
    descriptor: RawFd,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
) -> RuntimeResult<InputEvent> {
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

            return Err(core_platform::io_error("read", None));
        }

        read_offset += read_status as usize;
    }

    let event = unsafe { event.assume_init() };
    Ok(map_linux_event(event, handle))
}

pub(super) fn set_unix_grab(
    _descriptor: RawFd,
    backend: UnixInputBackend,
    _enable: bool,
) -> RuntimeResult<()> {
    match backend {
        #[cfg(target_os = "linux")]
        UnixInputBackend::LinuxEvdev => set_linux_grab(_descriptor, _enable),
        UnixInputBackend::UnixTerminal => Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.event.setGrab",
        ))
        .boxed()),
    }
}

#[cfg(target_os = "linux")]
fn set_linux_grab(descriptor: RawFd, enable: bool) -> RuntimeResult<()> {
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

fn list_terminal_device(context: &RuntimeCallContext) -> RuntimeResult<Option<InputDeviceInfo>> {
    let path = CString::new(UNIX_INPUT_TTY_PATH).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "terminal path contains nul byte",
        ))
        .boxed()
    })?;
    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOENT
            || errno == libc::ENOTTY
            || errno == libc::ENXIO
            || errno == libc::EACCES
            || errno == libc::EPERM
        {
            return Ok(None);
        }

        return Err(core_platform::io_error("open", Some(UNIX_INPUT_TTY_PATH)));
    }

    unsafe {
        libc::close(descriptor);
    }

    Ok(Some(InputDeviceInfo {
        id: context.store_string(UNIX_INPUT_TTY_ID),
        name: context.store_string(UNIX_INPUT_TTY_NAME),
        kind: InputDeviceKind::Keyboard,
        vendor_id: 0,
        product_id: 0,
        connected: true,
    }))
}

fn read_terminal_event(
    descriptor: RawFd,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
) -> RuntimeResult<InputEvent> {
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

    let mut byte = 0u8;
    loop {
        let read_status = unsafe {
            libc::read(
                descriptor,
                (&mut byte as *mut u8).cast::<libc::c_void>(),
                std::mem::size_of::<u8>(),
            )
        };
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

            return Err(core_platform::io_error("read", None));
        }

        break;
    }

    let kind = if byte.is_ascii_graphic() || matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
        InputEventKind::Text
    } else {
        InputEventKind::Key
    };
    let value = if kind == InputEventKind::Text {
        byte as i64
    } else {
        1
    };

    Ok(InputEvent {
        kind,
        timestamp_ns: current_unix_timestamp_ns(),
        device: handle,
        code: byte as u32,
        value,
        x: 0.0,
        y: 0.0,
        modifiers: 0,
    })
}

fn current_unix_timestamp_ns() -> u64 {
    let mut timestamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut timestamp) };
    if status < 0 || timestamp.tv_sec < 0 || timestamp.tv_nsec < 0 {
        return 0;
    }

    (timestamp.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(timestamp.tv_nsec as u64)
}

#[cfg(target_os = "linux")]
fn bit_is_set(bits: &[u8], index: usize) -> bool {
    let byte_index = index / 8;
    if byte_index >= bits.len() {
        return false;
    }

    let bit_mask = 1u8 << (index % 8);
    (bits[byte_index] & bit_mask) != 0
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn timestamp_ns(time: libc::timeval) -> u64 {
    if time.tv_sec < 0 || time.tv_usec < 0 {
        return 0;
    }

    let seconds = time.tv_sec as u64;
    let micros = time.tv_usec as u64;
    seconds
        .saturating_mul(1_000_000_000)
        .saturating_add(micros.saturating_mul(1_000))
}

#[cfg(target_os = "linux")]
fn map_linux_event(raw: LinuxInputEvent, device: resource::InputDeviceHandle) -> InputEvent {
    let kind = input_event_kind(raw.kind, raw.code);

    let mut x = 0.0_f64;
    let mut y = 0.0_f64;
    if raw.code == REL_X || raw.code == ABS_X {
        x = raw.value as f64;
    } else if raw.code == REL_Y
        || raw.code == ABS_Y
        || raw.code == REL_WHEEL
        || raw.code == REL_HWHEEL
    {
        y = raw.value as f64;
    }

    InputEvent {
        kind,
        timestamp_ns: timestamp_ns(raw.time),
        device,
        code: raw.code as u32,
        value: raw.value as i64,
        x,
        y,
        modifiers: 0,
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn query_device_metadata(path: &str, fallback_name: &str) -> InputDeviceMetadata {
    let mut metadata = InputDeviceMetadata {
        name: fallback_name.to_string(),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
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

    unsafe {
        libc::close(descriptor);
    }

    metadata
}

#[cfg(target_os = "linux")]
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

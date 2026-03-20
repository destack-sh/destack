use super::core::*;
use super::state::*;
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_vendor = "apple")]
use std::ffi::{CString, c_char, c_void};

#[cfg(target_vendor = "apple")]
use core_foundation_sys::base::{CFAllocatorRef, CFRelease, CFTypeRef};
#[cfg(target_vendor = "apple")]
use core_foundation_sys::number::{CFNumberGetValue, kCFNumberSInt16Type};
#[cfg(target_vendor = "apple")]
use core_foundation_sys::string::{
    CFStringCreateWithCString, CFStringGetCString, CFStringGetLength,
    CFStringGetMaximumSizeForEncoding, CFStringRef, kCFStringEncodingUTF8,
};

/// Cast one raw unix ioctl request into the host-specific request type.
pub(super) fn unix_ioctl_request(value: libc::c_ulong) -> UnixIoctlRequest {
    value as UnixIoctlRequest
}

/// Resolve one unix serial identifier into raw path bytes.
pub(super) fn decode_serial_id(id: &str) -> RuntimeResult<Vec<u8>> {
    // decode the byte-safe prefix when present
    if let Some(encoded_bytes) = id.strip_prefix(SERIAL_BYTES_ID_PREFIX) {
        if encoded_bytes.len() % 2 != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "encoded serial identifier has one malformed hex payload",
            ))
            .boxed());
        }

        let mut bytes = Vec::with_capacity(encoded_bytes.len() / 2);
        let raw = encoded_bytes.as_bytes();
        let mut index = 0usize;

        while index < raw.len() {
            let high = decode_hex_digit(raw[index], "id")?;
            let low = decode_hex_digit(raw[index + 1], "id")?;
            bytes.push((high << 4) | low);
            index += 2;
        }

        return Ok(bytes);
    }

    Ok(id.as_bytes().to_vec())
}

/// Encode one unix serial identifier from raw path bytes.
pub(super) fn encode_serial_id(path_bytes: &[u8]) -> String {
    // preserve plain utf8 identifiers directly
    if let Ok(path) = std::str::from_utf8(path_bytes) {
        return path.to_string();
    }

    // fall back to the byte-safe hex form
    let mut encoded = String::from(SERIAL_BYTES_ID_PREFIX);
    for byte in path_bytes {
        use std::fmt::Write;

        let _ = write!(&mut encoded, "{byte:02x}");
    }

    encoded
}

/// Decode one ASCII hex digit.
pub(super) fn decode_hex_digit(value: u8, field: &'static str) -> RuntimeResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(10 + value - b'a'),
        b'A'..=b'F' => Ok(10 + value - b'A'),
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "encoded serial identifier contains one non-hex digit",
        ))
        .boxed()),
    }
}

/// Candidate serial prefixes on Linux.
#[cfg(target_os = "linux")]
const LINUX_SERIAL_DEVICE_PREFIXES: &[&str] = &[
    "ttyS", "ttyUSB", "ttyACM", "ttyAMA", "ttyAP", "ttyXRUSB", "rfcomm",
];

/// Known Linux native-controller serial prefixes.
#[cfg(target_os = "linux")]
const LINUX_NATIVE_SERIAL_DEVICE_PREFIXES: &[&str] = &["ttyS", "ttyAMA", "ttyAP"];

/// Return whether one unix host path should be treated as one serial candidate.
#[cfg(target_os = "linux")]
pub(super) fn is_serial_device_name(name: &OsStr) -> bool {
    let name = name.as_bytes();

    LINUX_SERIAL_DEVICE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix.as_bytes()))
}

/// Return whether one unix host path names one known native serial controller.
fn is_likely_native_serial_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    let file_name = file_name.as_bytes();

    #[cfg(target_os = "linux")]
    {
        return LINUX_NATIVE_SERIAL_DEVICE_PREFIXES
            .iter()
            .any(|prefix| file_name.starts_with(prefix.as_bytes()));
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = file_name;

        false
    }
}

/// Classify one unix serial transport from one visible path.
pub(super) fn classify_serial_transport(path: &Path) -> SerialPortTransport {
    let lower = path.to_string_lossy().to_ascii_lowercase();

    // usb serial naming conventions
    if lower.contains("usb") {
        return SerialPortTransport::Usb;
    }

    // bluetooth serial naming conventions
    if lower.contains("bluetooth") || lower.contains("rfcomm") {
        return SerialPortTransport::Bluetooth;
    }

    // known native controller paths
    if is_likely_native_serial_path(path) {
        return SerialPortTransport::Native;
    }

    SerialPortTransport::Unknown
}

/// Validate unix serial open options before touching the host.
pub(super) fn validate_open_options(options: &SerialPortOpenOptions) -> RuntimeResult<()> {
    // unix serial backends do not expose portable queue-size tuning
    if options.read_buffer_size.is_some() || options.write_buffer_size.is_some() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.device.serial.open")).boxed(),
        );
    }

    Ok(())
}

/// Null master-port value used by current IOKit matching entry points.
#[cfg(target_vendor = "apple")]
const APPLE_IOKIT_MASTER_PORT_DEFAULT: u32 = 0;

/// Successful IOKit result code.
#[cfg(target_vendor = "apple")]
const APPLE_KERN_SUCCESS: i32 = 0;

/// One IOKit object handle.
#[cfg(target_vendor = "apple")]
type AppleIoObject = u32;

/// One retained apple corefoundation payload.
#[cfg(target_vendor = "apple")]
struct AppleCfValue(CFTypeRef);

#[cfg(target_vendor = "apple")]
impl Drop for AppleCfValue {
    /// Release the retained CoreFoundation value.
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

/// One retained apple serial service.
#[cfg(target_vendor = "apple")]
struct AppleIoService(AppleIoObject);

#[cfg(target_vendor = "apple")]
impl Drop for AppleIoService {
    /// Release the retained IOKit object.
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}

/// One retained apple iterator.
#[cfg(target_vendor = "apple")]
struct AppleIoIterator(AppleIoObject);

#[cfg(target_vendor = "apple")]
impl Drop for AppleIoIterator {
    /// Release the retained iterator object.
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}

// link apple serial discovery entry points
#[cfg(target_vendor = "apple")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    /// Create one matching dictionary for one IOKit service class.
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    /// Resolve one iterator over services matching one dictionary.
    fn IOServiceGetMatchingServices(
        master_port: u32,
        matching: *mut c_void,
        iterator: *mut AppleIoObject,
    ) -> i32;
    /// Advance one iterator and return the next object.
    fn IOIteratorNext(iterator: AppleIoObject) -> AppleIoObject;
    /// Release one IOKit object.
    fn IOObjectRelease(object: AppleIoObject) -> i32;
    /// Return one CoreFoundation property value from one registry entry.
    fn IORegistryEntryCreateCFProperty(
        entry: AppleIoObject,
        key: CFStringRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> CFTypeRef;
    /// Resolve the parent registry entry in one named plane.
    fn IORegistryEntryGetParentEntry(
        entry: AppleIoObject,
        plane: *const c_char,
        parent: *mut AppleIoObject,
    ) -> i32;
    /// Copy one registry entry name into one fixed-size C buffer.
    fn IORegistryEntryGetName(entry: AppleIoObject, name: *mut c_char) -> i32;
    /// Copy one class name into one fixed-size C buffer.
    fn IOObjectGetClass(object: AppleIoObject, name: *mut c_char) -> i32;
}

/// Encode one static apple registry key into one retained `CFString`.
#[cfg(target_vendor = "apple")]
fn apple_cfstring(value: &str) -> Option<CFStringRef> {
    let value = CString::new(value).ok()?;

    Some(unsafe {
        CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), kCFStringEncodingUTF8)
    })
}

/// Copy one apple cfstring into one owned Rust string.
#[cfg(target_vendor = "apple")]
fn apple_cfstring_to_string(value: CFStringRef) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(value) };
    if length < 0 {
        return None;
    }

    let max_utf8 = unsafe { CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8) };
    if max_utf8 < 0 {
        return None;
    }

    let mut bytes = vec![0u8; max_utf8 as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            value,
            bytes.as_mut_ptr().cast(),
            bytes.len() as isize,
            kCFStringEncodingUTF8,
        )
    };
    if converted == 0 {
        return None;
    }

    let terminator = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());

    String::from_utf8(bytes[..terminator].to_vec()).ok()
}

/// Read one retained apple string property.
#[cfg(target_vendor = "apple")]
fn apple_string_property(entry: AppleIoObject, key: &str) -> Option<String> {
    let key = apple_cfstring(key)?;
    let value = unsafe { IORegistryEntryCreateCFProperty(entry, key, std::ptr::null(), 0) };

    unsafe {
        CFRelease(key.cast());
    }

    let value = AppleCfValue(value);

    apple_cfstring_to_string(value.0.cast())
}

/// Read one retained apple integer property.
#[cfg(target_vendor = "apple")]
fn apple_u16_property(entry: AppleIoObject, key: &str) -> Option<u16> {
    let key = apple_cfstring(key)?;
    let value = unsafe { IORegistryEntryCreateCFProperty(entry, key, std::ptr::null(), 0) };

    unsafe {
        CFRelease(key.cast());
    }

    let value = AppleCfValue(value);
    if value.0.is_null() {
        return None;
    }

    let mut number = 0u16;
    let success = unsafe {
        CFNumberGetValue(
            value.0.cast::<std::ffi::c_void>().cast::<_>(),
            kCFNumberSInt16Type,
            (&mut number as *mut u16).cast(),
        )
    };
    if !success {
        return None;
    }

    Some(number)
}

/// Read one fixed-size IOKit object class string.
#[cfg(target_vendor = "apple")]
fn apple_object_class(object: AppleIoObject) -> Option<String> {
    let mut buffer = [0i8; 128];
    let status = unsafe { IOObjectGetClass(object, buffer.as_mut_ptr()) };
    if status != APPLE_KERN_SUCCESS {
        return None;
    }

    let value = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };

    Some(value.to_string_lossy().into_owned())
}

/// Read one fixed-size IOKit registry entry name.
#[cfg(target_vendor = "apple")]
fn apple_object_name(object: AppleIoObject) -> Option<String> {
    let mut buffer = [0i8; 128];
    let status = unsafe { IORegistryEntryGetName(object, buffer.as_mut_ptr()) };
    if status != APPLE_KERN_SUCCESS {
        return None;
    }

    let value = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };

    Some(value.to_string_lossy().into_owned())
}

/// One usb metadata snapshot gathered from the apple registry tree.
#[cfg(target_vendor = "apple")]
struct AppleUsbMetadata {
    /// USB vendor string when available.
    manufacturer: Option<String>,
    /// USB product string when available.
    product: Option<String>,
    /// USB serial number when available.
    serial_number: Option<String>,
    /// USB vendor identifier when available.
    usb_vendor_id: Option<u16>,
    /// USB product identifier when available.
    usb_product_id: Option<u16>,
}

/// Walk one apple registry parent chain and read usb device metadata when present.
#[cfg(target_vendor = "apple")]
fn apple_usb_metadata(service: AppleIoObject) -> Option<AppleUsbMetadata> {
    let io_service_plane = CString::new("IOService").ok()?;
    let mut current = service;
    let mut retained_parents = Vec::new();

    loop {
        let current_class = apple_object_class(current)?;
        if current_class == "IOUSBHostDevice" || current_class == "IOUSBDevice" {
            return Some(AppleUsbMetadata {
                manufacturer: apple_string_property(current, "USB Vendor Name"),
                product: apple_object_name(current),
                serial_number: apple_string_property(current, "USB Serial Number"),
                usb_vendor_id: apple_u16_property(current, "idVendor"),
                usb_product_id: apple_u16_property(current, "idProduct"),
            });
        }

        let mut parent = 0u32;
        let status = unsafe {
            IORegistryEntryGetParentEntry(current, io_service_plane.as_ptr(), &mut parent)
        };
        if status != APPLE_KERN_SUCCESS || parent == 0 {
            return None;
        }

        retained_parents.push(AppleIoService(parent));
        current = parent;
    }
}

/// Enumerate apple serial services through `IOSerialBSDClient`.
#[cfg(target_vendor = "apple")]
fn apple_serial_services(operation: &'static str) -> RuntimeResult<Vec<AppleIoService>> {
    let class_name = CString::new("IOSerialBSDClient").map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "{operation}: failed to encode macos serial class name: {error}",
        )))
        .boxed()
    })?;

    let matching = unsafe { IOServiceMatching(class_name.as_ptr()) };
    if matching.is_null() {
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("IOServiceMatching")),
            String::from("failed to create macos serial matching dictionary"),
        ))
        .boxed());
    }

    let mut iterator = 0u32;
    let status = unsafe {
        IOServiceGetMatchingServices(APPLE_IOKIT_MASTER_PORT_DEFAULT, matching, &mut iterator)
    };
    if status != APPLE_KERN_SUCCESS {
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(status),
            Some(operation.to_string()),
            Some(String::from("IOServiceGetMatchingServices")),
            String::from("failed to enumerate macos serial services"),
        ))
        .boxed());
    }

    let iterator = AppleIoIterator(iterator);
    let mut services = Vec::new();

    loop {
        let service = unsafe { IOIteratorNext(iterator.0) };
        if service == 0 {
            break;
        }

        services.push(AppleIoService(service));
    }

    Ok(services)
}

/// Build one apple serial descriptor snapshot from one ioserialbsdclient entry.
#[cfg(target_vendor = "apple")]
fn apple_descriptor_info_from_service(service: AppleIoObject) -> Option<UnixSerialDescriptorInfo> {
    let callout_path = apple_string_property(service, "IOCalloutDevice")?;
    let callout_path = PathBuf::from(callout_path);
    let path_bytes = callout_path.as_os_str().as_bytes().to_vec();
    let id = encode_serial_id(&path_bytes);
    let name = callout_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| callout_path.to_string_lossy().into_owned());

    let usb_metadata = apple_usb_metadata(service);
    let transport = if usb_metadata.is_some() {
        SerialPortTransport::Usb
    } else {
        classify_serial_transport(&callout_path)
    };

    let manufacturer = usb_metadata
        .as_ref()
        .and_then(|metadata| metadata.manufacturer.clone());
    let product = usb_metadata
        .as_ref()
        .and_then(|metadata| metadata.product.clone());
    let serial_number = usb_metadata
        .as_ref()
        .and_then(|metadata| metadata.serial_number.clone());
    let usb_vendor_id = usb_metadata
        .as_ref()
        .and_then(|metadata| metadata.usb_vendor_id);
    let usb_product_id = usb_metadata
        .as_ref()
        .and_then(|metadata| metadata.usb_product_id);

    Some(UnixSerialDescriptorInfo {
        id,
        transport,
        name,
        manufacturer,
        product,
        serial_number,
        path_bytes,
        usb_vendor_id,
        usb_product_id,
        bluetooth_service_class_id: None,
    })
}

/// Build one apple serial descriptor snapshot through `IOSerialBSDClient`.
#[cfg(target_vendor = "apple")]
fn apple_serial_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, UnixSerialDescriptorInfo>> {
    let mut snapshot = BTreeMap::new();

    // enumerate the ioserialbsdclient namespace directly
    for service in apple_serial_services(operation)? {
        let Some(info) = apple_descriptor_info_from_service(service.0) else {
            continue;
        };

        snapshot.insert(info.id.clone(), info);
    }

    Ok(snapshot)
}

/// Build one stable unix serial descriptor snapshot keyed by identifier.
pub(super) fn serial_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, UnixSerialDescriptorInfo>> {
    // linux: use sysfs-backed enumeration
    #[cfg(target_os = "linux")]
    {
        linux_serial_descriptor_snapshot_at(
            Path::new("/sys"),
            Path::new("/dev"),
            Path::new("/dev/serial/by-id"),
        )
    }

    // apple: use ioserialbsdclient enumeration
    #[cfg(target_vendor = "apple")]
    {
        apple_serial_descriptor_snapshot(operation)
    }

    // other unix: keep the current filesystem-backed fallback
    #[cfg(all(
        unix,
        not(any(target_os = "linux", target_os = "macos")),
        not(target_vendor = "apple")
    ))]
    {
        let _ = operation;
        Ok(fallback_serial_descriptor_snapshot_at(Path::new("/dev")))
    }
}

/// Build one descriptor snapshot from one unix device path.
pub(super) fn descriptor_info_from_path(
    path: &Path,
    operation: &'static str,
) -> UnixSerialDescriptorInfo {
    // linux: enrich from sysfs when possible
    #[cfg(target_os = "linux")]
    {
        let _ = operation;
        linux_descriptor_info_from_path(path, Path::new("/sys"))
    }

    // apple: preserve the selected path while preferring the live iokit snapshot
    #[cfg(target_vendor = "apple")]
    {
        let snapshot = apple_serial_descriptor_snapshot(operation);
        let path_bytes = path.as_os_str().as_bytes().to_vec();
        let id = encode_serial_id(&path_bytes);

        if let Ok(snapshot) = snapshot
            && let Some(info) = snapshot.get(&id)
        {
            return info.clone();
        }

        fallback_descriptor_info_from_path(path)
    }

    #[cfg(all(
        unix,
        not(any(target_os = "linux", target_os = "macos")),
        not(target_vendor = "apple")
    ))]
    {
        let _ = operation;
        fallback_descriptor_info_from_path(path)
    }
}

/// Read one trimmed sysfs text value.
#[cfg(target_os = "linux")]
fn read_trimmed_file(path: &Path) -> RuntimeResult<Option<String>> {
    // skip absent optional metadata files
    if !path.exists() {
        return Ok(None);
    }

    // read the raw file and trim trailing whitespace
    let value = std::fs::read_to_string(path).map_err(|error| {
        let errno = error.raw_os_error();
        let platform_code = errno.and_then(crate::platform::diagnostic::io_error_code_from_errno);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            errno,
            None,
            None,
            format!(
                "failed to read linux serial metadata file {}: {error}",
                path.display(),
            ),
        ))
        .boxed()
    })?;
    let value = value.trim().to_string();

    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value))
}

/// Parse one trimmed hexadecimal sysfs value.
#[cfg(target_os = "linux")]
fn read_hex_u16(path: &Path) -> RuntimeResult<Option<u16>> {
    // parse the value as lowercase or uppercase hex
    let value = read_trimmed_file(path)?;
    let Some(value) = value else {
        return Ok(None);
    };

    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value.as_str());

    let parsed = u16::from_str_radix(value, 16).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to parse linux serial metadata file {} as hexadecimal u16: {error}",
            path.display(),
        )))
        .boxed()
    })?;

    Ok(Some(parsed))
}

/// Resolve one canonical path when the filesystem exposes one.
#[cfg(target_os = "linux")]
fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Return whether one sysfs directory belongs to one usb-backed serial device.
#[cfg(target_os = "linux")]
fn usb_device_directory(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);

    // walk toward the root until usb identity files appear
    while let Some(directory) = current {
        let vendor_path = directory.join("idVendor");
        let product_path = directory.join("idProduct");
        if vendor_path.exists() && product_path.exists() {
            return Some(directory.to_path_buf());
        }

        current = directory.parent();
    }

    None
}

/// Build one preferred device path map from `/dev/serial/by-id`.
#[cfg(target_os = "linux")]
fn linux_alias_map(by_id_root: &Path) -> HashMap<String, PathBuf> {
    let mut aliases = HashMap::new();

    // keep one deterministic alias per tty target
    let Ok(entries) = std::fs::read_dir(by_id_root) else {
        return aliases;
    };

    for entry in entries.flatten() {
        let alias_path = entry.path();
        let canonical = canonical_path(&alias_path);
        let Some(tty_name) = canonical.file_name() else {
            continue;
        };

        let tty_name = tty_name.to_string_lossy().into_owned();
        match aliases.get(&tty_name) {
            Some(existing) if existing <= &alias_path => {}
            _ => {
                aliases.insert(tty_name, alias_path);
            }
        }
    }

    aliases
}

/// Return whether one tty name belongs to one visible serial candidate.
#[cfg(target_os = "linux")]
fn is_linux_serial_tty_name(name: &str) -> bool {
    let name = name.as_bytes();

    LINUX_SERIAL_DEVICE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix.as_bytes()))
}

/// Build one linux descriptor snapshot from one sysfs tty entry.
#[cfg(target_os = "linux")]
fn linux_descriptor_info_for_tty(
    tty_name: &str,
    sys_root: &Path,
    dev_root: &Path,
    aliases: &HashMap<String, PathBuf>,
) -> RuntimeResult<Option<UnixSerialDescriptorInfo>> {
    // reject non-serial tty names early
    if !is_linux_serial_tty_name(tty_name) {
        return Ok(None);
    }

    // require a visible device node in the host namespace
    let device_path = dev_root.join(tty_name);
    if !device_path.exists() {
        return Ok(None);
    }

    // resolve the sysfs device directory for metadata lookup
    let tty_class_path = sys_root.join("class").join("tty").join(tty_name);
    let device_directory = canonical_path(&tty_class_path.join("device"));

    // choose the stable preferred host path first
    let preferred_path = aliases
        .get(tty_name)
        .cloned()
        .unwrap_or_else(|| device_path.clone());
    let path_bytes = preferred_path.as_os_str().as_bytes().to_vec();
    let id = encode_serial_id(&path_bytes);
    let name = preferred_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| preferred_path.to_string_lossy().into_owned());

    // derive transport and metadata from sysfs ancestry
    let usb_directory = usb_device_directory(&device_directory);
    let transport = if tty_name.starts_with("rfcomm") {
        SerialPortTransport::Bluetooth
    } else if usb_directory.is_some() {
        SerialPortTransport::Usb
    } else {
        classify_serial_transport(&preferred_path)
    };

    let manufacturer = usb_directory
        .as_ref()
        .map(|directory| read_trimmed_file(&directory.join("manufacturer")))
        .transpose()?
        .flatten();
    let product = usb_directory
        .as_ref()
        .map(|directory| read_trimmed_file(&directory.join("product")))
        .transpose()?
        .flatten();
    let serial_number = usb_directory
        .as_ref()
        .map(|directory| read_trimmed_file(&directory.join("serial")))
        .transpose()?
        .flatten();
    let usb_vendor_id = usb_directory
        .as_ref()
        .map(|directory| read_hex_u16(&directory.join("idVendor")))
        .transpose()?
        .flatten();
    let usb_product_id = usb_directory
        .as_ref()
        .map(|directory| read_hex_u16(&directory.join("idProduct")))
        .transpose()?
        .flatten();

    Ok(Some(UnixSerialDescriptorInfo {
        id,
        transport,
        name,
        manufacturer,
        product,
        serial_number,
        path_bytes,
        usb_vendor_id,
        usb_product_id,
        bluetooth_service_class_id: None,
    }))
}

/// Build one linux serial descriptor snapshot from explicit roots.
#[cfg(target_os = "linux")]
fn linux_serial_descriptor_snapshot_at(
    sys_root: &Path,
    dev_root: &Path,
    by_id_root: &Path,
) -> RuntimeResult<BTreeMap<String, UnixSerialDescriptorInfo>> {
    let mut snapshot = BTreeMap::new();
    let aliases = linux_alias_map(by_id_root);
    let tty_root = sys_root.join("class").join("tty");

    // enumerate the tty namespace from sysfs
    let entries = std::fs::read_dir(&tty_root).map_err(|error| {
        let errno = error.raw_os_error();
        let platform_code = errno.and_then(crate::platform::diagnostic::io_error_code_from_errno);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            errno,
            None,
            None,
            format!(
                "failed to read linux serial sysfs namespace {}: {error}",
                tty_root.display(),
            ),
        ))
        .boxed()
    })?;

    // enumerate the tty namespace deterministically
    let mut tty_names = Vec::new();
    for entry in entries.flatten() {
        let tty_name = entry.file_name();
        tty_names.push(tty_name.to_string_lossy().into_owned());
    }
    tty_names.sort();

    // build enriched descriptors from sysfs and host paths
    for tty_name in tty_names {
        let Some(info) = linux_descriptor_info_for_tty(&tty_name, sys_root, dev_root, &aliases)?
        else {
            continue;
        };

        snapshot.insert(info.id.clone(), info);
    }

    Ok(snapshot)
}

/// Build one linux descriptor snapshot from one selected host path.
#[cfg(target_os = "linux")]
fn linux_descriptor_info_from_path(path: &Path, sys_root: &Path) -> UnixSerialDescriptorInfo {
    // preserve the caller-selected path but canonicalize for metadata lookup
    let canonical = canonical_path(path);
    let tty_name = canonical
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| canonical.to_string_lossy().into_owned());
    let aliases = HashMap::new();
    let dev_root = Path::new("/dev");

    if let Ok(Some(info)) = linux_descriptor_info_for_tty(&tty_name, sys_root, dev_root, &aliases) {
        return UnixSerialDescriptorInfo {
            path_bytes: path.as_os_str().as_bytes().to_vec(),
            id: encode_serial_id(path.as_os_str().as_bytes()),
            name: path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            ..info
        };
    }

    fallback_descriptor_info_from_path(path)
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "macos")),
    not(target_vendor = "apple")
))]
/// Collect filesystem-backed unix serial candidate paths.
fn fallback_serial_paths_at(dev_root: &Path) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();

    // scan the device directory for conventional serial node prefixes
    let Ok(entries) = std::fs::read_dir(dev_root) else {
        return output;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        if !is_serial_device_name(name) {
            continue;
        }

        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.insert(canonical) {
            output.push(path);
        }
    }

    // keep the result deterministic for tests and replay
    output.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));

    output
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "macos")),
    not(target_vendor = "apple")
))]
/// Build one fallback unix serial descriptor snapshot.
fn fallback_serial_descriptor_snapshot_at(
    dev_root: &Path,
) -> BTreeMap<String, UnixSerialDescriptorInfo> {
    let mut snapshot = BTreeMap::new();

    // collect one stable descriptor snapshot from the current device paths
    for path in fallback_serial_paths_at(dev_root) {
        let info = fallback_descriptor_info_from_path(&path);

        snapshot.insert(info.id.clone(), info);
    }

    snapshot
}

/// Build one fallback descriptor snapshot from one unix device path.
fn fallback_descriptor_info_from_path(path: &Path) -> UnixSerialDescriptorInfo {
    let path_bytes = path.as_os_str().as_bytes().to_vec();
    let id = encode_serial_id(&path_bytes);
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    UnixSerialDescriptorInfo {
        id,
        transport: classify_serial_transport(path),
        name,
        manufacturer: None,
        product: None,
        serial_number: None,
        path_bytes,
        usb_vendor_id: None,
        usb_product_id: None,
        bluetooth_service_class_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::device::{
        SerialFlowControl, SerialParity, SerialPortConfig, SerialPortOpenOptions, SerialStopBits,
    };
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::assert_runtime_error_code;

    /// Build one default serial configuration for unix helper tests.
    fn default_serial_config() -> SerialPortConfig {
        SerialPortConfig {
            baud_rate: 115_200,
            data_bits: SerialDataBits::Eight,
            parity: SerialParity::None,
            stop_bits: SerialStopBits::One,
            flow_control: SerialFlowControl {
                request_to_send_clear_to_send_enabled: false,
                data_terminal_ready_data_set_ready_enabled: false,
                xon_xoff_enabled: false,
            },
        }
    }

    /// Build one throwaway temporary directory path.
    #[cfg(target_os = "linux")]
    fn temp_path(label: &str) -> PathBuf {
        let process_id = std::process::id();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        std::env::temp_dir().join(format!(
            "destack-runtime-serial-{label}-{process_id}-{nonce}"
        ))
    }

    /// Build one temporary filesystem sandbox for serial identity tests.
    #[cfg(target_os = "linux")]
    fn with_temp_root<T>(label: &str, callback: impl FnOnce(&Path) -> T) -> T {
        let root = temp_path(label);
        std::fs::create_dir_all(&root).unwrap();

        let result = callback(&root);

        let _ = std::fs::remove_dir_all(&root);

        result
    }

    /// Write one text file into the temporary serial fixture.
    #[cfg(target_os = "linux")]
    fn write_text_file(path: &Path, value: &str) {
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent).unwrap();
        std::fs::write(path, value).unwrap();
    }

    /// Create one empty file into the temporary serial fixture.
    #[cfg(target_os = "linux")]
    fn create_file(path: &Path) {
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent).unwrap();
        std::fs::write(path, []).unwrap();
    }

    /// Roundtrip one non-utf8 unix serial identifier through the byte-safe encoding.
    #[test]
    fn test_encode_serial_id_roundtrips_non_utf8_paths() {
        let path_bytes = b"/dev/tty\xffserial";
        let encoded = encode_serial_id(path_bytes);
        let decoded = decode_serial_id(&encoded).unwrap();

        assert_eq!(decoded, path_bytes);
    }

    /// Preserve plain utf8 serial identifiers without hex encoding.
    #[test]
    fn test_encode_serial_id_preserves_utf8_paths() {
        let path_bytes = b"/dev/ttyUSB0";
        let encoded = encode_serial_id(path_bytes);
        let decoded = decode_serial_id(&encoded).unwrap();

        assert_eq!(encoded, "/dev/ttyUSB0");
        assert_eq!(decoded, path_bytes);
    }

    /// Reject malformed hex payloads in byte-safe identifiers.
    #[test]
    fn test_decode_serial_id_rejects_malformed_hex_payloads() {
        assert!(decode_serial_id("serial-unix-bytes:0").is_err());
        assert!(decode_serial_id("serial-unix-bytes:0x").is_err());
    }

    /// Classify visible unix transports without overclaiming native controllers.
    #[test]
    fn test_classify_serial_transport_detects_usb_bluetooth_and_unknown() {
        let usb_path = Path::new("/dev/serial/by-id/usb-Audio_Interface");
        let bluetooth_path = Path::new("/dev/rfcomm0");
        let native_path = OsStr::from_bytes(b"/dev/ttyS0");
        let unknown_path = Path::new("/dev/cu.debug-console");

        #[cfg(target_os = "linux")]
        let native_transport = SerialPortTransport::Native;

        #[cfg(not(target_os = "linux"))]
        let native_transport = SerialPortTransport::Unknown;

        assert_eq!(
            classify_serial_transport(usb_path),
            SerialPortTransport::Usb
        );
        assert_eq!(
            classify_serial_transport(bluetooth_path),
            SerialPortTransport::Bluetooth
        );
        assert_eq!(
            classify_serial_transport(Path::new(native_path)),
            native_transport
        );
        assert_eq!(
            classify_serial_transport(unknown_path),
            SerialPortTransport::Unknown
        );
    }

    /// Reject unix queue-size hints loudly instead of accepting them silently.
    #[test]
    fn test_validate_open_options_rejects_queue_size_hints() {
        let options = SerialPortOpenOptions {
            config: default_serial_config(),
            exclusive: None,
            read_buffer_size: Some(4096),
            write_buffer_size: None,
        };
        let error = validate_open_options(&options).unwrap_err();

        assert_runtime_error_code(&error, PlatformErrorCode::NotSupported);
    }

    /// Build one linux usb serial descriptor from sysfs metadata and by-id aliases.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_snapshot_prefers_by_id_paths_and_reads_usb_metadata() {
        with_temp_root("linux-usb", |root| {
            let sys_root = root.join("sys");
            let dev_root = root.join("dev");
            let by_id_root = dev_root.join("serial").join("by-id");

            let tty_path = dev_root.join("ttyUSB0");
            let tty_class = sys_root.join("class").join("tty").join("ttyUSB0");
            let usb_device = sys_root
                .join("devices")
                .join("pci0000:00")
                .join("0000:00:14.0")
                .join("usb1")
                .join("1-1")
                .join("1-1:1.0");

            create_file(&tty_path);
            std::fs::create_dir_all(&tty_class).unwrap();
            std::fs::create_dir_all(usb_device.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&usb_device, tty_class.join("device")).unwrap();

            write_text_file(&usb_device.join("idVendor"), "1a86\n");
            write_text_file(&usb_device.join("idProduct"), "7523\n");
            write_text_file(&usb_device.join("manufacturer"), "Acme\n");
            write_text_file(&usb_device.join("product"), "Debug Cable\n");
            write_text_file(&usb_device.join("serial"), "SER123\n");

            std::fs::create_dir_all(&by_id_root).unwrap();
            std::os::unix::fs::symlink("../../ttyUSB0", by_id_root.join("usb-Acme_Debug_Cable"))
                .unwrap();

            let snapshot =
                linux_serial_descriptor_snapshot_at(&sys_root, &dev_root, &by_id_root).unwrap();
            let descriptor = snapshot.values().next().unwrap();

            assert_eq!(snapshot.len(), 1);
            assert_eq!(descriptor.transport, SerialPortTransport::Usb);
            assert_eq!(descriptor.name, "usb-Acme_Debug_Cable");
            assert_eq!(descriptor.manufacturer.as_deref(), Some("Acme"));
            assert_eq!(descriptor.product.as_deref(), Some("Debug Cable"));
            assert_eq!(descriptor.serial_number.as_deref(), Some("SER123"));
            assert_eq!(descriptor.usb_vendor_id, Some(0x1a86));
            assert_eq!(descriptor.usb_product_id, Some(0x7523));
            assert_eq!(
                PathBuf::from(OsStr::from_bytes(&descriptor.path_bytes)),
                by_id_root.join("usb-Acme_Debug_Cable"),
            );
        });
    }

    /// Classify rfcomm devices as bluetooth without requiring usb metadata.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_snapshot_classifies_rfcomm_as_bluetooth() {
        with_temp_root("linux-rfcomm", |root| {
            let sys_root = root.join("sys");
            let dev_root = root.join("dev");
            let by_id_root = dev_root.join("serial").join("by-id");

            create_file(&dev_root.join("rfcomm0"));
            std::fs::create_dir_all(sys_root.join("class").join("tty").join("rfcomm0")).unwrap();

            let snapshot =
                linux_serial_descriptor_snapshot_at(&sys_root, &dev_root, &by_id_root).unwrap();
            let descriptor = snapshot.values().next().unwrap();

            assert_eq!(snapshot.len(), 1);
            assert_eq!(descriptor.transport, SerialPortTransport::Bluetooth);
            assert_eq!(descriptor.name, "rfcomm0");
        });
    }

    /// Fail loudly when the linux sysfs tty namespace is unavailable.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_snapshot_fails_when_sysfs_tty_namespace_is_missing() {
        with_temp_root("linux-missing-sysfs", |root| {
            let sys_root = root.join("sys");
            let dev_root = root.join("dev");
            let by_id_root = dev_root.join("serial").join("by-id");

            let error =
                linux_serial_descriptor_snapshot_at(&sys_root, &dev_root, &by_id_root).unwrap_err();

            assert_runtime_error_code(&error, PlatformErrorCode::IoNotFound);
        });
    }
}

/// CoreAudio constant `K_NO_ERR`.
#[cfg(target_os = "macos")]
pub(super) const K_NO_ERR: OSStatus = 0;
/// CoreAudio constant `K_AUDIO_QUEUE_ERR_INVALID_RUN_STATE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_QUEUE_ERR_INVALID_RUN_STATE: OSStatus = -66678;
/// CoreAudio constant `K_AUDIO_OBJECT_SYSTEM_OBJECT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_SYSTEM_OBJECT: AudioObjectID = 1;
/// CoreAudio constant `K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: AudioObjectPropertyElement = 0;
/// CoreAudio constant `K_CF_STRING_ENCODING_UTF8`.
#[cfg(target_os = "macos")]
pub(super) const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;
/// CoreAudio constant `K_FALLBACK_SAMPLE_RATE`.
#[cfg(target_os = "macos")]
pub(super) const K_FALLBACK_SAMPLE_RATE: u32 = 48_000;
/// CoreAudio constant `K_FALLBACK_PERIOD_FRAMES`.
#[cfg(target_os = "macos")]
pub(super) const K_FALLBACK_PERIOD_FRAMES: u32 = 256;
/// CoreAudio constant `K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: AudioObjectPropertyScope = fourcc(*b"glob");
/// CoreAudio constant `K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT: AudioObjectPropertyScope = fourcc(*b"inpt");
/// CoreAudio constant `K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT: AudioObjectPropertyScope = fourcc(*b"outp");
/// CoreAudio constant `K_AUDIO_HARDWARE_PROPERTY_DEVICES`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_HARDWARE_PROPERTY_DEVICES: AudioObjectPropertySelector = fourcc(*b"dev#");
/// CoreAudio constant `K_AUDIO_HARDWARE_PROPERTY_HOG_MODE_IS_ALLOWED`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_HARDWARE_PROPERTY_HOG_MODE_IS_ALLOWED: AudioObjectPropertySelector =
    fourcc(*b"hogr");
/// CoreAudio constant `K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE: AudioObjectPropertySelector =
    fourcc(*b"dIn ");
/// CoreAudio constant `K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: AudioObjectPropertySelector =
    fourcc(*b"dOut");
/// CoreAudio constant `K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE:
    AudioObjectPropertySelector = fourcc(*b"sOut");
/// CoreAudio constant `K_AUDIO_OBJECT_PROPERTY_NAME`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_OBJECT_PROPERTY_NAME: AudioObjectPropertySelector = fourcc(*b"lnam");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_DEVICE_UID`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_DEVICE_UID: AudioObjectPropertySelector = fourcc(*b"uid ");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_MODEL_UID`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_MODEL_UID: AudioObjectPropertySelector = fourcc(*b"muid");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE: AudioObjectPropertySelector =
    fourcc(*b"livn");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE: AudioObjectPropertySelector =
    fourcc(*b"tran");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE: AudioObjectPropertySelector =
    fourcc(*b"nsrt");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES:
    AudioObjectPropertySelector = fourcc(*b"nsr#");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE: AudioObjectPropertySelector =
    fourcc(*b"fsiz");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE_RANGE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE_RANGE: AudioObjectPropertySelector =
    fourcc(*b"fsz#");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION: AudioObjectPropertySelector =
    fourcc(*b"slay");
/// CoreAudio constant `K_AUDIO_DEVICE_PROPERTY_HOG_MODE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_PROPERTY_HOG_MODE: AudioObjectPropertySelector = fourcc(*b"oink");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_BUILT_IN`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_BUILT_IN: u32 = fourcc(*b"bltn");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_AGGREGATE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_AGGREGATE: u32 = fourcc(*b"grup");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_VIRTUAL`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_VIRTUAL: u32 = fourcc(*b"virt");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_PCI`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_PCI: u32 = fourcc(*b"pci ");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_USB`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_USB: u32 = fourcc(*b"usb ");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_FIREWIRE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_FIREWIRE: u32 = fourcc(*b"1394");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH: u32 = fourcc(*b"blue");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH_LE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_BLUETOOTH_LE: u32 = fourcc(*b"blea");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_HDMI`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_HDMI: u32 = fourcc(*b"hdmi");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_DISPLAY_PORT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_DISPLAY_PORT: u32 = fourcc(*b"dprt");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_AIRPLAY`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_AIRPLAY: u32 = fourcc(*b"airp");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_AVB`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_AVB: u32 = fourcc(*b"eavb");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_THUNDERBOLT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_THUNDERBOLT: u32 = fourcc(*b"thun");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRED`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRED: u32 = fourcc(*b"ccwd");
/// CoreAudio constant `K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRELESS`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_DEVICE_TRANSPORT_TYPE_CONTINUITY_CAPTURE_WIRELESS: u32 = fourcc(*b"ccwl");
/// CoreAudio constant `K_AUDIO_FORMAT_LINEAR_PCM`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_FORMAT_LINEAR_PCM: u32 = fourcc(*b"lpcm");
/// CoreAudio constant `K_AUDIO_FORMAT_FLAG_IS_FLOAT`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_FORMAT_FLAG_IS_FLOAT: u32 = 1u32 << 0;
/// CoreAudio constant `K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER: u32 = 1u32 << 2;
/// CoreAudio constant `K_AUDIO_FORMAT_FLAG_IS_PACKED`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_FORMAT_FLAG_IS_PACKED: u32 = 1u32 << 3;
/// CoreAudio constant `K_AUDIO_QUEUE_PROPERTY_CURRENT_DEVICE`.
#[cfg(target_os = "macos")]
pub(super) const K_AUDIO_QUEUE_PROPERTY_CURRENT_DEVICE: AudioQueuePropertyID = fourcc(*b"aqcd");
/// CoreAudio constant `COREAUDIO_PLAYBACK_BUFFER_COUNT`.
#[cfg(target_os = "macos")]
pub(super) const COREAUDIO_PLAYBACK_BUFFER_COUNT: usize = 3;
/// CoreAudio constant `COREAUDIO_LOOPBACK_PROBE_FRAMES`.
#[cfg(target_os = "macos")]
pub(super) const COREAUDIO_LOOPBACK_PROBE_FRAMES: u32 = 128;

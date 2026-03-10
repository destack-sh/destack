#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use crate::platform::audio::core::model::AudioStreamHostState;

/// CoreAudio type alias `AudioObjectID`.
#[cfg(target_os = "macos")]
pub(super) type AudioObjectID = u32;
/// CoreAudio type alias `AudioDeviceID`.
#[cfg(target_os = "macos")]
pub(super) type AudioDeviceID = AudioObjectID;
/// CoreAudio type alias `AudioObjectPropertySelector`.
#[cfg(target_os = "macos")]
pub(super) type AudioObjectPropertySelector = u32;
/// CoreAudio type alias `AudioObjectPropertyScope`.
#[cfg(target_os = "macos")]
pub(super) type AudioObjectPropertyScope = u32;
/// CoreAudio type alias `AudioObjectPropertyElement`.
#[cfg(target_os = "macos")]
pub(super) type AudioObjectPropertyElement = u32;
/// CoreAudio type alias `OSStatus`.
#[cfg(target_os = "macos")]
pub(super) type OSStatus = i32;
/// CoreAudio type alias `CFStringRef`.
#[cfg(target_os = "macos")]
pub(super) type CFStringRef = *mut c_void;
/// CoreAudio type alias `CFTypeRef`.
#[cfg(target_os = "macos")]
pub(super) type CFTypeRef = *const c_void;
/// CoreAudio type alias `CFIndex`.
#[cfg(target_os = "macos")]
pub(super) type CFIndex = isize;
/// CoreAudio type alias `CFStringEncoding`.
#[cfg(target_os = "macos")]
pub(super) type CFStringEncoding = u32;
/// CoreAudio type alias `AudioQueueRef`.
#[cfg(target_os = "macos")]
pub(super) type AudioQueueRef = *mut c_void;
/// CoreAudio type alias `AudioQueuePropertyID`.
#[cfg(target_os = "macos")]
pub(super) type AudioQueuePropertyID = u32;
/// CoreAudio type alias `AudioQueueBufferRef`.
#[cfg(target_os = "macos")]
pub(super) type AudioQueueBufferRef = *mut AudioQueueBuffer;
/// CoreAudio type alias `AudioObjectPropertyListenerProc`.
#[cfg(target_os = "macos")]
pub(super) type AudioObjectPropertyListenerProc = unsafe extern "C" fn(
    AudioObjectID,
    u32,
    *const AudioObjectPropertyAddress,
    *mut c_void,
) -> OSStatus;

/// CoreAudio struct `AudioObjectPropertyAddress`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioObjectPropertyAddress {
    /// The property selector.
    pub(super) selector: AudioObjectPropertySelector,
    /// The property scope.
    pub(super) scope: AudioObjectPropertyScope,
    /// The property element.
    pub(super) element: AudioObjectPropertyElement,
}

/// CoreAudio struct `AudioBuffer`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioBuffer {
    /// The channel count for this buffer.
    pub(super) number_channels: u32,
    /// The data size in bytes.
    pub(super) data_byte_size: u32,
    /// The data pointer, null for configuration snapshots.
    pub(super) data: *mut c_void,
}

/// CoreAudio struct `AudioBufferList`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioBufferList {
    /// The number of buffers in this list.
    pub(super) number_buffers: u32,
    /// The first buffer entry, with trailing entries packed inline.
    pub(super) buffers: [AudioBuffer; 1],
}

/// CoreAudio struct `AudioValueRange`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioValueRange {
    /// The minimum value in this range.
    pub(super) minimum: f64,
    /// The maximum value in this range.
    pub(super) maximum: f64,
}

/// CoreAudio struct `SMPTETime`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct SMPTETime {
    /// The SMPTE subframes.
    pub(super) subframes: i16,
    /// The SMPTE subframe divisor.
    pub(super) subframe_divisor: i16,
    /// The SMPTE counter.
    pub(super) counter: u32,
    /// The SMPTE type.
    pub(super) r#type: u32,
    /// The SMPTE flags.
    pub(super) flags: u32,
    /// The SMPTE hours.
    pub(super) hours: i16,
    /// The SMPTE minutes.
    pub(super) minutes: i16,
    /// The SMPTE seconds.
    pub(super) seconds: i16,
    /// The SMPTE frames.
    pub(super) frames: i16,
}

/// CoreAudio struct `AudioTimeStamp`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioTimeStamp {
    /// The sample-time value.
    pub(super) sample_time: f64,
    /// The host-time value.
    pub(super) host_time: u64,
    /// The rate scalar.
    pub(super) rate_scalar: f64,
    /// The word clock time.
    pub(super) word_clock_time: u64,
    /// The SMPTE time payload.
    pub(super) smpte_time: SMPTETime,
    /// The timestamp validity flags.
    pub(super) flags: u32,
    /// Reserved field.
    pub(super) reserved: u32,
}

/// CoreAudio struct `AudioStreamBasicDescription`.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct AudioStreamBasicDescription {
    /// The sample rate in hertz.
    pub(super) sample_rate: f64,
    /// The stream format identifier.
    pub(super) format_id: u32,
    /// The stream format flags.
    pub(super) format_flags: u32,
    /// The bytes per packet.
    pub(super) bytes_per_packet: u32,
    /// The frames per packet.
    pub(super) frames_per_packet: u32,
    /// The bytes per frame.
    pub(super) bytes_per_frame: u32,
    /// The channels per frame.
    pub(super) channels_per_frame: u32,
    /// The bits per channel.
    pub(super) bits_per_channel: u32,
    /// Reserved field.
    pub(super) reserved: u32,
}

/// CoreAudio struct `AudioQueueBuffer`.
#[cfg(target_os = "macos")]
#[repr(C)]
pub(super) struct AudioQueueBuffer {
    /// The buffer capacity in bytes.
    pub(super) audio_data_bytes_capacity: u32,
    /// The mutable audio data pointer.
    pub(super) audio_data: *mut c_void,
    /// The active audio byte count.
    pub(super) audio_data_byte_size: u32,
    /// User data payload.
    pub(super) user_data: *mut c_void,
    /// Packet description capacity.
    pub(super) packet_description_capacity: u32,
    /// Packet description pointer.
    pub(super) packet_descriptions: *mut c_void,
    /// Packet description count.
    pub(super) packet_description_count: u32,
}

/// CoreAudio struct `CoreAudioStreamContext`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioStreamContext {
    /// Shared stream host state payload.
    pub(super) stream: Arc<AudioStreamHostState>,
}

/// One retained callback-context token owned by a queue.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CoreAudioCallbackContextToken(pub(super) usize);

/// CoreAudio struct `CoreAudioQueueHandle`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioQueueHandle {
    /// One CoreAudio queue reference.
    pub(super) queue: AudioQueueRef,
    /// One strong owner reference held by runtime control flow.
    pub(super) _context_owner: Arc<CoreAudioStreamContext>,
    /// One retained callback context token owned by the queue.
    pub(super) callback_context_token: CoreAudioCallbackContextToken,
}

#[cfg(target_os = "macos")]
unsafe impl Send for CoreAudioQueueHandle {}

/// CoreAudio struct `CoreAudioStreamRuntime`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioStreamRuntime {
    /// Active queue handles for this stream.
    pub(super) queue_handles: Mutex<Vec<CoreAudioQueueHandle>>,
    /// Device identifier used for optional hog-mode release.
    pub(super) device_id: AudioDeviceID,
    /// Whether this stream acquired hog mode and must release it.
    pub(super) release_hog_mode_on_drop: bool,
}

/// CoreAudio struct `CoreAudioHostStreamOps`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioHostStreamOps {
    /// Shared queue runtime state for this stream.
    pub(super) runtime: Arc<CoreAudioStreamRuntime>,
}
#[cfg(target_os = "macos")]
#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    /// Return whether an object has a property address.
    pub(super) fn AudioObjectHasProperty(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
    ) -> u8;

    /// Return the property payload byte size for an address.
    pub(super) fn AudioObjectGetPropertyDataSize(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        out_data_size: *mut u32,
    ) -> OSStatus;

    /// Return a property payload for an address.
    pub(super) fn AudioObjectGetPropertyData(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> OSStatus;

    /// Set a property payload for an address.
    pub(super) fn AudioObjectSetPropertyData(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        in_data_size: u32,
        in_data: *const c_void,
    ) -> OSStatus;

    /// Add a property-change listener callback for an address.
    pub(super) fn AudioObjectAddPropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: Option<AudioObjectPropertyListenerProc>,
        in_client_data: *mut c_void,
    ) -> OSStatus;

    /// Remove a property-change listener callback for an address.
    pub(super) fn AudioObjectRemovePropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: Option<AudioObjectPropertyListenerProc>,
        in_client_data: *mut c_void,
    ) -> OSStatus;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Return a utf8 conversion for a CFString.
    pub(super) fn CFStringGetCString(
        the_string: CFStringRef,
        buffer: *mut libc::c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> u8;

    /// Return the UTF-16 code-unit count for a CFString.
    pub(super) fn CFStringGetLength(the_string: CFStringRef) -> CFIndex;

    /// Return the maximum byte count for an encoding conversion.
    pub(super) fn CFStringGetMaximumSizeForEncoding(
        length: CFIndex,
        encoding: CFStringEncoding,
    ) -> CFIndex;

    /// Release a CoreFoundation object.
    pub(super) fn CFRelease(cf: CFTypeRef);
}

/// CoreAudio type alias `AudioQueueOutputCallback`.
#[cfg(target_os = "macos")]
pub(super) type AudioQueueOutputCallback =
    unsafe extern "C" fn(*mut c_void, AudioQueueRef, AudioQueueBufferRef);
/// CoreAudio type alias `AudioQueueInputCallback`.
#[cfg(target_os = "macos")]
pub(super) type AudioQueueInputCallback = unsafe extern "C" fn(
    *mut c_void,
    AudioQueueRef,
    AudioQueueBufferRef,
    *const AudioTimeStamp,
    u32,
    *const c_void,
);

#[cfg(target_os = "macos")]
#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    /// Create an output queue.
    pub(super) fn AudioQueueNewOutput(
        in_format: *const AudioStreamBasicDescription,
        in_callback_proc: Option<AudioQueueOutputCallback>,
        in_user_data: *mut c_void,
        in_callback_run_loop: *mut c_void,
        in_callback_run_loop_mode: CFStringRef,
        in_flags: u32,
        out_aq: *mut AudioQueueRef,
    ) -> OSStatus;

    /// Create an input queue.
    pub(super) fn AudioQueueNewInput(
        in_format: *const AudioStreamBasicDescription,
        in_callback_proc: Option<AudioQueueInputCallback>,
        in_user_data: *mut c_void,
        in_callback_run_loop: *mut c_void,
        in_callback_run_loop_mode: CFStringRef,
        in_flags: u32,
        out_aq: *mut AudioQueueRef,
    ) -> OSStatus;

    /// Dispose an audio queue.
    pub(super) fn AudioQueueDispose(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;

    /// Allocate a queue buffer.
    pub(super) fn AudioQueueAllocateBuffer(
        in_aq: AudioQueueRef,
        in_buffer_byte_size: u32,
        out_buffer: *mut AudioQueueBufferRef,
    ) -> OSStatus;

    /// Enqueue a queue buffer.
    pub(super) fn AudioQueueEnqueueBuffer(
        in_aq: AudioQueueRef,
        in_buffer: AudioQueueBufferRef,
        in_num_packet_descs: u32,
        in_packet_descs: *const c_void,
    ) -> OSStatus;

    /// Start a queue.
    pub(super) fn AudioQueueStart(in_aq: AudioQueueRef, in_start_time: *const c_void) -> OSStatus;

    /// Pause a queue.
    pub(super) fn AudioQueuePause(in_aq: AudioQueueRef) -> OSStatus;

    /// Stop a queue.
    pub(super) fn AudioQueueStop(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;

    /// Reset a queue and discard queued data.
    pub(super) fn AudioQueueReset(in_aq: AudioQueueRef) -> OSStatus;

    /// Set a queue property.
    pub(super) fn AudioQueueSetProperty(
        in_aq: AudioQueueRef,
        in_id: AudioQueuePropertyID,
        in_data: *const c_void,
        in_data_size: u32,
    ) -> OSStatus;

    /// Return the current host time from CoreAudio's host-time clock.
    pub(super) fn AudioGetCurrentHostTime() -> u64;
}

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
    /// Shared stream binding payload.
    pub(super) binding: Arc<audio_core::AudioStreamBinding>,
}

/// CoreAudio struct `CoreAudioQueueHandle`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioQueueHandle {
    /// One CoreAudio queue reference.
    pub(super) queue: AudioQueueRef,
    /// One strong owner reference held by runtime control flow.
    pub(super) _context_owner: Arc<CoreAudioStreamContext>,
    /// One raw pointer reference retained by CoreAudio callbacks.
    pub(super) context_raw: *const CoreAudioStreamContext,
}

#[cfg(target_os = "macos")]
unsafe impl Send for CoreAudioQueueHandle {}

/// CoreAudio struct `CoreAudioStreamRuntime`.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(super) struct CoreAudioStreamRuntime {
    /// Active queue handles for this stream.
    pub(super) queue_handles: audio_core::Mutex<Vec<CoreAudioQueueHandle>>,
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

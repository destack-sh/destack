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

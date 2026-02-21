#[cfg(target_os = "macos")]
pub(super) fn error(
    operation: &'static str,
    status: OSStatus,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("{} (osstatus {status})", message.into()),
    ))
    .boxed()
}

/// Return whether one CoreAudio object exposes one property at one scope.
#[cfg(target_os = "macos")]
pub(super) fn has_property(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> bool {
    let address = property_address(selector, scope);
    unsafe { AudioObjectHasProperty(object_id, &address) != 0 }
}

/// Read one CoreAudio property payload size.
#[cfg(target_os = "macos")]
pub(super) fn get_data_size(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<u32, OSStatus> {
    let address = property_address(selector, scope);
    let mut size = 0u32;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(object_id, &address, 0, std::ptr::null(), &mut size)
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    Ok(size)
}

/// Read one CoreAudio property payload into one byte buffer.
#[cfg(target_os = "macos")]
pub(super) fn get_data(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    bytes: &mut [u8],
) -> Result<(), OSStatus> {
    let address = property_address(selector, scope);
    let mut size = bytes.len() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            bytes.as_mut_ptr() as *mut c_void,
        )
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    if size != bytes.len() as u32 {
        return Err(-1);
    }

    Ok(())
}

/// Write one CoreAudio property payload from one byte buffer.
#[cfg(target_os = "macos")]
pub(super) fn set_data(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    bytes: &[u8],
) -> Result<(), OSStatus> {
    let address = property_address(selector, scope);
    let status = unsafe {
        AudioObjectSetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            bytes.len() as u32,
            bytes.as_ptr() as *const c_void,
        )
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    Ok(())
}

/// Read one typed scalar CoreAudio property when available.
#[cfg(target_os = "macos")]
pub(super) fn get_scalar_optional<T: Copy>(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Option<T> {
    if !has_property(object_id, selector, scope) {
        return None;
    }

    let address = property_address(selector, scope);
    let mut value = MaybeUninit::<T>::uninit();
    let mut size = size_of::<T>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            value.as_mut_ptr() as *mut c_void,
        )
    };
    if status != K_NO_ERR || size != size_of::<T>() as u32 {
        return None;
    }

    Some(unsafe { value.assume_init() })
}

/// Read one CoreAudio CFString property into one UTF-8 string.
#[cfg(target_os = "macos")]
pub(super) fn get_cfstring_optional(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Option<String> {
    let value = get_scalar_optional::<CFStringRef>(object_id, selector, scope)?;
    if value.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(value) };
    if length <= 0 {
        unsafe {
            CFRelease(value as CFTypeRef);
        }
        return None;
    }

    let max_utf8 = unsafe { CFStringGetMaximumSizeForEncoding(length, K_CF_STRING_ENCODING_UTF8) };
    if max_utf8 <= 0 {
        unsafe {
            CFRelease(value as CFTypeRef);
        }
        return None;
    }

    let mut buffer = vec![0i8; max_utf8 as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr(),
            buffer.len() as isize,
            K_CF_STRING_ENCODING_UTF8,
        ) != 0
    };

    unsafe {
        CFRelease(value as CFTypeRef);
    }

    if !converted {
        return None;
    }

    let text = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    Some(text.to_string_lossy().into_owned())
}

/// Return one channel count for one CoreAudio stream configuration scope.
#[cfg(target_os = "macos")]
pub(super) fn get_stream_channel_count(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<u16> {
    let size = get_data_size(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
        scope,
    )
    .ok()?;
    if size == 0 {
        return Some(0);
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_STREAM_CONFIGURATION,
        scope,
        &mut bytes,
    )
    .ok()?;

    if bytes.len() < size_of::<AudioBufferList>() {
        return None;
    }

    let buffer_list = bytes.as_ptr() as *const AudioBufferList;
    let buffer_count = unsafe { (*buffer_list).number_buffers as usize };
    let first_buffer = unsafe { std::ptr::addr_of!((*buffer_list).buffers) as *const AudioBuffer };
    let base_pointer = buffer_list as usize;
    let first_pointer = first_buffer as usize;
    let offset = first_pointer.checked_sub(base_pointer)?;
    let expected_size = offset.checked_add(buffer_count.checked_mul(size_of::<AudioBuffer>())?)?;
    if expected_size > bytes.len() {
        return None;
    }

    let mut channels = 0u64;
    for index in 0..buffer_count {
        let buffer = unsafe { first_buffer.add(index).read_unaligned() };
        channels = channels.saturating_add(buffer.number_channels as u64);
    }

    Some(channels.min(u16::MAX as u64) as u16)
}

/// Return one sample-rate range reported by CoreAudio for one scope.
#[cfg(target_os = "macos")]
pub(super) fn get_sample_rate_range(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<(u32, u32)> {
    let size = get_data_size(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES,
        scope,
    )
    .ok()?;
    if size == 0 {
        return None;
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_AVAILABLE_NOMINAL_SAMPLE_RATES,
        scope,
        &mut bytes,
    )
    .ok()?;

    let stride = size_of::<AudioValueRange>();
    if stride == 0 || bytes.len() < stride {
        return None;
    }

    let mut min_rate = u32::MAX;
    let mut max_rate = 0u32;
    for index in 0..(bytes.len() / stride) {
        let pointer = unsafe { bytes.as_ptr().add(index * stride) as *const AudioValueRange };
        let range = unsafe { pointer.read_unaligned() };
        let minimum = rate_to_u32(range.minimum)?;
        let maximum = rate_to_u32(range.maximum)?;
        let low = minimum.min(maximum);
        let high = minimum.max(maximum);
        min_rate = min_rate.min(low);
        max_rate = max_rate.max(high);
    }

    if min_rate == u32::MAX || max_rate == 0 {
        return None;
    }

    Some((min_rate, max_rate))
}

/// Return one period-frame range reported by CoreAudio for one scope.
#[cfg(target_os = "macos")]
pub(super) fn get_buffer_frame_size_range(
    device_id: AudioDeviceID,
    scope: AudioObjectPropertyScope,
) -> Option<(u32, u32)> {
    let range = get_scalar_optional::<AudioValueRange>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE_RANGE,
        scope,
    )?;
    let minimum = frames_to_u32(range.minimum)?;
    let maximum = frames_to_u32(range.maximum)?;
    Some((minimum.min(maximum), minimum.max(maximum)))
}

/// Intersect two optional closed integer ranges.
#[cfg(target_os = "macos")]
pub(super) fn merge_intersected_range(
    left: Option<(u32, u32)>,
    right: Option<(u32, u32)>,
) -> Option<(u32, u32)> {
    match (left, right) {
        (Some(left), Some(right)) => {
            let minimum = left.0.max(right.0);
            let maximum = left.1.min(right.1);
            if minimum > maximum {
                return None;
            }

            Some((minimum, maximum))
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

/// Validate one stream configuration against CoreAudio device limits.
#[cfg(target_os = "macos")]
pub(super) fn validate_open_stream_config(
    device_id: AudioDeviceID,
    direction: audio_core::AudioDeviceDirection,
    config: audio_core::AudioStreamConfig,
) -> RuntimeResult<()> {
    // resolve directional channel limits for the requested direction
    let playback_channels =
        get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT).unwrap_or(0);
    let capture_channels =
        get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT).unwrap_or(0);
    let max_channels = match direction {
        audio_core::AudioDeviceDirection::Playback => playback_channels,
        audio_core::AudioDeviceDirection::Capture => capture_channels,
        audio_core::AudioDeviceDirection::Duplex => playback_channels.min(capture_channels),
        audio_core::AudioDeviceDirection::Loopback => playback_channels,
    };

    // reject directions with no usable lane on this device
    if max_channels == 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio direction",
        ))
        .boxed());
    }

    // validate the requested channel count against direction limits
    if config.channels > max_channels {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            format!(
                "requested channel count exceeds CoreAudio direction channel limit {max_channels}",
            ),
        ))
        .boxed());
    }

    // resolve the valid sample-rate range for the requested direction
    let sample_rate_range = match direction {
        audio_core::AudioDeviceDirection::Playback => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
        audio_core::AudioDeviceDirection::Capture => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT)
        }
        audio_core::AudioDeviceDirection::Duplex => merge_intersected_range(
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT),
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT),
        ),
        audio_core::AudioDeviceDirection::Loopback => {
            get_sample_rate_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
    };

    // reject sample rates outside the backend-reported range
    if let Some((minimum, maximum)) = sample_rate_range
        && (config.sample_rate < minimum || config.sample_rate > maximum)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRate",
            format!("requested sample rate is outside CoreAudio range [{minimum}, {maximum}]"),
        ))
        .boxed());
    }

    // resolve the valid period range for the requested direction
    let period_range = match direction {
        audio_core::AudioDeviceDirection::Playback => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
        audio_core::AudioDeviceDirection::Capture => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT)
        }
        audio_core::AudioDeviceDirection::Duplex => merge_intersected_range(
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT),
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT),
        ),
        audio_core::AudioDeviceDirection::Loopback => {
            get_buffer_frame_size_range(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT)
        }
    };

    // reject explicit period requests outside the supported range
    if config.period_frames > 0
        && let Some((minimum, maximum)) = period_range
        && (config.period_frames < minimum || config.period_frames > maximum)
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.periodFrames",
            format!("requested period is outside CoreAudio range [{minimum}, {maximum}]"),
        ))
        .boxed());
    }

    Ok(())
}

/// Return whether CoreAudio hog mode is globally allowed on this host.
#[cfg(target_os = "macos")]
pub(super) fn hog_mode_allowed() -> bool {
    get_scalar_optional::<u32>(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_HOG_MODE_IS_ALLOWED,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .unwrap_or(1)
        != 0
}

/// Return the current hog-mode owner process identifier for one device.
#[cfg(target_os = "macos")]
pub(super) fn hog_owner_pid(device_id: AudioDeviceID) -> RuntimeResult<i32> {
    get_scalar_optional::<i32>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_HOG_MODE,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio hog mode",
        ))
        .boxed()
    })
}

/// Toggle CoreAudio hog mode and return the resulting owner process identifier.
#[cfg(target_os = "macos")]
pub(super) fn toggle_hog_mode(
    device_id: AudioDeviceID,
    operation: &'static str,
) -> RuntimeResult<i32> {
    let mut owner_pid = 0i32;
    let owner_pid_bytes = owner_pid.to_ne_bytes();
    set_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_HOG_MODE,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        &owner_pid_bytes,
    )
    .map_err(|status| error(operation, status, "failed to toggle CoreAudio hog mode"))?;

    owner_pid = hog_owner_pid(device_id)?;
    Ok(owner_pid)
}

/// Acquire CoreAudio hog mode for one device when available.
#[cfg(target_os = "macos")]
pub(super) fn enable_hog_mode(device_id: AudioDeviceID) -> RuntimeResult<bool> {
    // reject exclusive mode when the host disables hog mode globally
    if !hog_mode_allowed() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio exclusive mode",
        ))
        .boxed());
    }

    // inspect current ownership to enforce exclusive semantics
    let process_id = unsafe { libc::getpid() };
    let current_owner = hog_owner_pid(device_id)?;

    // no toggle is needed when this process already owns the device
    if current_owner == process_id {
        return Ok(false);
    }

    // return a would-block error when another process owns the device
    if current_owner != -1 {
        return Err(audio_core::audio_would_block(
            "destack.audio.stream.open",
            format!("audio device is already hogged by pid {current_owner}"),
        ));
    }

    // claim ownership and verify this process became the owner
    let owner_after_toggle = toggle_hog_mode(device_id, "destack.audio.stream.open")?;
    if owner_after_toggle != process_id {
        return Err(audio_core::audio_would_block(
            "destack.audio.stream.open",
            "audio device could not be acquired in exclusive mode",
        ));
    }

    Ok(true)
}

/// Release CoreAudio hog mode previously acquired by this runtime.
#[cfg(target_os = "macos")]
pub(super) fn release_hog_mode(runtime: &CoreAudioStreamRuntime) {
    // skip release when this stream did not acquire hog mode
    if !runtime.release_hog_mode_on_drop {
        return;
    }

    // release only when this process currently owns the device
    let process_id = unsafe { libc::getpid() };
    let current_owner = hog_owner_pid(runtime.device_id);
    let Ok(current_owner) = current_owner else {
        return;
    };
    if current_owner != process_id {
        return;
    }

    let _ = toggle_hog_mode(runtime.device_id, "destack.audio.stream.close");
}

/// Convert one finite CoreAudio sample-rate scalar into one u32 value.
#[cfg(target_os = "macos")]
pub(super) fn rate_to_u32(rate: f64) -> Option<u32> {
    if !rate.is_finite() || rate < 1.0 || rate > u32::MAX as f64 {
        return None;
    }

    Some(rate.round() as u32)
}

/// Convert one finite CoreAudio frame-count scalar into one u32 value.
#[cfg(target_os = "macos")]
pub(super) fn frames_to_u32(frames: f64) -> Option<u32> {
    if !frames.is_finite() || frames < 1.0 || frames > u32::MAX as f64 {
        return None;
    }

    Some(frames.round() as u32)
}

/// Return one default CoreAudio device identifier for one selector.
#[cfg(target_os = "macos")]
pub(super) fn default_device_id(selector: AudioObjectPropertySelector) -> Option<AudioDeviceID> {
    get_scalar_optional(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        selector,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
}

/// Enumerate CoreAudio device identifiers from the system object.
#[cfg(target_os = "macos")]
pub(super) fn device_ids() -> RuntimeResult<Vec<AudioDeviceID>> {
    let size = get_data_size(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .map_err(|status| {
        error(
            "destack.audio.device.list",
            status,
            "failed to query CoreAudio device list size",
        )
    })?;

    if size == 0 {
        return Ok(Vec::new());
    }

    if !(size as usize).is_multiple_of(size_of::<AudioDeviceID>()) {
        return Err(error(
            "destack.audio.device.list",
            -1,
            "CoreAudio device list has invalid element size",
        ));
    }

    let mut bytes = vec![0u8; size as usize];
    get_data(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        &mut bytes,
    )
    .map_err(|status| {
        error(
            "destack.audio.device.list",
            status,
            "failed to query CoreAudio device list",
        )
    })?;

    let mut devices = Vec::with_capacity(bytes.len() / size_of::<AudioDeviceID>());
    for chunk in bytes.chunks_exact(size_of::<AudioDeviceID>()) {
        let value = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        devices.push(value);
    }

    Ok(devices)
}

/// Return bits-per-channel for one runtime sample format.
#[cfg(target_os = "macos")]
pub(super) fn bits_per_channel(format: audio_core::AudioSampleFormat) -> Option<u32> {
    match format {
        audio_core::AudioSampleFormat::U8 => Some(8),
        audio_core::AudioSampleFormat::S16 => Some(16),
        audio_core::AudioSampleFormat::S24 => Some(32),
        audio_core::AudioSampleFormat::S32 => Some(32),
        audio_core::AudioSampleFormat::F32 => Some(32),
        audio_core::AudioSampleFormat::F64 => Some(64),
    }
}

/// Return CoreAudio PCM format flags for one runtime sample format.
#[cfg(target_os = "macos")]
pub(super) fn format_flags(format: audio_core::AudioSampleFormat) -> Option<u32> {
    let base = K_AUDIO_FORMAT_FLAG_IS_PACKED;
    match format {
        audio_core::AudioSampleFormat::U8 => Some(base),
        audio_core::AudioSampleFormat::S16
        | audio_core::AudioSampleFormat::S24
        | audio_core::AudioSampleFormat::S32 => Some(base | K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER),
        audio_core::AudioSampleFormat::F32 | audio_core::AudioSampleFormat::F64 => {
            Some(base | K_AUDIO_FORMAT_FLAG_IS_FLOAT)
        }
    }
}

/// Build one CoreAudio stream description from one runtime stream config.
#[cfg(target_os = "macos")]
pub(super) fn stream_description(
    config: audio_core::AudioStreamConfig,
) -> RuntimeResult<AudioStreamBasicDescription> {
    let bits_per_channel = bits_per_channel(config.format).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio format",
        ))
        .boxed()
    })?;
    let format_flags = format_flags(config.format).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio format",
        ))
        .boxed()
    })?;
    let bytes_per_frame = audio_core::frame_bytes(config.format, config.channels)? as u32;

    Ok(AudioStreamBasicDescription {
        sample_rate: config.sample_rate as f64,
        format_id: K_AUDIO_FORMAT_LINEAR_PCM,
        format_flags,
        bytes_per_packet: bytes_per_frame,
        frames_per_packet: 1,
        bytes_per_frame,
        channels_per_frame: config.channels as u32,
        bits_per_channel,
        reserved: 0,
    })
}

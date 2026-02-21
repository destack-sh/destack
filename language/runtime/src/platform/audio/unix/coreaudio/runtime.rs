/// Dispose all queue handles in one queue-handle list.
#[cfg(target_os = "macos")]
pub(super) fn dispose_queue_list(queue_handles: Vec<CoreAudioQueueHandle>) {
    for queue_handle in queue_handles {
        dispose_queue_handle(queue_handle);
    }
}

/// Drain queue handles from one runtime queue registry.
#[cfg(target_os = "macos")]
pub(super) fn take_runtime_queues(runtime: &CoreAudioStreamRuntime) -> Vec<CoreAudioQueueHandle> {
    let mut queue_handles = runtime
        .queue_handles
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    queue_handles.drain(..).collect::<Vec<_>>()
}

/// Dispose runtime queue handles and release optional hog mode ownership.
#[cfg(target_os = "macos")]
pub(super) fn dispose_runtime_handles(runtime: &CoreAudioStreamRuntime) {
    let queue_handles = take_runtime_queues(runtime);
    dispose_queue_list(queue_handles);
    release_hog_mode(runtime);
}

/// Apply one queue operation to all active queue handles.
#[cfg(target_os = "macos")]
pub(super) fn apply_queue_operation(
    runtime: &CoreAudioStreamRuntime,
    operation: &'static str,
    message: &'static str,
    mut callback: impl FnMut(AudioQueueRef) -> OSStatus,
) -> RuntimeResult<()> {
    let queue_handles = runtime
        .queue_handles
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    for queue_handle in queue_handles.iter() {
        let status = callback(queue_handle.queue);
        if status != K_NO_ERR && status != K_AUDIO_QUEUE_ERR_INVALID_RUN_STATE {
            return Err(error(operation, status, message));
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
impl audio_core::AudioHostStreamOps for CoreAudioHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        apply_queue_operation(
            &self.runtime,
            "destack.audio.stream.start",
            "failed to start CoreAudio queue",
            |queue| unsafe { AudioQueueStart(queue, ptr::null()) },
        )
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        if pause {
            return apply_queue_operation(
                &self.runtime,
                "destack.audio.stream.pause",
                "failed to pause CoreAudio queue",
                |queue| unsafe { AudioQueuePause(queue) },
            );
        }

        apply_queue_operation(
            &self.runtime,
            "destack.audio.stream.pause",
            "failed to resume CoreAudio queue",
            |queue| unsafe { AudioQueueStart(queue, ptr::null()) },
        )
    }

    fn stop(&self) -> RuntimeResult<()> {
        apply_queue_operation(
            &self.runtime,
            "destack.audio.stream.stop",
            "failed to stop CoreAudio queue",
            |queue| unsafe { AudioQueueStop(queue, 1) },
        )
    }

    fn flush(&self) -> RuntimeResult<()> {
        apply_queue_operation(
            &self.runtime,
            "destack.audio.stream.flush",
            "failed to flush CoreAudio queue",
            |queue| unsafe { AudioQueueReset(queue) },
        )
    }
}

/// Create and prime one CoreAudio playback queue.
#[cfg(target_os = "macos")]
fn build_queue_context(
    binding: &Arc<audio_core::AudioStreamBinding>,
) -> (Arc<CoreAudioStreamContext>, *const CoreAudioStreamContext) {
    let context_owner = Arc::new(CoreAudioStreamContext {
        binding: binding.clone(),
    });
    let context_raw = Arc::into_raw(context_owner.clone());
    (context_owner, context_raw)
}

/// Build one queue handle value from one queue pointer and one callback context owner.
#[cfg(target_os = "macos")]
fn build_queue_handle(
    queue: AudioQueueRef,
    context_owner: &Arc<CoreAudioStreamContext>,
    context_raw: *const CoreAudioStreamContext,
) -> CoreAudioQueueHandle {
    CoreAudioQueueHandle {
        queue,
        _context_owner: context_owner.clone(),
        context_raw,
    }
}

/// Allocate and enqueue one initial queue-buffer set for one stream.
#[cfg(target_os = "macos")]
fn initialize_queue_buffers(
    binding: &Arc<audio_core::AudioStreamBinding>,
    queue: AudioQueueRef,
    context_owner: &Arc<CoreAudioStreamContext>,
    context_raw: *const CoreAudioStreamContext,
    prefill_playback: bool,
    allocate_error_message: &'static str,
    enqueue_error_message: &'static str,
) -> RuntimeResult<()> {
    let buffer_bytes = buffer_bytes(binding)?;

    for _ in 0..COREAUDIO_PLAYBACK_BUFFER_COUNT {
        let mut buffer = ptr::null_mut();
        let allocate_status = unsafe { AudioQueueAllocateBuffer(queue, buffer_bytes, &mut buffer) };
        if allocate_status != K_NO_ERR {
            dispose_queue_handle(build_queue_handle(queue, context_owner, context_raw));
            return Err(error(
                "destack.audio.stream.open",
                allocate_status,
                allocate_error_message,
            ));
        }

        let buffer_mut = unsafe { &mut *buffer };
        if prefill_playback {
            // prefill each buffer to avoid initial underflow before start
            if !buffer_mut.audio_data.is_null() && buffer_mut.audio_data_bytes_capacity > 0 {
                let output = unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer_mut.audio_data as *mut u8,
                        buffer_mut.audio_data_bytes_capacity as usize,
                    )
                };
                fill_playback_bytes(binding, output);
                buffer_mut.audio_data_byte_size = buffer_mut.audio_data_bytes_capacity;
            } else {
                buffer_mut.audio_data_byte_size = 0;
            }
        } else {
            // arm each buffer before the first callback
            buffer_mut.audio_data_byte_size = buffer_mut.audio_data_bytes_capacity;
        }

        let enqueue_status = unsafe { AudioQueueEnqueueBuffer(queue, buffer, 0, ptr::null()) };
        if enqueue_status != K_NO_ERR {
            dispose_queue_handle(build_queue_handle(queue, context_owner, context_raw));
            return Err(error(
                "destack.audio.stream.open",
                enqueue_status,
                enqueue_error_message,
            ));
        }
    }

    Ok(())
}

/// Create one CoreAudio queue, bind it to one device, and prime initial buffers.
#[cfg(target_os = "macos")]
fn create_queue(
    binding: &Arc<audio_core::AudioStreamBinding>,
    device_id: AudioDeviceID,
    create_queue: impl FnOnce(
        &AudioStreamBasicDescription,
        *const CoreAudioStreamContext,
        &mut AudioQueueRef,
    ) -> OSStatus,
    create_error_message: &'static str,
    allocate_error_message: &'static str,
    enqueue_error_message: &'static str,
    prefill_playback: bool,
) -> RuntimeResult<CoreAudioQueueHandle> {
    let stream_description = stream_description(binding.requested)?;
    let mut queue = ptr::null_mut();
    let (context_owner, context_raw) = build_queue_context(binding);

    let create_status = create_queue(&stream_description, context_raw, &mut queue);
    if create_status != K_NO_ERR {
        unsafe {
            Arc::decrement_strong_count(context_raw);
        }
        return Err(error(
            "destack.audio.stream.open",
            create_status,
            create_error_message,
        ));
    }

    if let Err(error) = bind_queue_device(queue, device_id) {
        dispose_queue_handle(build_queue_handle(queue, &context_owner, context_raw));
        return Err(error);
    }

    initialize_queue_buffers(
        binding,
        queue,
        &context_owner,
        context_raw,
        prefill_playback,
        allocate_error_message,
        enqueue_error_message,
    )?;

    Ok(build_queue_handle(queue, &context_owner, context_raw))
}

/// Create and prime one CoreAudio playback queue.
#[cfg(target_os = "macos")]
pub(super) fn create_playback_queue(
    binding: &Arc<audio_core::AudioStreamBinding>,
    device_id: AudioDeviceID,
) -> RuntimeResult<CoreAudioQueueHandle> {
    create_queue(
        binding,
        device_id,
        |stream_description, context_raw, queue| unsafe {
            AudioQueueNewOutput(
                stream_description,
                Some(output_callback),
                context_raw as *mut c_void,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                queue,
            )
        },
        "failed to create CoreAudio output queue",
        "failed to allocate CoreAudio output buffer",
        "failed to enqueue CoreAudio output buffer",
        true,
    )
}

/// Create and prime one CoreAudio capture queue.
#[cfg(target_os = "macos")]
pub(super) fn create_capture_queue(
    binding: &Arc<audio_core::AudioStreamBinding>,
    device_id: AudioDeviceID,
) -> RuntimeResult<CoreAudioQueueHandle> {
    create_queue(
        binding,
        device_id,
        |stream_description, context_raw, queue| unsafe {
            AudioQueueNewInput(
                stream_description,
                Some(input_callback),
                context_raw as *mut c_void,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                queue,
            )
        },
        "failed to create CoreAudio input queue",
        "failed to allocate CoreAudio input buffer",
        "failed to enqueue CoreAudio input buffer",
        false,
    )
}

/// Spawn one cleanup worker that disposes runtime handles after shutdown.
#[cfg(target_os = "macos")]
pub(super) fn spawn_cleanup_thread(
    binding: Arc<audio_core::AudioStreamBinding>,
    runtime: Arc<CoreAudioStreamRuntime>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !state.shutdown {
            let wait = binding
                .sync
                .wake
                .wait_timeout(state, Duration::from_millis(25))
                .unwrap_or_else(|error| error.into_inner());
            state = wait.0;
        }
        drop(state);

        dispose_runtime_handles(&runtime);
    })
}

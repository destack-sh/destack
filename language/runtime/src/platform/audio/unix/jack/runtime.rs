use std::ffi::{c_int, c_ulong, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::abi::{JackClient, JackPort};
use super::core::{
    JackLibrary, c_str_to_string, c_string, capture_source_flags, client_input_port_flags,
    client_output_port_flags, jack_audio_type_pointer, jack_error, jack_not_supported,
    jack_succeeded, playback_sink_flags, require_jack_library, stream_client_name,
};
use super::host::{close_jack_client, list_ports, open_jack_client};
use super::ids::{parse_jack_stable_id, validate_jack_stable_id_direction};

/// One JACK callback context payload.
#[derive(Debug)]
struct JackCallbackContext {
    /// Loaded JACK dynamic library table.
    library: &'static Arc<JackLibrary>,
    /// Weak link to one stream binding.
    binding: audio_core::Mutex<Weak<audio_core::AudioStreamBinding>>,
    /// Registered client output ports.
    output_ports: Vec<*mut JackPort>,
    /// Registered client input ports.
    input_ports: Vec<*mut JackPort>,
    /// Negotiated runtime sample rate in hertz.
    sample_rate: u32,
}

unsafe impl Send for JackCallbackContext {}
unsafe impl Sync for JackCallbackContext {}

/// One JACK stream runtime payload.
#[derive(Debug)]
struct JackStreamRuntime {
    /// Loaded JACK dynamic library table.
    library: &'static Arc<JackLibrary>,
    /// Opened JACK client handle.
    client: *mut JackClient,
    /// Raw callback context pointer owned by this runtime.
    callback_context: *mut JackCallbackContext,
    /// Whether the client is currently activated.
    activated: AtomicBool,
    /// Whether close has already been applied.
    closed: AtomicBool,
}

unsafe impl Send for JackStreamRuntime {}
unsafe impl Sync for JackStreamRuntime {}

impl Drop for JackStreamRuntime {
    fn drop(&mut self) {
        close_runtime(self);
    }
}

/// One host-operations payload for one JACK stream binding.
#[derive(Debug)]
struct JackHostStreamOps {
    /// Shared JACK runtime payload.
    runtime: Arc<JackStreamRuntime>,
}

impl audio_core::AudioHostStreamOps for JackHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        activate_runtime(&self.runtime, "destack.audio.stream.start")
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        // pause and stop share one deactivate transition
        if pause {
            return deactivate_runtime(&self.runtime, "destack.audio.stream.pause");
        }

        activate_runtime(&self.runtime, "destack.audio.stream.pause")
    }

    fn stop(&self) -> RuntimeResult<()> {
        deactivate_runtime(&self.runtime, "destack.audio.stream.stop")
    }

    fn flush(&self) -> RuntimeResult<()> {
        Ok(())
    }
}

/// Activate one JACK runtime client when it is currently inactive.
fn activate_runtime(runtime: &JackStreamRuntime, operation: &'static str) -> RuntimeResult<()> {
    if runtime.activated.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let status = unsafe { (runtime.library.api.jack_activate)(runtime.client) };
    if jack_succeeded(status) {
        return Ok(());
    }

    runtime.activated.store(false, Ordering::SeqCst);
    Err(jack_error(
        operation,
        format!("failed to activate JACK client (status {status})"),
    ))
}

/// Deactivate one JACK runtime client when it is currently active.
fn deactivate_runtime(runtime: &JackStreamRuntime, operation: &'static str) -> RuntimeResult<()> {
    if !runtime.activated.swap(false, Ordering::SeqCst) {
        return Ok(());
    }

    let status = unsafe { (runtime.library.api.jack_deactivate)(runtime.client) };
    if jack_succeeded(status) {
        return Ok(());
    }

    runtime.activated.store(true, Ordering::SeqCst);
    Err(jack_error(
        operation,
        format!("failed to deactivate JACK client (status {status})"),
    ))
}

/// Open one JACK stream binding.
pub(super) fn open_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    // keep ports disconnected when no-autoconnect is explicitly requested
    let request_no_autoconnect =
        (backend_flags.0 & audio_core::BACKEND_OPEN_JACK_NO_AUTOCONNECT.0) != 0;

    // reject unsupported direction and share-mode requests
    if device_info.direction == audio_core::AudioDeviceDirection::Loopback {
        return Err(jack_not_supported(
            "destack.audio.stream.open",
            "JACK loopback direction is not implemented",
        ));
    }

    if share_mode != audio_core::AudioShareMode::Shared {
        return Err(jack_not_supported(
            "destack.audio.stream.open",
            "JACK exclusive mode is not supported",
        ));
    }

    // validate one stable-id contract before opening one runtime client
    let parsed = parse_jack_stable_id(&device_info.id)?;
    validate_jack_stable_id_direction(&parsed, device_info.direction)?;
    let _ = (
        parsed.playback_name.as_str(),
        parsed.capture_name.as_deref(),
    );

    let library = require_jack_library("destack.audio.stream.open")?;
    let client_name = stream_client_name();
    let client = open_jack_client(library, "destack.audio.stream.open", &client_name)?;

    // resolve host endpoint ports for lane negotiation
    let playback_sinks = list_ports(
        library,
        client,
        playback_sink_flags(),
        "destack.audio.stream.open",
    )?;
    let capture_sources = list_ports(
        library,
        client,
        capture_source_flags(),
        "destack.audio.stream.open",
    )?;

    let requested_channels = config.channels.max(1) as usize;
    let needs_playback = matches!(
        device_info.direction,
        audio_core::AudioDeviceDirection::Playback | audio_core::AudioDeviceDirection::Duplex
    );
    let needs_capture = matches!(
        device_info.direction,
        audio_core::AudioDeviceDirection::Capture | audio_core::AudioDeviceDirection::Duplex
    );

    // negotiate channel counts from host endpoint availability
    let mut playback_channels = if needs_playback {
        requested_channels.min(playback_sinks.len())
    } else {
        0
    };
    let mut capture_channels = if needs_capture {
        requested_channels.min(capture_sources.len())
    } else {
        0
    };

    if needs_playback && playback_channels == 0 {
        close_jack_client(library, client);
        return Err(audio_core::audio_not_found(
            "destack.audio.stream.open",
            "JACK reported no playback sink ports",
        ));
    }

    if needs_capture && capture_channels == 0 {
        close_jack_client(library, client);
        return Err(audio_core::audio_not_found(
            "destack.audio.stream.open",
            "JACK reported no capture source ports",
        ));
    }

    if device_info.direction == audio_core::AudioDeviceDirection::Duplex {
        let duplex_channels = playback_channels.min(capture_channels);
        if duplex_channels == 0 {
            close_jack_client(library, client);
            return Err(audio_core::audio_not_found(
                "destack.audio.stream.open",
                "JACK reported no shared duplex channel lanes",
            ));
        }

        playback_channels = duplex_channels;
        capture_channels = duplex_channels;
    }

    // register one client output-port set for playback lanes
    let output_ports = register_ports(
        library,
        client,
        "out",
        playback_channels,
        client_output_port_flags(),
        "destack.audio.stream.open",
    )?;

    // register one client input-port set for capture lanes
    let input_ports = register_ports(
        library,
        client,
        "in",
        capture_channels,
        client_input_port_flags(),
        "destack.audio.stream.open",
    )?;

    let sample_rate = unsafe { (library.api.jack_get_sample_rate)(client) }.max(1);
    let period_frames = unsafe { (library.api.jack_get_buffer_size)(client) }
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES);

    let callback_context = Box::new(JackCallbackContext {
        library,
        binding: audio_core::Mutex::new(Weak::new()),
        output_ports,
        input_ports,
        sample_rate,
    });
    let callback_context = Box::into_raw(callback_context);

    // install one process callback before client activation
    let callback_status = unsafe {
        (library.api.jack_set_process_callback)(
            client,
            Some(jack_process_callback),
            callback_context.cast::<c_void>(),
        )
    };
    if !jack_succeeded(callback_status) {
        unsafe {
            let _ = Box::from_raw(callback_context);
        }
        close_jack_client(library, client);

        return Err(jack_error(
            "destack.audio.stream.open",
            format!("failed to install JACK process callback (status {callback_status})"),
        ));
    }

    let runtime = Arc::new(JackStreamRuntime {
        library,
        client,
        callback_context,
        activated: AtomicBool::new(false),
        closed: AtomicBool::new(false),
    });

    let host_ops: Arc<dyn audio_core::AudioHostStreamOps> = Arc::new(JackHostStreamOps {
        runtime: runtime.clone(),
    });

    let negotiated_channels = playback_channels.max(capture_channels).max(1) as u16;

    // build one stream binding before activating JACK callbacks
    let binding = Arc::new(audio_core::AudioStreamBinding {
        device: device_info.clone(),
        direction: device_info.direction,
        requested: config,
        sample_rate,
        channels: negotiated_channels,
        period_frames,
        max_queued_frames: audio_core::resolved_max_queued_frames(),
        share_mode,
        runtime_capabilities: audio_core::AudioStreamRuntimeCapabilities {
            supports_write_at: false,
            supports_pause: true,
            supports_non_interleaved: false,
            supports_volume: true,
            supports_mute: true,
            supports_hardware_timestamps: false,
        },
        host_ops: audio_core::Mutex::new(Some(host_ops)),
        name: audio_core::Mutex::new(String::new()),
        sync: Arc::new(audio_core::AudioStreamSync {
            state: audio_core::Mutex::new(audio_core::initial_stream_state()),
            wake: audio_core::Condvar::new(),
        }),
        stream_handle_raw: std::sync::atomic::AtomicU64::new(0),
        event_runtime_state: audio_core::Mutex::new(None),
        null_worker: audio_core::Mutex::new(None),
    });

    // publish one weak binding handle for callback-side queue access
    {
        let context = unsafe { &*callback_context };
        *context
            .binding
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Arc::downgrade(&binding);
    }

    // activate once so this client can be wired into the live graph
    activate_runtime(&runtime, "destack.audio.stream.open")?;

    // connect lanes only when one autoconnect request is active
    if !request_no_autoconnect {
        // connect playback lanes from runtime outputs to system sinks
        connect_playback_ports(
            &runtime,
            playback_channels,
            &playback_sinks,
            "destack.audio.stream.open",
        )?;

        // connect capture lanes from system sources into runtime inputs
        connect_capture_ports(
            &runtime,
            capture_channels,
            &capture_sources,
            "destack.audio.stream.open",
        )?;
    }

    // leave the stream deactivated until one explicit start request
    deactivate_runtime(&runtime, "destack.audio.stream.open")?;

    Ok(binding)
}

/// Close one JACK stream runtime payload once.
fn close_runtime(runtime: &JackStreamRuntime) {
    if runtime.closed.swap(true, Ordering::SeqCst) {
        return;
    }

    // deactivate one active client before close
    let _ = deactivate_runtime(runtime, "destack.audio.stream.close");

    // close one opened JACK client handle
    if !runtime.client.is_null() {
        unsafe {
            let _ = (runtime.library.api.jack_client_close)(runtime.client);
        }
    }

    // release one callback context payload
    if !runtime.callback_context.is_null() {
        unsafe {
            let _ = Box::from_raw(runtime.callback_context);
        }
    }
}

/// Register one ordered JACK port set.
fn register_ports(
    library: &JackLibrary,
    client: *mut JackClient,
    prefix: &str,
    channel_count: usize,
    flags: c_ulong,
    operation: &'static str,
) -> RuntimeResult<Vec<*mut JackPort>> {
    let mut ports = Vec::with_capacity(channel_count);

    // register one port per requested channel lane
    for channel_index in 0..channel_count {
        let port_name = format!("{prefix}_{}", channel_index + 1);
        let port_name = c_string(&port_name, "portName")?;

        let port = unsafe {
            (library.api.jack_port_register)(
                client,
                port_name.as_ptr(),
                jack_audio_type_pointer(),
                flags,
                0,
            )
        };
        if port.is_null() {
            return Err(jack_error(
                operation,
                format!(
                    "failed to register JACK {prefix} port {}",
                    channel_index + 1
                ),
            ));
        }

        ports.push(port);
    }

    Ok(ports)
}

/// Connect one runtime playback port set to one physical sink set.
fn connect_playback_ports(
    runtime: &JackStreamRuntime,
    channel_count: usize,
    playback_sinks: &[String],
    operation: &'static str,
) -> RuntimeResult<()> {
    if channel_count == 0 {
        return Ok(());
    }

    let context = unsafe { &*runtime.callback_context };

    // connect one client output port into one physical sink lane
    for channel_index in 0..channel_count {
        let Some(port) = context.output_ports.get(channel_index).copied() else {
            return Err(jack_error(
                operation,
                "missing JACK runtime output port during connect",
            ));
        };

        let Some(target_port_name) = playback_sinks.get(channel_index) else {
            return Err(jack_error(
                operation,
                "missing JACK playback sink during connect",
            ));
        };

        let source_port_name = unsafe { (runtime.library.api.jack_port_name)(port.cast_const()) };
        let Some(source_port_name) = c_str_to_string(source_port_name) else {
            return Err(jack_error(
                operation,
                "failed to resolve JACK output port name",
            ));
        };

        let source_port_name = c_string(&source_port_name, "sourcePort")?;
        let target_port_name = c_string(target_port_name, "targetPort")?;

        let status = unsafe {
            (runtime.library.api.jack_connect)(
                runtime.client,
                source_port_name.as_ptr(),
                target_port_name.as_ptr(),
            )
        };
        if !jack_succeeded(status) {
            return Err(jack_error(
                operation,
                format!("failed to connect JACK playback lane {}", channel_index + 1),
            ));
        }
    }

    Ok(())
}

/// Connect one physical source set into one runtime capture port set.
fn connect_capture_ports(
    runtime: &JackStreamRuntime,
    channel_count: usize,
    capture_sources: &[String],
    operation: &'static str,
) -> RuntimeResult<()> {
    if channel_count == 0 {
        return Ok(());
    }

    let context = unsafe { &*runtime.callback_context };

    // connect one physical source lane into one client input port
    for channel_index in 0..channel_count {
        let Some(port) = context.input_ports.get(channel_index).copied() else {
            return Err(jack_error(
                operation,
                "missing JACK runtime input port during connect",
            ));
        };

        let Some(source_port_name) = capture_sources.get(channel_index) else {
            return Err(jack_error(
                operation,
                "missing JACK capture source during connect",
            ));
        };

        let target_port_name = unsafe { (runtime.library.api.jack_port_name)(port.cast_const()) };
        let Some(target_port_name) = c_str_to_string(target_port_name) else {
            return Err(jack_error(
                operation,
                "failed to resolve JACK input port name",
            ));
        };

        let source_port_name = c_string(source_port_name, "sourcePort")?;
        let target_port_name = c_string(&target_port_name, "targetPort")?;

        let status = unsafe {
            (runtime.library.api.jack_connect)(
                runtime.client,
                source_port_name.as_ptr(),
                target_port_name.as_ptr(),
            )
        };
        if !jack_succeeded(status) {
            return Err(jack_error(
                operation,
                format!("failed to connect JACK capture lane {}", channel_index + 1),
            ));
        }
    }

    Ok(())
}

/// JACK process callback entrypoint.
unsafe extern "C" fn jack_process_callback(nframes: u32, argument: *mut c_void) -> c_int {
    if argument.is_null() {
        return 0;
    }

    let context = unsafe { &*(argument.cast::<JackCallbackContext>()) };

    // resolve one live stream binding from callback context
    let binding = {
        let binding = context
            .binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        binding.upgrade()
    };

    let Some(binding) = binding else {
        silence_output_ports(context, nframes);
        return 0;
    };

    let mut state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // keep the graph silent whenever this stream is not actively running
    if state.shutdown || !state.running || state.paused {
        silence_output_ports(context, nframes);
        return 0;
    }

    // clear per-callback status bits before one new transfer cycle
    state.status_flags = audio_core::AudioStreamStatusFlags(0);

    // move queued playback samples into runtime output buffers
    process_playback_callback(context, &binding, &mut state, nframes);

    // collect runtime input buffers into queued capture samples
    process_capture_callback(context, &binding, &mut state, nframes);

    // publish one callback timing sample
    let callback_mono_ns = audio_core::host_monotonic_nanos();
    audio_core::record_stream_callback_timing(
        &mut state,
        context.sample_rate,
        nframes,
        callback_mono_ns,
        None,
        None,
    );

    drop(state);
    binding.sync.wake.notify_all();

    0
}

/// Fill all registered runtime output ports with silence.
fn silence_output_ports(context: &JackCallbackContext, nframes: u32) {
    for port in &context.output_ports {
        let buffer = unsafe { (context.library.api.jack_port_get_buffer)(*port, nframes) };
        if buffer.is_null() {
            continue;
        }

        let buffer =
            unsafe { std::slice::from_raw_parts_mut(buffer.cast::<f32>(), nframes as usize) };
        buffer.fill(0.0);
    }
}

/// Process one callback-side playback transfer cycle.
fn process_playback_callback(
    context: &JackCallbackContext,
    binding: &audio_core::AudioStreamBinding,
    state: &mut audio_core::AudioStreamStateInner,
    nframes: u32,
) {
    if context.output_ports.is_empty() {
        return;
    }

    let frame_count = nframes as usize;
    let channel_count = context.output_ports.len();
    let scalar_count = frame_count.saturating_mul(channel_count);
    let mut packet = Vec::with_capacity(scalar_count);
    let mut underflow = false;

    // gather one interleaved playback packet from queued user samples
    for _ in 0..scalar_count {
        if let Some(sample) = state.playback_samples.pop_front() {
            packet.push(sample);
        } else {
            packet.push(0.0);
            underflow = true;
        }
    }

    // apply one stream-wide mute and gain transform in-place
    if state.muted {
        packet.fill(0.0);
    } else if (state.volume - 1.0).abs() > f64::EPSILON {
        let gain = state.volume as f32;
        for sample in &mut packet {
            *sample = audio_core::clamp_audio_scalar(*sample * gain);
        }
    }

    if underflow {
        state.xrun_count = state.xrun_count.saturating_add(1);
        state.output_underflow_count = state.output_underflow_count.saturating_add(1);
        state.status_flags = audio_core::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
        );
    }

    // scatter one interleaved packet into one per-port JACK buffer set
    for (channel_index, port) in context.output_ports.iter().enumerate() {
        let buffer = unsafe { (context.library.api.jack_port_get_buffer)(*port, nframes) };
        if buffer.is_null() {
            continue;
        }

        let buffer = unsafe { std::slice::from_raw_parts_mut(buffer.cast::<f32>(), frame_count) };
        for frame_index in 0..frame_count {
            buffer[frame_index] = packet[frame_index * channel_count + channel_index];
        }
    }

    let output_delay = (nframes as u64)
        .saturating_mul(1_000_000_000u64)
        .checked_div(context.sample_rate.max(1) as u64)
        .unwrap_or(0);
    state.last_output_dac_ns = audio_core::host_monotonic_nanos().saturating_add(output_delay);

    let _ = binding;
}

/// Process one callback-side capture transfer cycle.
fn process_capture_callback(
    context: &JackCallbackContext,
    binding: &audio_core::AudioStreamBinding,
    state: &mut audio_core::AudioStreamStateInner,
    nframes: u32,
) {
    if context.input_ports.is_empty() {
        return;
    }

    let frame_count = nframes as usize;
    let channel_count = context.input_ports.len();
    let scalar_count = frame_count.saturating_mul(channel_count);
    let mut packet = vec![0.0f32; scalar_count];

    // gather one interleaved capture packet from one per-port JACK buffer set
    for (channel_index, port) in context.input_ports.iter().enumerate() {
        let buffer = unsafe { (context.library.api.jack_port_get_buffer)(*port, nframes) };
        if buffer.is_null() {
            continue;
        }

        let buffer = unsafe { std::slice::from_raw_parts(buffer.cast::<f32>(), frame_count) };
        for frame_index in 0..frame_count {
            packet[frame_index * channel_count + channel_index] =
                audio_core::clamp_audio_scalar(buffer[frame_index]);
        }
    }

    // append one capture packet to the shared capture queue
    for sample in packet {
        state.capture_samples.push_back(sample);
    }

    // enforce one bounded capture queue and report overflow
    let capture_capacity = binding.capture_capacity_samples();
    if state.capture_samples.len() > capture_capacity {
        let overflow = state.capture_samples.len() - capture_capacity;
        for _ in 0..overflow {
            let _ = state.capture_samples.pop_front();
        }

        state.xrun_count = state.xrun_count.saturating_add(1);
        state.input_overflow_count = state.input_overflow_count.saturating_add(1);
        state.status_flags = audio_core::AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
        );
    }
}

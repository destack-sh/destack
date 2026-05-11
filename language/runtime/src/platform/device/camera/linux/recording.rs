use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use super::core::*;
use super::device::{read_host_frame_bytes, wait_for_frame};

/// One Linux recording file extension.
const LINUX_CAMERA_RECORDING_EXTENSION: &str = "mp4";

/// One Linux recording poll chunk.
const LINUX_CAMERA_RECORDING_POLL_TIMEOUT_NS: u64 = 50_000_000;

/// Return whether the host ffmpeg binary is available.
fn linux_ffmpeg_available() -> bool {
    static LINUX_CAMERA_RECORDING_AVAILABLE: OnceLock<bool> = OnceLock::new();

    *LINUX_CAMERA_RECORDING_AVAILABLE.get_or_init(|| {
        let output = Command::new("ffmpeg")
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let Ok(output) = output else {
            return false;
        };
        if !output.status.success() {
            return false;
        }

        let encoders = Command::new("ffmpeg")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-encoders")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let Ok(encoders) = encoders else {
            return false;
        };
        if !encoders.status.success() {
            return false;
        }

        String::from_utf8_lossy(&encoders.stdout).contains("libx264")
    })
}

/// Return the supported Linux recording capability descriptor.
pub(super) fn linux_recording_capabilities_value() -> CameraRecordingCapabilitiesValue {
    // unavailable pipeline
    if !linux_ffmpeg_available() {
        return CameraRecordingCapabilitiesValue {
            containers: Vec::new(),
            video_codecs: Vec::new(),
            audio_supported: false,
            audio_codecs: None,
            pause_supported: false,
            maximum_video_bit_rate: None,
            maximum_audio_bit_rate: None,
        };
    }

    CameraRecordingCapabilitiesValue {
        containers: vec![CameraRecordingContainer::Mp4],
        video_codecs: vec![CameraVideoCodec::H264],
        audio_supported: false,
        audio_codecs: None,
        pause_supported: false,
        maximum_video_bit_rate: None,
        maximum_audio_bit_rate: None,
    }
}

/// Validate one Linux recording request against the current implementation.
pub(super) fn validate_linux_recording_options(
    options: &CameraRecordingOptionsValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    // recording pipeline availability
    if !linux_ffmpeg_available() {
        return Err(shared_camera_not_supported(operation));
    }

    // container selection
    if let Some(container) = options.container
        && container != CameraRecordingContainer::Mp4
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.container",
            "Linux camera recording currently supports only mp4 output",
        ))
        .boxed());
    }

    // codec selection
    if let Some(video_codec) = options.video_codec
        && video_codec != CameraVideoCodec::H264
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.video_codec",
            "Linux camera recording currently supports only h264 video",
        ))
        .boxed());
    }

    // unsupported audio
    if options.audio_enabled.unwrap_or(false) || options.audio_codec.is_some() {
        return Err(shared_camera_not_supported(operation));
    }

    // unsupported controls
    if options.audio_bit_rate.is_some()
        || options.maximum_duration_ns.is_some()
        || options.maximum_bytes.is_some()
    {
        return Err(shared_camera_not_supported(operation));
    }

    Ok(())
}

/// Start one Linux recording worker on one opened stream.
pub(super) fn start_linux_recording_worker(
    resource: &Arc<LinuxCameraStreamResource>,
    options: &CameraRecordingOptionsValue,
) -> RuntimeResult<()> {
    // output path
    let output_path = camera_recording_output_path(
        options,
        LINUX_CAMERA_RECORDING_EXTENSION,
        "destack.device.camera.stream.startRecording",
    )?;

    // worker state
    let stop_signal = Arc::new(AtomicBool::new(false));
    let completion = Arc::new(LinuxCameraRecordingCompletion {
        result: Mutex::new(None),
        wake: Condvar::new(),
    });

    // worker inputs
    let descriptor = resource.descriptor;
    let mode = resource.mode.clone();
    let config = resource.config.clone();
    let path = output_path.clone();
    let stop_signal_clone = Arc::clone(&stop_signal);
    let completion_clone = Arc::clone(&completion);
    let options = options.clone();

    // recording worker
    let handle = start_with_policy(
        "destack-camera-linux-recording",
        "destack.device.camera.stream.startRecording",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || {
            let result = run_linux_recording_worker(
                descriptor,
                mode,
                config,
                path,
                options,
                stop_signal_clone,
            );

            let mut completion_state = completion_clone.result.lock();
            *completion_state = Some(result);
            completion_clone.wake.notify_all();
        },
    )?;

    // runtime state
    let mut recording = resource.recording.lock();
    recording.runtime.is_active = true;
    recording.runtime.is_paused = false;
    recording.runtime.options = Some(options);
    recording.runtime.started_timestamp_ns = Some(core_platform::monotonic_now_ns());
    recording.path = Some(output_path);
    recording.worker = Some(LinuxCameraRecordingWorker {
        stop_signal,
        completion,
        handle: Some(handle),
    });

    Ok(())
}

/// Stop one active Linux recording worker.
pub(super) fn stop_active_recording_worker(
    recording: &Arc<Mutex<LinuxCameraRecordingState>>,
    timeout_ns: Option<u64>,
) -> RuntimeResult<Option<LinuxCameraRecordingFinish>> {
    // request stop
    let completion = {
        let mut state = recording.lock();
        let Some(worker) = state.worker.as_mut() else {
            return Ok(None);
        };

        worker.stop_signal.store(true, Ordering::Relaxed);

        Arc::clone(&worker.completion)
    };

    // worker completion
    let result = if let Some(timeout_ns) = timeout_ns {
        let timeout = Duration::from_nanos(timeout_ns);
        let mut completion_state = completion.result.lock();
        if completion_state.is_none()
            && completion
                .wake
                .wait_for(&mut completion_state, timeout)
                .timed_out()
        {
            return Err(core_platform::io_would_block(
                "destack.device.camera.stream.stopRecording",
                "camera recording stop timed out",
            ));
        }

        completion_state.take()
    } else {
        let mut completion_state = completion.result.lock();
        while completion_state.is_none() {
            completion.wake.wait(&mut completion_state);
        }

        completion_state.take()
    };

    // worker join
    let result = result.unwrap_or_else(|| {
        Err(String::from(
            "camera recording worker exited without one result",
        ))
    });

    let worker = {
        let mut state = recording.lock();
        state.worker.take()
    };

    if let Some(mut worker) = worker
        && let Some(handle) = worker.handle.take()
        && handle.join().is_err()
    {
        return Err(core_platform::io_operation_error(
            "destack.device.camera.stream.stopRecording",
            None,
            String::from("camera recording worker panicked"),
        ));
    }

    result.map(Some).map_err(|error| {
        core_platform::io_operation_error("destack.device.camera.stream.stopRecording", None, error)
    })
}

/// Run one Linux recording worker until one stop request arrives.
fn run_linux_recording_worker(
    descriptor: RawFd,
    mode: LinuxCameraStreamMode,
    config: CameraStreamConfigValue,
    output_path: PathBuf,
    options: CameraRecordingOptionsValue,
    stop_signal: Arc<AtomicBool>,
) -> Result<LinuxCameraRecordingFinish, String> {
    // encoder bootstrap
    let mut child = spawn_linux_recording_process(&config, &output_path, &options)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| String::from("ffmpeg recording process did not expose one stdin pipe"))?;
    let started = Instant::now();

    // frame forwarding
    while !stop_signal.load(Ordering::Relaxed) {
        // frame wait
        let has_frame = wait_for_frame(descriptor, LINUX_CAMERA_RECORDING_POLL_TIMEOUT_NS)
            .map_err(|error| error.to_string())?;
        if !has_frame {
            continue;
        }

        // host read
        let bytes = read_host_frame_bytes(
            descriptor,
            &mode,
            "destack.device.camera.stream.startRecording",
        )
        .map_err(|error| error.to_string())?;

        // encoder write
        stdin
            .write_all(&bytes)
            .map_err(|error| format!("failed to write one recording frame to ffmpeg: {error}"))?;
    }

    drop(stdin);

    // encoder shutdown
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to finalize ffmpeg camera recording: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "ffmpeg camera recording failed with exit status {}",
                output.status
            ));
        }

        return Err(format!(
            "ffmpeg camera recording failed with exit status {}: {stderr}",
            output.status
        ));
    }

    let duration_ns = u64::try_from(started.elapsed().as_nanos()).ok();
    let size_bytes = std::fs::metadata(&output_path)
        .ok()
        .map(|metadata| metadata.len());

    Ok(LinuxCameraRecordingFinish {
        duration_ns,
        size_bytes,
    })
}

/// Spawn one ffmpeg recording process for one Linux camera stream.
fn spawn_linux_recording_process(
    config: &CameraStreamConfigValue,
    output_path: &Path,
    options: &CameraRecordingOptionsValue,
) -> Result<Child, String> {
    // command bootstrap
    let mut command = Command::new("ffmpeg");
    command
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-nostdin")
        .arg("-y");

    // input description
    match config.pixel_format.format {
        CameraPixelFormat::Bgra8 => {
            command
                .arg("-f")
                .arg("rawvideo")
                .arg("-pixel_format")
                .arg("bgra");
        }
        CameraPixelFormat::Rgba8 => {
            command
                .arg("-f")
                .arg("rawvideo")
                .arg("-pixel_format")
                .arg("rgba");
        }
        CameraPixelFormat::Yuv420 => {
            command
                .arg("-f")
                .arg("rawvideo")
                .arg("-pixel_format")
                .arg("yuv420p");
        }
        CameraPixelFormat::Jpeg => {
            command
                .arg("-f")
                .arg("image2pipe")
                .arg("-vcodec")
                .arg("mjpeg");
        }
    }

    // frame description
    command
        .arg("-video_size")
        .arg(format!("{}x{}", config.width, config.height))
        .arg("-framerate")
        .arg(format!("{}/1000", config.frame_rate_milli_hz))
        .arg("-i")
        .arg("pipe:0")
        .arg("-an")
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p");

    if let Some(video_bit_rate) = options.video_bit_rate {
        command.arg("-b:v").arg(video_bit_rate.to_string());
    }

    if let Some(key_frame_interval_frames) = options.key_frame_interval_frames {
        command.arg("-g").arg(key_frame_interval_frames.to_string());
    }

    command
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path.as_os_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    // process spawn
    command
        .spawn()
        .map_err(|error| format!("failed to spawn ffmpeg camera recording process: {error}"))
}

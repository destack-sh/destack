use std::ffi::c_void;
use std::sync::Arc;

use super::abi::asio_driver_output_ready;
use super::constants::{
    ASE_NOT_PRESENT, ASE_OK, K_ASIO_ENGINE_VERSION, K_ASIO_LATENCIES_CHANGED, K_ASIO_RESET_REQUEST,
    K_ASIO_RESYNC_REQUEST, K_ASIO_SELECTOR_SUPPORTED, K_ASIO_SUPPORTS_TIME_CODE,
    K_ASIO_SUPPORTS_TIME_INFO,
};
use super::core::{AsioBufferLane, AsioSampleEncoding, AsioStreamRuntime, active_runtime};
use crate::platform::audio::core as audio_core;

/// Handle one ASIO buffer-switch callback.
pub(super) extern "system" fn asio_buffer_switch(buffer_index: i32, _direct_process: i32) {
    let Some(runtime) = active_runtime() else {
        return;
    };

    process_callback_transfer(&runtime, buffer_index.max(0) as usize);
}

/// Handle one ASIO sample-rate-change callback.
pub(super) extern "system" fn asio_sample_rate_did_change(sample_rate: f64) {
    let Some(runtime) = active_runtime() else {
        return;
    };

    // mark one runtime as device-lost when sample rate diverges
    let rounded_sample_rate = sample_rate.round().clamp(1.0, u32::MAX as f64) as u32;
    if rounded_sample_rate != runtime.sample_rate {
        mark_device_lost(
            &runtime,
            "ASIO sample rate changed while stream was running",
        );
    }
}

/// Handle one ASIO message callback.
pub(super) extern "system" fn asio_message(
    selector: i32,
    value: i32,
    _message: *mut c_void,
    _opt: *mut f64,
) -> i32 {
    let Some(runtime) = active_runtime() else {
        return 0;
    };

    // advertise the selector set handled by this host runtime
    if selector == K_ASIO_SELECTOR_SUPPORTED {
        if value == K_ASIO_RESET_REQUEST
            || value == K_ASIO_ENGINE_VERSION
            || value == K_ASIO_RESYNC_REQUEST
            || value == K_ASIO_LATENCIES_CHANGED
            || value == K_ASIO_SUPPORTS_TIME_INFO
            || value == K_ASIO_SUPPORTS_TIME_CODE
        {
            return 1;
        }

        return 0;
    }

    // report one ASIO host engine version
    if selector == K_ASIO_ENGINE_VERSION {
        return 2;
    }

    // mark one runtime as device-lost for reset and resync requests
    if selector == K_ASIO_RESET_REQUEST || selector == K_ASIO_RESYNC_REQUEST {
        mark_device_lost(&runtime, "ASIO driver requested reset or resync");
        return 1;
    }

    // acknowledge one latency-change signal
    if selector == K_ASIO_LATENCIES_CHANGED {
        return 1;
    }

    // opt out of time-info and time-code callbacks for now
    if selector == K_ASIO_SUPPORTS_TIME_INFO || selector == K_ASIO_SUPPORTS_TIME_CODE {
        return 0;
    }

    0
}

/// Process one callback transfer cycle for one active ASIO runtime.
fn process_callback_transfer(runtime: &Arc<AsioStreamRuntime>, buffer_index: usize) {
    let stream_binding = runtime.binding.get().and_then(std::sync::Weak::upgrade);
    let Some(stream_binding) = stream_binding else {
        mark_device_lost(runtime, "ASIO stream binding is no longer available");
        return;
    };

    let mut state = stream_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let is_active = state.running && !state.paused && !state.shutdown;
    let output_volume = if state.muted {
        0.0
    } else {
        state.volume as f32
    };

    // process one playback transfer block when playback lanes exist
    if !runtime.output_lanes.is_empty() {
        if let Some(encoding) = runtime.output_encoding {
            transfer_playback_block(
                runtime,
                &mut state,
                &runtime.output_lanes,
                encoding,
                buffer_index,
                is_active,
                output_volume,
            );
        } else {
            mark_device_lost(runtime, "ASIO output encoding is unavailable");
            return;
        }
    }

    // process one capture transfer block when capture lanes exist
    if !runtime.input_lanes.is_empty() {
        if let Some(encoding) = runtime.input_encoding {
            transfer_capture_block(
                &binding,
                runtime,
                &mut state,
                &runtime.input_lanes,
                encoding,
                buffer_index,
                is_active,
            );
        } else {
            mark_device_lost(runtime, "ASIO input encoding is unavailable");
            return;
        }
    }

    // update one callback timing snapshot for this cycle
    if is_active {
        let callback_mono_ns = audio_core::host_monotonic_nanos();
        audio_core::record_stream_callback_timing(
            &mut state,
            runtime.sample_rate,
            runtime.period_frames,
            callback_mono_ns,
            None,
            None,
        );
    }

    drop(state);
    stream_binding.sync.wake.notify_all();

    // issue one output-ready hint for drivers that use explicit host signaling
    let output_ready_status = unsafe { asio_driver_output_ready(runtime.session.driver.raw) };
    if output_ready_status != ASE_OK && output_ready_status != ASE_NOT_PRESENT {
        mark_device_lost(runtime, "ASIO driver output-ready signal failed");
    }
}

/// Transfer one playback callback block into ASIO output lane buffers.
fn transfer_playback_block(
    runtime: &Arc<AsioStreamRuntime>,
    state: &mut audio_core::AudioStreamStateInner,
    output_lanes: &[AsioBufferLane],
    encoding: AsioSampleEncoding,
    buffer_index: usize,
    is_active: bool,
    output_volume: f32,
) {
    if output_lanes.is_empty() {
        return;
    }

    // write one de-interleaved output block from the playback sample queue
    for frame_index in 0usize..runtime.period_frames as usize {
        for output_lane in output_lanes.iter().copied() {
            let Some(pointer) = lane_buffer_pointer(output_lane, buffer_index) else {
                mark_runtime_state_device_lost(
                    state,
                    "ASIO output lane buffer is unavailable for the callback half",
                );
                return;
            };

            let sample = if is_active {
                match state.playback_samples.pop_front() {
                    Some(value) => value * output_volume,
                    None => {
                        state.xrun_count = state.xrun_count.saturating_add(1);
                        state.output_underflow_count =
                            state.output_underflow_count.saturating_add(1);
                        state.status_flags = audio_core::AudioStreamStatusFlags(
                            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_UNDERFLOW.0,
                        );
                        0.0
                    }
                }
            } else {
                0.0
            };

            let lane_offset = frame_index.saturating_mul(encoding.bytes_per_sample);
            let output = unsafe {
                std::slice::from_raw_parts_mut(pointer.add(lane_offset), encoding.bytes_per_sample)
            };
            encode_asio_sample(encoding, sample, output);
        }
    }
}

/// Transfer one capture callback block from ASIO input lane buffers.
fn transfer_capture_block(
    binding: &Arc<audio_core::AudioStreamBinding>,
    runtime: &Arc<AsioStreamRuntime>,
    state: &mut audio_core::AudioStreamStateInner,
    input_lanes: &[AsioBufferLane],
    encoding: AsioSampleEncoding,
    buffer_index: usize,
    is_active: bool,
) {
    if !is_active {
        return;
    }

    let capture_capacity = binding.capture_capacity_samples();

    // read one de-interleaved input block into the capture sample queue
    for frame_index in 0usize..runtime.period_frames as usize {
        for input_lane in input_lanes.iter().copied() {
            let Some(pointer) = lane_buffer_pointer(input_lane, buffer_index) else {
                mark_runtime_state_device_lost(
                    state,
                    "ASIO input lane buffer is unavailable for the callback half",
                );
                return;
            };

            let lane_offset = frame_index.saturating_mul(encoding.bytes_per_sample);
            let input = unsafe {
                std::slice::from_raw_parts(pointer.add(lane_offset), encoding.bytes_per_sample)
            };
            let sample = decode_asio_sample(encoding, input).unwrap_or(0.0);

            if state.capture_samples.len() >= capture_capacity {
                state.capture_samples.pop_front();
                state.xrun_count = state.xrun_count.saturating_add(1);
                state.input_overflow_count = state.input_overflow_count.saturating_add(1);
                state.status_flags = audio_core::AudioStreamStatusFlags(
                    state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_OVERFLOW.0,
                );
            }

            state.capture_samples.push_back(sample);
        }
    }
}

/// Return one selected buffer pointer for one lane and callback half index.
fn lane_buffer_pointer(lane: AsioBufferLane, buffer_index: usize) -> Option<*mut u8> {
    if buffer_index == 0 {
        return (!lane.buffer_a.is_null()).then_some(lane.buffer_a);
    }

    if buffer_index == 1 {
        return (!lane.buffer_b.is_null()).then_some(lane.buffer_b);
    }

    None
}

/// Decode one ASIO sample into one normalized scalar.
fn decode_asio_sample(encoding: AsioSampleEncoding, bytes: &[u8]) -> Option<f32> {
    if bytes.len() < encoding.bytes_per_sample {
        return None;
    }

    match encoding.format {
        audio_core::AudioSampleFormat::S16 => {
            let raw = if encoding.is_big_endian {
                i16::from_be_bytes([bytes[0], bytes[1]])
            } else {
                i16::from_le_bytes([bytes[0], bytes[1]])
            };
            Some(raw as f32 / 32_768.0)
        }
        audio_core::AudioSampleFormat::S24 => {
            if encoding.is_packed_24 {
                let raw = if encoding.is_big_endian {
                    (((bytes[0] as i32) << 24)
                        | ((bytes[1] as i32) << 16)
                        | ((bytes[2] as i32) << 8))
                        >> 8
                } else {
                    (((bytes[2] as i32) << 24)
                        | ((bytes[1] as i32) << 16)
                        | ((bytes[0] as i32) << 8))
                        >> 8
                };
                Some(raw as f32 / 8_388_608.0)
            } else {
                let raw = if encoding.is_big_endian {
                    i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                } else {
                    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                };
                Some(raw as f32 / 8_388_608.0)
            }
        }
        audio_core::AudioSampleFormat::S32 => {
            let raw = if encoding.is_big_endian {
                i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            } else {
                i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            };
            Some(raw as f32 / 2_147_483_648.0)
        }
        audio_core::AudioSampleFormat::F32 => {
            let raw = if encoding.is_big_endian {
                f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            } else {
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            };
            Some(audio_core::clamp_audio_scalar(raw))
        }
        audio_core::AudioSampleFormat::F64 => {
            let raw = if encoding.is_big_endian {
                f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ])
            } else {
                f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ])
            };
            Some(audio_core::clamp_audio_scalar(raw as f32))
        }
        audio_core::AudioSampleFormat::U8 => None,
    }
}

/// Encode one normalized scalar into one ASIO lane sample.
fn encode_asio_sample(encoding: AsioSampleEncoding, sample: f32, output: &mut [u8]) {
    let sample = audio_core::clamp_audio_scalar(sample);

    match encoding.format {
        audio_core::AudioSampleFormat::S16 => {
            let value = (sample * 32_767.0).round() as i16;
            let bytes = if encoding.is_big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            };
            output[..2].copy_from_slice(&bytes);
        }
        audio_core::AudioSampleFormat::S24 => {
            if encoding.is_packed_24 {
                let value = ((sample * 8_388_607.0).round() as i32).clamp(-8_388_608, 8_388_607);
                if encoding.is_big_endian {
                    output[0] = ((value >> 16) & 0xff) as u8;
                    output[1] = ((value >> 8) & 0xff) as u8;
                    output[2] = (value & 0xff) as u8;
                } else {
                    output[0] = (value & 0xff) as u8;
                    output[1] = ((value >> 8) & 0xff) as u8;
                    output[2] = ((value >> 16) & 0xff) as u8;
                }
            } else {
                let value = (sample * 8_388_607.0).round() as i32;
                let bytes = if encoding.is_big_endian {
                    value.to_be_bytes()
                } else {
                    value.to_le_bytes()
                };
                output[..4].copy_from_slice(&bytes);
            }
        }
        audio_core::AudioSampleFormat::S32 => {
            let value = (sample * 2_147_483_647.0).round() as i32;
            let bytes = if encoding.is_big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            };
            output[..4].copy_from_slice(&bytes);
        }
        audio_core::AudioSampleFormat::F32 => {
            let bytes = if encoding.is_big_endian {
                sample.to_be_bytes()
            } else {
                sample.to_le_bytes()
            };
            output[..4].copy_from_slice(&bytes);
        }
        audio_core::AudioSampleFormat::F64 => {
            let value = sample as f64;
            let bytes = if encoding.is_big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            };
            output[..8].copy_from_slice(&bytes);
        }
        audio_core::AudioSampleFormat::U8 => {}
    }
}

/// Mark one stream binding as device-lost from callback paths.
fn mark_device_lost(runtime: &Arc<AsioStreamRuntime>, message: &'static str) {
    let stream_binding = runtime.binding.get().and_then(std::sync::Weak::upgrade);
    let Some(stream_binding) = stream_binding else {
        return;
    };

    let mut state = stream_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    mark_runtime_state_device_lost(&mut state, message);
    drop(state);

    stream_binding.sync.wake.notify_all();
}

/// Mark one mutable stream state payload as device-lost.
fn mark_runtime_state_device_lost(
    state: &mut audio_core::AudioStreamStateInner,
    message: &'static str,
) {
    state.running = false;
    state.paused = false;
    state.state = audio_core::AudioStreamStateKind::DeviceLost;
    state.last_backend_message = Some(message.to_string());
}

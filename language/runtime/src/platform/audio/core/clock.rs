use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::{
    AudioClockDomain, AudioClockQuality, AudioClockSnapshot, AudioDeviceDirection,
    AudioStreamClockDomain,
};
use crate::runtime::BindingCallContext;

use super::AudioStreamBinding;

/// Return whether one stream exposes one requested clock domain.
fn stream_supports_clock_domain(
    binding: &AudioStreamBinding,
    domain: AudioStreamClockDomain,
) -> bool {
    match domain {
        AudioStreamClockDomain::Monotonic | AudioStreamClockDomain::Wall => true,
        AudioStreamClockDomain::Device => binding.runtime_capabilities.supports_hardware_timestamps,
        AudioStreamClockDomain::Callback => true,
        AudioStreamClockDomain::InputAdc => {
            binding.runtime_capabilities.supports_hardware_timestamps
                && matches!(
                    binding.direction,
                    AudioDeviceDirection::Capture
                        | AudioDeviceDirection::Duplex
                        | AudioDeviceDirection::Loopback
                )
        }
        AudioStreamClockDomain::OutputDac => {
            binding.runtime_capabilities.supports_hardware_timestamps
                && matches!(
                    binding.direction,
                    AudioDeviceDirection::Playback
                        | AudioDeviceDirection::Duplex
                        | AudioDeviceDirection::Loopback
                )
        }
    }
}

/// Return one operation timestamp for one requested domain.
pub(crate) fn clock_now_for_domain(
    binding: &BindingCallContext,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    match domain {
        AudioClockDomain::Monotonic => Ok(binding.world().mono_nanos()),
        AudioClockDomain::Wall => Ok(binding.world().wall_nanos()),
    }
}

/// Write one clock snapshot for one stream.
pub(crate) fn stream_clock_snapshot(
    binding_2: &BindingCallContext,
    binding: &AudioStreamBinding,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<AudioClockSnapshot> {
    if !stream_supports_clock_domain(binding, domain) {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.audio.clock.stream")).boxed(),
        );
    }

    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let monotonic_ns = binding_2.world().mono_nanos();
    let callback_ns = state.last_callback_mono_ns;
    let input_adc_ns = state.last_input_adc_ns;
    let output_dac_ns = state.last_output_dac_ns;

    let clock_ns = match domain {
        AudioStreamClockDomain::Monotonic => monotonic_ns,
        AudioStreamClockDomain::Wall => binding_2.world().wall_nanos(),
        AudioStreamClockDomain::Device => {
            if output_dac_ns > 0 {
                output_dac_ns
            } else if input_adc_ns > 0 {
                input_adc_ns
            } else {
                callback_ns
            }
        }
        AudioStreamClockDomain::Callback => {
            if callback_ns > 0 {
                callback_ns
            } else {
                monotonic_ns
            }
        }
        AudioStreamClockDomain::InputAdc => {
            if input_adc_ns > 0 {
                input_adc_ns
            } else {
                0
            }
        }
        AudioStreamClockDomain::OutputDac => {
            if output_dac_ns > 0 {
                output_dac_ns
            } else {
                0
            }
        }
    };

    let callback_quality = if callback_ns > 0 {
        AudioClockQuality::Estimated
    } else {
        AudioClockQuality::None
    };
    let input_quality = if input_adc_ns > 0 {
        AudioClockQuality::Hardware
    } else {
        AudioClockQuality::None
    };
    let output_quality = if output_dac_ns > 0 {
        AudioClockQuality::Hardware
    } else {
        AudioClockQuality::None
    };
    let device_ns = if output_dac_ns > 0 {
        output_dac_ns
    } else if input_adc_ns > 0 {
        input_adc_ns
    } else {
        callback_ns
    };
    let device_quality = if output_dac_ns > 0 || input_adc_ns > 0 {
        AudioClockQuality::Hardware
    } else if callback_ns > 0 {
        AudioClockQuality::Estimated
    } else {
        AudioClockQuality::None
    };
    let clock_quality = match domain {
        AudioStreamClockDomain::Monotonic | AudioStreamClockDomain::Wall => {
            AudioClockQuality::Estimated
        }
        AudioStreamClockDomain::Device => device_quality,
        AudioStreamClockDomain::Callback => callback_quality,
        AudioStreamClockDomain::InputAdc => input_quality,
        AudioStreamClockDomain::OutputDac => output_quality,
    };

    Ok(AudioClockSnapshot {
        stream_frames: state.stream_frames,
        clock_ns,
        clock_quality,
        callback_ns: if callback_ns > 0 {
            Some(callback_ns)
        } else {
            None
        },
        callback_quality,
        input_adc_ns: if input_adc_ns > 0 {
            Some(input_adc_ns)
        } else {
            None
        },
        input_adc_quality: input_quality,
        output_dac_ns: if output_dac_ns > 0 {
            Some(output_dac_ns)
        } else {
            None
        },
        output_dac_quality: output_quality,
        device_ns: if device_ns > 0 { Some(device_ns) } else { None },
        device_quality,
        monotonic_ns,
    })
}

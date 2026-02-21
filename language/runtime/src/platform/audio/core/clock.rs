use super::*;

/// Return one operation timestamp for one requested domain.
pub(crate) fn clock_now_for_domain(
    context: &BindingCallContext,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    match domain {
        AudioClockDomain::Monotonic => Ok(context.runtime().time.mono_nanos()),
        AudioClockDomain::Wall => Ok(context.runtime().time.wall_nanos()),
        AudioClockDomain::Device => Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.clock.now device domain",
        ))
        .boxed()),
    }
}

/// Write one clock snapshot for one stream.
pub(crate) fn stream_clock_snapshot(
    context: &BindingCallContext,
    binding: &AudioStreamBinding,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<AudioClockSnapshot> {
    let state = binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let monotonic_ns = context.runtime().time.mono_nanos();
    let callback_ns = state.last_callback_mono_ns;
    let input_adc_ns = state.last_input_adc_ns;
    let output_dac_ns = state.last_output_dac_ns;

    let clock_ns = match domain {
        AudioStreamClockDomain::Monotonic => monotonic_ns,
        AudioStreamClockDomain::Wall => context.runtime().time.wall_nanos(),
        AudioStreamClockDomain::Device => callback_ns.max(monotonic_ns),
        AudioStreamClockDomain::Callback => callback_ns.max(monotonic_ns),
        AudioStreamClockDomain::InputAdc => input_adc_ns.max(monotonic_ns),
        AudioStreamClockDomain::OutputDac => output_dac_ns.max(monotonic_ns),
    };

    Ok(AudioClockSnapshot {
        stream_frames: state.stream_frames,
        clock_ns,
        has_callback_ns: callback_ns > 0,
        callback_ns,
        has_input_adc_ns: input_adc_ns > 0,
        input_adc_ns,
        has_output_dac_ns: output_dac_ns > 0,
        output_dac_ns,
        monotonic_ns,
    })
}

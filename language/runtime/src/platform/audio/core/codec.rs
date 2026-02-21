use super::*;

/// Bytes per sample for unsigned 8-bit PCM.
const PCM_U8_BYTES: usize = 1;
/// Bytes per sample for signed 16-bit PCM.
const PCM_S16_BYTES: usize = 2;
/// Bytes per sample for packed 24-bit PCM in one 32-bit lane.
const PCM_S24_BYTES: usize = 4;
/// Bytes per sample for signed 32-bit PCM.
const PCM_S32_BYTES: usize = 4;
/// Bytes per sample for 32-bit float PCM.
const PCM_F32_BYTES: usize = 4;
/// Bytes per sample for 64-bit float PCM.
const PCM_F64_BYTES: usize = 8;

/// Unsigned 8-bit midpoint.
const PCM_U8_MIDPOINT: f32 = 128.0;
/// Unsigned 8-bit decode scale.
const PCM_U8_DECODE_SCALE: f32 = 128.0;
/// Unsigned 8-bit encode scale.
const PCM_U8_ENCODE_SCALE: f32 = 127.0;
/// Unsigned 8-bit minimum scalar value.
const PCM_U8_MIN: f32 = 0.0;
/// Unsigned 8-bit maximum scalar value.
const PCM_U8_MAX: f32 = 255.0;

/// Signed 16-bit decode scale.
const PCM_S16_DECODE_SCALE: f32 = 32_768.0;
/// Signed 16-bit encode scale.
const PCM_S16_ENCODE_SCALE: f32 = 32_767.0;

/// Signed 24-bit minimum scalar value.
const PCM_S24_MIN: i32 = -8_388_608;
/// Signed 24-bit maximum scalar value.
const PCM_S24_MAX: i32 = 8_388_607;
/// Signed 24-bit decode scale.
const PCM_S24_DECODE_SCALE: f32 = 8_388_608.0;
/// Signed 24-bit encode scale.
const PCM_S24_ENCODE_SCALE: f32 = 8_388_607.0;

/// Signed 32-bit decode scale.
const PCM_S32_DECODE_SCALE: f32 = 2_147_483_648.0;
/// Signed 32-bit encode scale.
const PCM_S32_ENCODE_SCALE: f32 = 2_147_483_647.0;

/// Return one format mask bit for one sample format.
pub(crate) fn sample_format_bit(format: AudioSampleFormat) -> u32 {
    1u32 << ((format as u8 as u32).saturating_sub(1))
}

/// Return one mask for all language-level sample formats.
pub(crate) fn all_sample_format_mask() -> u32 {
    sample_format_bit(AudioSampleFormat::U8)
        | sample_format_bit(AudioSampleFormat::S16)
        | sample_format_bit(AudioSampleFormat::S24)
        | sample_format_bit(AudioSampleFormat::S32)
        | sample_format_bit(AudioSampleFormat::F32)
        | sample_format_bit(AudioSampleFormat::F64)
}

/// Return bytes per scalar sample for one format.
pub(crate) fn sample_bytes(format: AudioSampleFormat) -> usize {
    match format {
        AudioSampleFormat::U8 => PCM_U8_BYTES,
        AudioSampleFormat::S16 => PCM_S16_BYTES,
        AudioSampleFormat::S24 => PCM_S24_BYTES,
        AudioSampleFormat::S32 => PCM_S32_BYTES,
        AudioSampleFormat::F32 => PCM_F32_BYTES,
        AudioSampleFormat::F64 => PCM_F64_BYTES,
    }
}

/// Return bytes per frame for one stream format.
pub(crate) fn frame_bytes(format: AudioSampleFormat, channels: u16) -> RuntimeResult<usize> {
    if channels == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.channels",
            "config.channels must be greater than zero",
        ))
        .boxed());
    }

    let value = sample_bytes(format)
        .checked_mul(channels as usize)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "config.channels",
                "frame size overflows host limits",
            ))
            .boxed()
        })?;

    Ok(value)
}

/// Clamp one floating-point scalar into normalized sample bounds.
pub(crate) fn clamp_audio_scalar(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

/// Decode one scalar sample lane from one byte slice.
pub(crate) fn decode_scalar_sample(format: AudioSampleFormat, bytes: &[u8]) -> Option<f32> {
    match format {
        AudioSampleFormat::U8 => {
            Some((bytes.first().copied()? as f32 - PCM_U8_MIDPOINT) / PCM_U8_DECODE_SCALE)
        }
        AudioSampleFormat::S16 => {
            let bytes = [bytes.first().copied()?, bytes.get(1).copied()?];
            Some(i16::from_le_bytes(bytes) as f32 / PCM_S16_DECODE_SCALE)
        }
        AudioSampleFormat::S24 => {
            let bytes = [
                bytes.first().copied()?,
                bytes.get(1).copied()?,
                bytes.get(2).copied()?,
                bytes.get(3).copied()?,
            ];
            let value = i32::from_le_bytes(bytes).clamp(PCM_S24_MIN, PCM_S24_MAX);
            Some(value as f32 / PCM_S24_DECODE_SCALE)
        }
        AudioSampleFormat::S32 => {
            let bytes = [
                bytes.first().copied()?,
                bytes.get(1).copied()?,
                bytes.get(2).copied()?,
                bytes.get(3).copied()?,
            ];
            Some(i32::from_le_bytes(bytes) as f32 / PCM_S32_DECODE_SCALE)
        }
        AudioSampleFormat::F32 => {
            let bytes = [
                bytes.first().copied()?,
                bytes.get(1).copied()?,
                bytes.get(2).copied()?,
                bytes.get(3).copied()?,
            ];
            Some(clamp_audio_scalar(f32::from_le_bytes(bytes)))
        }
        AudioSampleFormat::F64 => {
            let bytes = [
                bytes.first().copied()?,
                bytes.get(1).copied()?,
                bytes.get(2).copied()?,
                bytes.get(3).copied()?,
                bytes.get(4).copied()?,
                bytes.get(5).copied()?,
                bytes.get(6).copied()?,
                bytes.get(7).copied()?,
            ];
            Some(clamp_audio_scalar(f64::from_le_bytes(bytes) as f32))
        }
    }
}

/// Encode one scalar sample lane into one mutable byte slice.
pub(crate) fn encode_scalar_sample(
    format: AudioSampleFormat,
    sample: f32,
    output: &mut [u8],
) -> usize {
    let sample = clamp_audio_scalar(sample);
    match format {
        AudioSampleFormat::U8 => {
            output[0] = ((sample * PCM_U8_ENCODE_SCALE) + PCM_U8_MIDPOINT)
                .round()
                .clamp(PCM_U8_MIN, PCM_U8_MAX) as u8;
            PCM_U8_BYTES
        }
        AudioSampleFormat::S16 => {
            let value = (sample * PCM_S16_ENCODE_SCALE).round() as i16;
            output[..PCM_S16_BYTES].copy_from_slice(&value.to_le_bytes());
            PCM_S16_BYTES
        }
        AudioSampleFormat::S24 => {
            let value = (sample * PCM_S24_ENCODE_SCALE).round() as i32;
            output[..PCM_S24_BYTES].copy_from_slice(&value.to_le_bytes());
            PCM_S24_BYTES
        }
        AudioSampleFormat::S32 => {
            let value = (sample * PCM_S32_ENCODE_SCALE).round() as i32;
            output[..PCM_S32_BYTES].copy_from_slice(&value.to_le_bytes());
            PCM_S32_BYTES
        }
        AudioSampleFormat::F32 => {
            output[..PCM_F32_BYTES].copy_from_slice(&sample.to_le_bytes());
            PCM_F32_BYTES
        }
        AudioSampleFormat::F64 => {
            output[..PCM_F64_BYTES].copy_from_slice(&(sample as f64).to_le_bytes());
            PCM_F64_BYTES
        }
    }
}

/// Decode one byte payload into normalized scalar samples.
pub(crate) fn decode_audio_bytes(
    bytes: &[u8],
    format: AudioSampleFormat,
) -> RuntimeResult<Vec<f32>> {
    let unit = sample_bytes(format);
    if !bytes.len().is_multiple_of(unit) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "data",
            "payload length must align to sample size",
        ))
        .boxed());
    }

    let mut samples = Vec::with_capacity(bytes.len() / unit);
    for chunk in bytes.chunks_exact(unit) {
        let sample = decode_scalar_sample(format, chunk).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "data",
                "payload contains malformed sample lane",
            ))
            .boxed()
        })?;
        samples.push(sample);
    }

    Ok(samples)
}

/// Encode normalized scalar samples into one byte payload.
pub(crate) fn encode_audio_bytes(samples: &[f32], format: AudioSampleFormat) -> Vec<u8> {
    let unit = sample_bytes(format);
    let mut encoded = vec![0; samples.len().saturating_mul(unit)];

    for (index, sample) in samples.iter().enumerate() {
        let start = index.saturating_mul(unit);
        let end = start.saturating_add(unit);
        let _ = encode_scalar_sample(format, *sample, &mut encoded[start..end]);
    }

    encoded
}

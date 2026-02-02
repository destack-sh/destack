use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::native_call;
use crate::platform::random::bindings_generated as bindings;
use crate::platform::{NativeSlice, PlatformError, PlatformErrorCode, RuntimeStatus};
use crate::random::RandomStreamId;
use crate::replay::{RandomEvent, RandomEventKind, ReplayEvent};
use crate::runtime::RuntimeCallContext;

/// Return a deterministic random u64 for native code.
#[unsafe(export_name = "destack.random.nextU64")]
pub unsafe extern "C" fn destack_random_next_u64(out: *mut u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::NEXT_U64)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let value = context.runtime().random.next_u64();
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::RandomEvent(RandomEvent {
                stream_id: RandomStreamId::new(0),
                kind: RandomEventKind::NextU64,
                bytes: value.to_le_bytes().to_vec(),
            }));
        unsafe {
            *out = value;
        }

        Ok(())
    })
}

/// Fill a byte slice with deterministic random data for native code.
#[unsafe(export_name = "destack.random.fillBytes")]
pub unsafe extern "C" fn destack_random_fill_bytes(buffer: NativeSlice<u8>) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::FILL_BYTES)?;

        let slice = unsafe { buffer.as_mut_slice()? };
        fill_bytes_deterministic(context, slice)
    })
}

/// Fill a byte slice with secure random data for native code.
#[unsafe(export_name = "destack.random.secureBytes")]
pub unsafe extern "C" fn destack_random_secure_bytes(buffer: NativeSlice<u8>) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::SECURE_BYTES)?;

        let slice = unsafe { buffer.as_mut_slice()? };
        fill_bytes_secure(slice)?;
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::RandomEvent(RandomEvent {
                stream_id: RandomStreamId::new(0),
                kind: RandomEventKind::Bytes,
                bytes: slice.to_vec(),
            }));
        Ok(())
    })
}

/// Fill a buffer with deterministic random bytes from the runtime stream.
fn fill_bytes_deterministic(context: &RuntimeCallContext, buffer: &mut [u8]) -> RuntimeResult<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let value = context.runtime().random.next_u64().to_le_bytes();
        let remaining = buffer.len() - offset;
        let copy_len = remaining.min(value.len());
        buffer[offset..offset + copy_len].copy_from_slice(&value[..copy_len]);
        offset += copy_len;
    }

    context
        .runtime()
        .replay
        .record_event(ReplayEvent::RandomEvent(RandomEvent {
            stream_id: RandomStreamId::new(0),
            kind: RandomEventKind::Bytes,
            bytes: buffer.to_vec(),
        }));

    Ok(())
}

/// Fill a buffer with secure random bytes from the OS.
fn fill_bytes_secure(buffer: &mut [u8]) -> RuntimeResult<()> {
    getrandom::fill(buffer).map_err(|error| {
        RuntimeError::platform(PlatformError::random(
            Some(PlatformErrorCode::RandomUnavailable),
            format!("secure random failed: {error}"),
        ))
        .boxed()
    })?;

    Ok(())
}

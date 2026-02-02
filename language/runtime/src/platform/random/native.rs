use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::bindings_generated as bindings;
use crate::platform::{PlatformError, PlatformSlice, RuntimeStatus};
use crate::runtime::{RuntimeCallContext, with_runtime_call_context};

/// Return a deterministic random u64 for native code.
#[unsafe(export_name = "destack.random.nextU64")]
pub unsafe extern "C" fn destack_random_next_u64(out: *mut u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::NEXT_U64)?;

            if out.is_null() {
                return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
            }

            let value = context.runtime().random.next_u64();
            unsafe {
                *out = value;
            }

            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

/// Fill a byte slice with deterministic random data for native code.
#[unsafe(export_name = "destack.random.fillBytes")]
pub unsafe extern "C" fn destack_random_fill_bytes(buffer: PlatformSlice<u8>) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::FILL_BYTES)?;

            let slice = unsafe { buffer.as_mut_slice()? };
            fill_bytes(context, slice)
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

/// Fill a byte slice with secure random data for native code.
#[unsafe(export_name = "destack.random.secureBytes")]
pub unsafe extern "C" fn destack_random_secure_bytes(buffer: PlatformSlice<u8>) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::SECURE_BYTES)?;

            let slice = unsafe { buffer.as_mut_slice()? };
            fill_bytes(context, slice)
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

fn fill_bytes(context: &RuntimeCallContext, buffer: &mut [u8]) -> RuntimeResult<()> {
    // TODO #Incomplete: fill bytes uses the deterministic stream for now
    let mut offset = 0;
    while offset < buffer.len() {
        let value = context.runtime().random.next_u64().to_le_bytes();
        let remaining = buffer.len() - offset;
        let copy_len = remaining.min(value.len());
        buffer[offset..offset + copy_len].copy_from_slice(&value[..copy_len]);
        offset += copy_len;
    }

    Ok(())
}

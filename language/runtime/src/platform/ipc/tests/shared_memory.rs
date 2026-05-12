use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource;

use super::core::{assert_runtime_error_code, decode_mapping_value, unique_ipc_name};
use super::with_harness_context;

/// Verify shared-memory create, open, map, unmap, and close across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_shared_memory_roundtrip_mapping() {
    with_harness_context(|mut context| {
        // create one uniquely named shared-memory object
        let name = unique_ipc_name("ipc_shared_memory_roundtrip");
        let create_name = context.string_value(&name)?;
        let created = context.destack_ipc_shared_memory_create(create_name, 4096, 0)?;

        // open the same shared-memory object through a second handle
        let open_name = context.string_value(&name)?;
        let opened = context.destack_ipc_shared_memory_open(open_name, 0)?;

        // map both handles into process address space
        let created_mapping =
            decode_mapping_value(context.destack_ipc_shared_memory_map(created, 0, 4096, 0)?);
        let opened_mapping =
            decode_mapping_value(context.destack_ipc_shared_memory_map(opened, 0, 4096, 0)?);
        assert_ne!(created_mapping.address, 0);
        assert_eq!(created_mapping.length, 4096);
        assert_ne!(opened_mapping.address, 0);
        assert_eq!(opened_mapping.length, 4096);

        // write bytes through one mapping and verify from the other mapping
        unsafe {
            let writer = created_mapping.address as usize as *mut u8;
            writer.write(0x5A);
            writer.add(1).write(0xA5);

            let reader = opened_mapping.address as usize as *const u8;
            assert_eq!(reader.read(), 0x5A);
            assert_eq!(reader.add(1).read(), 0xA5);
        }

        // unmap both views and close both handles
        context.destack_ipc_shared_memory_unmap(opened_mapping.address, opened_mapping.length)?;
        context.destack_ipc_shared_memory_unmap(created_mapping.address, created_mapping.length)?;
        context.destack_ipc_shared_memory_close(opened)?;
        context.destack_ipc_shared_memory_close(created)?;

        Ok(())
    });
}

/// Verify shared-memory create and map reject unsupported flags.
#[cfg(any(unix, windows))]
#[test]
fn test_shared_memory_rejects_unsupported_flags() {
    with_harness_context(|mut context| {
        // create with non-zero flags should fail
        let name = unique_ipc_name("ipc_shared_memory_flags");
        let create_name = context.string_value(&name)?;
        let create_error = context.destack_ipc_shared_memory_create(create_name, 4096, 1);
        let create_error = match create_error {
            Ok(_) => panic!("expected sharedMemoryCreate to fail for unsupported flags"),
            Err(error) => error,
        };
        assert_runtime_error_code(&create_error, PlatformErrorCode::InvalidArgumentValue);

        // create one valid handle and verify map rejects non-zero flags
        let create_name = context.string_value(&name)?;
        let handle = context.destack_ipc_shared_memory_create(create_name, 4096, 0)?;
        let map_error = context.destack_ipc_shared_memory_map(handle, 0, 4096, 1);
        let map_error = match map_error {
            Ok(_) => panic!("expected sharedMemoryMap to fail for unsupported flags"),
            Err(error) => error,
        };
        assert_runtime_error_code(&map_error, PlatformErrorCode::InvalidArgumentValue);
        context.destack_ipc_shared_memory_close(handle)?;

        Ok(())
    });
}

/// Verify shared-memory operations reject unknown handles.
#[cfg(any(unix, windows))]
#[test]
fn test_shared_memory_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        // build one unknown shared-memory handle
        let unknown = resource::SharedMemoryHandle(resource::ResourceId::local(0));

        // map should fail with invalid-argument for unknown handle
        let map_error = context.destack_ipc_shared_memory_map(unknown, 0, 4096, 0);
        let map_error = match map_error {
            Ok(_) => panic!("expected sharedMemoryMap to fail for unknown handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&map_error, PlatformErrorCode::InvalidArgumentValue);

        // close should fail with invalid-argument for unknown handle
        let close_error = context.destack_ipc_shared_memory_close(unknown);
        let close_error = match close_error {
            Ok(_) => panic!("expected sharedMemoryClose to fail for unknown handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&close_error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

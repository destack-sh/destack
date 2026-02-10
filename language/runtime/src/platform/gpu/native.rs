#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::gpu::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::gpu::{
    GpuAdapterInfo, GpuBufferOptions, GpuCommandListOptions, GpuDeviceOptions, GpuMappedMemory,
    GpuPipelineOptions, GpuPresentOptions, GpuSamplerOptions, GpuShaderOptions, GpuSubmitOptions,
    GpuTextureOptions,
};
use crate::platform::resource;

/// Stub for destack.gpu.adapter.close.
pub unsafe fn destack_gpu_adapter_close(
    context: &RuntimeCallContext,
    handle: resource::GpuAdapterHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_ADAPTER_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.adapter.close")).boxed())
}

/// Stub for destack.gpu.adapter.list.
pub unsafe fn destack_gpu_adapter_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<GpuAdapterInfo>,
) -> RuntimeResult<()> {
    context.check_policy(GPU_ADAPTER_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.adapter.list")).boxed())
}

/// Stub for destack.gpu.adapter.open.
pub unsafe fn destack_gpu_adapter_open(
    context: &RuntimeCallContext,
    out: *mut resource::GpuAdapterHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(GPU_ADAPTER_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.adapter.open")).boxed())
}

/// Stub for destack.gpu.command.bindPipeline.
pub unsafe fn destack_gpu_command_bind_pipeline(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
    pipeline: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_BIND_PIPELINE)?;
    let _ = (handle, pipeline);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.bindPipeline",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.copyBuffer.
pub unsafe fn destack_gpu_command_copy_buffer(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
    src: resource::GpuBufferHandle,
    srcoffset: u64,
    dst: resource::GpuBufferHandle,
    dstoffset: u64,
    bytes: u64,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_COPY_BUFFER)?;
    let _ = (handle, src, srcoffset, dst, dstoffset, bytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyBuffer",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.dispatch.
pub unsafe fn destack_gpu_command_dispatch(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
    groupx: u32,
    groupy: u32,
    groupz: u32,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_DISPATCH)?;
    let _ = (handle, groupx, groupy, groupz);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.command.dispatch")).boxed())
}

/// Stub for destack.gpu.command.listBegin.
pub unsafe fn destack_gpu_command_list_begin(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_LIST_BEGIN)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listBegin",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listClose.
pub unsafe fn destack_gpu_command_list_close(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_LIST_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listClose",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listEnd.
pub unsafe fn destack_gpu_command_list_end(
    context: &RuntimeCallContext,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_LIST_END)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.command.listEnd")).boxed())
}

/// Stub for destack.gpu.command.listOpen.
pub unsafe fn destack_gpu_command_list_open(
    context: &RuntimeCallContext,
    out: *mut resource::GpuCommandListHandle,
    device: resource::GpuDeviceHandle,
    options: GpuCommandListOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_LIST_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.command.listOpen")).boxed())
}

/// Stub for destack.gpu.command.queueSubmit.
pub unsafe fn destack_gpu_queue_submit(
    context: &RuntimeCallContext,
    queue: resource::GpuQueueHandle,
    commandlists: NativeSlice<resource::GpuCommandListHandle>,
    options: GpuSubmitOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_QUEUE_SUBMIT)?;
    let _ = (queue, commandlists, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueSubmit",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.queueWaitIdle.
pub unsafe fn destack_gpu_queue_wait_idle(
    context: &RuntimeCallContext,
    queue: resource::GpuQueueHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(GPU_COMMAND_QUEUE_WAIT_IDLE)?;
    let _ = (queue, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueWaitIdle",
    ))
    .boxed())
}

/// Stub for destack.gpu.device.close.
pub unsafe fn destack_gpu_device_close(
    context: &RuntimeCallContext,
    handle: resource::GpuDeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_DEVICE_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.device.close")).boxed())
}

/// Stub for destack.gpu.device.open.
pub unsafe fn destack_gpu_device_open(
    context: &RuntimeCallContext,
    out: *mut resource::GpuDeviceHandle,
    adapter: resource::GpuAdapterHandle,
    options: GpuDeviceOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_DEVICE_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, adapter, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.device.open")).boxed())
}

/// Stub for destack.gpu.device.queue.
pub unsafe fn destack_gpu_device_queue(
    context: &RuntimeCallContext,
    out: *mut resource::GpuQueueHandle,
    device: resource::GpuDeviceHandle,
    family: u32,
    index: u32,
) -> RuntimeResult<()> {
    context.check_policy(GPU_DEVICE_QUEUE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, family, index);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.device.queue")).boxed())
}

/// Stub for destack.gpu.pipeline.create.
pub unsafe fn destack_gpu_pipeline_create(
    context: &RuntimeCallContext,
    out: *mut resource::GpuPipelineHandle,
    device: resource::GpuDeviceHandle,
    shaders: NativeSlice<resource::GpuShaderHandle>,
    options: GpuPipelineOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_PIPELINE_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, shaders, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.pipeline.create")).boxed())
}

/// Stub for destack.gpu.pipeline.destroy.
pub unsafe fn destack_gpu_pipeline_destroy(
    context: &RuntimeCallContext,
    handle: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_PIPELINE_DESTROY)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.gpu.pipeline.destroy")).boxed())
}

/// Stub for destack.gpu.pipeline.shaderCreate.
pub unsafe fn destack_gpu_shader_create(
    context: &RuntimeCallContext,
    out: *mut resource::GpuShaderHandle,
    device: resource::GpuDeviceHandle,
    options: GpuShaderOptions,
    bytes: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(GPU_PIPELINE_SHADER_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, options, bytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderCreate",
    ))
    .boxed())
}

/// Stub for destack.gpu.pipeline.shaderDestroy.
pub unsafe fn destack_gpu_shader_destroy(
    context: &RuntimeCallContext,
    handle: resource::GpuShaderHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_PIPELINE_SHADER_DESTROY)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderDestroy",
    ))
    .boxed())
}

/// Stub for destack.gpu.present.queuePresent.
pub unsafe fn destack_gpu_queue_present(
    context: &RuntimeCallContext,
    queue: resource::GpuQueueHandle,
    window: resource::WindowHandle,
    options: GpuPresentOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_PRESENT_QUEUE_PRESENT)?;
    let _ = (queue, window, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.queuePresent",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferCreate.
pub unsafe fn destack_gpu_buffer_create(
    context: &RuntimeCallContext,
    out: *mut resource::GpuBufferHandle,
    device: resource::GpuDeviceHandle,
    options: GpuBufferOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_BUFFER_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferCreate",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferDestroy.
pub unsafe fn destack_gpu_buffer_destroy(
    context: &RuntimeCallContext,
    handle: resource::GpuBufferHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_BUFFER_DESTROY)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferDestroy",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferRead.
pub unsafe fn destack_gpu_buffer_read(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::GpuBufferHandle,
    offset: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_BUFFER_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, offset, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferRead",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferWrite.
pub unsafe fn destack_gpu_buffer_write(
    context: &RuntimeCallContext,
    handle: resource::GpuBufferHandle,
    offset: u64,
    bytes: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_BUFFER_WRITE)?;
    let _ = (handle, offset, bytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferWrite",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.memoryMap.
pub unsafe fn destack_gpu_memory_map(
    context: &RuntimeCallContext,
    out: *mut GpuMappedMemory,
    handle: resource::GpuMemoryHandle,
    offset: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_MEMORY_MAP)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, offset, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.memoryMap",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.memoryUnmap.
pub unsafe fn destack_gpu_memory_unmap(
    context: &RuntimeCallContext,
    handle: resource::GpuMemoryHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_MEMORY_UNMAP)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.memoryUnmap",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.samplerCreate.
pub unsafe fn destack_gpu_sampler_create(
    context: &RuntimeCallContext,
    out: *mut resource::GpuSamplerHandle,
    device: resource::GpuDeviceHandle,
    options: GpuSamplerOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_SAMPLER_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.samplerCreate",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.samplerDestroy.
pub unsafe fn destack_gpu_sampler_destroy(
    context: &RuntimeCallContext,
    handle: resource::GpuSamplerHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_SAMPLER_DESTROY)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.samplerDestroy",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.textureCreate.
pub unsafe fn destack_gpu_texture_create(
    context: &RuntimeCallContext,
    out: *mut resource::GpuTextureHandle,
    device: resource::GpuDeviceHandle,
    options: GpuTextureOptions,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_TEXTURE_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureCreate",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.textureDestroy.
pub unsafe fn destack_gpu_texture_destroy(
    context: &RuntimeCallContext,
    handle: resource::GpuTextureHandle,
) -> RuntimeResult<()> {
    context.check_policy(GPU_RESOURCE_TEXTURE_DESTROY)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureDestroy",
    ))
    .boxed())
}

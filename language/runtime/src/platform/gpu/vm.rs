use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::gpu::{
    GpuAdapterInfoVm, GpuBufferOptionsVm, GpuCommandListOptionsVm, GpuDeviceOptionsVm,
    GpuMappedMemoryVm, GpuPipelineOptionsVm, GpuPresentOptionsVm, GpuSamplerOptionsVm,
    GpuShaderOptionsVm, GpuSubmitOptionsVm, GpuTextureOptionsVm,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.gpu.adapter.close.
pub(super) fn destack_gpu_adapter_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.adapter.list.
pub(super) fn destack_gpu_adapter_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<GpuAdapterInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.adapter.open.
pub(super) fn destack_gpu_adapter_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::GpuAdapterHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.bindPipeline.
pub(super) fn destack_gpu_command_bind_pipeline(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    pipeline: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    let _ = (handle, pipeline);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.bindPipeline is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.copyBuffer.
pub(super) fn destack_gpu_command_copy_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    src: resource::GpuBufferHandle,
    srcoffset: u64,
    dst: resource::GpuBufferHandle,
    dstoffset: u64,
    bytes: u64,
) -> RuntimeResult<()> {
    let _ = (handle, src, srcoffset, dst, dstoffset, bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.dispatch.
pub(super) fn destack_gpu_command_dispatch(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    groupx: u32,
    groupy: u32,
    groupz: u32,
) -> RuntimeResult<()> {
    let _ = (handle, groupx, groupy, groupz);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.dispatch is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listBegin.
pub(super) fn destack_gpu_command_list_begin(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listBegin is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listClose.
pub(super) fn destack_gpu_command_list_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listEnd.
pub(super) fn destack_gpu_command_list_end(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listEnd is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.listOpen.
pub(super) fn destack_gpu_command_list_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuCommandListOptionsVm,
) -> RuntimeResult<resource::GpuCommandListHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.listOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.queueSubmit.
pub(super) fn destack_gpu_queue_submit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    commandlists: VmSlice<resource::GpuCommandListHandle>,
    options: GpuSubmitOptionsVm,
) -> RuntimeResult<()> {
    let _ = (queue, commandlists, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueSubmit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.command.queueWaitIdle.
pub(super) fn destack_gpu_queue_wait_idle(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (queue, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueWaitIdle is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.device.close.
pub(super) fn destack_gpu_device_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.device.open.
pub(super) fn destack_gpu_device_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    adapter: resource::GpuAdapterHandle,
    options: GpuDeviceOptionsVm,
) -> RuntimeResult<resource::GpuDeviceHandle> {
    let _ = (adapter, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.device.queue.
pub(super) fn destack_gpu_device_queue(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    family: u32,
    index: u32,
) -> RuntimeResult<resource::GpuQueueHandle> {
    let _ = (device, family, index);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.queue is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.pipeline.create.
pub(super) fn destack_gpu_pipeline_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    shaders: VmSlice<resource::GpuShaderHandle>,
    options: GpuPipelineOptionsVm,
) -> RuntimeResult<resource::GpuPipelineHandle> {
    let _ = (device, shaders, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.create is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.pipeline.destroy.
pub(super) fn destack_gpu_pipeline_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.destroy is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.pipeline.shaderCreate.
pub(super) fn destack_gpu_shader_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuShaderOptionsVm,
    bytes: VmSlice<u8>,
) -> RuntimeResult<resource::GpuShaderHandle> {
    let _ = (device, options, bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.pipeline.shaderDestroy.
pub(super) fn destack_gpu_shader_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuShaderHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.present.queuePresent.
pub(super) fn destack_gpu_queue_present(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    window: resource::WindowHandle,
    options: GpuPresentOptionsVm,
) -> RuntimeResult<()> {
    let _ = (queue, window, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.queuePresent is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferCreate.
pub(super) fn destack_gpu_buffer_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuBufferOptionsVm,
) -> RuntimeResult<resource::GpuBufferHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferDestroy.
pub(super) fn destack_gpu_buffer_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferRead.
pub(super) fn destack_gpu_buffer_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
    offset: u64,
    length: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, offset, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.bufferWrite.
pub(super) fn destack_gpu_buffer_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
    offset: u64,
    bytes: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, offset, bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.memoryMap.
pub(super) fn destack_gpu_memory_map(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuMemoryHandle,
    offset: u64,
    length: u64,
) -> RuntimeResult<GpuMappedMemoryVm> {
    let _ = (handle, offset, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.memoryMap is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.memoryUnmap.
pub(super) fn destack_gpu_memory_unmap(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuMemoryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.memoryUnmap is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.samplerCreate.
pub(super) fn destack_gpu_sampler_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuSamplerOptionsVm,
) -> RuntimeResult<resource::GpuSamplerHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.samplerCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.samplerDestroy.
pub(super) fn destack_gpu_sampler_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuSamplerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.samplerDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.textureCreate.
pub(super) fn destack_gpu_texture_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuTextureOptionsVm,
) -> RuntimeResult<resource::GpuTextureHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.gpu.resource.textureDestroy.
pub(super) fn destack_gpu_texture_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuTextureHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureDestroy is not available in the VM yet",
    ))
    .boxed())
}

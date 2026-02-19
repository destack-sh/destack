#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::gpu::{
    GpuAdapterFormatCapabilitiesVm, GpuAdapterInfoVm, GpuAdapterLimitsVm, GpuAdapterRequestVm,
    GpuAdapterType, GpuBackend, GpuBindGroupEntryVm, GpuBindGroupLayoutEntryVm,
    GpuBindingResourceKind, GpuBlendComponentVm, GpuBlendFactor, GpuBlendOperation,
    GpuBlendStateVm, GpuBufferBindingType, GpuBufferCopyLayoutVm, GpuBufferCopyVm, GpuBufferInfoVm,
    GpuBufferMapState, GpuBufferOptionsVm, GpuCapturedErrorVm, GpuColorTargetStateVm,
    GpuColorWriteMask, GpuCommandEncoderOptionsVm, GpuCompareFunction, GpuCompilationInfoVm,
    GpuCompilationMessageKind, GpuCompilationMessageVm, GpuComputePassOptionsVm,
    GpuComputePipelineOptionsVm, GpuComputeStateVm, GpuCullMode, GpuDepthStencilStateVm,
    GpuDeviceInfoVm, GpuDeviceLossReason, GpuDeviceOptionsVm, GpuDeviceStatusVm, GpuErrorFilter,
    GpuExtent3DVm, GpuFeatureId, GpuFenceMode, GpuFenceOptionsVm, GpuFragmentStateVm, GpuFrontFace,
    GpuIndexFormat, GpuLoadOp, GpuMapMode, GpuMappedBufferRangeVm, GpuMultisampleStateVm,
    GpuPassTimestampWritesVm, GpuPipelineConstantVm, GpuPipelineLayoutOptionsVm,
    GpuPipelineMetadataVm, GpuPipelineStatisticsMask, GpuPowerPreference, GpuPresentMode,
    GpuPresentOptionsVm, GpuPrimitiveStateVm, GpuPrimitiveTopology, GpuQuerySetInfoVm,
    GpuQuerySetOptionsVm, GpuQueryType, GpuRenderBundleEncoderOptionsVm,
    GpuRenderPassColorAttachmentVm, GpuRenderPassDepthStencilAttachmentVm, GpuRenderPassOptionsVm,
    GpuRenderPipelineOptionsVm, GpuRenderStateVm, GpuSamplerBindingType, GpuSamplerOptionsVm,
    GpuShaderOptionsVm, GpuShaderVisibilityMask, GpuStencilFaceStateVm, GpuStencilOperation,
    GpuStorageTextureAccess, GpuStoreOp, GpuSubmitOptionsVm, GpuSurfaceAcquireStatus,
    GpuSurfaceAlphaMode, GpuSurfaceCapabilitiesVm, GpuSurfaceFrameVm, GpuSurfaceOptionsVm,
    GpuTextureCopyVm, GpuTextureDimension, GpuTextureInfoVm, GpuTextureOptionsVm,
    GpuTextureSampleType, GpuTextureViewDimension, GpuTextureViewOptionsVm, GpuVertexAttributeVm,
    GpuVertexBufferLayoutVm, GpuVertexStateVm, GpuVertexStepMode,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Close one GPU adapter.
///
/// Close one opened adapter endpoint and release host backend references.
/// The adapter handle becomes invalid after close.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style release or destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_close(
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

/// Read one adapter feature snapshot.
///
/// Read one feature identifier snapshot from one opened adapter.
/// Feature identifiers follow runtime GPU feature contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter feature query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_features(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
) -> RuntimeResult<VmSlice<GpuFeatureId>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.features is not available in the VM yet",
    ))
    .boxed())
}

/// Query one adapter texture-format capability snapshot.
///
/// Query one texture format on one adapter and return normalized format capability data.
/// Capabilities can vary by backend and device generation.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter format-capability query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_format_capabilities(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
    format: u32,
) -> RuntimeResult<GpuAdapterFormatCapabilitiesVm> {
    let _ = (handle, format);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.formatCapabilities is not available in the VM yet",
    ))
    .boxed())
}

/// Check whether one adapter supports one feature.
///
/// Check one feature identifier against one opened adapter feature set.
/// Feature identifiers follow runtime GPU feature contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter feature query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_has_feature(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
    feature: GpuFeatureId,
) -> RuntimeResult<bool> {
    let _ = (handle, feature);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.hasFeature is not available in the VM yet",
    ))
    .boxed())
}

/// Read metadata for one opened adapter.
///
/// Query the current normalized adapter metadata through an opened adapter handle.
/// Returned values reflect host backend limits and feature reporting.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter property and feature query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
) -> RuntimeResult<GpuAdapterInfoVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.info is not available in the VM yet",
    ))
    .boxed())
}

/// Read one adapter limits snapshot.
///
/// Read one normalized limits snapshot from one opened adapter.
/// Limits remain stable for the adapter lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter limits query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_limits(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuAdapterHandle,
) -> RuntimeResult<GpuAdapterLimitsVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.limits is not available in the VM yet",
    ))
    .boxed())
}

/// List available GPU adapters.
///
/// Enumerate host GPU adapters and return stable identifiers with normalized capability metadata.
/// Adapter visibility and ordering follow host graphics API enumeration behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter discovery and enumeration on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: GpuAdapterRequestVm,
) -> RuntimeResult<VmArray<GpuAdapterInfoVm>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.adapter.list is not available in the VM yet",
    ))
    .boxed())
}

/// Open one GPU adapter.
///
/// Open one host GPU adapter endpoint for device creation and capability queries.
/// Handle lifetime follows host backend object ownership semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style adapter open or retain operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.adapter`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_adapter_open(
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

/// Create one bind group.
///
/// Create one bind group from one layout and explicit resource entries.
/// Resource compatibility is validated against the target layout.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bind-group or descriptor-set allocation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_bind_group_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    layout: resource::GpuBindGroupLayoutHandle,
    entries: VmSlice<GpuBindGroupEntryVm>,
    flags: u32,
) -> RuntimeResult<resource::GpuBindGroupHandle> {
    let _ = (device, layout, entries, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.groupCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one bind group.
///
/// Destroy one bind group and release backend descriptor allocation resources.
/// The bind group handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bind-group destroy or free operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_bind_group_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBindGroupHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.groupDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Create one bind group layout.
///
/// Create one bind group layout descriptor from ordered binding entries.
/// Validation rules follow backend pipeline-layout contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bind-group or descriptor-set layout creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_bind_group_layout_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    entries: VmSlice<GpuBindGroupLayoutEntryVm>,
    flags: u32,
) -> RuntimeResult<resource::GpuBindGroupLayoutHandle> {
    let _ = (device, entries, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.groupLayoutCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one bind group layout.
///
/// Destroy one bind group layout and release backend descriptor-layout resources.
/// The layout handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style layout destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_bind_group_layout_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBindGroupLayoutHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.groupLayoutDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Create one pipeline layout.
///
/// Create one pipeline layout from ordered bind group layouts.
/// Backend layout compatibility checks occur during creation.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style pipeline-layout creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_pipeline_layout_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuPipelineLayoutOptionsVm,
) -> RuntimeResult<resource::GpuPipelineLayoutHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.pipelineLayoutCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one pipeline layout.
///
/// Destroy one pipeline layout and release backend layout resources.
/// The layout handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style pipeline-layout destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_pipeline_layout_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuPipelineLayoutHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.bind.pipelineLayoutDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one compute pipeline.
///
/// Bind one compute pipeline to one active compute pass for subsequent dispatch operations.
/// Binding state remains active until changed or encoder reset.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style compute pipeline bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_bind_compute_pipeline(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    pipeline: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    let _ = (handle, pipeline);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.bindComputePipeline is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one render pipeline.
///
/// Bind one render pipeline to one active render pass for subsequent draw operations.
/// Binding state remains active until changed or encoder reset.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render pipeline bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_bind_render_pipeline(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    pipeline: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    let _ = (handle, pipeline);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.bindRenderPipeline is not available in the VM yet",
    ))
    .boxed())
}

/// Clear one buffer range.
///
/// Encode one clear operation that writes zero to one buffer range.
/// Offset and size alignment follow backend clear-buffer constraints.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style clear-buffer commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_clear_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.clearBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Begin one compute pass.
///
/// Begin compute-pass encoding on one command encoder with optional timestamp writes.
/// Returns one compute-pass handle for pass-scoped commands.
/// Nested pass semantics follow backend command recording rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style begin compute pass commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_compute_pass_begin(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    options: GpuComputePassOptionsVm,
) -> RuntimeResult<()> {
    let _ = (handle, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.computePassBegin is not available in the VM yet",
    ))
    .boxed())
}

/// End one compute pass.
///
/// End compute-pass encoding for one active compute-pass handle.
/// Pass finalization follows backend validation behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style end compute pass commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_compute_pass_end(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.computePassEnd is not available in the VM yet",
    ))
    .boxed())
}

/// Copy one buffer range.
///
/// Encode one buffer-to-buffer copy command with explicit offsets and size.
/// Copy constraints follow backend alignment and usage rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style copy buffer commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_copy_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    src: resource::GpuBufferHandle,
    srcoffset: u64,
    dst: resource::GpuBufferHandle,
    dstoffset: u64,
    argument_bytes: u64,
) -> RuntimeResult<()> {
    let _ = (handle, src, srcoffset, dst, dstoffset, argument_bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Copy one buffer range into one texture range.
///
/// Encode one buffer-to-texture copy command with explicit source buffer layout and destination texture metadata.
/// Copy constraints follow backend format conversion and row alignment rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style copy buffer to texture commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_copy_buffer_to_texture(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    source: GpuBufferCopyVm,
    destination: GpuTextureCopyVm,
    size: GpuExtent3DVm,
) -> RuntimeResult<()> {
    let _ = (handle, source, destination, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyBufferToTexture is not available in the VM yet",
    ))
    .boxed())
}

/// Copy one texture range into one buffer range.
///
/// Encode one texture-to-buffer copy command with explicit source texture metadata and destination buffer layout.
/// Copy constraints follow backend format conversion and row alignment rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style copy texture to buffer commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_copy_texture_to_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    source: GpuTextureCopyVm,
    destination: GpuBufferCopyVm,
    size: GpuExtent3DVm,
) -> RuntimeResult<()> {
    let _ = (handle, source, destination, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyTextureToBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Copy one texture range to another texture.
///
/// Encode one texture-to-texture copy command with explicit source, destination, and extent metadata.
/// Copy constraints follow backend format compatibility and alignment rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style copy texture commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_copy_texture_to_texture(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    source: GpuTextureCopyVm,
    destination: GpuTextureCopyVm,
    size: GpuExtent3DVm,
) -> RuntimeResult<()> {
    let _ = (handle, source, destination, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.copyTextureToTexture is not available in the VM yet",
    ))
    .boxed())
}

/// Dispatch one compute workgroup grid.
///
/// Dispatch one compute workload with explicit workgroup counts.
/// Grid interpretation follows backend compute pipeline contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style dispatch commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_dispatch(
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

/// Dispatch one compute workload from one indirect buffer argument.
///
/// Dispatch one compute workload using dispatch dimensions loaded from one buffer.
/// Buffer layout must match backend indirect-dispatch argument encoding.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style dispatch-indirect commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_dispatch_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.dispatchIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Draw one non-indexed primitive range.
///
/// Encode one non-indexed draw call with explicit vertex and instance ranges.
/// Vertex fetch and instance stepping follow backend render pipeline rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style draw commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_draw(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    vertexcount: u32,
    instancecount: u32,
    firstvertex: u32,
    firstinstance: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        vertexcount,
        instancecount,
        firstvertex,
        firstinstance,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.draw is not available in the VM yet",
    ))
    .boxed())
}

/// Draw one indexed primitive range.
///
/// Encode one indexed draw call with explicit index and instance ranges.
/// Index format and base vertex behavior follow backend draw-indexed semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style indexed draw commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_draw_indexed(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    indexcount: u32,
    instancecount: u32,
    firstindex: u32,
    basevertex: i32,
    firstinstance: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        indexcount,
        instancecount,
        firstindex,
        basevertex,
        firstinstance,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.drawIndexed is not available in the VM yet",
    ))
    .boxed())
}

/// Draw one indirect indexed command range.
///
/// Encode one or more indexed draw calls loaded from one argument buffer.
/// Indirect argument layout and alignment follow backend indexed draw-indirect contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style indexed draw-indirect commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_draw_indexed_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.drawIndexedIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Draw one indirect non-indexed command range.
///
/// Encode one or more non-indexed draw calls loaded from one argument buffer.
/// Indirect argument layout and alignment follow backend draw-indirect contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style draw-indirect commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_draw_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.drawIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Close one command encoder.
///
/// Close one command encoder object and release backend recording resources.
/// The encoder handle becomes invalid after close.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style command encoder destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_encoder_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.encoderClose is not available in the VM yet",
    ))
    .boxed())
}

/// Finish one command encoder.
///
/// Finalize recording for one command encoder and make it ready for queue submission.
/// Finalization semantics follow backend validation rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style end command recording calls on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_encoder_finish(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.encoderFinish is not available in the VM yet",
    ))
    .boxed())
}

/// Open one command encoder.
///
/// Open one command encoder object for recording backend commands.
/// Encoder lifecycle follows backend recording and submission semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style command encoder or command buffer creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_encoder_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuCommandEncoderOptionsVm,
) -> RuntimeResult<resource::GpuCommandListHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.encoderOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Execute one batch of render bundles in the active render pass.
///
/// Execute recorded render bundles in-order within one active render pass.
/// Bundle compatibility is validated against current render-pass configuration.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style execute-bundle commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_execute_bundles(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    bundles: VmSlice<resource::GpuRenderBundleHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, bundles);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.executeBundles is not available in the VM yet",
    ))
    .boxed())
}

/// Insert one debug marker in one command-encoder scope.
///
/// Insert one lightweight debug marker in one active command-encoder scope.
/// Marker visibility is backend-defined and intended for tooling.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style debug-marker insertion commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_insert_debug_marker(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    marker: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, marker);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.insertDebugMarker is not available in the VM yet",
    ))
    .boxed())
}

/// Draw multiple indirect indexed command ranges.
///
/// Encode multiple indexed draws loaded from one argument buffer.
/// Draw-count and argument layout follow backend multi-draw contracts.
/// This operation is one optional feature lane and can return `notSupported`.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific multi-draw-indexed-indirect commands where exposed.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render.multiDraw`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_multi_draw_indexed_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.multiDrawIndexedIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Draw multiple indirect indexed command ranges with one host-visible count buffer.
///
/// Encode multiple indexed draws loaded from one argument buffer with draw count read from one count buffer.
/// This operation requires one backend feature lane that enables indirect-count draws.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific multi-draw-indexed-indirect-count commands where exposed.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render.multiDrawCount`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_multi_draw_indexed_indirect_count(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    countbuffer: resource::GpuBufferHandle,
    countoffset: u64,
    maxdrawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        buffer,
        offset,
        countbuffer,
        countoffset,
        maxdrawcount,
        stride,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.multiDrawIndexedIndirectCount is not available in the VM yet",
    ))
    .boxed())
}

/// Draw multiple indirect non-indexed command ranges.
///
/// Encode multiple non-indexed draws loaded from one argument buffer.
/// Draw-count and argument layout follow backend multi-draw contracts.
/// This operation is one optional feature lane and can return `notSupported`.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific multi-draw-indirect commands where exposed.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render.multiDraw`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_multi_draw_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.multiDrawIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Draw multiple indirect non-indexed command ranges with one host-visible count buffer.
///
/// Encode multiple non-indexed draws loaded from one argument buffer with draw count read from one count buffer.
/// This operation requires one backend feature lane that enables indirect-count draws.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific multi-draw-indirect-count commands where exposed.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render.multiDrawCount`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_multi_draw_indirect_count(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    countbuffer: resource::GpuBufferHandle,
    countoffset: u64,
    maxdrawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        buffer,
        offset,
        countbuffer,
        countoffset,
        maxdrawcount,
        stride,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.multiDrawIndirectCount is not available in the VM yet",
    ))
    .boxed())
}

/// Pop one debug group in one command-encoder scope.
///
/// Pop one previously pushed debug group in one active command-encoder scope.
/// Pop fails when no matching group exists.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style debug-group pop commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_pop_debug_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.popDebugGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Push one debug group in one command-encoder scope.
///
/// Push one nested debug group in one active command-encoder scope.
/// Groups must be balanced with matching pop operations.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style debug-group push commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_push_debug_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    label: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, label);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.pushDebugGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Submit one command encoder batch to one queue.
///
/// Submit one batch of finalized command encoders to one queue with explicit submit options.
/// Queue ordering and dependency behavior follow backend queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue submit APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_submit(
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

/// Wait for one queue to become idle.
///
/// Wait until one queue has no remaining submitted work.
/// Wait semantics follow backend queue fence and idle behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue wait-idle or fence wait APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_wait_idle(
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

/// Write host bytes into one GPU buffer through one queue.
///
/// Upload one byte range into a destination buffer using backend queue upload operations.
/// Data offset and size are interpreted in bytes.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue write-buffer operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.queue`, `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_write_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    buffer: resource::GpuBufferHandle,
    bufferoffset: u64,
    data: VmSlice<u8>,
    dataoffset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (queue, buffer, bufferoffset, data, dataoffset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueWriteBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Write host bytes into one GPU texture through one queue.
///
/// Upload one texel payload into a destination texture using backend queue upload operations.
/// Layout metadata controls row and image stride for source bytes.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue write-texture operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.queue`, `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_write_texture(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    destination: GpuTextureCopyVm,
    data: VmSlice<u8>,
    layout: GpuBufferCopyLayoutVm,
    size: GpuExtent3DVm,
) -> RuntimeResult<()> {
    let _ = (queue, destination, data, layout, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.queueWriteTexture is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one render bundle.
///
/// Destroy one render-bundle object and release backend command storage.
/// The bundle handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Encode one non-indexed draw in one bundle encoder.
///
/// Encode one non-indexed draw call with explicit vertex and instance ranges.
/// Draw semantics follow backend bundle-encoding contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle draw commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_draw(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    vertexcount: u32,
    instancecount: u32,
    firstvertex: u32,
    firstinstance: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        vertexcount,
        instancecount,
        firstvertex,
        firstinstance,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleDraw is not available in the VM yet",
    ))
    .boxed())
}

/// Encode one indexed draw in one bundle encoder.
///
/// Encode one indexed draw call with explicit index and instance ranges.
/// Draw semantics follow backend bundle-encoding contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle indexed draw commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_draw_indexed(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    indexcount: u32,
    instancecount: u32,
    firstindex: u32,
    basevertex: i32,
    firstinstance: u32,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        indexcount,
        instancecount,
        firstindex,
        basevertex,
        firstinstance,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleDrawIndexed is not available in the VM yet",
    ))
    .boxed())
}

/// Encode one indirect indexed draw range in one bundle encoder.
///
/// Encode one or more indexed draws loaded from one argument buffer.
/// Indirect argument layout and alignment follow backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle indexed draw-indirect commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_draw_indexed_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleDrawIndexedIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Encode one indirect non-indexed draw range in one bundle encoder.
///
/// Encode one or more non-indexed draws loaded from one argument buffer.
/// Indirect argument layout and alignment follow backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle draw-indirect commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_draw_indirect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    drawcount: u32,
    stride: u32,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, offset, drawcount, stride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleDrawIndirect is not available in the VM yet",
    ))
    .boxed())
}

/// Close one render-bundle encoder without producing a bundle.
///
/// Close one opened bundle encoder and release recording resources.
/// Any recorded commands are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bundle-encoder destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_encoder_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleEncoderClose is not available in the VM yet",
    ))
    .boxed())
}

/// Finish one render-bundle encoder.
///
/// Finalize one render-bundle encoder and produce one reusable render bundle.
/// The encoder handle becomes invalid after successful finish.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bundle finalize operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_encoder_finish(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
) -> RuntimeResult<resource::GpuRenderBundleHandle> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleEncoderFinish is not available in the VM yet",
    ))
    .boxed())
}

/// Open one render-bundle encoder.
///
/// Open one render-bundle encoder object for pre-recording reusable draw commands.
/// Bundle-encoder configuration defines attachment compatibility requirements.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle or secondary-command-buffer creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_encoder_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuRenderBundleEncoderOptionsVm,
) -> RuntimeResult<resource::GpuRenderBundleEncoderHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleEncoderOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Insert one debug marker in one bundle encoder.
///
/// Insert one lightweight marker in one active render-bundle encoding scope.
/// Marker visibility is backend-defined and intended for tooling.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle marker commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_insert_debug_marker(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    marker: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, marker);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleInsertDebugMarker is not available in the VM yet",
    ))
    .boxed())
}

/// Pop one debug group in one bundle encoder.
///
/// Pop one previously pushed debug group in one bundle-encoder scope.
/// Pop fails when no matching group exists.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle debug-group pop commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_pop_debug_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundlePopDebugGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Push one debug group in one bundle encoder.
///
/// Push one nested debug group in one render-bundle encoder scope.
/// Groups must be balanced with matching pop operations.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle debug-group push commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_push_debug_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    label: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, label);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundlePushDebugGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one bind group in one bundle encoder.
///
/// Bind one bind group at the requested index for bundle recording.
/// Dynamic offsets are interpreted in backend-defined binding order.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle bind-group commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_set_bind_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    index: u32,
    bindgroup: resource::GpuBindGroupHandle,
    dynamicoffsets: VmSlice<u32>,
) -> RuntimeResult<()> {
    let _ = (handle, index, bindgroup, dynamicoffsets);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleSetBindGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one index buffer in one bundle encoder.
///
/// Bind one index buffer with explicit format and byte range metadata.
/// Index fetch semantics follow backend draw-indexed contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle index-buffer bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_set_index_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    buffer: resource::GpuBufferHandle,
    format: GpuIndexFormat,
    offset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, format, offset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleSetIndexBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one render pipeline for one bundle encoder.
///
/// Bind one render pipeline in one render-bundle encoder.
/// Binding state remains active for subsequent bundle draw commands.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle pipeline bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_set_pipeline(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    pipeline: resource::GpuPipelineHandle,
) -> RuntimeResult<()> {
    let _ = (handle, pipeline);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleSetPipeline is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one vertex buffer in one bundle encoder.
///
/// Bind one vertex buffer slot with explicit byte range metadata.
/// Vertex fetch semantics follow the bound render pipeline.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render-bundle vertex-buffer bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_bundle_set_vertex_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuRenderBundleEncoderHandle,
    slot: u32,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (handle, slot, buffer, offset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderBundleSetVertexBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Begin one render pass.
///
/// Begin render-pass encoding on one command encoder with explicit attachments and optional query wiring.
/// Attachment load, clear, timestamp, and occlusion behavior follow backend render pass semantics.
/// Returns one render-pass handle for pass-scoped commands.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style begin render pass commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_render_pass_begin(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    options: GpuRenderPassOptionsVm,
) -> RuntimeResult<()> {
    let _ = (handle, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderPassBegin is not available in the VM yet",
    ))
    .boxed())
}

/// End one render pass.
///
/// End render-pass encoding for one active render-pass handle.
/// Pass finalization follows backend validation behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style end render pass commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_render_pass_end(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.renderPassEnd is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one bind group for subsequent commands.
///
/// Bind one bind group at the requested index for the current pass state.
/// Dynamic offsets are interpreted in backend-defined order for dynamic bindings.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style bind-group or descriptor-set bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_bind_group(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    index: u32,
    bindgroup: resource::GpuBindGroupHandle,
    dynamicoffsets: VmSlice<u32>,
) -> RuntimeResult<()> {
    let _ = (handle, index, bindgroup, dynamicoffsets);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setBindGroup is not available in the VM yet",
    ))
    .boxed())
}

/// Set one blend constant for the active render pass.
///
/// Set one blend constant used by blend factors that reference constant color.
/// Constant color remains active until changed or render pass ends.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style blend-constant state commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_blend_constant(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
) -> RuntimeResult<()> {
    let _ = (handle, r, g, b, a);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setBlendConstant is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one index buffer for subsequent indexed draw commands.
///
/// Bind one index buffer with explicit format and byte range metadata.
/// Index fetch semantics follow backend draw-indexed contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style index-buffer bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_index_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    buffer: resource::GpuBufferHandle,
    format: GpuIndexFormat,
    offset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (handle, buffer, format, offset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setIndexBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Set one scissor rectangle for the active render pass.
///
/// Set one scissor rectangle that clips subsequent draw calls.
/// Rectangle coordinates are expressed in framebuffer pixels.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style scissor state commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_scissor(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> RuntimeResult<()> {
    let _ = (handle, x, y, width, height);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setScissor is not available in the VM yet",
    ))
    .boxed())
}

/// Set one stencil-reference value for the active render pass.
///
/// Set one stencil-reference value consumed by stencil compare operations.
/// The reference value remains active until changed or render pass ends.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style stencil-reference state commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_stencil_reference(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    reference: u32,
) -> RuntimeResult<()> {
    let _ = (handle, reference);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setStencilReference is not available in the VM yet",
    ))
    .boxed())
}

/// Bind one vertex buffer for subsequent draw commands.
///
/// Bind one vertex buffer slot with explicit byte range metadata.
/// Vertex fetch semantics follow the active render pipeline state.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style vertex-buffer bind commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_vertex_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    slot: u32,
    buffer: resource::GpuBufferHandle,
    offset: u64,
    size: u64,
) -> RuntimeResult<()> {
    let _ = (handle, slot, buffer, offset, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setVertexBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Set one viewport state for the active render pass.
///
/// Set one viewport rectangle and depth range for subsequent draw calls.
/// Coordinate interpretation follows backend clip-space conventions.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style viewport state commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_set_viewport(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuCommandListHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    mindepth: f64,
    maxdepth: f64,
) -> RuntimeResult<()> {
    let _ = (handle, x, y, width, height, mindepth, maxdepth);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.command.setViewport is not available in the VM yet",
    ))
    .boxed())
}

/// Set one debug label on one GPU resource object.
///
/// Set one human-readable label on one GPU resource or pipeline object.
/// Label visibility and truncation behavior are backend-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style object-label operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.debug`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_set_label(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::ResourceId,
    label: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, label);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.debug.setLabel is not available in the VM yet",
    ))
    .boxed())
}

/// Close one logical GPU device.
///
/// Close one logical device and release backend device resources.
/// Outstanding queue work behavior follows host backend teardown guarantees.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device destroy or release operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_close(
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

/// Read one device feature snapshot.
///
/// Read one enabled feature identifier snapshot from one logical device.
/// Feature identifiers follow runtime GPU feature contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device feature query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_features(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
) -> RuntimeResult<VmSlice<GpuFeatureId>> {
    let _ = device;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.features is not available in the VM yet",
    ))
    .boxed())
}

/// Check whether one device supports one feature.
///
/// Check one feature identifier against one logical device enabled feature set.
/// Feature identifiers follow runtime GPU feature contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device feature query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_has_feature(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    feature: GpuFeatureId,
) -> RuntimeResult<bool> {
    let _ = (device, feature);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.hasFeature is not available in the VM yet",
    ))
    .boxed())
}

/// Read metadata for one logical device.
///
/// Query effective features and limits for one created logical device.
/// Results reflect backend feature enablement and negotiation outcomes.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device feature and limits query paths on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
) -> RuntimeResult<GpuDeviceInfoVm> {
    let _ = device;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.info is not available in the VM yet",
    ))
    .boxed())
}

/// Read one device limits snapshot.
///
/// Read one effective limits snapshot from one logical device.
/// Limits remain stable for the device lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device limits query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_limits(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
) -> RuntimeResult<GpuAdapterLimitsVm> {
    let _ = device;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.limits is not available in the VM yet",
    ))
    .boxed())
}

/// Open one logical GPU device.
///
/// Create one logical device from one adapter using required features and limits.
/// Device creation fails when the backend cannot satisfy the requested contract.
///
/// # Platform
/// Unix and Windows.
/// Uses vkCreateDevice, MTLDevice creation, or D3D12 device creation.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_open(
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

/// Poll one logical device for completion progress.
///
/// Poll one device and optionally wait for work completion until the timeout expires.
/// Returned value is a backend-defined count of completed submission units.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device poll or fence polling operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_poll(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    wait: bool,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (device, wait, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.poll is not available in the VM yet",
    ))
    .boxed())
}

/// Pop one device error scope.
///
/// Pop one previously pushed error scope and return captured error details.
/// Pop fails when no scope exists on the device scope stack.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device error-scope pop operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_pop_error_scope(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<GpuCapturedErrorVm> {
    let _ = (device, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.popErrorScope is not available in the VM yet",
    ))
    .boxed())
}

/// Push one device error scope.
///
/// Push one error scope to capture asynchronous validation and runtime errors.
/// Scopes are popped in last-in-first-out order.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device error-scope push operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_push_error_scope(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    filter: GpuErrorFilter,
) -> RuntimeResult<()> {
    let _ = (device, filter);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.pushErrorScope is not available in the VM yet",
    ))
    .boxed())
}

/// Resolve one default queue for one logical device.
///
/// Resolve one queue endpoint for command submission from one logical device.
/// Queue identity follows backend default-queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue lookup operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.queue`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_queue(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
) -> RuntimeResult<resource::GpuQueueHandle> {
    let _ = device;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.queue is not available in the VM yet",
    ))
    .boxed())
}

/// Read health status for one logical device.
///
/// Return one snapshot of device-loss and backend health state.
/// Status can transition asynchronously as backend work progresses.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style device-lost and status query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_device_status(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
) -> RuntimeResult<GpuDeviceStatusVm> {
    let _ = device;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.device.status is not available in the VM yet",
    ))
    .boxed())
}

/// Resolve one bind-group layout from one pipeline.
///
/// Resolve one bind-group layout at the requested group index from one pipeline object.
/// This follows pipeline-layout reflection rules of the selected backend.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style pipeline bind-group-layout query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.bind`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_pipeline_bind_group_layout(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    pipeline: resource::GpuPipelineHandle,
    groupindex: u32,
) -> RuntimeResult<resource::GpuBindGroupLayoutHandle> {
    let _ = (pipeline, groupindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.bindGroupLayout is not available in the VM yet",
    ))
    .boxed())
}

/// Create one compute pipeline.
///
/// Create one compute pipeline from one compute stage descriptor and one explicit or inferred layout.
/// Pipeline compilation and cache behavior follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style compute pipeline creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.compute`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_compute_pipeline_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuComputePipelineOptionsVm,
) -> RuntimeResult<resource::GpuPipelineHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.computeCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one pipeline object.
///
/// Destroy one pipeline object and release backend compiled state.
/// Outstanding command lists referencing the pipeline must be synchronized by callers.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style pipeline destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.compute`, `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_pipeline_destroy(
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

/// Create one render pipeline.
///
/// Create one render pipeline with explicit shader stages, one explicit or inferred layout, and render-state descriptors.
/// Pipeline compilation and backend render-state linkage follow host API semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style render pipeline creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.render`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_render_pipeline_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuRenderPipelineOptionsVm,
) -> RuntimeResult<resource::GpuPipelineHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.renderCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Read one shader-compilation diagnostics snapshot.
///
/// Read one snapshot of compilation diagnostics for one shader module.
/// Diagnostics are backend-defined and can include warnings and informational messages.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style shader compilation-info query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.shader`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_shader_compilation_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuShaderHandle,
    timeoutns: u64,
) -> RuntimeResult<GpuCompilationInfoVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderCompilationInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Create one shader module.
///
/// Create one shader module from one source or binary payload and module metadata.
/// Stage and entry-point selection are defined by pipeline stage descriptors, not by shader-module creation.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style shader module creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.shader`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_shader_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuShaderOptionsVm,
    argument_bytes: VmSlice<u8>,
) -> RuntimeResult<resource::GpuShaderHandle> {
    let _ = (device, options, argument_bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.pipeline.shaderCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one shader module.
///
/// Destroy one shader module and release backend compiler or cache resources.
/// The shader handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style shader module destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.shader`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_shader_destroy(
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

/// Acquire one presentable surface texture.
///
/// Acquire one surface texture for rendering the next frame.
/// Returned status indicates whether presentation can proceed or whether reconfiguration is required.
/// When `hasTexture` is false, `texture` and `frameId` are unspecified.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style next-image acquisition APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_acquire(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    surface: resource::GpuSurfaceHandle,
    timeoutns: u64,
) -> RuntimeResult<GpuSurfaceFrameVm> {
    let _ = (surface, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceAcquire is not available in the VM yet",
    ))
    .boxed())
}

/// Read surface capabilities for one adapter.
///
/// Query one surface and adapter pair for compatible formats and present modes.
/// Capabilities can change when the window or monitor configuration changes.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style surface capabilities query APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_capabilities(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    surface: resource::GpuSurfaceHandle,
    adapter: resource::GpuAdapterHandle,
) -> RuntimeResult<GpuSurfaceCapabilitiesVm> {
    let _ = (surface, adapter);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceCapabilities is not available in the VM yet",
    ))
    .boxed())
}

/// Close one present surface.
///
/// Close one present surface and release host compositor resources.
/// Surface handle becomes invalid after close.
///
/// # Platform
/// Unix and Windows.
/// Uses surface destroy operations on host backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    surface: resource::GpuSurfaceHandle,
) -> RuntimeResult<()> {
    let _ = surface;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceClose is not available in the VM yet",
    ))
    .boxed())
}

/// Configure one present surface.
///
/// Configure one present surface with explicit dimensions and swap behavior.
/// Surface configuration must precede frame acquisition and presentation.
///
/// # Platform
/// Unix and Windows.
/// Uses swapchain or layer configuration APIs on Vulkan, Metal, and DXGI.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_configure(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    surface: resource::GpuSurfaceHandle,
    options: GpuSurfaceOptionsVm,
) -> RuntimeResult<()> {
    let _ = (device, surface, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceConfigure is not available in the VM yet",
    ))
    .boxed())
}

/// Open one present surface.
///
/// Open one present surface bound to one window host object.
/// Surface lifetime is independent from device lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses swapchain-surface or layer surface creation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<resource::GpuSurfaceHandle> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Present one acquired frame to one surface.
///
/// Present one previously acquired frame on one configured surface.
/// The `frameId` must match one outstanding successful `surfaceAcquire` result.
/// Presentation timing and tearing behavior follow compositor and backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses swapchain present APIs on Vulkan, Metal layer present, or DXGI present.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_present(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    surface: resource::GpuSurfaceHandle,
    options: GpuPresentOptionsVm,
) -> RuntimeResult<()> {
    let _ = (surface, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfacePresent is not available in the VM yet",
    ))
    .boxed())
}

/// Remove active configuration from one present surface.
///
/// Remove swapchain configuration from one surface and release configured present resources.
/// Surface must be configured again before the next frame acquisition.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style surface unconfigure operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.present`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_gpu_surface_unconfigure(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    surface: resource::GpuSurfaceHandle,
) -> RuntimeResult<()> {
    let _ = surface;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.present.surfaceUnconfigure is not available in the VM yet",
    ))
    .boxed())
}

/// Create one GPU buffer.
///
/// Create one buffer resource on one logical device with explicit size and usage flags.
/// Allocation placement and memory domain are host backend decisions.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style buffer creation operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_create(
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

/// Destroy one GPU buffer.
///
/// Destroy one buffer resource and release host backend memory references.
/// Outstanding use of the buffer is host-undefined and must be synchronized by callers.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style buffer destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_destroy(
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

/// Read one buffer metadata snapshot.
///
/// Read one metadata snapshot for one buffer resource.
/// Snapshot values remain stable except map state, which can change asynchronously.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style buffer property query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
) -> RuntimeResult<GpuBufferInfoVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Map one buffer range.
///
/// Map one host-visible byte range for direct CPU access.
/// Mapping coherence and cache behavior follow host backend memory rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style buffer map operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_map(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
    offset: u64,
    length: u64,
    mode: GpuMapMode,
) -> RuntimeResult<GpuMappedBufferRangeVm> {
    let _ = (handle, offset, length, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferMap is not available in the VM yet",
    ))
    .boxed())
}

/// Read one byte range from one GPU buffer.
///
/// Read one byte range from one buffer resource into host-visible memory.
/// Readback can stall based on backend synchronization and transfer state.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style readback or mapped-read paths on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_read(
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

/// Unmap one mapped buffer.
///
/// Unmap one previously mapped buffer range and flush host synchronization as required.
/// Visibility of writes follows backend memory model semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style buffer unmap operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_unmap(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferUnmap is not available in the VM yet",
    ))
    .boxed())
}

/// Write one byte range to one GPU buffer.
///
/// Write one byte range into one buffer resource from host memory.
/// Host staging and synchronization behavior follow backend upload semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style upload or mapped-write paths on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_buffer_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuBufferHandle,
    offset: u64,
    argument_bytes: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, offset, argument_bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.bufferWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Create one sampler resource.
///
/// Create one sampler resource with explicit filter and address mode selections.
/// Sampler state maps to host backend sampler descriptors.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style sampler creation operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_sampler_create(
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

/// Destroy one sampler resource.
///
/// Destroy one sampler resource and release host backend references.
/// The sampler handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style sampler destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_sampler_destroy(
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

/// Create one texture resource.
///
/// Create one texture resource with explicit dimensions, format, and usage flags.
/// Allocation placement and tiling are host backend decisions.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style texture creation operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_texture_create(
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

/// Destroy one texture resource.
///
/// Destroy one texture resource and release host backend memory references.
/// Outstanding use of the texture must be synchronized by callers.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style texture destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_texture_destroy(
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

/// Read one texture metadata snapshot.
///
/// Read one metadata snapshot for one texture resource.
/// Snapshot values remain stable for the texture lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style texture property query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_texture_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuTextureHandle,
) -> RuntimeResult<GpuTextureInfoVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Create one texture view.
///
/// Create one view of one texture resource for bindings and render attachments.
/// View range and dimension are validated against the source texture descriptor.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style texture-view creation operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_texture_view_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    texture: resource::GpuTextureHandle,
    options: GpuTextureViewOptionsVm,
) -> RuntimeResult<resource::GpuTextureViewHandle> {
    let _ = (texture, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureViewCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one texture view.
///
/// Destroy one texture-view endpoint and release backend view resources.
/// The texture-view handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style texture-view destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_texture_view_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuTextureViewHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.resource.textureViewDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Begin one occlusion query.
///
/// Begin one occlusion query in the active render pass.
/// Query nesting and pass compatibility follow backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style begin-occlusion-query commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_begin_occlusion_query(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
    queryset: resource::GpuQuerySetHandle,
    queryindex: u32,
) -> RuntimeResult<()> {
    let _ = (commandlist, queryset, queryindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandBeginOcclusionQuery is not available in the VM yet",
    ))
    .boxed())
}

/// Begin one pipeline-statistics query.
///
/// Begin one pipeline-statistics query in the active render or compute pass.
/// Pipeline-statistics queries cannot be nested and require one matching end call.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style begin-pipeline-statistics-query commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_begin_pipeline_statistics_query(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
    queryset: resource::GpuQuerySetHandle,
    queryindex: u32,
) -> RuntimeResult<()> {
    let _ = (commandlist, queryset, queryindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandBeginPipelineStatisticsQuery is not available in the VM yet",
    ))
    .boxed())
}

/// End one occlusion query.
///
/// End one occlusion query in the active render pass.
/// The current query must match the last begin call for this pass.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style end-occlusion-query commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_end_occlusion_query(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = commandlist;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandEndOcclusionQuery is not available in the VM yet",
    ))
    .boxed())
}

/// End one pipeline-statistics query.
///
/// End one active pipeline-statistics query in the current pass.
/// The active query must match the most recent begin call.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style end-pipeline-statistics-query commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_end_pipeline_statistics_query(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
) -> RuntimeResult<()> {
    let _ = commandlist;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandEndPipelineStatisticsQuery is not available in the VM yet",
    ))
    .boxed())
}

/// Resolve one query range into one destination buffer.
///
/// Resolve one query range and write results into one destination buffer.
/// Destination alignment and encoding follow backend query-result rules.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style resolve-query commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_resolve_queries(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
    queryset: resource::GpuQuerySetHandle,
    firstquery: u32,
    querycount: u32,
    destination: resource::GpuBufferHandle,
    destinationoffset: u64,
) -> RuntimeResult<()> {
    let _ = (
        commandlist,
        queryset,
        firstquery,
        querycount,
        destination,
        destinationoffset,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandResolveQueries is not available in the VM yet",
    ))
    .boxed())
}

/// Write one timestamp query from one command list.
///
/// Emit one timestamp query at the current command-list position.
/// Query availability and timestamp period follow backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style write-timestamp commands on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_command_write_timestamp(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    commandlist: resource::GpuCommandListHandle,
    queryset: resource::GpuQuerySetHandle,
    queryindex: u32,
) -> RuntimeResult<()> {
    let _ = (commandlist, queryset, queryindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.commandWriteTimestamp is not available in the VM yet",
    ))
    .boxed())
}

/// Create one synchronization fence.
///
/// Create one fence object for queue wait and signal coordination.
/// Timeline mode availability depends on backend feature support.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style fence or timeline semaphore creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_fence_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuFenceOptionsVm,
) -> RuntimeResult<resource::GpuFenceHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.fenceCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one synchronization fence.
///
/// Destroy one fence object and release backend synchronization resources.
/// The fence handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style fence destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_fence_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuFenceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.fenceDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Create one query set.
///
/// Create one query set for timestamp, occlusion, or pipeline-statistics queries.
/// Query set size and type are fixed for the object lifetime.
/// Pipeline-statistics queries are one optional lane and require feature support.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style query-pool creation APIs on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_query_set_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::GpuDeviceHandle,
    options: GpuQuerySetOptionsVm,
) -> RuntimeResult<resource::GpuQuerySetHandle> {
    let _ = (device, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.querySetCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Destroy one query set.
///
/// Destroy one query set and release backend query resources.
/// The query set handle becomes invalid after destroy.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style query-pool destroy operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_query_set_destroy(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuQuerySetHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.querySetDestroy is not available in the VM yet",
    ))
    .boxed())
}

/// Read one query-set metadata snapshot.
///
/// Read one query-set type and count snapshot from one query-set handle.
/// Values remain stable for the query-set lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style query-set metadata operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_query_set_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::GpuQuerySetHandle,
) -> RuntimeResult<GpuQuerySetInfoVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.querySetInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Signal one fence value from one queue.
///
/// Signal one fence from one queue after prior submissions complete.
/// Value handling follows backend binary or timeline semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue signal operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_signal(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    fence: resource::GpuFenceHandle,
    argument_value: u64,
) -> RuntimeResult<()> {
    let _ = (queue, fence, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.queueSignal is not available in the VM yet",
    ))
    .boxed())
}

/// Read one queue timestamp period.
///
/// Read the queue timestamp period in nanoseconds per hardware timestamp tick.
/// Multiply timestamp query deltas by this value to convert to nanoseconds.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue timestamp-period query operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_timestamp_period(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
) -> RuntimeResult<f64> {
    let _ = queue;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.queueTimestampPeriod is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one fence value.
///
/// Wait for one fence to reach or exceed one requested value.
/// Timeout uses nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style fence wait operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    fence: resource::GpuFenceHandle,
    argument_value: u64,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (queue, fence, argument_value, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.queueWait is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for all previously submitted queue work.
///
/// Wait for one queue to complete all prior submissions.
/// Timeout uses nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses WebGPU-style queue completion wait operations on Vulkan, Metal, D3D12, and OpenGL-class backends.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `gpu.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_gpu_queue_work_done(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    queue: resource::GpuQueueHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (queue, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.gpu.sync.queueWorkDone is not available in the VM yet",
    ))
    .boxed())
}

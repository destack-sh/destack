mod macros;
mod native;
mod policy;
mod registry;
mod spec;
mod vm;

pub use native::{NativeBinding, NativeBindingSet, native_call};
pub use policy::{BindingPolicy, ExecutionMode};
pub use registry::BindingRegistry;
pub use spec::{
    BindingDescriptor, BindingEffectMask, BindingId, BindingReplayKind, CODEC_POSTCARD_V1, CodecId,
    EffectClass, ReplayPayload, ReplayPolicy, SignatureHash,
};
pub use vm::VmBindingSet;

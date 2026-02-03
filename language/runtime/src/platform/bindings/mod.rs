mod macros;
mod native;
mod policy;
mod registry;
mod vm;
mod spec;

pub use native::{NativeBinding, NativeBindingSet, native_call};
pub use policy::{BindingPolicy, ExecutionMode};
pub use registry::BindingRegistry;
pub use vm::VmBindingSet;
pub use spec::{
    BindingDescriptor, BindingEffectMask, BindingId, BindingReplayKind, CODEC_POSTCARD_V1, CodecId,
    EffectClass, ReplayPayload, ReplayPolicy, SignatureHash,
};

mod macros;
mod native;
mod policy;
mod registry;
mod spec;
mod vm;

pub use destack_workspace::{BindingEngine, RuntimeWorld};
pub use native::{NativeBinding, NativeBindingSet, native_call};
pub use policy::BindingPolicy;
pub use registry::BindingRegistry;
pub use spec::{
    BindingBlocking, BindingDescriptor, BindingEffect, BindingEffectClass, BindingEffectMask,
    BindingId, BindingReplayKind, BindingReplayPayload, BindingReplayPolicy, BindingScope,
    CODEC_POSTCARD_V1, CodecId, SignatureHash,
};
pub use vm::VmBindingSet;

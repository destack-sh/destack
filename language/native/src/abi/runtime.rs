use serde::{Deserialize, Serialize};

use super::{Activation, Space, Unwind};

macro_rules! runtime_operations {
    ($macro:ident) => {
        $macro! {
            /// Allocate one heap object.
            Allocate = 0x0000 => allocate: Allocate(Uint32, Uint32, Uint32) -> Pointer,
            /// Allocate one repeated heap backing.
            AllocateRepeated = 0x0001 =>
                allocate_repeated: AllocateRepeated(Uint32, Uint32, Pointer, Uint32) -> Pointer,
            /// Release one unique heap value.
            Release = 0x0002 => release: Release(Pointer, Uint32, Pointer) -> Void,
            /// Free one unique heap value holding no live values.
            Free = 0x0003 => free: Free(Pointer) -> Void,
            /// Record one managed reference write.
            WriteBarrier = 0x0004 => write_barrier: WriteBarrier(Pointer, Pointer, Pointer) -> Void,

            /// Poll pending runtime work.
            Poll = 0x0010 => poll: Poll(Uint32, Pointer) -> Never,
            /// Stop execution for host inspection.
            Stop = 0x0011 => stop: Stop(Uint32, Uint32, Pointer) -> Never,
            /// Deoptimize native execution.
            Deopt = 0x0012 => deopt: Deopt(Uint32, Pointer) -> Never,

            /// Report one payloadless language panic.
            Panic = 0x0014 => panic: Panic() -> Never,
            /// Report one typed language panic.
            PanicValue = 0x0015 => panic_value: PanicValue(Uint32, Pointer) -> Never,
            /// Classify the active unwind.
            UnwindClassify = 0x0016 => unwind_classify: ClassifyUnwind() -> Uint32,
            /// Continue the active unwind.
            UnwindResume = 0x0017 => unwind_resume: ResumeUnwind(Pointer) -> Never,

            /// Return whether one concrete type satisfies another.
            IsSubtype = 0x0020 => is_subtype: IsSubtype(Uint32, Uint32) -> Uint32,

            /// Increment one explicit profile counter.
            ProfileIncrement = 0x0030 => profile_increment: IncrementProfile(Uint32) -> Void,
            /// Record one explicit profile sample.
            ProfileSample = 0x0031 => profile_sample: SampleProfile(Uint32, Uint64) -> Void,

            /// Call one runtime binding.
            BindingCall = 0x0040 =>
                binding_call: BindingCall(Uint32, Pointer, Pointer, Pointer, Pointer) -> Void,

            /// Read one volatile byte range.
            VolatileRead = 0x0050 => volatile_read: VolatileRead(Pointer, Pointer, Pointer) -> Void,
            /// Write one volatile byte range.
            VolatileWrite = 0x0051 =>
                volatile_write: VolatileWrite(Pointer, Pointer, Pointer) -> Void,
        }
    };
}

pub(super) use runtime_operations;

macro_rules! define_runtime {
    ($(
        $(#[$meta:meta])*
        $operation:ident = $code:literal =>
            $field:ident: $ty:ident($($parameter:ident),*) -> $result:ident,
    )*) => {
        /// Runtime operations callable by generated native code.
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        pub struct Runtime {
            $(
                $(#[$meta])*
                pub $field: $ty,
            )*
        }
    };
}

runtime_operations!(define_runtime);

/// Native allocation initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationInitialization {
    /// Initialize bytes to zero.
    Zeroed = 0,
    /// Leave bytes uninitialized.
    Uninit = 1,
}

/// Allocate one heap object through the runtime.
pub type Allocate = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    space: Space,
    allocation_plan: u32,
    initialization: AllocationInitialization,
) -> usize;

/// Allocate one repeated heap backing through the runtime.
pub type AllocateRepeated = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    space: Space,
    element_allocation_plan: u32,
    length: usize,
    initialization: AllocationInitialization,
) -> usize;

/// Release one unique heap value through the runtime.
pub type Release = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    owner: usize,
    frame_map: u32,
    marker: *const u8,
);

/// Free one unique heap value holding no live values through the runtime.
pub type Free = unsafe extern "C-unwind" fn(activation: *mut Activation, owner: usize);

/// Record one managed reference write through the runtime.
pub type WriteBarrier = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    object: *const u8,
    offset: usize,
    byte_len: usize,
);

/// Read one volatile byte range into ordinary native storage.
pub type VolatileRead = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    source: *const u8,
    destination: *mut u8,
    byte_len: usize,
);

/// Write one ordinary native byte range into volatile storage.
pub type VolatileWrite = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    destination: *mut u8,
    source: *const u8,
    byte_len: usize,
);

/// Poll runtime work at one reconstructable native frame.
pub type Poll = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    marker: *const u8,
) -> !;

/// Pause execution and return control to the host.
pub type Stop = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    operation: u32,
    marker: *const u8,
) -> !;

/// Deoptimize native execution into interpreter state.
pub type Deopt = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    marker: *const u8,
) -> !;

/// Start unwinding one payloadless language panic.
pub type Panic = unsafe extern "C-unwind" fn(activation: *mut Activation) -> !;

/// Start unwinding one typed language panic.
pub type PanicValue =
    unsafe extern "C-unwind" fn(activation: *mut Activation, ty: u32, words: *const u64) -> !;

/// Classify the active native unwind.
pub type ClassifyUnwind = unsafe extern "C-unwind" fn(activation: *mut Activation) -> UnwindAction;

/// Continue the active platform unwind.
pub type ResumeUnwind =
    unsafe extern "C-unwind" fn(activation: *mut Activation, unwind: *mut Unwind) -> !;

/// Return whether one concrete Program type satisfies an expected type.
pub type IsSubtype =
    unsafe extern "C-unwind" fn(activation: *mut Activation, concrete: u32, expected: u32) -> u32;

/// Action selected for one active native unwind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnwindAction {
    /// Run the local cleanup path.
    Cleanup = 0,
    /// Skip the local cleanup path and continue the unwind.
    Skip = 1,
}

/// Increment one explicit profile counter.
pub type IncrementProfile = unsafe extern "C-unwind" fn(activation: *mut Activation, counter: u32);

/// Record one explicit profile sample.
pub type SampleProfile =
    unsafe extern "C-unwind" fn(activation: *mut Activation, sampler: u32, value: u64);

/// Call one runtime binding selected by its Program function.
pub type BindingCall = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    function: u32,
    arguments: *const u64,
    argument_count: usize,
    result: *mut u64,
    result_count: usize,
);

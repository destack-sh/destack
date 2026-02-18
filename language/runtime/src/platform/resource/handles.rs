use crate::diagnostic::RuntimeResult;
use crate::platform::VmValueCodec;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// The identifier for one runtime-managed resource table entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(
    /// The inner identifier value.
    pub u64,
);

impl VmValueCodec for ResourceId {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(Self(<u64 as VmValueCodec>::decode(value)?))
    }

    fn encode(self) -> vm::Value {
        <u64 as VmValueCodec>::encode(self.0)
    }
}

/// The ownership mode for one transferred resource.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceOwnership {
    /// The sender retains ownership.
    Borrowed = 1,
    /// The receiver takes ownership.
    Owned = 2,
}

/// The VM-facing resource kind payload.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ResourceKindVm(
    /// The inner resource kind string handle.
    pub vm::StringHandle,
);

impl VmValueCodec for ResourceKindVm {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(Self(vm::StringHandle::new(value)))
    }

    fn encode(self) -> vm::Value {
        self.0.value()
    }
}

/// Declare one resource handle newtype.
macro_rules! define_handle {
    ($doc:literal, $name:ident) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(
            /// The inner resource identifier.
            pub ResourceId,
        );

        impl VmValueCodec for $name {
            fn decode(value: vm::Value) -> RuntimeResult<Self> {
                Ok(Self(<ResourceId as VmValueCodec>::decode(value)?))
            }

            fn encode(self) -> vm::Value {
                <ResourceId as VmValueCodec>::encode(self.0)
            }
        }
    };
}

define_handle!("The handle for an open file.", FileHandle);
define_handle!("The handle for an open directory.", DirectoryHandle);
define_handle!("The handle for a network socket.", SocketHandle);
define_handle!("The handle for a network listener.", ListenerHandle);
define_handle!("The handle for a spawned process.", ProcessHandle);
define_handle!(
    "The handle for one process file descriptor style object.",
    ProcessFdHandle
);
define_handle!("The handle for a scheduled timer.", TimerHandle);
define_handle!("The handle for a filesystem watcher.", WatchHandle);
define_handle!("The handle for a pipe endpoint.", PipeHandle);
define_handle!(
    "The handle for one shared memory object.",
    SharedMemoryHandle
);
define_handle!("The handle for one semaphore object.", SemaphoreHandle);
define_handle!("The handle for one readiness poll instance.", PollHandle);
define_handle!(
    "The handle for one completion queue instance.",
    CompletionHandle
);
define_handle!(
    "The handle for one eventfd style descriptor.",
    EventFdHandle
);
define_handle!("The handle for one io_uring instance.", UringHandle);
define_handle!(
    "The handle for one timerfd style descriptor.",
    TimerFdHandle
);
define_handle!("The handle for one dynamic library.", LibraryHandle);
define_handle!(
    "The handle for one symbol in a dynamic library.",
    SymbolHandle
);
define_handle!("The handle for one device endpoint.", DeviceHandle);
define_handle!("The handle for one pseudo terminal endpoint.", PtyHandle);
define_handle!("The handle for one thread object.", ThreadHandle);
define_handle!("The handle for one mutex object.", MutexHandle);
define_handle!("The handle for one read write lock object.", RwLockHandle);
define_handle!(
    "The handle for one condition variable object.",
    CondVarHandle
);
define_handle!(
    "The handle for one thread-scoped semaphore object.",
    ThreadSemaphoreHandle
);
define_handle!(
    "The handle for one thread-scoped barrier object.",
    BarrierHandle
);
define_handle!("The handle for one thread local key.", ThreadLocalKey);
define_handle!("The handle for one sandbox scope.", SandboxHandle);
define_handle!("The handle for one inspector session.", InspectorHandle);
define_handle!("The handle for one profiler session.", ProfileHandle);
define_handle!("The handle for one trace session.", TraceHandle);
define_handle!("The handle for one tty endpoint.", TtyHandle);
define_handle!("The handle for one signal subscription.", SignalHandle);
define_handle!(
    "The handle for one signalfd style queue descriptor.",
    SignalFdHandle
);
define_handle!(
    "The handle for one transferred resource.",
    TransferredHandle
);
define_handle!("The handle for one message queue.", MessageQueueHandle);
define_handle!("The handle for one audio device.", AudioDeviceHandle);
define_handle!("The handle for one audio stream.", AudioStreamHandle);
define_handle!(
    "The handle for one audio event subscription endpoint.",
    AudioEventHandle
);
define_handle!("The handle for one display device.", DisplayHandle);
define_handle!("The handle for one window object.", WindowHandle);
define_handle!("The handle for one input device.", InputDeviceHandle);
define_handle!(
    "The handle for one global input monitor stream.",
    InputMonitorHandle
);
define_handle!("The handle for one gpu adapter.", GpuAdapterHandle);
define_handle!("The handle for one gpu device.", GpuDeviceHandle);
define_handle!("The handle for one gpu queue.", GpuQueueHandle);
define_handle!("The handle for one gpu surface endpoint.", GpuSurfaceHandle);
define_handle!("The handle for one gpu command list.", GpuCommandListHandle);
define_handle!(
    "The handle for one gpu render bundle endpoint.",
    GpuRenderBundleHandle
);
define_handle!(
    "The handle for one gpu render bundle encoder endpoint.",
    GpuRenderBundleEncoderHandle
);
define_handle!(
    "The handle for one gpu pipeline layout endpoint.",
    GpuPipelineLayoutHandle
);
define_handle!(
    "The handle for one gpu bind group layout endpoint.",
    GpuBindGroupLayoutHandle
);
define_handle!(
    "The handle for one gpu bind group endpoint.",
    GpuBindGroupHandle
);
define_handle!(
    "The handle for one gpu synchronization fence endpoint.",
    GpuFenceHandle
);
define_handle!(
    "The handle for one gpu query set endpoint.",
    GpuQuerySetHandle
);
define_handle!("The handle for one gpu memory allocation.", GpuMemoryHandle);
define_handle!("The handle for one gpu buffer.", GpuBufferHandle);
define_handle!("The handle for one gpu texture.", GpuTextureHandle);
define_handle!(
    "The handle for one gpu texture view endpoint.",
    GpuTextureViewHandle
);
define_handle!("The handle for one gpu sampler.", GpuSamplerHandle);
define_handle!("The handle for one gpu shader module.", GpuShaderHandle);
define_handle!("The handle for one gpu pipeline.", GpuPipelineHandle);
define_handle!(
    "The handle for one cryptographic certificate object.",
    CryptoCertificateHandle
);
define_handle!(
    "The handle for one cryptographic key object.",
    CryptoKeyHandle
);
define_handle!(
    "The handle for one cryptographic store object.",
    CryptoStoreHandle
);
define_handle!("The handle for one tls context object.", TlsContextHandle);
define_handle!("The handle for one tls session object.", TlsSessionHandle);

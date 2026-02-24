use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceKindVm;
use crate::platform::{PlatformError, ResourceKind};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> ResourceHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Return a stable label for one native resource kind.
    fn native_kind_label(kind: ResourceKind) -> &'static str {
        match kind {
            ResourceKind::File => "file",
            ResourceKind::Directory => "directory",
            ResourceKind::Pipe => "pipe",
            ResourceKind::Socket => "socket",
            ResourceKind::Listener => "listener",
            ResourceKind::Timer => "timer",
            ResourceKind::TimerFd => "timer_fd",
            ResourceKind::Watch => "watch",
            ResourceKind::Process => "process",
            ResourceKind::Poll => "poll",
            ResourceKind::Completion => "completion",
            ResourceKind::Event => "event",
            ResourceKind::Uring => "uring",
            ResourceKind::ProcessFd => "process_fd",
            ResourceKind::SharedMemory => "shared_memory",
            ResourceKind::Semaphore => "semaphore",
            ResourceKind::Signal => "signal",
            ResourceKind::SignalFd => "signal_fd",
            ResourceKind::Thread => "thread",
            ResourceKind::Mutex => "mutex",
            ResourceKind::RwLock => "rw_lock",
            ResourceKind::CondVar => "cond_var",
            ResourceKind::ThreadSemaphore => "thread_semaphore",
            ResourceKind::Barrier => "barrier",
            ResourceKind::ThreadLocal => "thread_local",
            ResourceKind::Library => "library",
            ResourceKind::Symbol => "symbol",
            ResourceKind::CryptoStore => "crypto_store",
            ResourceKind::CryptoKey => "crypto_key",
            ResourceKind::CryptoCertificate => "crypto_certificate",
            ResourceKind::CryptoDigest => "crypto_digest",
            ResourceKind::CryptoMac => "crypto_mac",
            ResourceKind::CryptoCipher => "crypto_cipher",
            ResourceKind::TlsContext => "tls_context",
            ResourceKind::TlsSession => "tls_session",
            ResourceKind::Device => "device",
            ResourceKind::Pty => "pty",
            ResourceKind::Tty => "tty",
            ResourceKind::Sandbox => "sandbox",
            ResourceKind::Inspector => "inspector",
            ResourceKind::Profile => "profile",
            ResourceKind::Trace => "trace",
            ResourceKind::Transferred => "transferred",
            ResourceKind::MessageQueue => "message_queue",
            ResourceKind::AudioDevice => "audio_device",
            ResourceKind::AudioStream => "audio_stream",
            ResourceKind::AudioEvent => "audio_event",
            ResourceKind::Display => "display",
            ResourceKind::Window => "window",
            ResourceKind::Input => "input",
            ResourceKind::GpuAdapter => "gpu_adapter",
            ResourceKind::GpuDevice => "gpu_device",
            ResourceKind::GpuQueue => "gpu_queue",
            ResourceKind::GpuCommandList => "gpu_command_list",
            ResourceKind::GpuMemory => "gpu_memory",
            ResourceKind::GpuBuffer => "gpu_buffer",
            ResourceKind::GpuTexture => "gpu_texture",
            ResourceKind::GpuSampler => "gpu_sampler",
            ResourceKind::GpuShader => "gpu_shader",
            ResourceKind::GpuPipeline => "gpu_pipeline",
            ResourceKind::Unknown => "unknown",
        }
    }

    /// Decode one vm resource kind payload into a stable label.
    fn vm_kind_label(&mut self, kind: ResourceKindVm) -> RuntimeResult<String> {
        let context = self.vm_context_mut().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "kind",
                "missing vm context for vm resource kind",
            ))
            .boxed()
        })?;
        let value = context
            .string_ref(kind.0)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        Ok(value.as_str().to_string())
    }

    /// Return one stable resource kind label for either backend.
    pub(crate) fn resource_kind_label_from_value(
        &mut self,
        value: HarnessValue<ResourceKind, ResourceKindVm>,
    ) -> RuntimeResult<String> {
        match value {
            HarnessValue::Native(kind) => Ok(Self::native_kind_label(kind).to_string()),
            HarnessValue::Vm(kind) => self.vm_kind_label(kind),
        }
    }
}

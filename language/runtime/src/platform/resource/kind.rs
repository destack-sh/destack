use serde::{Deserialize, Serialize};

/// Enumerate the canonical handle-to-kind registry entries.
macro_rules! for_each_resource_handle_kind {
    ($macro:ident) => {
        $macro! {
            (FileHandle, File, "resource.file", "file", "The handle for an open file."),
            (DirectoryHandle, Directory, "resource.directory", "directory", "The handle for an open directory."),
            (SocketHandle, Socket, "resource.socket", "socket", "The handle for a network socket."),
            (ListenerHandle, Listener, "resource.listener", "listener", "The handle for a network listener."),
            (ProcessHandle, Process, "resource.process", "process", "The handle for a spawned process."),
            (ProcessFdHandle, ProcessFd, "resource.process.fd", "process_fd", "The handle for one process file descriptor style object."),
            (TimerHandle, Timer, "resource.timer", "timer", "The handle for a scheduled timer."),
            (WatchHandle, Watch, "resource.watch", "watch", "The handle for a filesystem watcher."),
            (PipeHandle, Pipe, "resource.pipe", "pipe", "The handle for a pipe endpoint."),
            (SharedMemoryHandle, SharedMemory, "resource.shared.memory", "shared_memory", "The handle for one shared memory object."),
            (SemaphoreHandle, Semaphore, "resource.semaphore", "semaphore", "The handle for one semaphore object."),
            (PollHandle, Poll, "resource.poll", "poll", "The handle for one readiness poll instance."),
            (CompletionHandle, Completion, "resource.completion", "completion", "The handle for one completion queue instance."),
            (EventFdHandle, Event, "resource.event", "event", "The handle for one eventfd style descriptor."),
            (UringHandle, Uring, "resource.uring", "uring", "The handle for one io_uring instance."),
            (TimerFdHandle, TimerFd, "resource.timer.fd", "timer_fd", "The handle for one timerfd style descriptor."),
            (LibraryHandle, Library, "resource.library", "library", "The handle for one dynamic library."),
            (SymbolHandle, Symbol, "resource.symbol", "symbol", "The handle for one symbol in a dynamic library."),
            (DeviceHandle, Device, "resource.device", "device", "The handle for one device endpoint."),
            (PtyHandle, Pty, "resource.pty", "pty", "The handle for one pseudo terminal endpoint."),
            (ThreadHandle, Thread, "resource.thread", "thread", "The handle for one thread object."),
            (MutexHandle, Mutex, "resource.mutex", "mutex", "The handle for one mutex object."),
            (RwLockHandle, RwLock, "resource.rw.lock", "rw_lock", "The handle for one read write lock object."),
            (CondVarHandle, CondVar, "resource.cond.var", "cond_var", "The handle for one condition variable object."),
            (ThreadSemaphoreHandle, ThreadSemaphore, "resource.thread.semaphore", "thread_semaphore", "The handle for one thread-scoped semaphore object."),
            (BarrierHandle, Barrier, "resource.barrier", "barrier", "The handle for one thread-scoped barrier object."),
            (ThreadLocalKey, ThreadLocal, "resource.thread.local", "thread_local", "The handle for one thread local key."),
            (SandboxHandle, Sandbox, "resource.sandbox", "sandbox", "The handle for one sandbox scope."),
            (InspectorHandle, Inspector, "resource.inspector", "inspector", "The handle for one inspector session."),
            (ProfileHandle, Profile, "resource.profile", "profile", "The handle for one profiler session."),
            (TraceHandle, Trace, "resource.trace", "trace", "The handle for one trace session."),
            (TtyHandle, Tty, "resource.tty", "tty", "The handle for one tty endpoint."),
            (SignalHandle, Signal, "resource.signal", "signal", "The handle for one signal subscription."),
            (SignalFdHandle, SignalFd, "resource.signal.fd", "signal_fd", "The handle for one signalfd style queue descriptor."),
            (TransferredHandle, Transferred, "resource.transferred", "transferred", "The handle for one transferred resource."),
            (MessageQueueHandle, MessageQueue, "resource.message.queue", "message_queue", "The handle for one message queue."),
            (AudioDeviceHandle, AudioDevice, "resource.audio.device", "audio_device", "The handle for one audio device."),
            (AudioStreamHandle, AudioStream, "resource.audio.stream", "audio_stream", "The handle for one audio stream."),
            (AudioEventHandle, AudioEvent, "resource.audio.event", "audio_event", "The handle for one audio event subscription endpoint."),
            (DisplayHandle, Display, "resource.display", "display", "The handle for one display device."),
            (WindowHandle, Window, "resource.window", "window", "The handle for one window object."),
            (InputDeviceHandle, InputDevice, "resource.input.device", "input_device", "The handle for one input device."),
            (InputMonitorHandle, InputMonitor, "resource.input.monitor", "input_monitor", "The handle for one global input monitor stream."),
            (GpuAdapterHandle, GpuAdapter, "resource.gpu.adapter", "gpu_adapter", "The handle for one gpu adapter."),
            (GpuDeviceHandle, GpuDevice, "resource.gpu.device", "gpu_device", "The handle for one gpu device."),
            (GpuQueueHandle, GpuQueue, "resource.gpu.queue", "gpu_queue", "The handle for one gpu queue."),
            (GpuSurfaceHandle, GpuSurface, "resource.gpu.surface", "gpu_surface", "The handle for one gpu surface endpoint."),
            (GpuCommandListHandle, GpuCommandList, "resource.gpu.command.list", "gpu_command_list", "The handle for one gpu command list."),
            (GpuComputePassHandle, GpuComputePass, "resource.gpu.compute.pass", "gpu_compute_pass", "The handle for one gpu compute pass endpoint."),
            (GpuRenderPassHandle, GpuRenderPass, "resource.gpu.render.pass", "gpu_render_pass", "The handle for one gpu render pass endpoint."),
            (GpuRenderBundleHandle, GpuRenderBundle, "resource.gpu.render.bundle", "gpu_render_bundle", "The handle for one gpu render bundle endpoint."),
            (GpuRenderBundleEncoderHandle, GpuRenderBundleEncoder, "resource.gpu.render.bundle.encoder", "gpu_render_bundle_encoder", "The handle for one gpu render bundle encoder endpoint."),
            (GpuPipelineLayoutHandle, GpuPipelineLayout, "resource.gpu.pipeline.layout", "gpu_pipeline_layout", "The handle for one gpu pipeline layout endpoint."),
            (GpuBindGroupLayoutHandle, GpuBindGroupLayout, "resource.gpu.bind.group.layout", "gpu_bind_group_layout", "The handle for one gpu bind group layout endpoint."),
            (GpuBindGroupHandle, GpuBindGroup, "resource.gpu.bind.group", "gpu_bind_group", "The handle for one gpu bind group endpoint."),
            (GpuFenceHandle, GpuFence, "resource.gpu.fence", "gpu_fence", "The handle for one gpu synchronization fence endpoint."),
            (GpuQuerySetHandle, GpuQuerySet, "resource.gpu.query.set", "gpu_query_set", "The handle for one gpu query set endpoint."),
            (GpuMemoryHandle, GpuMemory, "resource.gpu.memory", "gpu_memory", "The handle for one gpu memory allocation."),
            (GpuBufferHandle, GpuBuffer, "resource.gpu.buffer", "gpu_buffer", "The handle for one gpu buffer."),
            (GpuTextureHandle, GpuTexture, "resource.gpu.texture", "gpu_texture", "The handle for one gpu texture."),
            (GpuTextureViewHandle, GpuTextureView, "resource.gpu.texture.view", "gpu_texture_view", "The handle for one gpu texture view endpoint."),
            (GpuSamplerHandle, GpuSampler, "resource.gpu.sampler", "gpu_sampler", "The handle for one gpu sampler."),
            (GpuShaderHandle, GpuShader, "resource.gpu.shader", "gpu_shader", "The handle for one gpu shader module."),
            (GpuPipelineHandle, GpuPipeline, "resource.gpu.pipeline", "gpu_pipeline", "The handle for one gpu pipeline."),
            (CryptoCertificateHandle, CryptoCertificate, "resource.crypto.certificate", "crypto_certificate", "The handle for one cryptographic certificate object."),
            (CryptoDigestHandle, CryptoDigest, "resource.crypto.digest", "crypto_digest", "The handle for one cryptographic digest context."),
            (CryptoMacHandle, CryptoMac, "resource.crypto.mac", "crypto_mac", "The handle for one cryptographic message-authentication context."),
            (CryptoCipherHandle, CryptoCipher, "resource.crypto.cipher", "crypto_cipher", "The handle for one cryptographic symmetric-cipher context."),
            (CryptoKeyHandle, CryptoKey, "resource.crypto.key", "crypto_key", "The handle for one cryptographic key object."),
            (CryptoStoreHandle, CryptoStore, "resource.crypto.store", "crypto_store", "The handle for one cryptographic store object."),
            (TlsContextHandle, TlsContext, "resource.tls.context", "tls_context", "The handle for one tls context object."),
            (TlsSessionHandle, TlsSession, "resource.tls.session", "tls_session", "The handle for one tls session object."),
            (BackgroundEventHandle, BackgroundEvent, "resource.background.event", "background_event", "The handle for one background event stream."),
            (BluetoothDeviceHandle, BluetoothDevice, "resource.bluetooth.device", "bluetooth_device", "The handle for one Bluetooth device session."),
            (BluetoothScanHandle, BluetoothScan, "resource.bluetooth.scan", "bluetooth_scan", "The handle for one Bluetooth scan session."),
            (BluetoothSubscriptionHandle, BluetoothSubscription, "resource.bluetooth.subscription", "bluetooth_subscription", "The handle for one Bluetooth GATT subscription session."),
            (CameraDeviceHandle, CameraDevice, "resource.camera.device", "camera_device", "The handle for one camera device session."),
            (CameraStreamHandle, CameraStream, "resource.camera.stream", "camera_stream", "The handle for one camera stream session."),
            (DocumentHandle, Document, "resource.document", "document", "The handle for one document session."),
            (IntentHandle, Intent, "resource.intent", "intent", "The handle for one intent session."),
            (LifecycleEventHandle, LifecycleEvent, "resource.lifecycle.event", "lifecycle_event", "The handle for one lifecycle event stream."),
            (LocationWatchHandle, LocationWatch, "resource.location.watch", "location_watch", "The handle for one location watch stream."),
            (MidiPortHandle, MidiPort, "resource.midi.port", "midi_port", "The handle for one MIDI port session."),
            (NetworkWatchHandle, NetworkWatch, "resource.network.watch", "network_watch", "The handle for one network watch stream."),
            (NotificationEventHandle, NotificationEvent, "resource.notification.event", "notification_event", "The handle for one notification event stream."),
            (SerialPortHandle, SerialPort, "resource.serial.port", "serial_port", "The handle for one serial port session."),
            (UsbDeviceHandle, UsbDevice, "resource.usb.device", "usb_device", "The handle for one USB device session."),
            (UsbWatchHandle, UsbWatch, "resource.usb.watch", "usb_watch", "The handle for one USB hotplug watch stream."),
            (WindowEventHandle, WindowEvent, "resource.window.event", "window_event", "The handle for one window event stream."),
            (DisplayEventHandle, DisplayEvent, "resource.display.event", "display_event", "The handle for one display event stream."),
        }
    };
}

pub(crate) use for_each_resource_handle_kind;

macro_rules! define_resource_kind {
    ($(($handle:ident, $kind:ident, $kind_id:literal, $label:literal, $doc:literal),)+) => {
        /// Resource classification for platform handles.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum ResourceKind {
            $(
                #[doc = concat!("Resource kind for `", $label, "` handles.")]
                $kind,
            )+
        }

        impl ResourceKind {
            /// Return all built-in resource kinds.
            pub const fn all() -> &'static [Self] {
                &[
                    $(Self::$kind,)+
                ]
            }

            /// Return the stable kind identifier for topology and policy matching.
            pub const fn kind_id(self) -> &'static str {
                match self {
                    $(Self::$kind => $kind_id,)+
                }
            }

            /// Return the stable binding label for this kind.
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$kind => $label,)+
                }
            }
        }
    };
}

for_each_resource_handle_kind!(define_resource_kind);

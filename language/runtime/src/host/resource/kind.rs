use serde::{Deserialize, Serialize};

/// Enumerate the canonical handle-to-kind registry entries.
macro_rules! for_each_resource_handle_kind {
    ($macro:ident) => {
        $macro! {
            (FileHandle, File, "host.fs.file", "file", "The handle for an open file."),
            (DirectoryHandle, Directory, "host.fs.directory", "directory", "The handle for an open directory."),
            (SocketHandle, Socket, "host.net.socket", "socket", "The handle for a network socket."),
            (ListenerHandle, Listener, "host.net.listener", "listener", "The handle for a network listener."),
            (ProcessHandle, Process, "host.process.child", "process", "The handle for a spawned process."),
            (ProcessFdHandle, ProcessFd, "host.process.fd", "process_fd", "The handle for one process file descriptor style object."),
            (TimerHandle, Timer, "host.time.timer", "timer", "The handle for a scheduled timer."),
            (WatchHandle, Watch, "host.fs.watch", "watch", "The handle for a filesystem watcher."),
            (PipeHandle, Pipe, "host.ipc.pipe", "pipe", "The handle for a pipe endpoint."),
            (SharedMemoryHandle, SharedMemory, "host.ipc.memory", "shared_memory", "The handle for one shared memory object."),
            (SemaphoreHandle, Semaphore, "host.ipc.semaphore", "semaphore", "The handle for one semaphore object."),
            (PollHandle, Poll, "host.io.poll", "poll", "The handle for one readiness poll instance."),
            (CompletionHandle, Completion, "host.io.completion", "completion", "The handle for one completion queue instance."),
            (EventFdHandle, Event, "host.ipc.event", "event", "The handle for one eventfd style descriptor."),
            (UringHandle, Uring, "host.io.uring", "uring", "The handle for one io_uring instance."),
            (DeviceHandle, Device, "host.device.handle", "device", "The handle for one device endpoint."),
            (PtyHandle, Pty, "host.tty.pty", "pty", "The handle for one pseudo terminal endpoint."),
            (TraceHandle, Trace, "runtime.trace", "trace", "The handle for one trace session."),
            (TtyHandle, Tty, "host.tty.terminal", "tty", "The handle for one tty endpoint."),
            (SignalHandle, Signal, "host.process.signal", "signal", "The handle for one signal subscription."),
            (SignalFdHandle, SignalFd, "host.process.signal.fd", "signal_fd", "The handle for one signalfd style queue descriptor."),
            (TransferredHandle, Transferred, "runtime.resource.transferred", "transferred", "The handle for one transferred resource."),
            (MessageQueueHandle, MessageQueue, "host.ipc.message.queue", "message_queue", "The handle for one message queue."),
            (AudioDeviceHandle, AudioDevice, "host.audio.device", "audio_device", "The handle for one audio device."),
            (AudioStreamHandle, AudioStream, "host.audio.stream", "audio_stream", "The handle for one audio stream."),
            (AudioEventHandle, AudioEvent, "host.audio.event", "audio_event", "The handle for one audio event subscription endpoint."),
            (DisplayHandle, Display, "host.display.device", "display", "The handle for one display device."),
            (DisplayBeginFrameHandle, DisplayBeginFrame, "host.display.frame", "display_begin_frame", "The handle for one display begin-frame stream."),
            (WindowHandle, Window, "host.display.window", "window", "The handle for one window object."),
            (AccessibilityActionHandle, AccessibilityAction, "host.accessibility.action", "accessibility_action", "The handle for one accessibility action stream."),
            (AccessibilityDocumentHandle, AccessibilityDocument, "host.accessibility.document", "accessibility_document", "The handle for one accessibility document query stream."),
            (InputDeviceHandle, InputDevice, "host.input.device", "input_device", "The handle for one input device."),
            (InputTextSessionHandle, InputTextSession, "host.input.text.session", "input_text_session", "The handle for one text input session."),
            (InputMonitorHandle, InputMonitor, "host.input.monitor", "input_monitor", "The handle for one global input monitor stream."),
            (DisplayDragSessionHandle, DisplayDragSession, "host.display.drag.session", "display_drag_session", "The handle for one display drag-transfer session."),
            (GpuAdapterHandle, GpuAdapter, "host.gpu.adapter", "gpu_adapter", "The handle for one gpu adapter."),
            (GpuDeviceHandle, GpuDevice, "host.gpu.device", "gpu_device", "The handle for one gpu device."),
            (GpuQueueHandle, GpuQueue, "host.gpu.queue", "gpu_queue", "The handle for one gpu queue."),
            (GpuSurfaceHandle, GpuSurface, "host.gpu.surface", "gpu_surface", "The handle for one gpu surface endpoint."),
            (GpuSurfacePresentationHandle, GpuSurfacePresentation, "host.gpu.surface.presentation", "gpu_surface_presentation", "The handle for one gpu surface presentation stream."),
            (GpuCommandBufferHandle, GpuCommandBuffer, "host.gpu.command.buffer", "gpu_command_buffer", "The handle for one gpu command buffer."),
            (GpuCommandEncoderHandle, GpuCommandEncoder, "host.gpu.command.encoder", "gpu_command_encoder", "The handle for one gpu command encoder."),
            (GpuCommandListHandle, GpuCommandList, "host.gpu.command.list", "gpu_command_list", "The handle for one gpu command list."),
            (GpuComputePassHandle, GpuComputePass, "host.gpu.compute.pass", "gpu_compute_pass", "The handle for one gpu compute pass endpoint."),
            (GpuExternalTextureHandle, GpuExternalTexture, "host.gpu.external.texture", "gpu_external_texture", "The handle for one gpu external texture."),
            (GpuRenderPassHandle, GpuRenderPass, "host.gpu.render.pass", "gpu_render_pass", "The handle for one gpu render pass endpoint."),
            (GpuRenderBundleHandle, GpuRenderBundle, "host.gpu.render.bundle", "gpu_render_bundle", "The handle for one gpu render bundle endpoint."),
            (GpuRenderBundleEncoderHandle, GpuRenderBundleEncoder, "host.gpu.render.bundle.encoder", "gpu_render_bundle_encoder", "The handle for one gpu render bundle encoder endpoint."),
            (GpuPipelineLayoutHandle, GpuPipelineLayout, "host.gpu.pipeline.layout", "gpu_pipeline_layout", "The handle for one gpu pipeline layout endpoint."),
            (GpuBindGroupLayoutHandle, GpuBindGroupLayout, "host.gpu.bind.group.layout", "gpu_bind_group_layout", "The handle for one gpu bind group layout endpoint."),
            (GpuBindGroupHandle, GpuBindGroup, "host.gpu.bind.group", "gpu_bind_group", "The handle for one gpu bind group endpoint."),
            (GpuFenceHandle, GpuFence, "host.gpu.fence", "gpu_fence", "The handle for one gpu synchronization fence endpoint."),
            (GpuQuerySetHandle, GpuQuerySet, "host.gpu.query.set", "gpu_query_set", "The handle for one gpu query set endpoint."),
            (GpuMemoryHandle, GpuMemory, "host.gpu.memory", "gpu_memory", "The handle for one gpu memory allocation."),
            (GpuBufferHandle, GpuBuffer, "host.gpu.buffer", "gpu_buffer", "The handle for one gpu buffer."),
            (GpuTextureHandle, GpuTexture, "host.gpu.texture", "gpu_texture", "The handle for one gpu texture."),
            (GpuTextureViewHandle, GpuTextureView, "host.gpu.texture.view", "gpu_texture_view", "The handle for one gpu texture view endpoint."),
            (GpuSamplerHandle, GpuSampler, "host.gpu.sampler", "gpu_sampler", "The handle for one gpu sampler."),
            (GpuShaderHandle, GpuShader, "host.gpu.shader", "gpu_shader", "The handle for one gpu shader module."),
            (GpuPipelineHandle, GpuPipeline, "host.gpu.pipeline", "gpu_pipeline", "The handle for one gpu pipeline."),
            (CryptoCertificateHandle, CryptoCertificate, "host.crypto.certificate", "crypto_certificate", "The handle for one cryptographic certificate object."),
            (CryptoDigestHandle, CryptoDigest, "host.crypto.digest", "crypto_digest", "The handle for one cryptographic digest context."),
            (CryptoMacHandle, CryptoMac, "host.crypto.mac", "crypto_mac", "The handle for one cryptographic message-authentication context."),
            (CryptoCipherHandle, CryptoCipher, "host.crypto.cipher", "crypto_cipher", "The handle for one cryptographic symmetric-cipher context."),
            (CryptoKeyHandle, CryptoKey, "host.crypto.key", "crypto_key", "The handle for one cryptographic key object."),
            (CryptoStoreHandle, CryptoStore, "host.crypto.store", "crypto_store", "The handle for one cryptographic store object."),
            (TlsContextHandle, TlsContext, "host.tls.context", "tls_context", "The handle for one tls context object."),
            (TlsSessionHandle, TlsSession, "host.tls.session", "tls_session", "The handle for one tls session object."),
            (BluetoothAdapterWatchHandle, BluetoothAdapterWatch, "host.device.bluetooth.adapter.watch", "bluetooth_adapter_watch", "The handle for one Bluetooth adapter watch stream."),
            (BluetoothDeviceHandle, BluetoothDevice, "host.device.bluetooth.device", "bluetooth_device", "The handle for one Bluetooth device session."),
            (BluetoothScanHandle, BluetoothScan, "host.device.bluetooth.scan", "bluetooth_scan", "The handle for one Bluetooth scan session."),
            (BluetoothSubscriptionHandle, BluetoothSubscription, "host.device.bluetooth.subscription", "bluetooth_subscription", "The handle for one Bluetooth GATT subscription session."),
            (CameraWatchHandle, CameraWatch, "host.device.camera.watch", "camera_watch", "The handle for one camera topology watch stream."),
            (CameraDeviceHandle, CameraDevice, "host.device.camera.device", "camera_device", "The handle for one camera device session."),
            (CameraStreamHandle, CameraStream, "host.device.camera.stream", "camera_stream", "The handle for one camera stream session."),
            (MidiEventHandle, MidiEvent, "host.device.midi.event", "midi_event", "The handle for one MIDI topology event stream."),
            (MidiInputPortHandle, MidiInputPort, "host.device.midi.input.port", "midi_input_port", "The handle for one opened MIDI input endpoint."),
            (MidiOutputPortHandle, MidiOutputPort, "host.device.midi.output.port", "midi_output_port", "The handle for one opened MIDI output endpoint."),
            (NetworkWatchHandle, NetworkWatch, "host.net.watch", "network_watch", "The handle for one network watch stream."),
            (SerialWatchHandle, SerialWatch, "host.device.serial.watch", "serial_watch", "The handle for one serial topology watch stream."),
            (SerialPortHandle, SerialPort, "host.device.serial.port", "serial_port", "The handle for one serial port session."),
            (UsbDeviceHandle, UsbDevice, "host.device.usb.device", "usb_device", "The handle for one USB device session."),
            (UsbWatchHandle, UsbWatch, "host.device.usb.watch", "usb_watch", "The handle for one USB hotplug watch stream."),
            (WindowEventHandle, WindowEvent, "host.display.window.event", "window_event", "The handle for one window event stream."),
            (DisplayEventHandle, DisplayEvent, "host.display.event", "display_event", "The handle for one display event stream."),
        }
    };
}

pub(crate) use for_each_resource_handle_kind;

macro_rules! define_resource_kind {
    ($(($handle:ident, $kind:ident, $kind_id:literal, $label:literal, $doc:literal),)+) => {
        /// Resource classification for host handles.
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

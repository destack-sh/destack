use super::ActionId;

/// Canonical runtime action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// `host.accessibility.actions`.
    AccessibilityActions,
    /// `host.accessibility.notify`.
    AccessibilityNotify,
    /// `host.accessibility.publish`.
    AccessibilityPublish,
    /// `host.audio.capture`.
    AudioCapture,
    /// `host.audio.control`.
    AudioControl,
    /// `host.audio.device`.
    AudioDevice,
    /// `host.audio.device.monitor`.
    AudioDeviceMonitor,
    /// `host.audio.playback`.
    AudioPlayback,
    /// `host.audio.playback.schedule`.
    AudioPlaybackSchedule,
    /// `host.audio.stream`.
    AudioStream,
    /// `host.console.write`.
    ConsoleWrite,
    /// `host.crypto.capability.read`.
    CryptoCapabilityRead,
    /// `host.crypto.certificate.read`.
    CryptoCertificateRead,
    /// `host.crypto.certificate.write`.
    CryptoCertificateWrite,
    /// `host.crypto.cipher`.
    CryptoCipher,
    /// `host.crypto.digest`.
    CryptoDigest,
    /// `host.crypto.kdf`.
    CryptoKdf,
    /// `host.crypto.key.agree`.
    CryptoKeyAgree,
    /// `host.crypto.key.decrypt`.
    CryptoKeyDecrypt,
    /// `host.crypto.key.delete`.
    CryptoKeyDelete,
    /// `host.crypto.key.encrypt`.
    CryptoKeyEncrypt,
    /// `host.crypto.key.export`.
    CryptoKeyExport,
    /// `host.crypto.key.generate`.
    CryptoKeyGenerate,
    /// `host.crypto.key.import`.
    CryptoKeyImport,
    /// `host.crypto.key.sign`.
    CryptoKeySign,
    /// `host.crypto.key.unwrap`.
    CryptoKeyUnwrap,
    /// `host.crypto.key.verify`.
    CryptoKeyVerify,
    /// `host.crypto.key.wrap`.
    CryptoKeyWrap,
    /// `host.crypto.mac`.
    CryptoMac,
    /// `host.crypto.store.read`.
    CryptoStoreRead,
    /// `host.crypto.store.write`.
    CryptoStoreWrite,
    /// `host.device.bluetooth.connect`.
    DeviceBluetoothConnect,
    /// `host.device.bluetooth.gatt`.
    DeviceBluetoothGatt,
    /// `host.device.bluetooth.list`.
    DeviceBluetoothList,
    /// `host.device.bluetooth.scan`.
    DeviceBluetoothScan,
    /// `host.device.camera.capture`.
    DeviceCameraCapture,
    /// `host.device.camera.configure`.
    DeviceCameraConfigure,
    /// `host.device.camera.list`.
    DeviceCameraList,
    /// `host.device.camera.open`.
    DeviceCameraOpen,
    /// `host.device.midi.list`.
    DeviceMidiList,
    /// `host.device.midi.observe`.
    DeviceMidiObserve,
    /// `host.device.midi.open`.
    DeviceMidiOpen,
    /// `host.device.midi.read`.
    DeviceMidiRead,
    /// `host.device.midi.virtual`.
    DeviceMidiVirtual,
    /// `host.device.midi.write`.
    DeviceMidiWrite,
    /// `host.device.serial.configure`.
    DeviceSerialConfigure,
    /// `host.device.serial.list`.
    DeviceSerialList,
    /// `host.device.serial.observe`.
    DeviceSerialObserve,
    /// `host.device.serial.open`.
    DeviceSerialOpen,
    /// `host.device.serial.read`.
    DeviceSerialRead,
    /// `host.device.serial.write`.
    DeviceSerialWrite,
    /// `host.device.usb.configure`.
    DeviceUsbConfigure,
    /// `host.device.usb.list`.
    DeviceUsbList,
    /// `host.device.usb.open`.
    DeviceUsbOpen,
    /// `host.device.usb.transfer`.
    DeviceUsbTransfer,
    /// `host.display.mode`.
    DisplayMode,
    /// `host.display.read`.
    DisplayRead,
    /// `host.display.window`.
    DisplayWindow,
    /// `host.display.window.events`.
    DisplayWindowEvents,
    /// `host.display.window.modal`.
    DisplayWindowModal,
    /// `host.display.window.parenting`.
    DisplayWindowParenting,
    /// `host.ffi.call`.
    FfiCall,
    /// `host.ffi.load`.
    FfiLoad,
    /// `host.ffi.pointer`.
    FfiPointer,
    /// `host.ffi.symbol`.
    FfiSymbol,
    /// `host.fs.link`.
    FsLink,
    /// `host.fs.lock`.
    FsLock,
    /// `host.fs.metadata`.
    FsMetadata,
    /// `host.fs.mode`.
    FsMode,
    /// `host.fs.mount.list`.
    FsMountList,
    /// `host.fs.node`.
    FsNode,
    /// `host.fs.open`.
    FsOpen,
    /// `host.fs.owner`.
    FsOwner,
    /// `host.fs.read`.
    FsRead,
    /// `host.fs.sync`.
    FsSync,
    /// `host.fs.temp`.
    FsTemp,
    /// `host.fs.watch`.
    FsWatch,
    /// `host.fs.write`.
    FsWrite,
    /// `host.fs.xattr`.
    FsXattr,
    /// `host.gpu.adapter`.
    GpuAdapter,
    /// `host.gpu.bind`.
    GpuBind,
    /// `host.gpu.compute`.
    GpuCompute,
    /// `host.gpu.debug`.
    GpuDebug,
    /// `host.gpu.device`.
    GpuDevice,
    /// `host.gpu.memory`.
    GpuMemory,
    /// `host.gpu.present`.
    GpuPresent,
    /// `host.gpu.queue`.
    GpuQueue,
    /// `host.gpu.render`.
    GpuRender,
    /// `host.gpu.render.multi.draw`.
    GpuRenderMultiDraw,
    /// `host.gpu.render.multi.draw.count`.
    GpuRenderMultiDrawCount,
    /// `host.gpu.shader`.
    GpuShader,
    /// `host.gpu.surface`.
    GpuSurface,
    /// `host.gpu.sync`.
    GpuSync,
    /// `host.gpu.sync.pipeline.statistics`.
    GpuSyncPipelineStatistics,
    /// `host.input.clipboard.read`.
    InputClipboardRead,
    /// `host.input.clipboard.write`.
    InputClipboardWrite,
    /// `host.input.control`.
    InputControl,
    /// `host.input.grab`.
    InputGrab,
    /// `host.input.haptics`.
    InputHaptics,
    /// `host.input.read`.
    InputRead,
    /// `host.input.text`.
    InputText,
    /// `host.input.write`.
    InputWrite,
    /// `host.io.completion`.
    IoCompletion,
    /// `host.io.control`.
    IoControl,
    /// `host.io.poll`.
    IoPoll,
    /// `host.io.uring`.
    IoUring,
    /// `host.ipc.eventCounter`.
    IpcEventCounter,
    /// `host.ipc.futex`.
    IpcFutex,
    /// `host.ipc.handle.pass`.
    IpcHandlePass,
    /// `host.ipc.memory`.
    IpcMemory,
    /// `host.ipc.message`.
    IpcMessage,
    /// `host.ipc.pipe`.
    IpcPipe,
    /// `host.ipc.semaphore`.
    IpcSemaphore,
    /// `host.ipc.unix`.
    IpcUnix,
    /// `host.memory.advise`.
    MemoryAdvise,
    /// `host.memory.discard`.
    MemoryDiscard,
    /// `host.memory.execute`.
    MemoryExecute,
    /// `host.memory.layout.read`.
    MemoryLayoutRead,
    /// `host.memory.lock`.
    MemoryLock,
    /// `host.memory.map`.
    MemoryMap,
    /// `host.memory.virtual`.
    MemoryVirtual,
    /// `host.net.accept`.
    NetAccept,
    /// `host.net.close`.
    NetClose,
    /// `host.net.connect`.
    NetConnect,
    /// `host.net.connectivity.read`.
    NetConnectivityRead,
    /// `host.net.connectivity.watch`.
    NetConnectivityWatch,
    /// `host.net.dns`.
    NetDns,
    /// `host.net.interface`.
    NetInterface,
    /// `host.net.listen`.
    NetListen,
    /// `host.net.multicast`.
    NetMulticast,
    /// `host.net.open`.
    NetOpen,
    /// `host.net.option`.
    NetOption,
    /// `host.net.raw`.
    NetRaw,
    /// `host.net.read`.
    NetRead,
    /// `host.net.route.read`.
    NetRouteRead,
    /// `host.net.route.write`.
    NetRouteWrite,
    /// `host.net.status`.
    NetStatus,
    /// `host.net.write`.
    NetWrite,
    /// `host.os.capacity.read`.
    OsCapacityRead,
    /// `host.os.identity.read`.
    OsIdentityRead,
    /// `host.os.power.read`.
    OsPowerRead,
    /// `host.os.pressure.read`.
    OsPressureRead,
    /// `host.process.affinity`.
    ProcessAffinity,
    /// `host.process.arguments`.
    ProcessArguments,
    /// `host.process.cgroup.read`.
    ProcessCgroupRead,
    /// `host.process.cgroup.write`.
    ProcessCgroupWrite,
    /// `host.process.environment.read`.
    ProcessEnvironmentRead,
    /// `host.process.environment.write`.
    ProcessEnvironmentWrite,
    /// `host.process.exec`.
    ProcessExec,
    /// `host.process.exit`.
    ProcessExit,
    /// `host.process.handle`.
    ProcessHandle,
    /// `host.process.identity.read`.
    ProcessIdentityRead,
    /// `host.process.identity.write`.
    ProcessIdentityWrite,
    /// `host.process.job`.
    ProcessJob,
    /// `host.process.limit.read`.
    ProcessLimitRead,
    /// `host.process.limit.write`.
    ProcessLimitWrite,
    /// `host.process.namespace`.
    ProcessNamespace,
    /// `host.process.priority`.
    ProcessPriority,
    /// `host.process.session`.
    ProcessSession,
    /// `host.process.signal.receive`.
    ProcessSignalReceive,
    /// `host.process.signal.send`.
    ProcessSignalSend,
    /// `host.process.spawn`.
    ProcessSpawn,
    /// `host.process.stdio`.
    ProcessStdio,
    /// `host.process.umask`.
    ProcessUmask,
    /// `host.process.wait`.
    ProcessWait,
    /// `host.process.workdir.read`.
    ProcessWorkdirRead,
    /// `host.process.workdir.write`.
    ProcessWorkdirWrite,
    /// `host.random.secure`.
    RandomSecure,
    /// `host.time.monotonic.read`.
    TimeMonotonicRead,
    /// `host.time.timer`.
    TimeTimer,
    /// `host.time.timerfd`.
    TimeTimerfd,
    /// `host.time.wall.read`.
    TimeWallRead,
    /// `host.time.wall.sleep`.
    TimeWallSleep,
    /// `host.tls.certificate.read`.
    TlsCertificateRead,
    /// `host.tls.connection`.
    TlsConnection,
    /// `host.tls.context`.
    TlsContext,
    /// `host.tls.exporter`.
    TlsExporter,
    /// `host.tls.handshake`.
    TlsHandshake,
    /// `host.tls.hostname.verify`.
    TlsHostnameVerify,
    /// `host.tls.identity.use`.
    TlsIdentityUse,
    /// `host.tls.identity.write`.
    TlsIdentityWrite,
    /// `host.tls.policy`.
    TlsPolicy,
    /// `host.tls.resumption`.
    TlsResumption,
    /// `host.tls.trust.write`.
    TlsTrustWrite,
    /// `host.tty.close`.
    TtyClose,
    /// `host.tty.mode`.
    TtyMode,
    /// `host.tty.open`.
    TtyOpen,
    /// `host.tty.pty`.
    TtyPty,
    /// `host.tty.pty.read`.
    TtyPtyRead,
    /// `host.tty.pty.write`.
    TtyPtyWrite,
    /// `host.tty.read`.
    TtyRead,
    /// `host.tty.size`.
    TtySize,
    /// `host.tty.status`.
    TtyStatus,
    /// `host.tty.termios`.
    TtyTermios,
    /// `host.tty.write`.
    TtyWrite,
    /// `runtime.debug.inspect`.
    DebugInspect,
    /// `runtime.debug.profile`.
    DebugProfile,
    /// `runtime.debug.trace`.
    DebugTrace,
    /// `runtime.diagnostic.read`.
    DiagnosticRead,
    /// `runtime.inspect.read`.
    InspectRead,
    /// `runtime.lineage.control`.
    LineageControl,
    /// `runtime.lineage.read`.
    LineageRead,
    /// `runtime.observation.read`.
    ObservationRead,
    /// `runtime.random.deterministic`.
    RandomDeterministic,
    /// `runtime.resource.close`.
    ResourceClose,
    /// `runtime.resource.manage`.
    ResourceManage,
    /// `runtime.resource.read`.
    ResourceRead,
    /// `runtime.resource.transfer`.
    ResourceTransfer,
    /// `runtime.resource.write`.
    ResourceWrite,
    /// `runtime.runtime.control`.
    RuntimeControl,
    /// `runtime.runtime.create`.
    RuntimeCreate,
    /// `runtime.runtime.read`.
    RuntimeRead,
    /// `runtime.security.filter`.
    SecurityFilter,
    /// `runtime.security.permission.read`.
    SecurityPermissionRead,
    /// `runtime.security.permission.request`.
    SecurityPermissionRequest,
    /// `runtime.security.policy.read`.
    SecurityPolicyRead,
    /// `runtime.security.policy.write`.
    SecurityPolicyWrite,
    /// `runtime.security.restrict`.
    SecurityRestrict,
    /// `runtime.security.sandbox`.
    SecuritySandbox,
    /// `runtime.snapshot.create`.
    SnapshotCreate,
    /// `runtime.snapshot.read`.
    SnapshotRead,
    /// `runtime.snapshot.restore`.
    SnapshotRestore,
    /// `runtime.trace.control`.
    TraceControl,
    /// `runtime.trace.read`.
    TraceRead,
    /// `runtime.worker.control`.
    WorkerControl,
    /// `runtime.worker.create`.
    WorkerCreate,
    /// `runtime.worker.read`.
    WorkerRead,
    /// `runtime.world.control`.
    WorldControl,
    /// `runtime.world.create`.
    WorldCreate,
    /// `runtime.world.read`.
    WorldRead,
}

impl Action {
    /// All canonical runtime action kinds.
    pub const ALL: &'static [Self] = &[
        Self::AccessibilityActions,
        Self::AccessibilityNotify,
        Self::AccessibilityPublish,
        Self::AudioCapture,
        Self::AudioControl,
        Self::AudioDevice,
        Self::AudioDeviceMonitor,
        Self::AudioPlayback,
        Self::AudioPlaybackSchedule,
        Self::AudioStream,
        Self::ConsoleWrite,
        Self::CryptoCapabilityRead,
        Self::CryptoCertificateRead,
        Self::CryptoCertificateWrite,
        Self::CryptoCipher,
        Self::CryptoDigest,
        Self::CryptoKdf,
        Self::CryptoKeyAgree,
        Self::CryptoKeyDecrypt,
        Self::CryptoKeyDelete,
        Self::CryptoKeyEncrypt,
        Self::CryptoKeyExport,
        Self::CryptoKeyGenerate,
        Self::CryptoKeyImport,
        Self::CryptoKeySign,
        Self::CryptoKeyUnwrap,
        Self::CryptoKeyVerify,
        Self::CryptoKeyWrap,
        Self::CryptoMac,
        Self::CryptoStoreRead,
        Self::CryptoStoreWrite,
        Self::DeviceBluetoothConnect,
        Self::DeviceBluetoothGatt,
        Self::DeviceBluetoothList,
        Self::DeviceBluetoothScan,
        Self::DeviceCameraCapture,
        Self::DeviceCameraConfigure,
        Self::DeviceCameraList,
        Self::DeviceCameraOpen,
        Self::DeviceMidiList,
        Self::DeviceMidiObserve,
        Self::DeviceMidiOpen,
        Self::DeviceMidiRead,
        Self::DeviceMidiVirtual,
        Self::DeviceMidiWrite,
        Self::DeviceSerialConfigure,
        Self::DeviceSerialList,
        Self::DeviceSerialObserve,
        Self::DeviceSerialOpen,
        Self::DeviceSerialRead,
        Self::DeviceSerialWrite,
        Self::DeviceUsbConfigure,
        Self::DeviceUsbList,
        Self::DeviceUsbOpen,
        Self::DeviceUsbTransfer,
        Self::DisplayMode,
        Self::DisplayRead,
        Self::DisplayWindow,
        Self::DisplayWindowEvents,
        Self::DisplayWindowModal,
        Self::DisplayWindowParenting,
        Self::FfiCall,
        Self::FfiLoad,
        Self::FfiPointer,
        Self::FfiSymbol,
        Self::FsLink,
        Self::FsLock,
        Self::FsMetadata,
        Self::FsMode,
        Self::FsMountList,
        Self::FsNode,
        Self::FsOpen,
        Self::FsOwner,
        Self::FsRead,
        Self::FsSync,
        Self::FsTemp,
        Self::FsWatch,
        Self::FsWrite,
        Self::FsXattr,
        Self::GpuAdapter,
        Self::GpuBind,
        Self::GpuCompute,
        Self::GpuDebug,
        Self::GpuDevice,
        Self::GpuMemory,
        Self::GpuPresent,
        Self::GpuQueue,
        Self::GpuRender,
        Self::GpuRenderMultiDraw,
        Self::GpuRenderMultiDrawCount,
        Self::GpuShader,
        Self::GpuSurface,
        Self::GpuSync,
        Self::GpuSyncPipelineStatistics,
        Self::InputClipboardRead,
        Self::InputClipboardWrite,
        Self::InputControl,
        Self::InputGrab,
        Self::InputHaptics,
        Self::InputRead,
        Self::InputText,
        Self::InputWrite,
        Self::IoCompletion,
        Self::IoControl,
        Self::IoPoll,
        Self::IoUring,
        Self::IpcEventCounter,
        Self::IpcFutex,
        Self::IpcHandlePass,
        Self::IpcMemory,
        Self::IpcMessage,
        Self::IpcPipe,
        Self::IpcSemaphore,
        Self::IpcUnix,
        Self::MemoryAdvise,
        Self::MemoryDiscard,
        Self::MemoryExecute,
        Self::MemoryLayoutRead,
        Self::MemoryLock,
        Self::MemoryMap,
        Self::MemoryVirtual,
        Self::NetAccept,
        Self::NetClose,
        Self::NetConnect,
        Self::NetConnectivityRead,
        Self::NetConnectivityWatch,
        Self::NetDns,
        Self::NetInterface,
        Self::NetListen,
        Self::NetMulticast,
        Self::NetOpen,
        Self::NetOption,
        Self::NetRaw,
        Self::NetRead,
        Self::NetRouteRead,
        Self::NetRouteWrite,
        Self::NetStatus,
        Self::NetWrite,
        Self::OsCapacityRead,
        Self::OsIdentityRead,
        Self::OsPowerRead,
        Self::OsPressureRead,
        Self::ProcessAffinity,
        Self::ProcessArguments,
        Self::ProcessCgroupRead,
        Self::ProcessCgroupWrite,
        Self::ProcessEnvironmentRead,
        Self::ProcessEnvironmentWrite,
        Self::ProcessExec,
        Self::ProcessExit,
        Self::ProcessHandle,
        Self::ProcessIdentityRead,
        Self::ProcessIdentityWrite,
        Self::ProcessJob,
        Self::ProcessLimitRead,
        Self::ProcessLimitWrite,
        Self::ProcessNamespace,
        Self::ProcessPriority,
        Self::ProcessSession,
        Self::ProcessSignalReceive,
        Self::ProcessSignalSend,
        Self::ProcessSpawn,
        Self::ProcessStdio,
        Self::ProcessUmask,
        Self::ProcessWait,
        Self::ProcessWorkdirRead,
        Self::ProcessWorkdirWrite,
        Self::RandomSecure,
        Self::TimeMonotonicRead,
        Self::TimeTimer,
        Self::TimeTimerfd,
        Self::TimeWallRead,
        Self::TimeWallSleep,
        Self::TlsCertificateRead,
        Self::TlsConnection,
        Self::TlsContext,
        Self::TlsExporter,
        Self::TlsHandshake,
        Self::TlsHostnameVerify,
        Self::TlsIdentityUse,
        Self::TlsIdentityWrite,
        Self::TlsPolicy,
        Self::TlsResumption,
        Self::TlsTrustWrite,
        Self::TtyClose,
        Self::TtyMode,
        Self::TtyOpen,
        Self::TtyPty,
        Self::TtyPtyRead,
        Self::TtyPtyWrite,
        Self::TtyRead,
        Self::TtySize,
        Self::TtyStatus,
        Self::TtyTermios,
        Self::TtyWrite,
        Self::DebugInspect,
        Self::DebugProfile,
        Self::DebugTrace,
        Self::DiagnosticRead,
        Self::InspectRead,
        Self::LineageControl,
        Self::LineageRead,
        Self::ObservationRead,
        Self::RandomDeterministic,
        Self::ResourceClose,
        Self::ResourceManage,
        Self::ResourceRead,
        Self::ResourceTransfer,
        Self::ResourceWrite,
        Self::RuntimeControl,
        Self::RuntimeCreate,
        Self::RuntimeRead,
        Self::SecurityFilter,
        Self::SecurityPermissionRead,
        Self::SecurityPermissionRequest,
        Self::SecurityPolicyRead,
        Self::SecurityPolicyWrite,
        Self::SecurityRestrict,
        Self::SecuritySandbox,
        Self::SnapshotCreate,
        Self::SnapshotRead,
        Self::SnapshotRestore,
        Self::TraceControl,
        Self::TraceRead,
        Self::WorkerControl,
        Self::WorkerCreate,
        Self::WorkerRead,
        Self::WorldControl,
        Self::WorldCreate,
        Self::WorldRead,
    ];

    /// Return the canonical action name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AccessibilityActions => "host.accessibility.actions",
            Self::AccessibilityNotify => "host.accessibility.notify",
            Self::AccessibilityPublish => "host.accessibility.publish",
            Self::AudioCapture => "host.audio.capture",
            Self::AudioControl => "host.audio.control",
            Self::AudioDevice => "host.audio.device",
            Self::AudioDeviceMonitor => "host.audio.device.monitor",
            Self::AudioPlayback => "host.audio.playback",
            Self::AudioPlaybackSchedule => "host.audio.playback.schedule",
            Self::AudioStream => "host.audio.stream",
            Self::ConsoleWrite => "host.console.write",
            Self::CryptoCapabilityRead => "host.crypto.capability.read",
            Self::CryptoCertificateRead => "host.crypto.certificate.read",
            Self::CryptoCertificateWrite => "host.crypto.certificate.write",
            Self::CryptoCipher => "host.crypto.cipher",
            Self::CryptoDigest => "host.crypto.digest",
            Self::CryptoKdf => "host.crypto.kdf",
            Self::CryptoKeyAgree => "host.crypto.key.agree",
            Self::CryptoKeyDecrypt => "host.crypto.key.decrypt",
            Self::CryptoKeyDelete => "host.crypto.key.delete",
            Self::CryptoKeyEncrypt => "host.crypto.key.encrypt",
            Self::CryptoKeyExport => "host.crypto.key.export",
            Self::CryptoKeyGenerate => "host.crypto.key.generate",
            Self::CryptoKeyImport => "host.crypto.key.import",
            Self::CryptoKeySign => "host.crypto.key.sign",
            Self::CryptoKeyUnwrap => "host.crypto.key.unwrap",
            Self::CryptoKeyVerify => "host.crypto.key.verify",
            Self::CryptoKeyWrap => "host.crypto.key.wrap",
            Self::CryptoMac => "host.crypto.mac",
            Self::CryptoStoreRead => "host.crypto.store.read",
            Self::CryptoStoreWrite => "host.crypto.store.write",
            Self::DeviceBluetoothConnect => "host.device.bluetooth.connect",
            Self::DeviceBluetoothGatt => "host.device.bluetooth.gatt",
            Self::DeviceBluetoothList => "host.device.bluetooth.list",
            Self::DeviceBluetoothScan => "host.device.bluetooth.scan",
            Self::DeviceCameraCapture => "host.device.camera.capture",
            Self::DeviceCameraConfigure => "host.device.camera.configure",
            Self::DeviceCameraList => "host.device.camera.list",
            Self::DeviceCameraOpen => "host.device.camera.open",
            Self::DeviceMidiList => "host.device.midi.list",
            Self::DeviceMidiObserve => "host.device.midi.observe",
            Self::DeviceMidiOpen => "host.device.midi.open",
            Self::DeviceMidiRead => "host.device.midi.read",
            Self::DeviceMidiVirtual => "host.device.midi.virtual",
            Self::DeviceMidiWrite => "host.device.midi.write",
            Self::DeviceSerialConfigure => "host.device.serial.configure",
            Self::DeviceSerialList => "host.device.serial.list",
            Self::DeviceSerialObserve => "host.device.serial.observe",
            Self::DeviceSerialOpen => "host.device.serial.open",
            Self::DeviceSerialRead => "host.device.serial.read",
            Self::DeviceSerialWrite => "host.device.serial.write",
            Self::DeviceUsbConfigure => "host.device.usb.configure",
            Self::DeviceUsbList => "host.device.usb.list",
            Self::DeviceUsbOpen => "host.device.usb.open",
            Self::DeviceUsbTransfer => "host.device.usb.transfer",
            Self::DisplayMode => "host.display.mode",
            Self::DisplayRead => "host.display.read",
            Self::DisplayWindow => "host.display.window",
            Self::DisplayWindowEvents => "host.display.window.events",
            Self::DisplayWindowModal => "host.display.window.modal",
            Self::DisplayWindowParenting => "host.display.window.parenting",
            Self::FfiCall => "host.ffi.call",
            Self::FfiLoad => "host.ffi.load",
            Self::FfiPointer => "host.ffi.pointer",
            Self::FfiSymbol => "host.ffi.symbol",
            Self::FsLink => "host.fs.link",
            Self::FsLock => "host.fs.lock",
            Self::FsMetadata => "host.fs.metadata",
            Self::FsMode => "host.fs.mode",
            Self::FsMountList => "host.fs.mount.list",
            Self::FsNode => "host.fs.node",
            Self::FsOpen => "host.fs.open",
            Self::FsOwner => "host.fs.owner",
            Self::FsRead => "host.fs.read",
            Self::FsSync => "host.fs.sync",
            Self::FsTemp => "host.fs.temp",
            Self::FsWatch => "host.fs.watch",
            Self::FsWrite => "host.fs.write",
            Self::FsXattr => "host.fs.xattr",
            Self::GpuAdapter => "host.gpu.adapter",
            Self::GpuBind => "host.gpu.bind",
            Self::GpuCompute => "host.gpu.compute",
            Self::GpuDebug => "host.gpu.debug",
            Self::GpuDevice => "host.gpu.device",
            Self::GpuMemory => "host.gpu.memory",
            Self::GpuPresent => "host.gpu.present",
            Self::GpuQueue => "host.gpu.queue",
            Self::GpuRender => "host.gpu.render",
            Self::GpuRenderMultiDraw => "host.gpu.render.multi.draw",
            Self::GpuRenderMultiDrawCount => "host.gpu.render.multi.draw.count",
            Self::GpuShader => "host.gpu.shader",
            Self::GpuSurface => "host.gpu.surface",
            Self::GpuSync => "host.gpu.sync",
            Self::GpuSyncPipelineStatistics => "host.gpu.sync.pipeline.statistics",
            Self::InputClipboardRead => "host.input.clipboard.read",
            Self::InputClipboardWrite => "host.input.clipboard.write",
            Self::InputControl => "host.input.control",
            Self::InputGrab => "host.input.grab",
            Self::InputHaptics => "host.input.haptics",
            Self::InputRead => "host.input.read",
            Self::InputText => "host.input.text",
            Self::InputWrite => "host.input.write",
            Self::IoCompletion => "host.io.completion",
            Self::IoControl => "host.io.control",
            Self::IoPoll => "host.io.poll",
            Self::IoUring => "host.io.uring",
            Self::IpcEventCounter => "host.ipc.eventCounter",
            Self::IpcFutex => "host.ipc.futex",
            Self::IpcHandlePass => "host.ipc.handle.pass",
            Self::IpcMemory => "host.ipc.memory",
            Self::IpcMessage => "host.ipc.message",
            Self::IpcPipe => "host.ipc.pipe",
            Self::IpcSemaphore => "host.ipc.semaphore",
            Self::IpcUnix => "host.ipc.unix",
            Self::MemoryAdvise => "host.memory.advise",
            Self::MemoryDiscard => "host.memory.discard",
            Self::MemoryExecute => "host.memory.execute",
            Self::MemoryLayoutRead => "host.memory.layout.read",
            Self::MemoryLock => "host.memory.lock",
            Self::MemoryMap => "host.memory.map",
            Self::MemoryVirtual => "host.memory.virtual",
            Self::NetAccept => "host.net.accept",
            Self::NetClose => "host.net.close",
            Self::NetConnect => "host.net.connect",
            Self::NetConnectivityRead => "host.net.connectivity.read",
            Self::NetConnectivityWatch => "host.net.connectivity.watch",
            Self::NetDns => "host.net.dns",
            Self::NetInterface => "host.net.interface",
            Self::NetListen => "host.net.listen",
            Self::NetMulticast => "host.net.multicast",
            Self::NetOpen => "host.net.open",
            Self::NetOption => "host.net.option",
            Self::NetRaw => "host.net.raw",
            Self::NetRead => "host.net.read",
            Self::NetRouteRead => "host.net.route.read",
            Self::NetRouteWrite => "host.net.route.write",
            Self::NetStatus => "host.net.status",
            Self::NetWrite => "host.net.write",
            Self::OsCapacityRead => "host.os.capacity.read",
            Self::OsIdentityRead => "host.os.identity.read",
            Self::OsPowerRead => "host.os.power.read",
            Self::OsPressureRead => "host.os.pressure.read",
            Self::ProcessAffinity => "host.process.affinity",
            Self::ProcessArguments => "host.process.arguments",
            Self::ProcessCgroupRead => "host.process.cgroup.read",
            Self::ProcessCgroupWrite => "host.process.cgroup.write",
            Self::ProcessEnvironmentRead => "host.process.environment.read",
            Self::ProcessEnvironmentWrite => "host.process.environment.write",
            Self::ProcessExec => "host.process.exec",
            Self::ProcessExit => "host.process.exit",
            Self::ProcessHandle => "host.process.handle",
            Self::ProcessIdentityRead => "host.process.identity.read",
            Self::ProcessIdentityWrite => "host.process.identity.write",
            Self::ProcessJob => "host.process.job",
            Self::ProcessLimitRead => "host.process.limit.read",
            Self::ProcessLimitWrite => "host.process.limit.write",
            Self::ProcessNamespace => "host.process.namespace",
            Self::ProcessPriority => "host.process.priority",
            Self::ProcessSession => "host.process.session",
            Self::ProcessSignalReceive => "host.process.signal.receive",
            Self::ProcessSignalSend => "host.process.signal.send",
            Self::ProcessSpawn => "host.process.spawn",
            Self::ProcessStdio => "host.process.stdio",
            Self::ProcessUmask => "host.process.umask",
            Self::ProcessWait => "host.process.wait",
            Self::ProcessWorkdirRead => "host.process.workdir.read",
            Self::ProcessWorkdirWrite => "host.process.workdir.write",
            Self::RandomSecure => "host.random.secure",
            Self::TimeMonotonicRead => "host.time.monotonic.read",
            Self::TimeTimer => "host.time.timer",
            Self::TimeTimerfd => "host.time.timerfd",
            Self::TimeWallRead => "host.time.wall.read",
            Self::TimeWallSleep => "host.time.wall.sleep",
            Self::TlsCertificateRead => "host.tls.certificate.read",
            Self::TlsConnection => "host.tls.connection",
            Self::TlsContext => "host.tls.context",
            Self::TlsExporter => "host.tls.exporter",
            Self::TlsHandshake => "host.tls.handshake",
            Self::TlsHostnameVerify => "host.tls.hostname.verify",
            Self::TlsIdentityUse => "host.tls.identity.use",
            Self::TlsIdentityWrite => "host.tls.identity.write",
            Self::TlsPolicy => "host.tls.policy",
            Self::TlsResumption => "host.tls.resumption",
            Self::TlsTrustWrite => "host.tls.trust.write",
            Self::TtyClose => "host.tty.close",
            Self::TtyMode => "host.tty.mode",
            Self::TtyOpen => "host.tty.open",
            Self::TtyPty => "host.tty.pty",
            Self::TtyPtyRead => "host.tty.pty.read",
            Self::TtyPtyWrite => "host.tty.pty.write",
            Self::TtyRead => "host.tty.read",
            Self::TtySize => "host.tty.size",
            Self::TtyStatus => "host.tty.status",
            Self::TtyTermios => "host.tty.termios",
            Self::TtyWrite => "host.tty.write",
            Self::DebugInspect => "runtime.debug.inspect",
            Self::DebugProfile => "runtime.debug.profile",
            Self::DebugTrace => "runtime.debug.trace",
            Self::DiagnosticRead => "runtime.diagnostic.read",
            Self::InspectRead => "runtime.inspect.read",
            Self::LineageControl => "runtime.lineage.control",
            Self::LineageRead => "runtime.lineage.read",
            Self::ObservationRead => "runtime.observation.read",
            Self::RandomDeterministic => "runtime.random.deterministic",
            Self::ResourceClose => "runtime.resource.close",
            Self::ResourceManage => "runtime.resource.manage",
            Self::ResourceRead => "runtime.resource.read",
            Self::ResourceTransfer => "runtime.resource.transfer",
            Self::ResourceWrite => "runtime.resource.write",
            Self::RuntimeControl => "runtime.runtime.control",
            Self::RuntimeCreate => "runtime.runtime.create",
            Self::RuntimeRead => "runtime.runtime.read",
            Self::SecurityFilter => "runtime.security.filter",
            Self::SecurityPermissionRead => "runtime.security.permission.read",
            Self::SecurityPermissionRequest => "runtime.security.permission.request",
            Self::SecurityPolicyRead => "runtime.security.policy.read",
            Self::SecurityPolicyWrite => "runtime.security.policy.write",
            Self::SecurityRestrict => "runtime.security.restrict",
            Self::SecuritySandbox => "runtime.security.sandbox",
            Self::SnapshotCreate => "runtime.snapshot.create",
            Self::SnapshotRead => "runtime.snapshot.read",
            Self::SnapshotRestore => "runtime.snapshot.restore",
            Self::TraceControl => "runtime.trace.control",
            Self::TraceRead => "runtime.trace.read",
            Self::WorkerControl => "runtime.worker.control",
            Self::WorkerCreate => "runtime.worker.create",
            Self::WorkerRead => "runtime.worker.read",
            Self::WorldControl => "runtime.world.control",
            Self::WorldCreate => "runtime.world.create",
            Self::WorldRead => "runtime.world.read",
        }
    }

    /// Return the stable action identifier.
    pub const fn id(self) -> ActionId {
        ActionId::from_name(self.name())
    }

    /// Resolve one action kind from one canonical action name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "host.accessibility.actions" => Some(Self::AccessibilityActions),
            "host.accessibility.notify" => Some(Self::AccessibilityNotify),
            "host.accessibility.publish" => Some(Self::AccessibilityPublish),
            "host.audio.capture" => Some(Self::AudioCapture),
            "host.audio.control" => Some(Self::AudioControl),
            "host.audio.device" => Some(Self::AudioDevice),
            "host.audio.device.monitor" => Some(Self::AudioDeviceMonitor),
            "host.audio.playback" => Some(Self::AudioPlayback),
            "host.audio.playback.schedule" => Some(Self::AudioPlaybackSchedule),
            "host.audio.stream" => Some(Self::AudioStream),
            "host.console.write" => Some(Self::ConsoleWrite),
            "host.crypto.capability.read" => Some(Self::CryptoCapabilityRead),
            "host.crypto.certificate.read" => Some(Self::CryptoCertificateRead),
            "host.crypto.certificate.write" => Some(Self::CryptoCertificateWrite),
            "host.crypto.cipher" => Some(Self::CryptoCipher),
            "host.crypto.digest" => Some(Self::CryptoDigest),
            "host.crypto.kdf" => Some(Self::CryptoKdf),
            "host.crypto.key.agree" => Some(Self::CryptoKeyAgree),
            "host.crypto.key.decrypt" => Some(Self::CryptoKeyDecrypt),
            "host.crypto.key.delete" => Some(Self::CryptoKeyDelete),
            "host.crypto.key.encrypt" => Some(Self::CryptoKeyEncrypt),
            "host.crypto.key.export" => Some(Self::CryptoKeyExport),
            "host.crypto.key.generate" => Some(Self::CryptoKeyGenerate),
            "host.crypto.key.import" => Some(Self::CryptoKeyImport),
            "host.crypto.key.sign" => Some(Self::CryptoKeySign),
            "host.crypto.key.unwrap" => Some(Self::CryptoKeyUnwrap),
            "host.crypto.key.verify" => Some(Self::CryptoKeyVerify),
            "host.crypto.key.wrap" => Some(Self::CryptoKeyWrap),
            "host.crypto.mac" => Some(Self::CryptoMac),
            "host.crypto.store.read" => Some(Self::CryptoStoreRead),
            "host.crypto.store.write" => Some(Self::CryptoStoreWrite),
            "host.device.bluetooth.connect" => Some(Self::DeviceBluetoothConnect),
            "host.device.bluetooth.gatt" => Some(Self::DeviceBluetoothGatt),
            "host.device.bluetooth.list" => Some(Self::DeviceBluetoothList),
            "host.device.bluetooth.scan" => Some(Self::DeviceBluetoothScan),
            "host.device.camera.capture" => Some(Self::DeviceCameraCapture),
            "host.device.camera.configure" => Some(Self::DeviceCameraConfigure),
            "host.device.camera.list" => Some(Self::DeviceCameraList),
            "host.device.camera.open" => Some(Self::DeviceCameraOpen),
            "host.device.midi.list" => Some(Self::DeviceMidiList),
            "host.device.midi.observe" => Some(Self::DeviceMidiObserve),
            "host.device.midi.open" => Some(Self::DeviceMidiOpen),
            "host.device.midi.read" => Some(Self::DeviceMidiRead),
            "host.device.midi.virtual" => Some(Self::DeviceMidiVirtual),
            "host.device.midi.write" => Some(Self::DeviceMidiWrite),
            "host.device.serial.configure" => Some(Self::DeviceSerialConfigure),
            "host.device.serial.list" => Some(Self::DeviceSerialList),
            "host.device.serial.observe" => Some(Self::DeviceSerialObserve),
            "host.device.serial.open" => Some(Self::DeviceSerialOpen),
            "host.device.serial.read" => Some(Self::DeviceSerialRead),
            "host.device.serial.write" => Some(Self::DeviceSerialWrite),
            "host.device.usb.configure" => Some(Self::DeviceUsbConfigure),
            "host.device.usb.list" => Some(Self::DeviceUsbList),
            "host.device.usb.open" => Some(Self::DeviceUsbOpen),
            "host.device.usb.transfer" => Some(Self::DeviceUsbTransfer),
            "host.display.mode" => Some(Self::DisplayMode),
            "host.display.read" => Some(Self::DisplayRead),
            "host.display.window" => Some(Self::DisplayWindow),
            "host.display.window.events" => Some(Self::DisplayWindowEvents),
            "host.display.window.modal" => Some(Self::DisplayWindowModal),
            "host.display.window.parenting" => Some(Self::DisplayWindowParenting),
            "host.ffi.call" => Some(Self::FfiCall),
            "host.ffi.load" => Some(Self::FfiLoad),
            "host.ffi.pointer" => Some(Self::FfiPointer),
            "host.ffi.symbol" => Some(Self::FfiSymbol),
            "host.fs.link" => Some(Self::FsLink),
            "host.fs.lock" => Some(Self::FsLock),
            "host.fs.metadata" => Some(Self::FsMetadata),
            "host.fs.mode" => Some(Self::FsMode),
            "host.fs.mount.list" => Some(Self::FsMountList),
            "host.fs.node" => Some(Self::FsNode),
            "host.fs.open" => Some(Self::FsOpen),
            "host.fs.owner" => Some(Self::FsOwner),
            "host.fs.read" => Some(Self::FsRead),
            "host.fs.sync" => Some(Self::FsSync),
            "host.fs.temp" => Some(Self::FsTemp),
            "host.fs.watch" => Some(Self::FsWatch),
            "host.fs.write" => Some(Self::FsWrite),
            "host.fs.xattr" => Some(Self::FsXattr),
            "host.gpu.adapter" => Some(Self::GpuAdapter),
            "host.gpu.bind" => Some(Self::GpuBind),
            "host.gpu.compute" => Some(Self::GpuCompute),
            "host.gpu.debug" => Some(Self::GpuDebug),
            "host.gpu.device" => Some(Self::GpuDevice),
            "host.gpu.memory" => Some(Self::GpuMemory),
            "host.gpu.present" => Some(Self::GpuPresent),
            "host.gpu.queue" => Some(Self::GpuQueue),
            "host.gpu.render" => Some(Self::GpuRender),
            "host.gpu.render.multi.draw" => Some(Self::GpuRenderMultiDraw),
            "host.gpu.render.multi.draw.count" => Some(Self::GpuRenderMultiDrawCount),
            "host.gpu.shader" => Some(Self::GpuShader),
            "host.gpu.surface" => Some(Self::GpuSurface),
            "host.gpu.sync" => Some(Self::GpuSync),
            "host.gpu.sync.pipeline.statistics" => Some(Self::GpuSyncPipelineStatistics),
            "host.input.clipboard.read" => Some(Self::InputClipboardRead),
            "host.input.clipboard.write" => Some(Self::InputClipboardWrite),
            "host.input.control" => Some(Self::InputControl),
            "host.input.grab" => Some(Self::InputGrab),
            "host.input.haptics" => Some(Self::InputHaptics),
            "host.input.read" => Some(Self::InputRead),
            "host.input.text" => Some(Self::InputText),
            "host.input.write" => Some(Self::InputWrite),
            "host.io.completion" => Some(Self::IoCompletion),
            "host.io.control" => Some(Self::IoControl),
            "host.io.poll" => Some(Self::IoPoll),
            "host.io.uring" => Some(Self::IoUring),
            "host.ipc.eventCounter" => Some(Self::IpcEventCounter),
            "host.ipc.futex" => Some(Self::IpcFutex),
            "host.ipc.handle.pass" => Some(Self::IpcHandlePass),
            "host.ipc.memory" => Some(Self::IpcMemory),
            "host.ipc.message" => Some(Self::IpcMessage),
            "host.ipc.pipe" => Some(Self::IpcPipe),
            "host.ipc.semaphore" => Some(Self::IpcSemaphore),
            "host.ipc.unix" => Some(Self::IpcUnix),
            "host.memory.advise" => Some(Self::MemoryAdvise),
            "host.memory.discard" => Some(Self::MemoryDiscard),
            "host.memory.execute" => Some(Self::MemoryExecute),
            "host.memory.layout.read" => Some(Self::MemoryLayoutRead),
            "host.memory.lock" => Some(Self::MemoryLock),
            "host.memory.map" => Some(Self::MemoryMap),
            "host.memory.virtual" => Some(Self::MemoryVirtual),
            "host.net.accept" => Some(Self::NetAccept),
            "host.net.close" => Some(Self::NetClose),
            "host.net.connect" => Some(Self::NetConnect),
            "host.net.connectivity.read" => Some(Self::NetConnectivityRead),
            "host.net.connectivity.watch" => Some(Self::NetConnectivityWatch),
            "host.net.dns" => Some(Self::NetDns),
            "host.net.interface" => Some(Self::NetInterface),
            "host.net.listen" => Some(Self::NetListen),
            "host.net.multicast" => Some(Self::NetMulticast),
            "host.net.open" => Some(Self::NetOpen),
            "host.net.option" => Some(Self::NetOption),
            "host.net.raw" => Some(Self::NetRaw),
            "host.net.read" => Some(Self::NetRead),
            "host.net.route.read" => Some(Self::NetRouteRead),
            "host.net.route.write" => Some(Self::NetRouteWrite),
            "host.net.status" => Some(Self::NetStatus),
            "host.net.write" => Some(Self::NetWrite),
            "host.os.capacity.read" => Some(Self::OsCapacityRead),
            "host.os.identity.read" => Some(Self::OsIdentityRead),
            "host.os.power.read" => Some(Self::OsPowerRead),
            "host.os.pressure.read" => Some(Self::OsPressureRead),
            "host.process.affinity" => Some(Self::ProcessAffinity),
            "host.process.arguments" => Some(Self::ProcessArguments),
            "host.process.cgroup.read" => Some(Self::ProcessCgroupRead),
            "host.process.cgroup.write" => Some(Self::ProcessCgroupWrite),
            "host.process.environment.read" => Some(Self::ProcessEnvironmentRead),
            "host.process.environment.write" => Some(Self::ProcessEnvironmentWrite),
            "host.process.exec" => Some(Self::ProcessExec),
            "host.process.exit" => Some(Self::ProcessExit),
            "host.process.handle" => Some(Self::ProcessHandle),
            "host.process.identity.read" => Some(Self::ProcessIdentityRead),
            "host.process.identity.write" => Some(Self::ProcessIdentityWrite),
            "host.process.job" => Some(Self::ProcessJob),
            "host.process.limit.read" => Some(Self::ProcessLimitRead),
            "host.process.limit.write" => Some(Self::ProcessLimitWrite),
            "host.process.namespace" => Some(Self::ProcessNamespace),
            "host.process.priority" => Some(Self::ProcessPriority),
            "host.process.session" => Some(Self::ProcessSession),
            "host.process.signal.receive" => Some(Self::ProcessSignalReceive),
            "host.process.signal.send" => Some(Self::ProcessSignalSend),
            "host.process.spawn" => Some(Self::ProcessSpawn),
            "host.process.stdio" => Some(Self::ProcessStdio),
            "host.process.umask" => Some(Self::ProcessUmask),
            "host.process.wait" => Some(Self::ProcessWait),
            "host.process.workdir.read" => Some(Self::ProcessWorkdirRead),
            "host.process.workdir.write" => Some(Self::ProcessWorkdirWrite),
            "host.random.secure" => Some(Self::RandomSecure),
            "host.time.monotonic.read" => Some(Self::TimeMonotonicRead),
            "host.time.timer" => Some(Self::TimeTimer),
            "host.time.timerfd" => Some(Self::TimeTimerfd),
            "host.time.wall.read" => Some(Self::TimeWallRead),
            "host.time.wall.sleep" => Some(Self::TimeWallSleep),
            "host.tls.certificate.read" => Some(Self::TlsCertificateRead),
            "host.tls.connection" => Some(Self::TlsConnection),
            "host.tls.context" => Some(Self::TlsContext),
            "host.tls.exporter" => Some(Self::TlsExporter),
            "host.tls.handshake" => Some(Self::TlsHandshake),
            "host.tls.hostname.verify" => Some(Self::TlsHostnameVerify),
            "host.tls.identity.use" => Some(Self::TlsIdentityUse),
            "host.tls.identity.write" => Some(Self::TlsIdentityWrite),
            "host.tls.policy" => Some(Self::TlsPolicy),
            "host.tls.resumption" => Some(Self::TlsResumption),
            "host.tls.trust.write" => Some(Self::TlsTrustWrite),
            "host.tty.close" => Some(Self::TtyClose),
            "host.tty.mode" => Some(Self::TtyMode),
            "host.tty.open" => Some(Self::TtyOpen),
            "host.tty.pty" => Some(Self::TtyPty),
            "host.tty.pty.read" => Some(Self::TtyPtyRead),
            "host.tty.pty.write" => Some(Self::TtyPtyWrite),
            "host.tty.read" => Some(Self::TtyRead),
            "host.tty.size" => Some(Self::TtySize),
            "host.tty.status" => Some(Self::TtyStatus),
            "host.tty.termios" => Some(Self::TtyTermios),
            "host.tty.write" => Some(Self::TtyWrite),
            "runtime.debug.inspect" => Some(Self::DebugInspect),
            "runtime.debug.profile" => Some(Self::DebugProfile),
            "runtime.debug.trace" => Some(Self::DebugTrace),
            "runtime.diagnostic.read" => Some(Self::DiagnosticRead),
            "runtime.inspect.read" => Some(Self::InspectRead),
            "runtime.lineage.control" => Some(Self::LineageControl),
            "runtime.lineage.read" => Some(Self::LineageRead),
            "runtime.observation.read" => Some(Self::ObservationRead),
            "runtime.random.deterministic" => Some(Self::RandomDeterministic),
            "runtime.resource.close" => Some(Self::ResourceClose),
            "runtime.resource.manage" => Some(Self::ResourceManage),
            "runtime.resource.read" => Some(Self::ResourceRead),
            "runtime.resource.transfer" => Some(Self::ResourceTransfer),
            "runtime.resource.write" => Some(Self::ResourceWrite),
            "runtime.runtime.control" => Some(Self::RuntimeControl),
            "runtime.runtime.create" => Some(Self::RuntimeCreate),
            "runtime.runtime.read" => Some(Self::RuntimeRead),
            "runtime.security.filter" => Some(Self::SecurityFilter),
            "runtime.security.permission.read" => Some(Self::SecurityPermissionRead),
            "runtime.security.permission.request" => Some(Self::SecurityPermissionRequest),
            "runtime.security.policy.read" => Some(Self::SecurityPolicyRead),
            "runtime.security.policy.write" => Some(Self::SecurityPolicyWrite),
            "runtime.security.restrict" => Some(Self::SecurityRestrict),
            "runtime.security.sandbox" => Some(Self::SecuritySandbox),
            "runtime.snapshot.create" => Some(Self::SnapshotCreate),
            "runtime.snapshot.read" => Some(Self::SnapshotRead),
            "runtime.snapshot.restore" => Some(Self::SnapshotRestore),
            "runtime.trace.control" => Some(Self::TraceControl),
            "runtime.trace.read" => Some(Self::TraceRead),
            "runtime.worker.control" => Some(Self::WorkerControl),
            "runtime.worker.create" => Some(Self::WorkerCreate),
            "runtime.worker.read" => Some(Self::WorkerRead),
            "runtime.world.control" => Some(Self::WorldControl),
            "runtime.world.create" => Some(Self::WorldCreate),
            "runtime.world.read" => Some(Self::WorldRead),
            _ => None,
        }
    }
}

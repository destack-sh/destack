use std::path::Path;

use destack_vm as vm;

use super::{
    FsDirent, FsHarnessContext, FsMapping, FsPathRef, FsWatchEvent, array_string_native,
    array_string_vm, array_u8_native, array_u8_vm, decode_dirent_vm, decode_watch_event_vm,
    native_slice, native_slice_mut, path_bytes_vec, path_ref_bytes_native, path_ref_bytes_vm,
    path_ref_string_native, path_ref_string_vm, path_utf16_vec,
    socket_address_native_from_host_port, socket_address_vm_from_host_port, tcp_protocol,
    tcp_stream_socket_type,
};
use crate::diagnostic::{DiagnosticId, RuntimeError, RuntimeResult, RuntimeStatus};
use crate::platform::abi::{NativeAbi, NativeSlice, NativeStringRef};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    self as platform_fs, Dirent, DirentNext, DirentNextVm, DirentVm, OpenOptions, OpenOptionsVm,
    OsPathVm, PathBytesAbi, PathUtf16Abi, WatchBatch, WatchBatchVm, WatchEvent, WatchEventVm,
    WatchOptions, core as core_fs,
};
use crate::platform::net::{self as core_net, AcceptFlags, SocketFamily, vm as platform_net_vm};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice};
use crate::tests::platform::{is_privileged_test_mode, vm_test_values};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

type ByteSlicesValue = HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>;

impl<'call> FsHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::BindingContext<'_>> {
        // safety: the harness guarantees the VM context pointer is valid for the callback
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
    }

    /// Build a byte path reference for this context.
    pub(crate) fn path_bytes(&mut self, path: &Path) -> FsPathRef {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = path_bytes_vec(path);
                let bytes = VmArray::from_bytes(&mut context.write(), &bytes)
                    .expect("vm test byte array should allocate");
                let kind = vm::StringHandle::new(
                    context
                        .intern_string("bytes")
                        .expect("vm test string should intern"),
                );
                let path = OsPathVm::OsPathBytes(platform_fs::OsPathBytesVm {
                    kind,
                    bytes: PathBytesAbi(bytes),
                });

                self.harness_value_vm(path)
            }
            None => {
                let bytes = path_bytes_vec(path);
                let bytes = self.call_context.store_array(bytes);
                let path = core_fs::path_ref_from_bytes(PathBytesAbi::<NativeAbi>(bytes));

                self.harness_value(path)
            }
        }
    }

    /// Build a UTF-16 path reference for this context.
    #[allow(dead_code)]
    pub(crate) fn path_utf16(&mut self, path: &Path) -> FsPathRef {
        match self.vm_context_mut() {
            Some(context) => {
                let utf16_units = path_utf16_vec(path);
                let utf16 = VmArray::from_values(&mut context.write(), &utf16_units)
                    .expect("vm utf16 path should encode");
                let kind = vm::StringHandle::new(
                    context
                        .intern_string("utf16")
                        .expect("vm test string should intern"),
                );
                let path = OsPathVm::OsPathUtf16(platform_fs::OsPathUtf16Vm {
                    kind,
                    utf16: PathUtf16Abi(utf16),
                });

                self.harness_value_vm(path)
            }
            None => {
                let utf16_units = path_utf16_vec(path);
                let utf16 = self.call_context.store_array(utf16_units);
                let path = core_fs::path_ref_from_utf16(PathUtf16Abi::<NativeAbi>(utf16));

                self.harness_value(path)
            }
        }
    }

    /// Render a path reference into a displayable string.
    #[allow(dead_code)]
    pub(crate) fn path_ref_string(&mut self, path: FsPathRef) -> String {
        match path {
            HarnessValue::Native(path) => path_ref_string_native(path),
            HarnessValue::Vm(path) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm path ref"),
                path,
            )
            .unwrap_or_else(|error| panic!("failed to decode vm path ref: {}", error.message())),
        }
    }

    /// Read raw bytes from a path reference.
    pub(crate) fn path_ref_bytes(&mut self, path: FsPathRef) -> Vec<u8> {
        match path {
            HarnessValue::Native(path) => path_ref_bytes_native(path),
            HarnessValue::Vm(path) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm path ref");
                path_ref_bytes_vm(context, path)
                    .unwrap_or_else(|error| panic!("failed to decode vm path bytes: {error:?}"))
            }
        }
    }

    /// Decode one generated directory-entry payload into unified test values.
    pub(crate) fn dirents_from_value(
        &mut self,
        entries: HarnessValue<NativeArray<Dirent>, VmArray<DirentVm>>,
    ) -> RuntimeResult<Vec<FsDirent>> {
        match entries {
            HarnessValue::Native(entries) => {
                let entries = unsafe { entries.as_slice()? };
                let mut decoded = Vec::with_capacity(entries.len());
                for entry in entries {
                    decoded.push(FsDirent::Native(*entry));
                }
                Ok(decoded)
            }
            HarnessValue::Vm(entries) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm dirents");
                let raw = entries.values(&context.read())?;
                let mut decoded = Vec::with_capacity(raw.len());
                for value in raw {
                    let dirent = decode_dirent_vm(context, value)?;
                    decoded.push(FsDirent::Vm(dirent));
                }
                Ok(decoded)
            }
        }
    }

    /// Decode one generated single-entry directory-step payload.
    pub(crate) fn dirent_next_from_value(
        &mut self,
        value: HarnessValue<DirentNext, DirentNextVm>,
    ) -> RuntimeResult<Option<FsDirent>> {
        match value {
            HarnessValue::Native(DirentNext::DirentNextEnd(_)) => Ok(None),
            HarnessValue::Native(DirentNext::DirentNextEntry(value)) => {
                Ok(Some(FsDirent::Native(value.entry)))
            }
            HarnessValue::Vm(DirentNextVm::DirentNextEnd(_)) => Ok(None),
            HarnessValue::Vm(DirentNextVm::DirentNextEntry(value)) => {
                Ok(Some(FsDirent::Vm(value.entry)))
            }
        }
    }

    /// Convert a directory entry name to a string.
    pub(crate) fn dirent_name(&mut self, entry: FsDirent) -> String {
        match entry {
            FsDirent::Native(entry) => path_ref_string_native(entry.name),
            FsDirent::Vm(entry) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm dirent"),
                entry.name,
            )
            .unwrap_or_else(|error| panic!("failed to decode vm dirent name: {}", error.message())),
        }
    }

    /// Build one backend-specific open-options value.
    pub(crate) fn open_options_value(
        &self,
        options: OpenOptions,
    ) -> HarnessValue<OpenOptions, OpenOptionsVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(OpenOptionsVm {
                flags: options.flags,
                mode: options.mode,
                resolve: options.resolve,
            }),
            None => self.harness_value(options),
        }
    }

    /// Build one backend-specific watch-options value.
    pub(crate) fn watch_options_value(
        &self,
        options: WatchOptions,
    ) -> HarnessValue<WatchOptions, platform_fs::WatchOptionsVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(platform_fs::WatchOptionsVm {
                mask: options.mask,
                recursive: options.recursive,
                follow_symlinks: options.follow_symlinks,
            }),
            None => self.harness_value(options),
        }
    }

    /// Decode one generated watch-batch payload into unified test values.
    pub(crate) fn watch_batch_from_value(
        &mut self,
        batch: HarnessValue<WatchBatch, WatchBatchVm>,
    ) -> RuntimeResult<(Vec<FsWatchEvent>, bool)> {
        match batch {
            HarnessValue::Native(batch) => {
                let events = unsafe { batch.events.as_slice()? };
                let mut decoded = Vec::with_capacity(events.len());
                for event in events {
                    decoded.push(FsWatchEvent::Native(*event));
                }

                Ok((decoded, batch.overflowed))
            }
            HarnessValue::Vm(batch) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm watch batch");
                let raw = batch.events.values(&context.read())?;
                let mut decoded = Vec::with_capacity(raw.len());
                for value in raw {
                    let event = decode_watch_event_vm(context, value)?;
                    decoded.push(FsWatchEvent::Vm(event));
                }

                Ok((decoded, batch.overflowed))
            }
        }
    }

    /// Return one normalized watch-event kind name.
    pub(crate) fn watch_event_kind_name(&self, event: FsWatchEvent) -> &'static str {
        match event {
            FsWatchEvent::Native(WatchEvent::WatchCreateEvent(_)) => "create",
            FsWatchEvent::Native(WatchEvent::WatchRemoveEvent(_)) => "remove",
            FsWatchEvent::Native(WatchEvent::WatchModifyEvent(_)) => "modify",
            FsWatchEvent::Native(WatchEvent::WatchRenameEvent(_)) => "rename",
            FsWatchEvent::Native(WatchEvent::WatchMetadataEvent(_)) => "metadata",
            FsWatchEvent::Native(WatchEvent::WatchOverflowEvent(_)) => "overflow",
            FsWatchEvent::Vm(WatchEventVm::WatchCreateEvent(_)) => "create",
            FsWatchEvent::Vm(WatchEventVm::WatchRemoveEvent(_)) => "remove",
            FsWatchEvent::Vm(WatchEventVm::WatchModifyEvent(_)) => "modify",
            FsWatchEvent::Vm(WatchEventVm::WatchRenameEvent(_)) => "rename",
            FsWatchEvent::Vm(WatchEventVm::WatchMetadataEvent(_)) => "metadata",
            FsWatchEvent::Vm(WatchEventVm::WatchOverflowEvent(_)) => "overflow",
        }
    }

    /// Convert one watch-event path into a displayable string.
    pub(crate) fn watch_event_path(&mut self, event: FsWatchEvent) -> String {
        match event {
            FsWatchEvent::Native(WatchEvent::WatchCreateEvent(event)) => {
                path_ref_string_native(event.path)
            }
            FsWatchEvent::Native(WatchEvent::WatchMetadataEvent(event)) => {
                path_ref_string_native(event.path)
            }
            FsWatchEvent::Native(WatchEvent::WatchModifyEvent(event)) => {
                path_ref_string_native(event.path)
            }
            FsWatchEvent::Native(WatchEvent::WatchRemoveEvent(event)) => {
                path_ref_string_native(event.path)
            }
            FsWatchEvent::Native(WatchEvent::WatchRenameEvent(event)) => {
                path_ref_string_native(event.path)
            }
            FsWatchEvent::Native(WatchEvent::WatchOverflowEvent(_)) => String::new(),
            FsWatchEvent::Vm(WatchEventVm::WatchCreateEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.path,
            )
            .unwrap_or_else(|error| {
                panic!("failed to decode vm watch-event path: {}", error.message())
            }),
            FsWatchEvent::Vm(WatchEventVm::WatchMetadataEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.path,
            )
            .unwrap_or_else(|error| {
                panic!("failed to decode vm watch-event path: {}", error.message())
            }),
            FsWatchEvent::Vm(WatchEventVm::WatchModifyEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.path,
            )
            .unwrap_or_else(|error| {
                panic!("failed to decode vm watch-event path: {}", error.message())
            }),
            FsWatchEvent::Vm(WatchEventVm::WatchRemoveEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.path,
            )
            .unwrap_or_else(|error| {
                panic!("failed to decode vm watch-event path: {}", error.message())
            }),
            FsWatchEvent::Vm(WatchEventVm::WatchRenameEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.path,
            )
            .unwrap_or_else(|error| {
                panic!("failed to decode vm watch-event path: {}", error.message())
            }),
            FsWatchEvent::Vm(WatchEventVm::WatchOverflowEvent(_)) => String::new(),
        }
    }

    /// Convert one watch-event related path into a displayable string when present.
    pub(crate) fn watch_event_related_path(&mut self, event: FsWatchEvent) -> Option<String> {
        let related_path = match event {
            FsWatchEvent::Native(WatchEvent::WatchRenameEvent(event)) => {
                path_ref_string_native(event.related_path)
            }
            FsWatchEvent::Vm(WatchEventVm::WatchRenameEvent(event)) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm watch event"),
                event.related_path,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "failed to decode vm watch-event related path: {}",
                    error.message()
                )
            }),
            _ => return None,
        };

        if related_path.is_empty() {
            return None;
        }

        Some(related_path)
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let value = vm::StringHandle::new(
                    context
                        .intern_string(value)
                        .expect("vm test string should intern"),
                );
                self.harness_value_vm(value)
            }
            None => self.harness_value(self.call_context.store_string(value)),
        }
    }

    /// Build one backend-specific byte-slice value.
    pub(crate) fn bytes_slice_value(
        &self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        match self.vm_context_mut() {
            Some(context) => Ok(self.harness_value_vm(
                VmSlice::from_bytes(&mut context.write(), bytes)
                    .expect("vm test byte slice should allocate"),
            )),
            None => Ok(self.harness_value(self.call_context.store_slice(bytes.to_vec()))),
        }
    }

    /// Build one backend-specific zeroed byte-slice value.
    pub(crate) fn zeroed_bytes_slice_value(
        &self,
        len: usize,
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        self.bytes_slice_value(&vec![0_u8; len])
    }

    /// Duplicate one harness value when both variants are copyable.
    pub(crate) fn duplicate_value<Native: Copy, Vm: Copy>(
        &self,
        value: HarnessValue<Native, Vm>,
    ) -> (HarnessValue<Native, Vm>, HarnessValue<Native, Vm>) {
        match value {
            HarnessValue::Native(value) => (self.harness_value(value), self.harness_value(value)),
            HarnessValue::Vm(value) => (self.harness_value_vm(value), self.harness_value_vm(value)),
        }
    }

    /// Decode one backend-specific byte-slice value into bytes.
    pub(crate) fn bytes_from_slice_value(
        &self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm byte-slice value");
                value.read_bytes(&context.read())
            }
        }
    }

    /// Decode one backend-specific byte-slice value prefix into bytes.
    pub(crate) fn bytes_prefix_from_slice_value(
        &self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
        len: usize,
    ) -> RuntimeResult<Vec<u8>> {
        let mut bytes = self.bytes_from_slice_value(value)?;
        bytes.truncate(len);
        Ok(bytes)
    }

    /// Build one backend-specific nested byte-slice value.
    pub(crate) fn bytes_slices_value(&self, buffers: &[&[u8]]) -> RuntimeResult<ByteSlicesValue> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| {
                        VmSlice::from_bytes(&mut context.write(), buffer)
                            .expect("vm test byte slice should allocate")
                    })
                    .collect::<Vec<_>>();
                let values = vm_slice_of_slices(context, &vm_buffers);
                Ok(self.harness_value_vm(values))
            }
            None => {
                let native_buffers = buffers
                    .iter()
                    .map(|buffer| native_slice(buffer))
                    .collect::<Vec<_>>();
                let values = self.call_context.store_slice(native_buffers);
                Ok(self.harness_value(values))
            }
        }
    }

    /// Build one backend-specific mutable nested byte-slice value.
    pub(crate) fn mutable_bytes_slices_value(
        &self,
        buffers: &mut [Vec<u8>],
    ) -> RuntimeResult<ByteSlicesValue> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| {
                        VmSlice::from_bytes(&mut context.write(), &vec![0_u8; buffer.len()])
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;
                let values = vm_slice_of_slices(context, &vm_buffers);

                Ok(self.harness_value_vm(values))
            }
            None => {
                let native_buffers = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let values = self.call_context.store_slice(native_buffers);
                Ok(self.harness_value(values))
            }
        }
    }

    /// Decode one backend-specific nested byte-slice value into byte vectors.
    pub(crate) fn bytes_slices_from_value(
        &self,
        value: HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>,
    ) -> RuntimeResult<Vec<Vec<u8>>> {
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                let mut buffers = Vec::with_capacity(value.len());
                for buffer in value {
                    let bytes = unsafe { (*buffer).as_slice()? };
                    buffers.push(bytes.to_vec());
                }
                Ok(buffers)
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm nested byte-slice value");
                let raw_values = value.values(&context.read())?;
                let mut buffers = Vec::with_capacity(raw_values.len());
                for raw in raw_values {
                    let buffer = VmSlice::<u8>::from_value(
                        &context.read(),
                        raw,
                        "buffers",
                        "VmSlice<VmSlice<u8>>",
                    )?;
                    buffers.push(buffer.read_bytes(&context.read())?);
                }
                Ok(buffers)
            }
        }
    }

    /// Decode one backend-specific byte-array value into bytes.
    pub(crate) fn bytes_from_array_value(
        &self,
        value: HarnessValue<NativeArray<u8>, VmArray<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        match value {
            HarnessValue::Native(value) => array_u8_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm byte-array value");
                array_u8_vm(context, value)
            }
        }
    }

    /// Decode one backend-specific string-array value into UTF-8 strings.
    pub(crate) fn string_list_from_value(
        &self,
        value: HarnessValue<NativeArray<NativeStringRef>, VmArray<vm::StringHandle>>,
    ) -> RuntimeResult<Vec<String>> {
        match value {
            HarnessValue::Native(value) => array_string_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm string-array value");
                array_string_vm(context, value)
            }
        }
    }

    /// Decode one backend-specific nested byte-array value into byte vectors.
    pub(crate) fn bytes_array_list_from_value(
        &self,
        value: HarnessValue<NativeArray<NativeArray<u8>>, VmArray<VmArray<u8>>>,
    ) -> RuntimeResult<Vec<Vec<u8>>> {
        match value {
            HarnessValue::Native(value) => {
                let arrays = unsafe { value.as_slice()? };
                let mut decoded = Vec::with_capacity(arrays.len());
                for array in arrays {
                    let bytes = unsafe { array.as_slice()? };
                    decoded.push(bytes.to_vec());
                }

                Ok(decoded)
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm byte-array list value");
                let arrays = value.read_values(&context.read())?;
                let mut decoded = Vec::with_capacity(arrays.len());
                for array in arrays {
                    decoded.push(array.read_bytes(&context.read())?);
                }

                Ok(decoded)
            }
        }
    }

    /// Convert one generated mapping value into one unified mapping payload.
    pub(crate) fn mapping_from_value(
        &self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
    ) -> FsMapping {
        match value {
            HarnessValue::Native(value) => FsMapping::Native(value),
            HarnessValue::Vm(value) => FsMapping::Vm(value),
        }
    }

    /// Convert one unified mapping payload into one generated mapping value.
    pub(crate) fn mapping_value(
        &self,
        mapping: FsMapping,
    ) -> HarnessValue<NativeSlice<u8>, VmSlice<u8>> {
        match mapping {
            FsMapping::Native(mapping) => self.harness_value(mapping),
            FsMapping::Vm(mapping) => self.harness_value_vm(mapping),
        }
    }

    /// Convert a native status into a runtime result.
    pub(crate) fn status_result(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        // fast path: success
        if status == RuntimeStatus::OK {
            return Ok(());
        }

        // decode status-only errors when no runtime id is attached
        if status.error_id == 0 {
            if status.code == PlatformErrorCode::NotSupported.number().saturating_add(1) {
                return Err(RuntimeError::from(PlatformError::not_supported(label)).boxed());
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "{label} failed without runtime error id",
            )))
            .boxed());
        }

        // load the captured runtime error
        let error = self
            .call_context
            .diagnostics()
            .take_error(DiagnosticId::from_raw(status.error_id))
            .unwrap_or_else(|| {
                RuntimeError::from(PlatformError::io(format!(
                    "{label} failed with missing runtime error",
                )))
                .boxed()
            });

        Err(error)
    }

    /// Convert a native status into a test-friendly result.
    pub(crate) fn status_ok(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.status_result(status, label)
    }

    /// Return true when one platform code represents permission denial.
    fn is_permission_denied_code(&self, code: PlatformErrorCode) -> bool {
        matches!(
            code,
            PlatformErrorCode::IoPermissionDenied
                | PlatformErrorCode::ProcessPermissionDenied
                | PlatformErrorCode::SecurityDenied
        )
    }

    /// Convert a VM result into Ok, or treat allowed platform codes as acceptable failures.
    pub(crate) fn result_ok_or_codes<T>(
        &self,
        result: RuntimeResult<T>,
        _label: &str,
        allowed: &[PlatformErrorCode],
    ) -> RuntimeResult<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                if let Some(platform) = error.platform_error() {
                    let code = platform.code;
                    if allowed.contains(&code) {
                        if is_privileged_test_mode() && self.is_permission_denied_code(code) {
                            panic!(
                                "permission-denied error {code:?} is not allowed when DESTACK_TEST_PRIVILEGED=1 (allowed {allowed:?})",
                            );
                        }

                        return Ok(None);
                    }
                }

                Err(error)
            }
        }
    }

    /// Read the port assigned to a listener handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        super::listener_port(self.call_context.worker(), handle)
    }

    /// Start listening on the given address.
    #[cfg(any(unix, windows))]
    pub(crate) fn listen(
        &mut self,
        host: &str,
        port: u16,
        backlog: u32,
    ) -> RuntimeResult<ListenerHandle> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;

                platform_net_vm::destack_net_listen(self.call_context, context, address, backlog)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;

                let mut handle = ListenerHandle(ResourceId::local(0));
                let status = unsafe {
                    core_net::destack_net_listener_listen(&mut handle, address.address(), backlog)
                };
                self.status_ok(status, "listen")?;

                Ok(handle)
            }
        }
    }

    /// Accept a new connection from a listener.
    #[cfg(any(unix, windows))]
    pub(crate) fn accept(&mut self, listener: ListenerHandle) -> RuntimeResult<SocketHandle> {
        let flags = AcceptFlags(0);
        match self.vm_context_mut() {
            Some(context) => {
                platform_net_vm::destack_net_accept(self.call_context, context, listener, flags)
            }
            None => {
                let mut handle = SocketHandle(ResourceId::local(0));
                let status =
                    unsafe { core_net::destack_net_listener_accept(&mut handle, listener, flags) };
                self.status_ok(status, "accept")?;

                Ok(handle)
            }
        }
    }

    /// Connect to a remote host and return a socket handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn connect(&mut self, host: &str, port: u16) -> RuntimeResult<SocketHandle> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };
        let socket_type = tcp_stream_socket_type();
        let protocol = tcp_protocol();

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;
                let handle = platform_net_vm::destack_net_socket(
                    self.call_context,
                    context,
                    family,
                    socket_type,
                    protocol,
                )?;
                platform_net_vm::destack_net_connect(self.call_context, context, handle, address)?;

                Ok(handle)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let mut handle = SocketHandle(ResourceId::local(0));
                let socket_status = unsafe {
                    core_net::destack_net_socket_open(&mut handle, family, socket_type, protocol)
                };
                self.status_ok(socket_status, "connect socket")?;

                let connect_status =
                    unsafe { core_net::destack_net_socket_connect(handle, address.address()) };
                self.status_ok(connect_status, "connect")?;

                Ok(handle)
            }
        }
    }

    /// Close a listener handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn close_listener(&mut self, handle: ListenerHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_net_vm::destack_net_close_listener(self.call_context, context, handle)
            }
            None => {
                let status = unsafe { core_net::destack_net_listener_close_listener(handle) };
                self.status_ok(status, "close_listener")
            }
        }
    }

    /// Close a socket handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn close_socket(&mut self, handle: SocketHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_net_vm::destack_net_close(self.call_context, context, handle),
            None => {
                let status = unsafe { core_net::destack_net_socket_close(handle) };
                self.status_ok(status, "close_socket")
            }
        }
    }

    /// Read from a socket into the provided buffer.
    #[cfg(any(unix, windows))]
    pub(crate) fn socket_read(
        &mut self,
        handle: SocketHandle,
        buffer: &mut [u8],
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_slice =
                    VmSlice::from_bytes(&mut context.write(), &vec![0_u8; buffer.len()])?;
                let bytes = platform_net_vm::destack_net_read(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                )?;

                let read_bytes = vm_slice.read_bytes(&context.read())?;
                let count = bytes as usize;
                if count > buffer.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "buffer",
                        "read size exceeded buffer length",
                    ))
                    .boxed());
                }
                buffer[..count].copy_from_slice(&read_bytes[..count]);

                Ok(bytes)
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = 0_u64;
                let status = unsafe { core_net::destack_net_socket_read(&mut out, handle, slice) };
                self.status_ok(status, "socket_read")?;

                Ok(out)
            }
        }
    }

    /// Build a connected socket pair.
    #[cfg(any(unix, windows))]
    pub(crate) fn tcp_pair(&mut self) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        let listener = self.listen("127.0.local_id.1", 0, 1)?;
        let port = self.listener_port(listener);
        let client = self.connect("127.0.local_id.1", port)?;
        let server = self.accept(listener)?;
        self.close_listener(listener)?;

        Ok((server, client))
    }

    /// Write bytes into a mapping for the current harness.
    pub(crate) fn write_mapping(&mut self, mapping: &FsMapping, bytes: &[u8]) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                if bytes.len() > mapping.len as usize {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "mapping",
                        "write exceeds mapping length",
                    ))
                    .boxed());
                }

                // vm slices require whole-slice writes, so preserve the tail
                let mut vm_bytes = mapping.read_bytes(&context.read())?;
                vm_bytes[..bytes.len()].copy_from_slice(bytes);
                mapping.write_bytes(&mut context.write(), &vm_bytes)?;

                Ok(())
            }
            (None, FsMapping::Native(mapping)) => {
                let slice = unsafe { mapping.as_mut_slice()? };
                if bytes.len() > slice.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "mapping",
                        "write exceeds mapping length",
                    ))
                    .boxed());
                }
                slice[..bytes.len()].copy_from_slice(bytes);
                Ok(())
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Read bytes from a mapping for the current harness.
    pub(crate) fn read_mapping(
        &mut self,
        mapping: &FsMapping,
        length: usize,
    ) -> RuntimeResult<Vec<u8>> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                let bytes = mapping.read_bytes(&context.read())?;
                Ok(bytes.into_iter().take(length).collect())
            }
            (None, FsMapping::Native(mapping)) => {
                let slice = unsafe { mapping.as_slice()? };
                Ok(slice.iter().copied().take(length).collect())
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }
}

impl<Native: Copy, Vm: Copy> Copy for HarnessValue<Native, Vm> {}

impl<T: std::fmt::Debug> std::fmt::Debug for HarnessValue<T, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessValue::Native(value) => value.fmt(formatter),
            HarnessValue::Vm(value) => value.fmt(formatter),
        }
    }
}

impl<T: PartialEq> PartialEq<T> for HarnessValue<T, T> {
    fn eq(&self, other: &T) -> bool {
        match self {
            HarnessValue::Native(value) => value.eq(other),
            HarnessValue::Vm(value) => value.eq(other),
        }
    }
}

impl<T> std::ops::Deref for HarnessValue<T, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }
}

/// Build one VM nested slice from one list of VM byte slices.
fn vm_slice_of_slices(
    context: &mut vm::BindingContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .copied()
        .map(|slice| slice.to_value(&mut context.write()))
        .collect::<RuntimeResult<Vec<_>>>()
        .expect("vm test slice values should encode");
    let data = vm_test_values(context, values);

    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData,
    }
}

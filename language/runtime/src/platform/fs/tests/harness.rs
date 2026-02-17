use super::*;

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> FsHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        // safety: the harness guarantees the VM context pointer is valid for the callback
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build a byte path reference for this context.
    pub(crate) fn path_bytes(&mut self, path: &Path) -> FsPathRef {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = path_bytes_vec(path);
                let bytes = VmArray::from_bytes(context, &bytes);
                let path = OsPathVm {
                    encoding: PathEncoding::Bytes,
                    bytes: PathBytesAbi(bytes),
                    utf16: PathUtf16Abi(empty_vm_array()),
                };

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
                let utf16 = VmArray::from_values(context, &utf16_units)
                    .expect("vm utf16 path should encode");
                let path = OsPathVm {
                    encoding: PathEncoding::Utf16,
                    bytes: PathBytesAbi(empty_vm_array()),
                    utf16: PathUtf16Abi(utf16),
                };

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
                let raw = entries.raw_values(context)?;
                let mut decoded = Vec::with_capacity(raw.len());
                for value in raw {
                    let dirent = decode_dirent_vm(context, value)?;
                    decoded.push(FsDirent::Vm(dirent));
                }
                Ok(decoded)
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

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let value = vm::StringHandle::new(context.intern_string(value));
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
            Some(context) => Ok(self.harness_value_vm(VmSlice::from_bytes(context, bytes))),
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
                value.read_bytes(context)
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
    pub(crate) fn bytes_slices_value(
        &self,
        buffers: &[&[u8]],
    ) -> RuntimeResult<HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| VmSlice::from_bytes(context, buffer))
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
    ) -> RuntimeResult<HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| VmSlice::from_bytes(context, &vec![0_u8; buffer.len()]))
                    .collect::<Vec<_>>();
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
                let raw_values = value.raw_values(context)?;
                let mut buffers = Vec::with_capacity(raw_values.len());
                for raw in raw_values {
                    let buffer =
                        VmSlice::<u8>::from_value(context, raw, "buffers", "VmSlice<VmSlice<u8>>")?;
                    buffers.push(buffer.read_bytes(context)?);
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
            .runtime
            .runtime
            .context
            .errors()
            .take(RuntimeErrorId::from_raw(status.error_id))
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

    /// Return true when privileged test mode is enabled.
    fn is_privileged_test_mode(&self) -> bool {
        let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
        matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
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
                        if self.is_privileged_test_mode() && self.is_permission_denied_code(code) {
                            panic!(
                                "permission-denied error {:?} is not allowed when DESTACK_TEST_PRIVILEGED=1 (allowed {:?})",
                                code, allowed,
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
        self.runtime.listener_port(handle)
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

                let mut handle = ListenerHandle(ResourceId(0));
                let status = unsafe {
                    core_net::destack_net_listen(&mut handle, address.address(), backlog)
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
                let mut handle = SocketHandle(ResourceId(0));
                let status = unsafe { core_net::destack_net_accept(&mut handle, listener, flags) };
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
                let mut handle = SocketHandle(ResourceId(0));
                let socket_status = unsafe {
                    core_net::destack_net_socket(&mut handle, family, socket_type, protocol)
                };
                self.status_ok(socket_status, "connect socket")?;

                let connect_status =
                    unsafe { core_net::destack_net_connect(handle, address.address()) };
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
                let status = unsafe { core_net::destack_net_close_listener(handle) };
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
                let status = unsafe { core_net::destack_net_close(handle) };
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
                let vm_slice = VmSlice::from_bytes(context, &vec![0_u8; buffer.len()]);
                let bytes = platform_net_vm::destack_net_read(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                )?;

                let read_bytes = vm_slice.read_bytes(context)?;
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
                let status = unsafe { core_net::destack_net_read(&mut out, handle, slice) };
                self.status_ok(status, "socket_read")?;

                Ok(out)
            }
        }
    }

    /// Build a connected socket pair.
    #[cfg(any(unix, windows))]
    pub(crate) fn tcp_pair(&mut self) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        let listener = self.listen("127.0.0.1", 0, 1)?;
        let port = self.listener_port(listener);
        let client = self.connect("127.0.0.1", port)?;
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
                mapping.write_bytes(context, bytes)?;
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
                let bytes = mapping.read_bytes(context)?;
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

impl<Native: Clone, Vm: Clone> Clone for HarnessValue<Native, Vm> {
    fn clone(&self) -> Self {
        match self {
            HarnessValue::Native(value) => HarnessValue::Native(value.clone()),
            HarnessValue::Vm(value) => HarnessValue::Vm(value.clone()),
        }
    }
}

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
    context: &mut vm::ExternalCallContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .copied()
        .map(|slice| slice.to_value(context))
        .collect::<Vec<_>>();
    let data = context.allocate_raw_values(values);

    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData,
    }
}

use super::*;
#[cfg(windows)]
use crate::diagnostic::{RuntimeErrorId, RuntimeStatus};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

type ByteSlicesValue = HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>;

impl<'call> NetHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Read the port assigned to a listener handle.
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        self.runtime.listener_port(handle)
    }

    /// Build one backend-specific UTF-8 string value.
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
    pub(crate) fn bytes_slices_value(&self, buffers: &[&[u8]]) -> RuntimeResult<ByteSlicesValue> {
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
    ) -> RuntimeResult<ByteSlicesValue> {
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

    /// Convert a native status into a runtime result.
    #[cfg(windows)]
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
    #[cfg(windows)]
    pub(crate) fn status_ok(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.status_result(status, label)
    }

    /// Write bytes to one socket handle.
    #[cfg(windows)]
    pub(crate) fn write(&mut self, handle: SocketHandle, buffer: &[u8]) -> RuntimeResult<u64> {
        let buffer = self.bytes_slice_value(buffer)?;
        self.destack_net_write(handle, buffer)
    }

    /// Read bytes from one socket handle into one mutable buffer.
    #[cfg(windows)]
    pub(crate) fn read(&mut self, handle: SocketHandle, buffer: &mut [u8]) -> RuntimeResult<u64> {
        // allocate one backend-specific mutable slice for socket reads
        let output = self.zeroed_bytes_slice_value(buffer.len())?;
        let (read_input, read_output) = self.duplicate_value(output);

        // run the read and decode the captured bytes
        let read = self.destack_net_read(handle, read_input)?;
        let read_len = usize::try_from(read).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "read",
                "read length does not fit usize",
            ))
            .boxed()
        })?;
        if read_len > buffer.len() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "read",
                "read length exceeds provided buffer length",
            ))
            .boxed());
        }

        let bytes = self.bytes_prefix_from_slice_value(read_output, read_len)?;
        buffer[..read_len].copy_from_slice(&bytes[..read_len]);

        Ok(read)
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

    /// Build one backend-specific socket-address value from host and port.
    pub(crate) fn socket_address_value(
        &self,
        host: &str,
        port: u16,
        family: SocketFamily,
    ) -> RuntimeResult<HarnessValue<SocketAddress, SocketAddressVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;
                Ok(self.harness_value_vm(address))
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let bytes = self.call_context.store_array(address.bytes().to_vec());
                let address = SocketAddress {
                    family: address.address().family,
                    length: address.address().length,
                    bytes,
                };
                Ok(self.harness_value(address))
            }
        }
    }

    /// Build one backend-specific socket-address value from host and port with inferred family.
    pub(crate) fn socket_address_value_for_host_port(
        &self,
        host: &str,
        port: u16,
    ) -> RuntimeResult<HarnessValue<SocketAddress, SocketAddressVm>> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };

        self.socket_address_value(host, port, family)
    }

    /// Decode one backend-specific socket-address value into host, port, and family.
    pub(crate) fn socket_address_from_value(
        &self,
        value: HarnessValue<SocketAddress, SocketAddressVm>,
    ) -> RuntimeResult<(String, u16, SocketFamily)> {
        let (family, bytes) = self.socket_address_raw_from_value(value)?;
        socket_address_from_raw(family, &bytes)
    }

    /// Decode one backend-specific socket-address value into raw family and bytes.
    pub(crate) fn socket_address_raw_from_value(
        &self,
        value: HarnessValue<SocketAddress, SocketAddressVm>,
    ) -> RuntimeResult<(u16, Vec<u8>)> {
        match value {
            HarnessValue::Native(value) => socket_address_raw_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm socket address value");
                socket_address_raw_vm(context, value)
            }
        }
    }

    /// Build one backend-specific keep-alive config value.
    pub(crate) fn keep_alive_config_value(
        &self,
        config: KeepAliveConfig,
    ) -> HarnessValue<KeepAliveConfig, KeepAliveConfigVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(KeepAliveConfigVm {
                enabled: config.enabled,
                idle_seconds: config.idle_seconds,
                interval_seconds: config.interval_seconds,
                probe_count: config.probe_count,
            }),
            None => self.harness_value(config),
        }
    }

    /// Build one backend-specific linger value.
    pub(crate) fn linger_value(&self, linger: Linger) -> HarnessValue<Linger, LingerVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(LingerVm {
                enabled: linger.enabled,
                seconds: linger.seconds,
            }),
            None => self.harness_value(linger),
        }
    }

    /// Build one backend-specific resolve-query value.
    pub(crate) fn resolve_query_value(
        &self,
        host: &str,
        port: u16,
        family: SocketFamily,
        flags: ResolveFlags,
    ) -> HarnessValue<ResolveQuery, platform_net::ResolveQueryVm> {
        match self.vm_context_mut() {
            Some(context) => {
                let service = port.to_string();
                self.harness_value_vm(platform_net::ResolveQueryVm {
                    has_host: true,
                    host: host_from_vm(context, host),
                    has_service: true,
                    service: host_from_vm(context, &service),
                    family,
                    flags,
                })
            }
            None => {
                let service = port.to_string();
                self.harness_value(ResolveQuery {
                    has_host: true,
                    host: NativeStringRef::from(host),
                    has_service: true,
                    service: NativeStringRef::from(service.as_str()),
                    family,
                    flags,
                })
            }
        }
    }

    /// Decode one backend-specific socket-address list value.
    pub(crate) fn socket_addresses_from_value(
        &self,
        value: HarnessValue<NativeArray<SocketAddress>, VmArray<SocketAddressVm>>,
    ) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
        match value {
            HarnessValue::Native(value) => socket_addresses_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm socket address list");
                socket_addresses_vm(context, value)
            }
        }
    }

    /// Decode one backend-specific reverse-lookup name list value.
    pub(crate) fn reverse_lookup_names_from_value(
        &self,
        value: HarnessValue<
            NativeArray<ReverseLookupName>,
            VmArray<platform_net::ReverseLookupNameVm>,
        >,
    ) -> RuntimeResult<Vec<String>> {
        match value {
            HarnessValue::Native(value) => reverse_lookup_names_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm reverse lookup names");
                reverse_lookup_names_vm(context, value)
            }
        }
    }

    /// Build one backend-specific UDS address from a path.
    pub(crate) fn uds_address_value(
        &self,
        path: &std::path::Path,
    ) -> HarnessValue<UdsAddress, platform_net::UdsAddressVm> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm(context, path);
                self.harness_value_vm(uds_path_address_vm(context, path))
            }
            None => {
                #[cfg(unix)]
                let path = {
                    use std::os::unix::ffi::OsStrExt;

                    let bytes = path.as_os_str().as_bytes().to_vec();
                    let bytes = self.call_context.store_array(bytes);
                    OsPath {
                        encoding: PathEncoding::Bytes,
                        bytes: PathBytesAbi(bytes),
                        utf16: core_fs::empty_path_utf16(),
                    }
                };

                #[cfg(windows)]
                let path = {
                    use std::os::windows::ffi::OsStrExt;

                    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
                    let units = self.call_context.store_array(units);
                    OsPath {
                        encoding: PathEncoding::Utf16,
                        bytes: core_fs::empty_path_bytes(),
                        utf16: PathUtf16Abi(units),
                    }
                };

                self.harness_value(uds_path_address_native(path))
            }
        }
    }

    /// Build one backend-specific UTF-16 UDS address from a path.
    #[cfg(unix)]
    pub(crate) fn uds_address_utf16_value(
        &self,
        path: &std::path::Path,
    ) -> HarnessValue<UdsAddress, platform_net::UdsAddressVm> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm_utf16(context, path);
                self.harness_value_vm(uds_path_address_vm(context, path))
            }
            None => {
                use std::os::unix::ffi::OsStrExt;

                let bytes = path.as_os_str().as_bytes();
                let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
                let units = text.encode_utf16().collect::<Vec<_>>();
                let units = self.call_context.store_array(units);
                let path = OsPath {
                    encoding: PathEncoding::Utf16,
                    bytes: core_fs::empty_path_bytes(),
                    utf16: PathUtf16Abi(units),
                };
                self.harness_value(uds_path_address_native(path))
            }
        }
    }

    /// Decode one backend-specific UDP receive value.
    pub(crate) fn udp_receive_from_value(
        &self,
        value: HarnessValue<UdpReceive, UdpReceiveVm>,
    ) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
        match value {
            HarnessValue::Native(value) => udp_receive_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm udp receive value");
                udp_receive_vm(context, value)
            }
        }
    }

    /// Decode one backend-specific recv-message value into a comparable tuple.
    pub(crate) fn recv_message_fields(
        &self,
        value: HarnessValue<SocketRecvMessage, platform_net::SocketRecvMessageVm>,
    ) -> (u64, u32, bool, u32, bool, bool) {
        match value {
            HarnessValue::Native(value) => (
                value.bytes,
                value.fds.len,
                value.has_credentials,
                value.recv_flags.0,
                value.payload_truncated,
                value.control_truncated,
            ),
            HarnessValue::Vm(value) => (
                value.bytes,
                value.fds.len,
                value.has_credentials,
                value.recv_flags.0,
                value.payload_truncated,
                value.control_truncated,
            ),
        }
    }

    /// Decode one backend-specific recv-mmsg value into byte counts.
    pub(crate) fn recv_mmsg_counts_from_value(
        &self,
        value: HarnessValue<
            NativeArray<SocketRecvMessage>,
            VmArray<platform_net::SocketRecvMessageVm>,
        >,
    ) -> RuntimeResult<Vec<u64>> {
        match value {
            HarnessValue::Native(value) => {
                let values = unsafe { value.as_slice()? };
                Ok(values.iter().map(|value| value.bytes).collect())
            }
            HarnessValue::Vm(_) => Err(RuntimeError::from(PlatformError::not_supported(
                "destack.net.recvMmsg",
            ))
            .boxed()),
        }
    }

    /// Build one backend-specific empty send-message value.
    pub(crate) fn empty_send_message_value(
        &self,
        flags: u32,
        has_credentials: bool,
    ) -> RuntimeResult<HarnessValue<SocketSendMessage, SocketSendMessageVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let message = SocketSendMessageVm {
                    has_address: false,
                    address: SocketAddressVm {
                        family: 0,
                        length: 0,
                        bytes: VmArray::from_bytes(context, &[]),
                    },
                    fds: VmArray::from_values(context, &[]).expect("empty fd array should encode"),
                    control: platform_net::SocketControlBufferAbi::<VmAbi>(VmArray::from_bytes(
                        context,
                        &[],
                    )),
                    flags: SocketMessageFlags(flags),
                    has_credentials,
                    credentials: SocketCredentialsVm {
                        pid: 0,
                        uid: 0,
                        gid: 0,
                    },
                };
                Ok(self.harness_value_vm(message))
            }
            None => {
                let message = SocketSendMessage {
                    has_address: false,
                    address: SocketAddress {
                        family: 0,
                        length: 0,
                        bytes: NativeArray {
                            data: std::ptr::null_mut(),
                            len: 0,
                            capacity: 0,
                        },
                    },
                    fds: NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    },
                    control: platform_net::SocketControlBufferAbi::<NativeAbi>(NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    }),
                    flags: SocketMessageFlags(flags),
                    has_credentials,
                    credentials: SocketCredentials {
                        pid: 0,
                        uid: 0,
                        gid: 0,
                    },
                };
                Ok(self.harness_value(message))
            }
        }
    }

    /// Build one backend-specific send-mmsg message list.
    pub(crate) fn send_mmsg_messages_value(
        &self,
        buffers: &[&[u8]],
        send_flags: u32,
    ) -> RuntimeResult<
        HarnessValue<
            NativeSlice<SocketSendBatchEntry>,
            VmSlice<platform_net::SocketSendBatchEntryVm>,
        >,
    > {
        match self.vm_context_mut() {
            Some(_) => Err(RuntimeError::from(PlatformError::not_supported(
                "destack.net.sendMmsg",
            ))
            .boxed()),
            None => {
                let payloads = buffers
                    .iter()
                    .map(|buffer| self.call_context.store_slice(buffer.to_vec()))
                    .collect::<Vec<_>>();
                let entries = payloads
                    .iter()
                    .map(|payload| SocketSendBatchEntry {
                        payload: *payload,
                        message: SocketSendMessage {
                            has_address: false,
                            address: SocketAddress {
                                family: 0,
                                length: 0,
                                bytes: NativeArray {
                                    data: std::ptr::null_mut(),
                                    len: 0,
                                    capacity: 0,
                                },
                            },
                            fds: NativeArray {
                                data: std::ptr::null_mut(),
                                len: 0,
                                capacity: 0,
                            },
                            control: platform_net::SocketControlBufferAbi::<NativeAbi>(
                                NativeArray {
                                    data: std::ptr::null_mut(),
                                    len: 0,
                                    capacity: 0,
                                },
                            ),
                            flags: SocketMessageFlags(send_flags),
                            has_credentials: false,
                            credentials: SocketCredentials {
                                pid: 0,
                                uid: 0,
                                gid: 0,
                            },
                        },
                    })
                    .collect::<Vec<_>>();
                Ok(self.harness_value(self.call_context.store_slice(entries)))
            }
        }
    }

    /// Build one backend-specific recv-mmsg request list.
    pub(crate) fn recv_mmsg_requests_value(
        &self,
        buffers: &mut [Vec<u8>],
        recv_flags: u32,
    ) -> RuntimeResult<
        HarnessValue<
            NativeSlice<SocketRecvBatchRequest>,
            VmSlice<platform_net::SocketRecvBatchRequestVm>,
        >,
    > {
        match self.vm_context_mut() {
            Some(_) => Err(RuntimeError::from(PlatformError::not_supported(
                "destack.net.recvMmsg",
            ))
            .boxed()),
            None => {
                let payloads = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let requests = payloads
                    .iter()
                    .map(|payload| SocketRecvBatchRequest {
                        payload: *payload,
                        recv_flags: SocketMessageFlags(recv_flags),
                    })
                    .collect::<Vec<_>>();
                Ok(self.harness_value(self.call_context.store_slice(requests)))
            }
        }
    }
}

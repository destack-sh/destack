use destack_vm as vm;

#[cfg(unix)]
use super::path_ref_vm_utf16;
use super::{
    NetHarnessContext, host_from_vm, native_slice, native_slice_mut, path_ref_vm,
    reverse_lookup_names_native, reverse_lookup_names_vm, reverse_lookup_records_native,
    reverse_lookup_records_vm, socket_address_from_raw, socket_address_native_from_host_port,
    socket_address_raw_native, socket_address_raw_vm, socket_address_vm_from_host_port,
    socket_addresses_native, socket_addresses_vm, udp_receive_native, udp_receive_vm,
    uds_path_address_native, uds_path_address_vm, vm_slice_of_slices,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, NativeSlice, NativeStringRef, VmAbi};
use crate::platform::fs::{OsPath, OsPathBytes, OsPathUtf16, PathBytesAbi, PathUtf16Abi};
use crate::platform::net::{
    KeepAliveConfig, KeepAliveConfigVm, Linger, LingerVm, NetInterface, NetInterfaceVm,
    ResolveFlags, ResolveQuery, ReverseLookupName, SocketAddress, SocketAddressVm,
    SocketCredentials, SocketCredentialsVm, SocketFamily, SocketMessageFlags,
    SocketRecvBatchRequest, SocketRecvMessage, SocketSendBatchEntry, SocketSendMessage,
    SocketSendMessageVm, UdpReceive, UdpReceiveVm, UdsAddress, vm as platform_vm,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::net::{
    PacketBackendDescriptor, PacketBackendDescriptorVm, RouteEntry, RouteEntryVm,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice, net as platform_net};

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
        super::listener_port(self.call_context.worker(), handle)
    }

    /// Return whether one socket handle is in nonblocking mode on unix.
    #[cfg(unix)]
    pub(crate) fn socket_is_nonblocking(&self, handle: SocketHandle) -> bool {
        super::socket_is_nonblocking(self.call_context.worker(), handle)
    }

    /// Return whether one socket handle is close-on-exec or non-inheritable.
    pub(crate) fn socket_is_close_on_exec(&self, handle: SocketHandle) -> bool {
        super::socket_is_close_on_exec(self.call_context.worker(), handle)
    }

    /// Connect to a host and port through the VM text helper.
    pub(crate) fn vm_connect_text(&mut self, host: &str, port: u16) -> RuntimeResult<SocketHandle> {
        let context = self
            .vm_context_mut()
            .expect("vm context required for vm connectText helper");
        let host = vm::StringHandle::new(context.intern_string(host)?);

        platform_vm::destack_net_connect_text(self.call_context, context, host, port)
    }

    /// Start listening through the VM text helper.
    pub(crate) fn vm_listen_text(
        &mut self,
        host: &str,
        port: u16,
        backlog: u32,
    ) -> RuntimeResult<ListenerHandle> {
        let context = self
            .vm_context_mut()
            .expect("vm context required for vm listenText helper");
        let host = vm::StringHandle::new(context.intern_string(host)?);

        platform_vm::destack_net_listen_text(self.call_context, context, host, port, backlog)
    }

    /// Build one backend-specific UTF-8 string value.
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

    /// Decode one backend-specific string value into a Rust string.
    pub(crate) fn string_from_value(
        &self,
        value: HarnessValue<NativeStringRef, vm::StringHandle>,
    ) -> RuntimeResult<String> {
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_str()? };
                Ok(value.to_string())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm string value");
                let value = context
                    .string_ref(value)
                    .map_err(|error| RuntimeError::from(error).boxed())?;
                Ok(value.as_str().to_string())
            }
        }
    }

    /// Decode one backend-specific interface list into name and index tuples.
    pub(crate) fn interface_name_index_list_from_value(
        &self,
        value: HarnessValue<NativeArray<NetInterface>, VmArray<NetInterfaceVm>>,
    ) -> RuntimeResult<Vec<(String, u32)>> {
        match value {
            HarnessValue::Native(value) => {
                let interfaces = unsafe { value.as_slice()? };
                let mut decoded = Vec::with_capacity(interfaces.len());
                for interface in interfaces {
                    let name = unsafe { interface.name.as_str()? };
                    decoded.push((name.to_string(), interface.index));
                }
                Ok(decoded)
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm interface list");
                let interfaces = value.read_values(&context.read())?;
                let mut decoded = Vec::with_capacity(interfaces.len());
                for interface in interfaces {
                    let name = context
                        .string_ref(interface.name)
                        .map_err(|error| RuntimeError::from(error).boxed())?;
                    decoded.push((name.as_str().to_string(), interface.index));
                }
                Ok(decoded)
            }
        }
    }

    /// Decode one backend-specific packet-backend descriptor list.
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    pub(crate) fn packet_backend_descriptors_from_value(
        &self,
        value: HarnessValue<
            NativeSlice<PacketBackendDescriptor>,
            VmSlice<PacketBackendDescriptorVm>,
        >,
    ) -> RuntimeResult<Vec<(String, PacketBackendDescriptor)>> {
        match value {
            HarnessValue::Native(value) => {
                let descriptors = unsafe { value.as_slice()? };
                let mut decoded = Vec::with_capacity(descriptors.len());
                for descriptor in descriptors {
                    let name = unsafe { descriptor.name.as_str()? };
                    decoded.push((name.to_string(), *descriptor));
                }
                Ok(decoded)
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm packet-backend list");
                let descriptors = value.read_values(&context.read())?;
                let mut decoded = Vec::with_capacity(descriptors.len());
                for descriptor in descriptors {
                    let name = context
                        .string_ref(descriptor.name)
                        .map_err(|error| RuntimeError::from(error).boxed())?;
                    decoded.push((
                        name.as_str().to_string(),
                        PacketBackendDescriptor {
                            backend: descriptor.backend,
                            name: self.call_context.store_string(name.as_str()),
                            available: descriptor.available,
                            priority: descriptor.priority,
                            capability_flags: descriptor.capability_flags,
                        },
                    ));
                }
                Ok(decoded)
            }
        }
    }

    /// Decode one backend-specific route-entry list.
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    pub(crate) fn route_entries_from_value(
        &self,
        value: HarnessValue<NativeArray<RouteEntry>, VmArray<RouteEntryVm>>,
    ) -> RuntimeResult<Vec<RouteEntry>> {
        match value {
            HarnessValue::Native(value) => {
                let routes = unsafe { value.as_slice()? };
                Ok(routes.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm route list");
                let routes = value.read_values(&context.read())?;
                let mut decoded = Vec::with_capacity(routes.len());
                for route in routes {
                    decoded.push(RouteEntry {
                        family: route.family,
                        destination: SocketAddress {
                            family: route.destination.family,
                            length: route.destination.length,
                            bytes: self
                                .call_context
                                .store_array(route.destination.bytes.read_bytes(&context.read())?),
                        },
                        prefix_length: route.prefix_length,
                        gateway: SocketAddress {
                            family: route.gateway.family,
                            length: route.gateway.length,
                            bytes: self
                                .call_context
                                .store_array(route.gateway.bytes.read_bytes(&context.read())?),
                        },
                        interface_index: route.interface_index,
                        metric: route.metric,
                        kind: route.kind,
                    });
                }
                Ok(decoded)
            }
        }
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
                            .expect("vm test byte slice should allocate")
                    })
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

    /// Write bytes to one socket handle.
    pub(crate) fn write(&mut self, handle: SocketHandle, buffer: &[u8]) -> RuntimeResult<u64> {
        let buffer = self.bytes_slice_value(buffer)?;
        self.destack_net_write(handle, buffer)
    }

    /// Read bytes from one socket handle into one mutable buffer.
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
                let raw_values = value.raw_values(&context.read())?;
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
        let service = port.to_string();
        self.resolve_query_parts_value(Some(host), Some(service.as_str()), family, flags)
    }

    /// Build one backend-specific resolve-query value from optional parts.
    pub(crate) fn resolve_query_parts_value(
        &self,
        host: Option<&str>,
        service: Option<&str>,
        family: SocketFamily,
        flags: ResolveFlags,
    ) -> HarnessValue<ResolveQuery, platform_net::ResolveQueryVm> {
        match self.vm_context_mut() {
            Some(context) => self.harness_value_vm(platform_net::ResolveQueryVm {
                host: host.map(|host| host_from_vm(context, host)),
                service: service.map(|service| host_from_vm(context, service)),
                family,
                flags,
            }),
            None => self.harness_value(ResolveQuery {
                host: host.map(|host| self.call_context.store_string(host)),
                service: service.map(|service| self.call_context.store_string(service)),
                family,
                flags,
            }),
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

    /// Decode one backend-specific reverse-lookup record list value.
    pub(crate) fn reverse_lookup_records_from_value(
        &self,
        value: HarnessValue<
            NativeArray<ReverseLookupName>,
            VmArray<platform_net::ReverseLookupNameVm>,
        >,
    ) -> RuntimeResult<Vec<(String, String)>> {
        match value {
            HarnessValue::Native(value) => reverse_lookup_records_native(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm reverse lookup records");
                reverse_lookup_records_vm(context, value)
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
                    OsPath::OsPathBytes(OsPathBytes {
                        kind: "bytes".into(),
                        bytes: PathBytesAbi(bytes),
                    })
                };

                #[cfg(windows)]
                let path = {
                    use std::os::windows::ffi::OsStrExt;

                    let units = path.as_os_str().encode_wide().collect::<Vec<_>>();
                    let units = self.call_context.store_array(units);
                    OsPath::OsPathUtf16(OsPathUtf16 {
                        kind: "utf16".into(),
                        utf16: PathUtf16Abi(units),
                    })
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
                let path = OsPath::OsPathUtf16(OsPathUtf16 {
                    kind: "utf16".into(),
                    utf16: PathUtf16Abi(units),
                });
                self.harness_value(uds_path_address_native(path))
            }
        }
    }

    /// Decode one backend-specific UDP receive value.
    pub(crate) fn udp_receive_from_value(
        &self,
        value: HarnessValue<UdpReceive, UdpReceiveVm>,
    ) -> RuntimeResult<(String, u16, SocketFamily, u64, u32)> {
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
                value.credentials.is_some(),
                value.recv_flags.0,
                value.payload_truncated,
                value.control_truncated,
            ),
            HarnessValue::Vm(value) => (
                value.bytes,
                value.fds.len,
                value.credentials.is_some(),
                value.recv_flags.0,
                value.payload_truncated,
                value.control_truncated,
            ),
        }
    }

    /// Decode one recv-message payload into address and control metadata.
    pub(crate) fn recv_message_meta(
        &self,
        value: HarnessValue<SocketRecvMessage, platform_net::SocketRecvMessageVm>,
    ) -> (bool, u32) {
        match value {
            HarnessValue::Native(value) => (value.address.is_some(), value.control.0.len),
            HarnessValue::Vm(value) => (value.address.is_some(), value.control.0.len),
        }
    }

    /// Extract one recv-message source address when present.
    pub(crate) fn recv_message_address(
        &self,
        value: HarnessValue<SocketRecvMessage, platform_net::SocketRecvMessageVm>,
    ) -> Option<HarnessValue<SocketAddress, SocketAddressVm>> {
        match value {
            HarnessValue::Native(value) => value.address.map(|address| self.harness_value(address)),
            HarnessValue::Vm(value) => value.address.map(|address| self.harness_value_vm(address)),
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
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm recv-mmsg decode");
                let values = value.read_values(&context.read())?;
                Ok(values.iter().map(|value| value.bytes).collect())
            }
        }
    }

    /// Decode recv-mmsg request payload buffers after a receive operation.
    pub(crate) fn recv_mmsg_payloads_from_requests(
        &self,
        requests: HarnessValue<
            NativeSlice<SocketRecvBatchRequest>,
            VmSlice<platform_net::SocketRecvBatchRequestVm>,
        >,
        counts: &[u64],
    ) -> RuntimeResult<Vec<Vec<u8>>> {
        match requests {
            HarnessValue::Native(requests) => {
                let requests = unsafe { requests.as_slice()? };
                let mut payloads = Vec::with_capacity(requests.len());
                for (request, count) in requests.iter().zip(counts.iter()) {
                    let payload = unsafe { request.payload.as_slice()? };
                    let count = (*count as usize).min(payload.len());
                    payloads.push(payload[..count].to_vec());
                }

                Ok(payloads)
            }
            HarnessValue::Vm(requests) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm recv-mmsg request decode");
                let requests = requests.read_values(&context.read())?;
                let mut payloads = Vec::with_capacity(requests.len());
                for (request, count) in requests.iter().zip(counts.iter()) {
                    let payload = request.payload.read_bytes(&context.read())?;
                    let count = (*count as usize).min(payload.len());
                    payloads.push(payload[..count].to_vec());
                }

                Ok(payloads)
            }
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
                    address: None,
                    fds: VmArray::from_values(&mut context.write(), &[])
                        .expect("empty fd array should encode"),
                    control: platform_net::SocketControlBufferAbi::<VmAbi>(
                        VmArray::from_bytes(&mut context.write(), &[])
                            .expect("vm test byte array should allocate"),
                    ),
                    flags: SocketMessageFlags(flags),
                    credentials: if has_credentials {
                        Some(SocketCredentialsVm {
                            pid: 0,
                            uid: 0,
                            gid: 0,
                        })
                    } else {
                        None
                    },
                };
                Ok(self.harness_value_vm(message))
            }
            None => {
                let message = SocketSendMessage {
                    address: None,
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
                    credentials: if has_credentials {
                        Some(SocketCredentials {
                            pid: 0,
                            uid: 0,
                            gid: 0,
                        })
                    } else {
                        None
                    },
                };
                Ok(self.harness_value(message))
            }
        }
    }

    /// Build one backend-specific send-message value with explicit destination and control bytes.
    pub(crate) fn send_message_value(
        &self,
        address: HarnessValue<SocketAddress, SocketAddressVm>,
        control: &[u8],
        flags: u32,
        has_credentials: bool,
    ) -> RuntimeResult<HarnessValue<SocketSendMessage, SocketSendMessageVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let address = match address {
                    HarnessValue::Native(_) => {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                            "address",
                            "SocketAddressVm",
                        ))
                        .boxed());
                    }
                    HarnessValue::Vm(address) => address,
                };
                let message = SocketSendMessageVm {
                    address: Some(address),
                    fds: VmArray::from_values(&mut context.write(), &[])
                        .expect("empty fd array should encode"),
                    control: platform_net::SocketControlBufferAbi::<VmAbi>(
                        VmArray::from_bytes(&mut context.write(), control)
                            .expect("vm test byte array should allocate"),
                    ),
                    flags: SocketMessageFlags(flags),
                    credentials: if has_credentials {
                        Some(SocketCredentialsVm {
                            pid: 0,
                            uid: 0,
                            gid: 0,
                        })
                    } else {
                        None
                    },
                };
                Ok(self.harness_value_vm(message))
            }
            None => {
                let address = match address {
                    HarnessValue::Native(address) => address,
                    HarnessValue::Vm(_) => {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                            "address",
                            "SocketAddress",
                        ))
                        .boxed());
                    }
                };
                let message = SocketSendMessage {
                    address: Some(address),
                    fds: NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    },
                    control: platform_net::SocketControlBufferAbi::<NativeAbi>(
                        self.call_context.store_array(control.to_vec()),
                    ),
                    flags: SocketMessageFlags(flags),
                    credentials: if has_credentials {
                        Some(SocketCredentials {
                            pid: 0,
                            uid: 0,
                            gid: 0,
                        })
                    } else {
                        None
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
            Some(context) => {
                let mut entries = Vec::with_capacity(buffers.len());
                for buffer in buffers {
                    let payload = VmSlice::from_bytes(&mut context.write(), buffer)
                        .expect("vm test byte slice should allocate");
                    let message = SocketSendMessageVm {
                        address: None,
                        fds: VmArray::from_values(&mut context.write(), &[])?,
                        control: platform_net::SocketControlBufferAbi::<VmAbi>(
                            VmArray::from_bytes(&mut context.write(), &[])
                                .expect("vm test byte array should allocate"),
                        ),
                        flags: SocketMessageFlags(send_flags),
                        credentials: None,
                    };
                    entries.push(platform_net::SocketSendBatchEntryVm { payload, message });
                }

                let entries = VmSlice::from_values(&mut context.write(), &entries)?;
                Ok(self.harness_value_vm(entries))
            }
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
                            address: None,
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
                            credentials: None,
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
            Some(context) => {
                let mut requests = Vec::with_capacity(buffers.len());
                for buffer in buffers.iter() {
                    let payload =
                        VmSlice::from_bytes(&mut context.write(), &vec![0u8; buffer.len()])
                            .expect("vm test byte slice should allocate");
                    requests.push(platform_net::SocketRecvBatchRequestVm {
                        payload,
                        recv_flags: SocketMessageFlags(recv_flags),
                    });
                }

                let requests = VmSlice::from_values(&mut context.write(), &requests)?;
                Ok(self.harness_value_vm(requests))
            }
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

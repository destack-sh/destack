use std::net::ToSocketAddrs;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeResult, RuntimeStatus};
use crate::platform::fs::{OsPath, OsPathVm, PathBytesAbi, PathEncoding, PathUtf16Abi};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice, VmValueCodec,
    net as platform_net,
};
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;
use platform_net::{
    AcceptFlags, KeepAliveConfig, Linger, LingerVm, ResolveFlags, SocketAddress, SocketAddressVm,
    SocketCredentials, SocketCredentialsVm, SocketFamily, SocketMessageFlags, SocketProtocol,
    SocketRecvMessage, SocketSendMessage, SocketSendMessageVm, SocketShutdown, SocketType,
    UdpMessageFlags, UdpReceive, UdpReceiveVm, vm as platform_vm,
};

/// Selects the backing harness kind for network tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetHarnessKind {
    /// Native bindings backed by the host ABI.
    Native,
    /// VM bindings backed by VM ABI values.
    Vm,
}

/// Network harness context used by tests.
pub(crate) struct NetHarnessContext<'call> {
    /// Runtime backing this harness.
    runtime: &'call TestRuntime,
    /// Runtime call context active for this operation.
    call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    vm_context: Option<*mut ()>,
}

impl<'call> NetHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::RuntimeContext<'_>> {
        // safety: the harness guarantees the VM context pointer is valid for the callback
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::RuntimeContext<'_>) })
    }

    /// Return the harness kind for this context.
    pub(crate) fn kind(&self) -> NetHarnessKind {
        if self.vm_context.is_some() {
            NetHarnessKind::Vm
        } else {
            NetHarnessKind::Native
        }
    }

    /// Convert a native status into a test-friendly result.
    pub(crate) fn status_ok(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.runtime.assert_status_ok(status, label);
        Ok(())
    }

    /// Convert a native status into a runtime result.
    pub(crate) fn status_result(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        if status == RuntimeStatus::OK {
            return Ok(());
        }

        if status.error_id == 0 {
            return Err(RuntimeError::from(PlatformError::io(format!(
                "{label} failed without runtime error id",
            )))
            .boxed());
        }

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

    /// Convert a native error status into a test-friendly result.
    pub(crate) fn status_err(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.runtime.assert_status_err(status, label);
        Ok(())
    }

    /// Read the port assigned to a listener handle.
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        self.runtime.listener_port(handle)
    }

    /// Start listening on the given address.
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

                platform_vm::destack_net_listen(self.call_context, context, address, backlog)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;

                let mut handle = ListenerHandle(ResourceId(0));
                let status = unsafe {
                    platform_net::destack_net_listen(&mut handle, address.address(), backlog)
                };
                self.status_ok(status, "listen")?;

                Ok(handle)
            }
        }
    }

    /// Connect to a remote host and return a socket handle.
    pub(crate) fn connect(&mut self, host: &str, port: u16) -> RuntimeResult<SocketHandle> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };
        let socket_type = SocketType(libc::SOCK_STREAM as u32);
        let protocol = SocketProtocol(libc::IPPROTO_TCP);

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;
                let handle = platform_vm::destack_net_socket(
                    self.call_context,
                    context,
                    family,
                    socket_type,
                    protocol,
                )?;
                platform_vm::destack_net_connect(self.call_context, context, handle, address)?;

                Ok(handle)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let mut handle = SocketHandle(ResourceId(0));
                let socket_status = unsafe {
                    platform_net::destack_net_socket(&mut handle, family, socket_type, protocol)
                };
                self.status_ok(socket_status, "connect socket")?;

                let connect_status =
                    unsafe { platform_net::destack_net_connect(handle, address.address()) };
                self.status_ok(connect_status, "connect")?;

                Ok(handle)
            }
        }
    }

    /// Accept a new connection from a listener.
    pub(crate) fn accept(&mut self, listener: ListenerHandle) -> RuntimeResult<SocketHandle> {
        let flags = AcceptFlags(0);
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_accept(self.call_context, context, listener, flags)
            }
            None => {
                let mut handle = SocketHandle(ResourceId(0));
                let status =
                    unsafe { platform_net::destack_net_accept(&mut handle, listener, flags) };
                self.status_ok(status, "accept")?;
                Ok(handle)
            }
        }
    }

    /// Read from a socket into the provided buffer.
    pub(crate) fn read(&mut self, handle: SocketHandle, buffer: &mut [u8]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                // allocate a VM buffer for the read
                let vm_slice = VmSlice::from_bytes(context, &vec![0u8; buffer.len()]);

                // perform the read
                let bytes =
                    platform_vm::destack_net_read(self.call_context, context, handle, vm_slice)?;

                // read bytes back into the host buffer
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
                let mut out = 0u64;
                let status = unsafe { platform_net::destack_net_read(&mut out, handle, slice) };
                self.status_ok(status, "read")?;
                Ok(out)
            }
        }
    }

    /// Write to a socket from the provided buffer.
    pub(crate) fn write(&mut self, handle: SocketHandle, buffer: &[u8]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                // allocate a VM buffer for the write
                let vm_slice = VmSlice::from_bytes(context, buffer);

                // dispatch to the VM binding
                platform_vm::destack_net_write(self.call_context, context, handle, vm_slice)
            }
            None => {
                let slice = native_slice(buffer);
                let mut out = 0u64;
                let status = unsafe { platform_net::destack_net_write(&mut out, handle, slice) };
                self.status_ok(status, "write")?;
                Ok(out)
            }
        }
    }

    /// Read from a socket into multiple buffers.
    pub(crate) fn readv(
        &mut self,
        handle: SocketHandle,
        buffers: &mut [Vec<u8>],
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                // allocate VM buffers for each iovec entry
                let mut vm_buffers = Vec::with_capacity(buffers.len());
                for buffer in buffers.iter() {
                    vm_buffers.push(VmSlice::from_bytes(context, &vec![0u8; buffer.len()]));
                }
                let vm_slices = vm_slice_of_slices(context, &vm_buffers);

                // read into VM iovecs
                let total =
                    platform_vm::destack_net_readv(self.call_context, context, handle, vm_slices)?;

                // copy bytes back into host buffers
                let mut remaining = total as usize;
                for (index, vm_buffer) in vm_buffers.into_iter().enumerate() {
                    if remaining == 0 {
                        break;
                    }

                    let bytes = vm_buffer.read_bytes(context)?;
                    let count = remaining.min(buffers[index].len()).min(bytes.len());
                    buffers[index][..count].copy_from_slice(&bytes[..count]);
                    remaining -= count;
                }

                Ok(total)
            }
            None => {
                // build native iovecs
                let mut native_buffers = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let native_slices = native_slice_slices_mut(&mut native_buffers);
                let mut out = 0u64;
                let status =
                    unsafe { platform_net::destack_net_readv(&mut out, handle, native_slices) };
                self.status_result(status, "readv")?;

                Ok(out)
            }
        }
    }

    /// Write to a socket from multiple buffers.
    pub(crate) fn writev(&mut self, handle: SocketHandle, buffers: &[&[u8]]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                // allocate VM buffers for each iovec entry
                let mut vm_buffers = Vec::with_capacity(buffers.len());
                for buffer in buffers {
                    vm_buffers.push(VmSlice::from_bytes(context, buffer));
                }
                let vm_slices = vm_slice_of_slices(context, &vm_buffers);

                platform_vm::destack_net_writev(self.call_context, context, handle, vm_slices)
            }
            None => {
                // build native iovecs
                let native_buffers = buffers
                    .iter()
                    .map(|buffer| native_slice(buffer))
                    .collect::<Vec<_>>();
                let native_slices = native_slice_slices(&native_buffers);
                let mut out = 0u64;
                let status =
                    unsafe { platform_net::destack_net_writev(&mut out, handle, native_slices) };
                self.status_result(status, "writev")?;

                Ok(out)
            }
        }
    }

    /// Receive a message with ancillary data.
    pub(crate) fn recv_msg(
        &mut self,
        handle: SocketHandle,
        buffer: &mut [u8],
        max_fds: u32,
        want_credentials: bool,
    ) -> RuntimeResult<(u64, u32, bool, u32, bool, bool)> {
        self.recv_msg_with_flags(handle, buffer, 0, max_fds, want_credentials)
    }

    /// Receive a message with ancillary data and explicit receive flags.
    pub(crate) fn recv_msg_with_flags(
        &mut self,
        handle: SocketHandle,
        buffer: &mut [u8],
        recv_flags: u32,
        max_fds: u32,
        want_credentials: bool,
    ) -> RuntimeResult<(u64, u32, bool, u32, bool, bool)> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_slice = VmSlice::from_bytes(context, &vec![0u8; buffer.len()]);
                let receive = platform_vm::destack_net_recv_msg(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                    SocketMessageFlags(recv_flags),
                    max_fds,
                    want_credentials,
                )?;
                let read_bytes = vm_slice.read_bytes(context)?;
                let count = receive.bytes as usize;
                if count > buffer.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "buffer",
                        "read size exceeded buffer length",
                    ))
                    .boxed());
                }
                buffer[..count].copy_from_slice(&read_bytes[..count]);
                Ok((
                    receive.bytes,
                    receive.fds.len,
                    receive.has_credentials,
                    receive.recv_flags.0,
                    receive.payload_truncated,
                    receive.control_truncated,
                ))
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = std::mem::MaybeUninit::<SocketRecvMessage>::uninit();
                let status = unsafe {
                    platform_net::destack_net_recv_msg(
                        out.as_mut_ptr(),
                        handle,
                        slice,
                        SocketMessageFlags(recv_flags),
                        max_fds,
                        want_credentials,
                    )
                };
                self.status_ok(status, "recv msg")?;
                let receive = unsafe { out.assume_init() };
                Ok((
                    receive.bytes,
                    receive.fds.len,
                    receive.has_credentials,
                    receive.recv_flags.0,
                    receive.payload_truncated,
                    receive.control_truncated,
                ))
            }
        }
    }

    /// Send a message with ancillary data.
    pub(crate) fn send_msg(&mut self, handle: SocketHandle, buffer: &[u8]) -> RuntimeResult<u64> {
        self.send_msg_with_options(handle, buffer, 0, false)
    }

    /// Send a message with ancillary data and explicit send flags.
    pub(crate) fn send_msg_with_flags(
        &mut self,
        handle: SocketHandle,
        buffer: &[u8],
        flags: u32,
    ) -> RuntimeResult<u64> {
        self.send_msg_with_options(handle, buffer, flags, false)
    }

    /// Send a message with ancillary flags and credential intent.
    pub(crate) fn send_msg_with_options(
        &mut self,
        handle: SocketHandle,
        buffer: &[u8],
        flags: u32,
        has_credentials: bool,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_slice = VmSlice::from_bytes(context, buffer);
                let fds = VmArray::from_values(context, &[]).expect("empty fd array should encode");
                let message = SocketSendMessageVm {
                    fds,
                    flags: SocketMessageFlags(flags),
                    has_credentials,
                    credentials: SocketCredentialsVm {
                        pid: 0,
                        uid: 0,
                        gid: 0,
                    },
                };
                platform_vm::destack_net_send_msg(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                    message,
                )
            }
            None => {
                let slice = native_slice(buffer);
                let mut out = 0u64;
                let message = SocketSendMessage {
                    fds: NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    },
                    flags: SocketMessageFlags(flags),
                    has_credentials,
                    credentials: SocketCredentials {
                        pid: 0,
                        uid: 0,
                        gid: 0,
                    },
                };
                let status =
                    unsafe { platform_net::destack_net_send_msg(&mut out, handle, slice, message) };
                self.status_result(status, "send msg")?;
                Ok(out)
            }
        }
    }

    /// Send multiple messages from multiple buffers.
    pub(crate) fn send_mmsg(
        &mut self,
        handle: SocketHandle,
        buffers: &[&[u8]],
        send_flags: u32,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            // call the VM binding
            Some(context) => {
                // encode nested VM slices
                let mut vm_buffers = Vec::with_capacity(buffers.len());
                for buffer in buffers {
                    vm_buffers.push(VmSlice::from_bytes(context, buffer));
                }
                let vm_buffers = vm_slice_of_slices(context, &vm_buffers);

                platform_vm::destack_net_send_mmsg(
                    self.call_context,
                    context,
                    handle,
                    vm_buffers,
                    SocketMessageFlags(send_flags),
                )
            }

            // call the native binding
            None => {
                // build nested native slices
                let native_buffers = buffers
                    .iter()
                    .map(|buffer| native_slice(buffer))
                    .collect::<Vec<_>>();
                let native_buffers = native_slice_slices(&native_buffers);
                let mut out = 0u64;
                let status = unsafe {
                    platform_net::destack_net_send_mmsg(
                        &mut out,
                        handle,
                        native_buffers,
                        SocketMessageFlags(send_flags),
                    )
                };
                self.status_result(status, "send mmsg")?;

                Ok(out)
            }
        }
    }

    /// Receive multiple messages into multiple buffers.
    pub(crate) fn recv_mmsg(
        &mut self,
        handle: SocketHandle,
        buffers: &mut [Vec<u8>],
        recv_flags: u32,
    ) -> RuntimeResult<Vec<u64>> {
        match self.vm_context_mut() {
            // call the VM binding
            Some(context) => {
                // encode nested VM slices
                let mut vm_buffers = Vec::with_capacity(buffers.len());
                for buffer in buffers.iter() {
                    vm_buffers.push(VmSlice::from_bytes(context, &vec![0u8; buffer.len()]));
                }
                let vm_buffer_slice = vm_slice_of_slices(context, &vm_buffers);

                // receive and decode counts
                let counts = platform_vm::destack_net_recv_mmsg(
                    self.call_context,
                    context,
                    handle,
                    vm_buffer_slice,
                    SocketMessageFlags(recv_flags),
                )?;
                let counts = counts.read_values(context)?;

                // copy bytes back into host buffers
                for (index, vm_buffer) in vm_buffers.into_iter().enumerate() {
                    if index >= counts.len() {
                        break;
                    }

                    let bytes = vm_buffer.read_bytes(context)?;
                    let count = counts[index] as usize;
                    if count > buffers[index].len() || count > bytes.len() {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "buffers",
                            "received byte count exceeded buffer length",
                        ))
                        .boxed());
                    }

                    buffers[index][..count].copy_from_slice(&bytes[..count]);
                }

                Ok(counts)
            }

            // call the native binding
            None => {
                // build nested native slices
                let mut native_buffers = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let native_buffers = native_slice_slices_mut(&mut native_buffers);

                // receive counts
                let mut out = std::mem::MaybeUninit::<NativeArray<u64>>::uninit();
                let status = unsafe {
                    platform_net::destack_net_recv_mmsg(
                        out.as_mut_ptr(),
                        handle,
                        native_buffers,
                        SocketMessageFlags(recv_flags),
                    )
                };
                self.status_result(status, "recv mmsg")?;
                let counts = unsafe { out.assume_init() };
                let counts = unsafe { counts.as_slice()? }.to_vec();

                Ok(counts)
            }
        }
    }

    /// Close a socket handle.
    pub(crate) fn close(&mut self, handle: SocketHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_net_close(self.call_context, context, handle),
            None => {
                let status = unsafe { platform_net::destack_net_close(handle) };
                self.status_ok(status, "close")
            }
        }
    }

    /// Close a listener handle.
    pub(crate) fn close_listener(&mut self, handle: ListenerHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_close_listener(self.call_context, context, handle)
            }
            None => {
                let status = unsafe { platform_net::destack_net_close_listener(handle) };
                self.status_ok(status, "close listener")
            }
        }
    }

    /// Shut down a socket for reads, writes, or both.
    pub(crate) fn shutdown(
        &mut self,
        handle: SocketHandle,
        how: SocketShutdown,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_shutdown(self.call_context, context, handle, how)
            }
            None => {
                let status = unsafe { platform_net::destack_net_shutdown(handle, how) };
                self.status_ok(status, "shutdown")
            }
        }
    }

    /// Enable or disable nonblocking mode on a socket.
    pub(crate) fn set_nonblocking(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_net_set_nonblocking(
                self.call_context,
                context,
                handle,
                enabled,
            ),
            None => {
                let status = unsafe { platform_net::destack_net_set_nonblocking(handle, enabled) };
                self.status_ok(status, "set nonblocking")
            }
        }
    }

    /// Enable or disable SO_REUSEADDR.
    pub(crate) fn set_reuse_addr(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_reuse_addr(self.call_context, context, handle, enabled)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_reuse_addr(handle, enabled) };
                self.status_ok(status, "set reuse addr")
            }
        }
    }

    /// Enable or disable SO_REUSEPORT.
    #[cfg(unix)]
    pub(crate) fn set_reuse_port(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_reuse_port(self.call_context, context, handle, enabled)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_reuse_port(handle, enabled) };
                self.status_ok(status, "set reuse port")
            }
        }
    }

    /// Read the local socket address.
    pub(crate) fn local_address(
        &mut self,
        handle: SocketHandle,
    ) -> RuntimeResult<(String, u16, SocketFamily)> {
        let (family, bytes) = self.local_address_raw(handle)?;
        socket_address_from_raw(family, &bytes)
    }

    /// Read the remote socket address.
    pub(crate) fn peer_address(
        &mut self,
        handle: SocketHandle,
    ) -> RuntimeResult<(String, u16, SocketFamily)> {
        let (family, bytes) = self.peer_address_raw(handle)?;
        socket_address_from_raw(family, &bytes)
    }

    /// Read the local socket address bytes.
    pub(crate) fn local_address_raw(
        &mut self,
        handle: SocketHandle,
    ) -> RuntimeResult<(u16, Vec<u8>)> {
        match self.vm_context_mut() {
            // call the VM binding
            Some(context) => {
                let address =
                    platform_vm::destack_net_local_address(self.call_context, context, handle)?;

                socket_address_raw_vm(context, address)
            }

            // call the native binding
            None => {
                let mut out = std::mem::MaybeUninit::<SocketAddress>::uninit();
                let status =
                    unsafe { platform_net::destack_net_local_address(out.as_mut_ptr(), handle) };
                self.status_ok(status, "local address raw")?;
                let address = unsafe { out.assume_init() };

                socket_address_raw_native(address)
            }
        }
    }

    /// Read the remote socket address bytes.
    pub(crate) fn peer_address_raw(
        &mut self,
        handle: SocketHandle,
    ) -> RuntimeResult<(u16, Vec<u8>)> {
        match self.vm_context_mut() {
            // call the VM binding
            Some(context) => {
                let address =
                    platform_vm::destack_net_peer_address(self.call_context, context, handle)?;

                socket_address_raw_vm(context, address)
            }

            // call the native binding
            None => {
                let mut out = std::mem::MaybeUninit::<SocketAddress>::uninit();
                let status =
                    unsafe { platform_net::destack_net_peer_address(out.as_mut_ptr(), handle) };
                self.status_ok(status, "peer address raw")?;
                let address = unsafe { out.assume_init() };

                socket_address_raw_native(address)
            }
        }
    }

    /// Enable or disable TCP_NODELAY.
    #[allow(dead_code)]
    pub(crate) fn set_no_delay(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_no_delay(self.call_context, context, handle, enabled)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_no_delay(handle, enabled) };
                self.status_ok(status, "set no delay")
            }
        }
    }

    /// Enable or disable TCP keepalive.
    #[allow(dead_code)]
    pub(crate) fn set_keep_alive(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
        delay_seconds: u32,
    ) -> RuntimeResult<()> {
        self.set_keep_alive_config(
            handle,
            KeepAliveConfig {
                enabled,
                idle_seconds: delay_seconds,
                interval_seconds: 0,
                probe_count: 0,
            },
        )
    }

    /// Write full TCP keepalive settings.
    #[allow(dead_code)]
    pub(crate) fn set_keep_alive_config(
        &mut self,
        handle: SocketHandle,
        config: KeepAliveConfig,
    ) -> RuntimeResult<()> {
        let config = KeepAliveConfig {
            enabled: config.enabled,
            idle_seconds: config.idle_seconds,
            interval_seconds: config.interval_seconds,
            probe_count: config.probe_count,
        };

        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_keep_alive(self.call_context, context, handle, config)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_keep_alive(handle, config) };
                self.status_result(status, "set keep alive")
            }
        }
    }

    /// Set socket linger behavior.
    pub(crate) fn set_linger(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
        seconds: u32,
    ) -> RuntimeResult<()> {
        let linger = Linger { enabled, seconds };
        match self.vm_context_mut() {
            Some(context) => {
                let linger = LingerVm {
                    enabled: linger.enabled,
                    seconds: linger.seconds,
                };
                platform_vm::destack_net_set_linger(self.call_context, context, handle, linger)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_linger(handle, linger) };
                self.status_result(status, "set linger")
            }
        }
    }

    /// Set receive buffer size.
    pub(crate) fn set_recv_buffer(&mut self, handle: SocketHandle, size: u32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_recv_buffer(self.call_context, context, handle, size)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_recv_buffer(handle, size) };
                self.status_result(status, "set recv buffer")
            }
        }
    }

    /// Set send buffer size.
    pub(crate) fn set_send_buffer(&mut self, handle: SocketHandle, size: u32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_send_buffer(self.call_context, context, handle, size)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_send_buffer(handle, size) };
                self.status_result(status, "set send buffer")
            }
        }
    }

    /// Set broadcast mode.
    pub(crate) fn set_broadcast(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_broadcast(self.call_context, context, handle, enabled)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_broadcast(handle, enabled) };
                self.status_result(status, "set broadcast")
            }
        }
    }

    /// Set socket TTL.
    pub(crate) fn set_ttl(&mut self, handle: SocketHandle, ttl: u32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_ttl(self.call_context, context, handle, ttl)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_ttl(handle, ttl) };
                self.status_result(status, "set ttl")
            }
        }
    }

    /// Set socket TOS/traffic class.
    pub(crate) fn set_tos(&mut self, handle: SocketHandle, tos: u32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_tos(self.call_context, context, handle, tos)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_tos(handle, tos) };
                self.status_result(status, "set tos")
            }
        }
    }

    /// Set read timeout in milliseconds.
    pub(crate) fn set_read_timeout(
        &mut self,
        handle: SocketHandle,
        timeout_ms: u32,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_net_set_read_timeout(
                self.call_context,
                context,
                handle,
                timeout_ms,
            ),
            None => {
                let status =
                    unsafe { platform_net::destack_net_set_read_timeout(handle, timeout_ms) };
                self.status_result(status, "set read timeout")
            }
        }
    }

    /// Set write timeout in milliseconds.
    pub(crate) fn set_write_timeout(
        &mut self,
        handle: SocketHandle,
        timeout_ms: u32,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_net_set_write_timeout(
                self.call_context,
                context,
                handle,
                timeout_ms,
            ),
            None => {
                let status =
                    unsafe { platform_net::destack_net_set_write_timeout(handle, timeout_ms) };
                self.status_result(status, "set write timeout")
            }
        }
    }

    /// Join a multicast group.
    pub(crate) fn join_multicast(
        &mut self,
        handle: SocketHandle,
        group: &str,
        interface_address: &str,
    ) -> RuntimeResult<()> {
        let is_ipv6_group = group.contains(':');
        match self.vm_context_mut() {
            Some(context) => {
                let group = host_from_vm(context, group);
                if is_ipv6_group {
                    let interface_index = interface_address.parse::<u32>().unwrap_or(0);
                    platform_vm::destack_net_join_multicast_v6(
                        self.call_context,
                        context,
                        handle,
                        group,
                        interface_index,
                    )
                } else {
                    let interface_address = host_from_vm(context, interface_address);
                    platform_vm::destack_net_join_multicast_v4(
                        self.call_context,
                        context,
                        handle,
                        group,
                        interface_address,
                    )
                }
            }
            None => {
                let group = NativeStringRef::from(group);
                let status = if is_ipv6_group {
                    let interface_index = interface_address.parse::<u32>().unwrap_or(0);
                    unsafe {
                        platform_net::destack_net_join_multicast_v6(handle, group, interface_index)
                    }
                } else {
                    let interface_address = NativeStringRef::from(interface_address);
                    unsafe {
                        platform_net::destack_net_join_multicast_v4(
                            handle,
                            group,
                            interface_address,
                        )
                    }
                };
                self.status_result(status, "join multicast")
            }
        }
    }

    /// Leave a multicast group.
    pub(crate) fn leave_multicast(
        &mut self,
        handle: SocketHandle,
        group: &str,
        interface_address: &str,
    ) -> RuntimeResult<()> {
        let is_ipv6_group = group.contains(':');
        match self.vm_context_mut() {
            Some(context) => {
                let group = host_from_vm(context, group);
                if is_ipv6_group {
                    let interface_index = interface_address.parse::<u32>().unwrap_or(0);
                    platform_vm::destack_net_leave_multicast_v6(
                        self.call_context,
                        context,
                        handle,
                        group,
                        interface_index,
                    )
                } else {
                    let interface_address = host_from_vm(context, interface_address);
                    platform_vm::destack_net_leave_multicast_v4(
                        self.call_context,
                        context,
                        handle,
                        group,
                        interface_address,
                    )
                }
            }
            None => {
                let group = NativeStringRef::from(group);
                let status = if is_ipv6_group {
                    let interface_index = interface_address.parse::<u32>().unwrap_or(0);
                    unsafe {
                        platform_net::destack_net_leave_multicast_v6(handle, group, interface_index)
                    }
                } else {
                    let interface_address = NativeStringRef::from(interface_address);
                    unsafe {
                        platform_net::destack_net_leave_multicast_v4(
                            handle,
                            group,
                            interface_address,
                        )
                    }
                };
                self.status_result(status, "leave multicast")
            }
        }
    }

    /// Set multicast loopback mode.
    pub(crate) fn set_multicast_loop(
        &mut self,
        handle: SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_net_set_multicast_loop(
                self.call_context,
                context,
                handle,
                enabled,
            ),
            None => {
                let status =
                    unsafe { platform_net::destack_net_set_multicast_loop(handle, enabled) };
                self.status_result(status, "set multicast loop")
            }
        }
    }

    /// Set multicast TTL.
    pub(crate) fn set_multicast_ttl(
        &mut self,
        handle: SocketHandle,
        ttl: u32,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_set_multicast_ttl(self.call_context, context, handle, ttl)
            }
            None => {
                let status = unsafe { platform_net::destack_net_set_multicast_ttl(handle, ttl) };
                self.status_result(status, "set multicast ttl")
            }
        }
    }

    /// Resolve a hostname into socket addresses.
    pub(crate) fn resolve(
        &mut self,
        host: &str,
        port: u16,
        family: SocketFamily,
        flags: ResolveFlags,
    ) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
        match self.vm_context_mut() {
            Some(context) => {
                let host = host_from_vm(context, host);
                let addresses = platform_vm::destack_net_resolve(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                    flags,
                )?;
                socket_addresses_vm(context, addresses)
            }
            None => {
                let host = NativeStringRef::from(host);
                let mut out = std::mem::MaybeUninit::<NativeArray<SocketAddress>>::uninit();
                let status = unsafe {
                    platform_net::destack_net_resolve(out.as_mut_ptr(), host, port, family, flags)
                };
                self.status_ok(status, "resolve")?;
                let addresses = unsafe { out.assume_init() };
                socket_addresses_native(addresses)
            }
        }
    }

    /// Reverse lookup an address into host names.
    pub(crate) fn reverse_lookup(
        &mut self,
        host: &str,
        port: u16,
        family: SocketFamily,
    ) -> RuntimeResult<Vec<String>> {
        let addresses = self.resolve(host, port, family, ResolveFlags(0))?;
        let (resolved_host, resolved_port, resolved_family) =
            addresses.first().cloned().ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "host",
                    "resolve returned no addresses",
                ))
                .boxed()
            })?;

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    &resolved_host,
                    resolved_port,
                    resolved_family,
                )?;
                let names =
                    platform_vm::destack_net_reverse_lookup(self.call_context, context, address)?;
                string_array_vm(context, names)
            }
            None => {
                let address = socket_address_native_from_host_port(
                    self.call_context,
                    &resolved_host,
                    resolved_port,
                    resolved_family,
                )?;
                let mut out = std::mem::MaybeUninit::<NativeArray<NativeStringRef>>::uninit();
                let status = unsafe {
                    platform_net::destack_net_reverse_lookup(out.as_mut_ptr(), address.address())
                };
                self.status_result(status, "reverse lookup")?;
                let names = unsafe { out.assume_init() };
                string_array_native(names)
            }
        }
    }

    /// Create a UDP socket.
    pub(crate) fn udp_socket(&mut self, family: SocketFamily) -> RuntimeResult<SocketHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_udp_socket(self.call_context, context, family)
            }
            None => {
                let mut handle = SocketHandle(ResourceId(0));
                let status = unsafe { platform_net::destack_net_udp_socket(&mut handle, family) };
                self.status_ok(status, "udp socket")?;
                Ok(handle)
            }
        }
    }

    /// Bind a UDP socket to an address.
    pub(crate) fn udp_bind(
        &mut self,
        handle: SocketHandle,
        host: &str,
        port: u16,
    ) -> RuntimeResult<()> {
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
                platform_vm::destack_net_udp_bind(self.call_context, context, handle, address)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let status =
                    unsafe { platform_net::destack_net_udp_bind(handle, address.address()) };
                self.status_ok(status, "udp bind")
            }
        }
    }

    /// Connect a UDP socket to a remote endpoint.
    pub(crate) fn udp_connect(
        &mut self,
        handle: SocketHandle,
        host: &str,
        port: u16,
    ) -> RuntimeResult<()> {
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
                platform_vm::destack_net_udp_connect(self.call_context, context, handle, address)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let status =
                    unsafe { platform_net::destack_net_udp_connect(handle, address.address()) };
                self.status_result(status, "udp connect")
            }
        }
    }

    /// Send a UDP packet to an address.
    pub(crate) fn udp_send_to(
        &mut self,
        handle: SocketHandle,
        host: &str,
        port: u16,
        buffer: &[u8],
    ) -> RuntimeResult<u64> {
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
                let buffer = VmSlice::from_bytes(context, buffer);
                platform_vm::destack_net_udp_send_to(
                    self.call_context,
                    context,
                    handle,
                    address,
                    buffer,
                    UdpMessageFlags(0),
                )
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let slice = native_slice(buffer);
                let mut out = 0u64;
                let status = unsafe {
                    platform_net::destack_net_udp_send_to(
                        &mut out,
                        handle,
                        address.address(),
                        slice,
                        UdpMessageFlags(0),
                    )
                };
                self.status_ok(status, "udp send_to")?;
                Ok(out)
            }
        }
    }

    /// Receive a UDP packet into the provided buffer.
    pub(crate) fn udp_recv_from(
        &mut self,
        handle: SocketHandle,
        buffer: &mut [u8],
    ) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffer = VmSlice::from_bytes(context, &vec![0u8; buffer.len()]);
                let receive = platform_vm::destack_net_udp_recv_from(
                    self.call_context,
                    context,
                    handle,
                    vm_buffer,
                    UdpMessageFlags(0),
                )?;
                let (host, port, family, bytes) = udp_receive_vm(context, receive)?;
                let read_bytes = vm_buffer.read_bytes(context)?;
                let count = bytes as usize;
                if count > buffer.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "buffer",
                        "read size exceeded buffer length",
                    ))
                    .boxed());
                }
                buffer[..count].copy_from_slice(&read_bytes[..count]);
                Ok((host, port, family, bytes))
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = std::mem::MaybeUninit::<UdpReceive>::uninit();
                let status = unsafe {
                    platform_net::destack_net_udp_recv_from(
                        out.as_mut_ptr(),
                        handle,
                        slice,
                        UdpMessageFlags(0),
                    )
                };
                self.status_ok(status, "udp recv_from")?;
                let receive = unsafe { out.assume_init() };
                udp_receive_native(receive)
            }
        }
    }

    /// Listen on a UNIX domain socket.
    pub(crate) fn uds_listen(
        &mut self,
        path: &std::path::Path,
        backlog: u32,
    ) -> RuntimeResult<ListenerHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm(context, path);
                platform_vm::destack_net_uds_listen(self.call_context, context, path, backlog)
            }
            None => {
                let (path_bytes, path_ref) = path_ref_native(path);
                let _ = path_bytes;
                let mut handle = ListenerHandle(ResourceId(0));
                let status =
                    unsafe { platform_net::destack_net_uds_listen(&mut handle, path_ref, backlog) };
                self.status_ok(status, "uds listen")?;
                Ok(handle)
            }
        }
    }

    /// Listen on a UNIX domain socket using an explicit UTF-16 path encoding.
    #[cfg(unix)]
    pub(crate) fn uds_listen_utf16(
        &mut self,
        path: &std::path::Path,
        backlog: u32,
    ) -> RuntimeResult<ListenerHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm_utf16(context, path);
                platform_vm::destack_net_uds_listen(self.call_context, context, path, backlog)
            }
            None => {
                let (path_utf16, path_ref) = path_ref_native_utf16(path);
                let _ = path_utf16;
                let mut handle = ListenerHandle(ResourceId(0));
                let status =
                    unsafe { platform_net::destack_net_uds_listen(&mut handle, path_ref, backlog) };
                self.status_ok(status, "uds listen utf16")?;
                Ok(handle)
            }
        }
    }

    /// Connect to a UNIX domain socket.
    pub(crate) fn uds_connect(&mut self, path: &std::path::Path) -> RuntimeResult<SocketHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm(context, path);
                platform_vm::destack_net_uds_connect(self.call_context, context, path)
            }
            None => {
                let (path_bytes, path_ref) = path_ref_native(path);
                let _ = path_bytes;
                let mut handle = SocketHandle(ResourceId(0));
                let status =
                    unsafe { platform_net::destack_net_uds_connect(&mut handle, path_ref) };
                self.status_ok(status, "uds connect")?;
                Ok(handle)
            }
        }
    }

    /// Connect to a UNIX domain socket using an explicit UTF-16 path encoding.
    #[cfg(unix)]
    pub(crate) fn uds_connect_utf16(
        &mut self,
        path: &std::path::Path,
    ) -> RuntimeResult<SocketHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = path_ref_vm_utf16(context, path);
                platform_vm::destack_net_uds_connect(self.call_context, context, path)
            }
            None => {
                let (path_utf16, path_ref) = path_ref_native_utf16(path);
                let _ = path_utf16;
                let mut handle = SocketHandle(ResourceId(0));
                let status =
                    unsafe { platform_net::destack_net_uds_connect(&mut handle, path_ref) };
                self.status_ok(status, "uds connect utf16")?;
                Ok(handle)
            }
        }
    }

    /// Accept a UNIX domain socket connection.
    pub(crate) fn uds_accept(&mut self, listener: ListenerHandle) -> RuntimeResult<SocketHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_uds_accept(self.call_context, context, listener)
            }
            None => {
                let mut handle = SocketHandle(ResourceId(0));
                let status = unsafe { platform_net::destack_net_uds_accept(&mut handle, listener) };
                self.status_ok(status, "uds accept")?;
                Ok(handle)
            }
        }
    }

    /// Close a UNIX domain socket listener.
    pub(crate) fn uds_close_listener(&mut self, handle: ListenerHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_net_uds_close_listener(self.call_context, context, handle)
            }
            None => {
                let status = unsafe { platform_net::destack_net_uds_close_listener(handle) };
                self.status_ok(status, "uds close listener")
            }
        }
    }
}

/// Native network harness backed by native bindings.
pub(crate) struct NativeNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeNetHarness {
    /// Create a new native network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM network harness backed by VM bindings.
#[allow(dead_code)]
pub(crate) struct VmNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

#[allow(dead_code)]
impl VmNetHarness {
    /// Create a new VM network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum NetHarnessHandle {
    /// Native network harness.
    Native(NativeNetHarness),
    /// VM network harness.
    Vm(VmNetHarness),
}

impl NetHarnessHandle {
    /// Run a native or VM call context around the callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(NetHarnessContext<'call>) -> R,
    {
        match self {
            NetHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(NetHarnessContext {
                        runtime: &harness.runtime,
                        call_context,
                        vm_context: None,
                    })
                })
            }
            NetHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::RuntimeContext<'_> as *mut ();
                        callback(NetHarnessContext {
                            runtime: &harness.runtime,
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run a test callback that returns a runtime result.
    pub(crate) fn run<F>(&self, callback: F)
    where
        F: for<'call> FnOnce(NetHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("network harness call should succeed");
    }
}

/// Run a test against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&NetHarnessHandle),
{
    let native = NetHarnessHandle::Native(NativeNetHarness::new());
    callback(&native);
    let vm = NetHarnessHandle::Vm(VmNetHarness::new());
    callback(&vm);
}

/// Run a test callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(NetHarnessContext<'call>) -> RuntimeResult<()>,
{
    // run the callback for each harness
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Build a NativeSlice from a mutable byte buffer.
pub(crate) fn native_slice_mut(buffer: &mut [u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
    }
}

/// Build a NativeSlice from an immutable byte buffer.
pub(crate) fn native_slice(buffer: &[u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_ptr() as *mut u8,
        len: buffer.len() as u32,
    }
}

/// Build a nested NativeSlice from immutable buffer slices.
pub(crate) fn native_slice_slices(buffers: &[NativeSlice<u8>]) -> NativeSlice<NativeSlice<u8>> {
    NativeSlice {
        data: buffers.as_ptr() as *mut NativeSlice<u8>,
        len: buffers.len() as u32,
    }
}

/// Build a nested NativeSlice from mutable buffer slices.
pub(crate) fn native_slice_slices_mut(
    buffers: &mut [NativeSlice<u8>],
) -> NativeSlice<NativeSlice<u8>> {
    NativeSlice {
        data: buffers.as_mut_ptr(),
        len: buffers.len() as u32,
    }
}

/// Build a VM slice that contains VM byte-slices.
fn vm_slice_of_slices(
    context: &mut vm::RuntimeContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .map(|slice| slice.to_value(context))
        .collect::<Vec<_>>();
    let data = context.allocate_raw_values(values);
    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData::<VmSlice<u8>>,
    }
}

fn host_from_vm(context: &mut vm::RuntimeContext<'_>, host: &str) -> vm::StringHandle {
    let value = context.intern_string(host);
    vm::StringHandle::new(value)
}

/// Decode a native raw socket address for assertions.
fn socket_address_raw_native(address: SocketAddress) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = unsafe { address.bytes.as_slice()? };
    Ok((address.family, bytes.to_vec()))
}

/// Decode a VM raw socket address for assertions.
fn socket_address_raw_vm(
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = address.bytes.read_values(context)?;
    Ok((address.family, bytes))
}

fn socket_addresses_native(
    addresses: NativeArray<SocketAddress>,
) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
    let addresses = unsafe { addresses.as_slice()? };
    let mut decoded = Vec::with_capacity(addresses.len());
    for address in addresses {
        let bytes = unsafe { address.bytes.as_slice()? };
        decoded.push(socket_address_from_raw(address.family, bytes)?);
    }
    Ok(decoded)
}

fn socket_addresses_vm(
    context: &mut vm::RuntimeContext<'_>,
    addresses: VmArray<SocketAddressVm>,
) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
    let values = addresses.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let address = socket_address_vm_from_value(context, value)?;
        let bytes = address.bytes.read_values(context)?;
        decoded.push(socket_address_from_raw(address.family, &bytes)?);
    }
    Ok(decoded)
}

/// Decode a native string array.
fn string_array_native(values: NativeArray<NativeStringRef>) -> RuntimeResult<Vec<String>> {
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let value = unsafe { value.as_str()? };
        decoded.push(value.to_string());
    }

    Ok(decoded)
}

/// Decode a VM string array.
fn string_array_vm(
    context: &mut vm::RuntimeContext<'_>,
    values: VmArray<vm::StringHandle>,
) -> RuntimeResult<Vec<String>> {
    let values = values.read_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let value = context
            .string_ref(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        decoded.push(value.as_str().to_string());
    }

    Ok(decoded)
}

fn udp_receive_native(receive: UdpReceive) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
    let bytes = unsafe { receive.address.bytes.as_slice()? };
    let (host, port, family) = socket_address_from_raw(receive.address.family, bytes)?;
    Ok((host, port, family, receive.bytes))
}

fn udp_receive_vm(
    context: &mut vm::RuntimeContext<'_>,
    receive: UdpReceiveVm,
) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
    let bytes = receive.address.bytes.read_values(context)?;
    let (host, port, family) = socket_address_from_raw(receive.address.family, &bytes)?;
    Ok((host, port, family, receive.bytes))
}

fn socket_address_from_raw(
    family: u16,
    bytes: &[u8],
) -> RuntimeResult<(String, u16, SocketFamily)> {
    if family == libc::AF_INET as u16 {
        if bytes.len() < std::mem::size_of::<libc::sockaddr_in>() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv4 socket address bytes",
            ))
            .boxed());
        }

        let mut storage = std::mem::MaybeUninit::<libc::sockaddr_in>::zeroed();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                storage.as_mut_ptr() as *mut u8,
                std::mem::size_of::<libc::sockaddr_in>(),
            );
            let address = storage.assume_init();
            let ip = std::net::Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes());
            let port = u16::from_be(address.sin_port);
            return Ok((ip.to_string(), port, SocketFamily::IPv4));
        }
    }

    if family == libc::AF_INET6 as u16 {
        if bytes.len() < std::mem::size_of::<libc::sockaddr_in6>() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv6 socket address bytes",
            ))
            .boxed());
        }

        let mut storage = std::mem::MaybeUninit::<libc::sockaddr_in6>::zeroed();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                storage.as_mut_ptr() as *mut u8,
                std::mem::size_of::<libc::sockaddr_in6>(),
            );
            let address = storage.assume_init();
            let ip = std::net::Ipv6Addr::from(address.sin6_addr.s6_addr);
            let port = u16::from_be(address.sin6_port);
            return Ok((ip.to_string(), port, SocketFamily::IPv6));
        }
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "address.family",
        "unsupported address family",
    ))
    .boxed())
}

fn socket_address_native_from_host_port(
    _context: &RuntimeCallContext,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<NativeSocketAddressArg> {
    let (family, mut bytes) = match family {
        SocketFamily::IPv4 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(address) => Some(*address.ip()),
                    std::net::SocketAddr::V6(_) => None,
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut socket_address = unsafe { std::mem::zeroed::<libc::sockaddr_in>() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                socket_address.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            socket_address.sin_family = libc::AF_INET as libc::sa_family_t;
            socket_address.sin_port = port.to_be();
            socket_address.sin_addr = libc::in_addr {
                s_addr: u32::from_ne_bytes(ip.octets()),
            };

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &socket_address as *const _ as *const u8,
                    std::mem::size_of::<libc::sockaddr_in>(),
                )
            };
            (libc::AF_INET as u16, bytes.to_vec())
        }
        SocketFamily::IPv6 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(_) => None,
                    std::net::SocketAddr::V6(address) => Some(*address.ip()),
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut socket_address = unsafe { std::mem::zeroed::<libc::sockaddr_in6>() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                socket_address.sin6_len = std::mem::size_of::<libc::sockaddr_in6>() as u8;
            }
            socket_address.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            socket_address.sin6_port = port.to_be();
            socket_address.sin6_addr = libc::in6_addr {
                s6_addr: ip.octets(),
            };

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &socket_address as *const _ as *const u8,
                    std::mem::size_of::<libc::sockaddr_in6>(),
                )
            };
            (libc::AF_INET6 as u16, bytes.to_vec())
        }
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "unspecified family is not supported for literal address encoding",
            ))
            .boxed());
        }
    };

    let address = SocketAddress {
        family,
        length: bytes.len() as u32,
        bytes: NativeArray {
            data: bytes.as_mut_ptr(),
            len: bytes.len() as u32,
            capacity: bytes.len() as u32,
        },
    };

    Ok(NativeSocketAddressArg {
        _bytes: bytes,
        address,
    })
}

fn socket_address_vm_from_host_port(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<SocketAddressVm> {
    let address = socket_address_native_from_host_port(runtime, host, port, family)?;
    let bytes = VmArray::from_values(context, address.bytes()).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "address.bytes",
            format!("failed to encode vm byte array: {error}"),
        ))
        .boxed()
    })?;

    Ok(SocketAddressVm {
        family: address.address().family,
        length: address.address().length,
        bytes,
    })
}

/// Owns the raw bytes backing a native socket-address binding value.
struct NativeSocketAddressArg {
    /// The owned raw bytes storage.
    _bytes: Vec<u8>,
    /// The socket-address ABI payload pointing into `_bytes`.
    address: SocketAddress,
}

impl NativeSocketAddressArg {
    /// Return the socket-address payload.
    fn address(&self) -> SocketAddress {
        self.address
    }

    /// Return the socket-address raw bytes.
    fn bytes(&self) -> &[u8] {
        &self._bytes
    }
}

/// Decode a VM socket address aggregate value.
fn socket_address_vm_from_value(
    context: &mut vm::RuntimeContext<'_>,
    value: vm::Value,
) -> RuntimeResult<SocketAddressVm> {
    // decode aggregate fields
    if value.tag() != vm::ValueTag::Aggregate {
        return Err(RuntimeError::from(PlatformError::invalid_argument_type(
            "address",
            "SocketAddress",
        ))
        .boxed());
    }
    let slots = context
        .aggregate_slots(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    if slots.len() != 2 && slots.len() != 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "expected SocketAddress aggregate with 2 or 3 fields",
        ))
        .boxed());
    }

    // decode family and bytes with backward compatible shape support
    let family = <u16 as VmValueCodec>::decode(slots[0])?;
    let (length, bytes) = if slots.len() == 3 {
        let length = <u32 as VmValueCodec>::decode(slots[1])?;
        let bytes = VmArray::<u8>::from_value(context, slots[2], "address.bytes", "VmArray<u8>")?;

        (length, bytes)
    } else {
        let bytes = VmArray::<u8>::from_value(context, slots[1], "address.bytes", "VmArray<u8>")?;
        let length = bytes.len;

        (length, bytes)
    };

    Ok(SocketAddressVm {
        family,
        length,
        bytes,
    })
}

#[cfg(unix)]
fn path_ref_native(path: &std::path::Path) -> (Vec<u8>, OsPath) {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes().to_vec();
    let path_ref = OsPath {
        encoding: PathEncoding::Bytes,
        bytes: PathBytesAbi(NativeArray {
            data: bytes.as_ptr() as *mut u8,
            len: bytes.len() as u32,
            capacity: bytes.len() as u32,
        }),
        utf16: PathUtf16Abi(NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }),
    };
    (bytes, path_ref)
}

#[cfg(unix)]
fn path_ref_native_utf16(path: &std::path::Path) -> (Vec<u16>, OsPath) {
    use std::os::unix::ffi::OsStrExt;

    // decode bytes as utf8 for deterministic utf16 test paths
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let path_ref = OsPath {
        encoding: PathEncoding::Utf16,
        bytes: PathBytesAbi(NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }),
        utf16: PathUtf16Abi(NativeArray {
            data: utf16.as_ptr() as *mut u16,
            len: utf16.len() as u32,
            capacity: utf16.len() as u32,
        }),
    };

    (utf16, path_ref)
}

#[cfg(windows)]
fn path_ref_native(path: &std::path::Path) -> (Vec<u16>, OsPath) {
    use std::os::windows::ffi::OsStrExt;
    let utf16: Vec<u16> = path.as_os_str().encode_wide().collect();
    let path_ref = OsPath {
        encoding: PathEncoding::Utf16,
        bytes: PathBytesAbi(NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }),
        utf16: PathUtf16Abi(NativeArray {
            data: utf16.as_ptr() as *mut u16,
            len: utf16.len() as u32,
            capacity: utf16.len() as u32,
        }),
    };
    (utf16, path_ref)
}

fn path_ref_vm(context: &mut vm::RuntimeContext<'_>, path: &std::path::Path) -> OsPathVm {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        let array = VmArray::from_bytes(context, bytes);
        OsPathVm {
            encoding: PathEncoding::Bytes,
            bytes: PathBytesAbi(array),
            utf16: PathUtf16Abi(
                VmArray::from_values(context, &[]).expect("empty vm utf16 should encode"),
            ),
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let utf16: Vec<u16> = path.as_os_str().encode_wide().collect();
        let array = VmArray::from_values(context, &utf16).expect("vm utf16 path should encode");
        OsPathVm {
            encoding: PathEncoding::Utf16,
            bytes: PathBytesAbi(VmArray::from_bytes(context, &[])),
            utf16: PathUtf16Abi(array),
        }
    }
}

#[cfg(unix)]
fn path_ref_vm_utf16(context: &mut vm::RuntimeContext<'_>, path: &std::path::Path) -> OsPathVm {
    use std::os::unix::ffi::OsStrExt;

    // decode bytes as utf8 for deterministic utf16 test paths
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let array = VmArray::from_values(context, &utf16).expect("vm utf16 path should encode");

    OsPathVm {
        encoding: PathEncoding::Utf16,
        bytes: PathBytesAbi(VmArray::from_bytes(context, &[])),
        utf16: PathUtf16Abi(array),
    }
}

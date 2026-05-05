use destack_vm as vm;

use super::TlsHarnessContext;
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
#[cfg(windows)]
use crate::platform::net::{SocketFamily, SocketProtocol};
use crate::platform::net::{SocketPair, SocketType, native as net_native, vm as net_vm};
use crate::platform::tls::{TlsContextOptions, TlsContextOptionsVm, TlsRole, TlsVersion};
use crate::platform::{ResourceId, VmSlice, resource};
use crate::tests::platform::{vm_test_string, vm_test_values};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

type ByteSlicesValue = HarnessValue<NativeSlice<NativeSlice<u8>>, VmSlice<VmSlice<u8>>>;

impl<'call> TlsHarnessContext<'call> {
    /// Return one VM context for this harness call.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::BindingContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
    }

    /// Build one backend-specific context options value.
    pub(crate) fn context_options_value(
        &mut self,
        role: TlsRole,
        min_version: TlsVersion,
        max_version: TlsVersion,
        verify_peer: bool,
        alpn_protocols: &[&[u8]],
    ) -> RuntimeResult<HarnessValue<TlsContextOptions, TlsContextOptionsVm>> {
        match self.vm_context_mut() {
            Some(_) => {
                let alpn_protocols = self.bytes_slices_value(alpn_protocols)?;
                let alpn_protocols = match alpn_protocols {
                    HarnessValue::Vm(value) => value,
                    HarnessValue::Native(_) => unreachable!("vm context should return vm values"),
                };
                Ok(self.harness_value_vm(TlsContextOptionsVm {
                    role,
                    min_version,
                    max_version,
                    verify_peer,
                    alpn_protocols,
                }))
            }
            None => {
                let alpn_protocols = self.bytes_slices_value(alpn_protocols)?;
                let alpn_protocols = match alpn_protocols {
                    HarnessValue::Native(value) => value,
                    HarnessValue::Vm(_) => {
                        unreachable!("native context should return native values")
                    }
                };
                Ok(self.harness_value(TlsContextOptions {
                    role,
                    min_version,
                    max_version,
                    verify_peer,
                    alpn_protocols,
                }))
            }
        }
    }

    /// Build default client context options for the active harness backend.
    pub(crate) fn default_client_context_options(
        &mut self,
    ) -> RuntimeResult<HarnessValue<TlsContextOptions, TlsContextOptionsVm>> {
        self.context_options_value(
            TlsRole::Client,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            true,
            &[],
        )
    }

    /// Build a backend-specific string value.
    pub(crate) fn string_value(
        &mut self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        match self.vm_context_mut() {
            Some(context) => self.harness_value_vm(vm_test_string(context, value)),
            None => self.harness_value(self.call_context.store_string(value)),
        }
    }

    /// Build a backend-specific string-slice value.
    pub(crate) fn string_slice_value(
        &mut self,
        values: &[&str],
    ) -> RuntimeResult<HarnessValue<NativeStringSlice, VmSlice<vm::StringHandle>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let handles = values
                    .iter()
                    .map(|value| vm_test_string(context, value))
                    .collect::<Vec<_>>();
                let values = VmSlice::from_values(&mut context.write(), &handles)?;
                Ok(self.harness_value_vm(values))
            }
            None => {
                let values = values
                    .iter()
                    .map(|value| self.call_context.store_string(value))
                    .collect::<Vec<_>>();
                Ok(self.harness_value(self.call_context.store_string_slice(values)))
            }
        }
    }

    /// Build a backend-specific byte-slice value.
    pub(crate) fn bytes_value(
        &mut self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = VmSlice::from_bytes(&mut context.write(), bytes)?;
                Ok(self.harness_value_vm(bytes))
            }
            None => {
                let bytes = self.call_context.store_slice(bytes.to_vec());
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Build a backend-specific zeroed byte-slice value.
    pub(crate) fn zeroed_bytes_value(
        &mut self,
        length: usize,
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        let bytes = vec![0u8; length];
        self.bytes_value(&bytes)
    }

    /// Build one backend-specific nested byte-slice value.
    pub(crate) fn bytes_slices_value(
        &mut self,
        values: &[&[u8]],
    ) -> RuntimeResult<ByteSlicesValue> {
        match self.vm_context_mut() {
            Some(context) => {
                let mut protocols = Vec::with_capacity(values.len());
                for value in values {
                    protocols.push(VmSlice::from_bytes(&mut context.write(), value)?);
                }
                let values = vm_slice_of_slices(context, &protocols);
                Ok(self.harness_value_vm(values))
            }
            None => {
                let mut protocols = Vec::with_capacity(values.len());
                for value in values {
                    protocols.push(self.call_context.store_slice(value.to_vec()));
                }
                let values = self.call_context.store_slice(protocols);
                Ok(self.harness_value(values))
            }
        }
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
    pub(crate) fn bytes_from_value(
        &mut self,
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

    /// Create one connected stream socket pair.
    pub(crate) fn socket_pair(
        &mut self,
    ) -> RuntimeResult<(resource::SocketHandle, resource::SocketHandle)> {
        #[cfg(unix)]
        {
            match self.vm_context_mut() {
                Some(context) => {
                    let pair = net_vm::destack_net_uds_socket_pair(
                        self.call_context,
                        context,
                        stream_socket_type(),
                    )?;
                    Ok((pair.first, pair.second))
                }
                None => {
                    let mut pair = SocketPair {
                        first: resource::SocketHandle(ResourceId(0)),
                        second: resource::SocketHandle(ResourceId(0)),
                    };
                    unsafe {
                        net_native::destack_net_uds_socket_pair(
                            self.call_context,
                            &mut pair,
                            stream_socket_type(),
                        )?;
                    }
                    Ok((pair.first, pair.second))
                }
            }
        }

        #[cfg(windows)]
        {
            match self.vm_context_mut() {
                Some(context) => {
                    let pair = net_vm::destack_net_socket_pair(
                        self.call_context,
                        context,
                        stream_socket_family(),
                        stream_socket_type(),
                        stream_socket_protocol(),
                    )?;
                    Ok((pair.first, pair.second))
                }
                None => {
                    let mut pair = SocketPair {
                        first: resource::SocketHandle(ResourceId(0)),
                        second: resource::SocketHandle(ResourceId(0)),
                    };
                    unsafe {
                        net_native::destack_net_socket_pair(
                            self.call_context,
                            &mut pair,
                            stream_socket_family(),
                            stream_socket_type(),
                            stream_socket_protocol(),
                        )?;
                    }
                    Ok((pair.first, pair.second))
                }
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            unreachable!("tls tests require unix or windows");
        }
    }

    /// Set one socket nonblocking mode.
    pub(crate) fn set_socket_nonblocking(
        &mut self,
        handle: resource::SocketHandle,
        enabled: bool,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                net_vm::destack_net_set_nonblocking(self.call_context, context, handle, enabled)
            }
            None => unsafe {
                net_native::destack_net_set_nonblocking(self.call_context, handle, enabled)
            },
        }
    }

    /// Close one socket handle.
    pub(crate) fn close_socket(&mut self, handle: resource::SocketHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => net_vm::destack_net_close(self.call_context, context, handle),
            None => unsafe { net_native::destack_net_close(self.call_context, handle) },
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

#[cfg(unix)]
fn stream_socket_type() -> SocketType {
    SocketType(libc::SOCK_STREAM as u32)
}

#[cfg(windows)]
fn stream_socket_type() -> SocketType {
    SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32)
}

#[cfg(windows)]
fn stream_socket_family() -> SocketFamily {
    SocketFamily::IPv4
}

#[cfg(windows)]
fn stream_socket_protocol() -> SocketProtocol {
    SocketProtocol(windows_sys::Win32::Networking::WinSock::IPPROTO_TCP)
}

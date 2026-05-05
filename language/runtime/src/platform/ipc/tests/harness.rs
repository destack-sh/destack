use super::IpcHarnessContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
#[cfg(target_os = "linux")]
use crate::platform::ipc::{MessageQueueReceive, MessageQueueReceiveVm};
use crate::platform::ipc::{UnixPeerCredentials, UnixReceiveAncillary, UnixReceiveAncillaryVm};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::tests::platform::vm_test_string;

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> IpcHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut destack_vm::BindingContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut destack_vm::BindingContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &mut self,
        value: &str,
    ) -> RuntimeResult<HarnessValue<NativeStringRef, destack_vm::StringHandle>> {
        match self.vm_context_mut() {
            Some(context) => {
                let handle = vm_test_string(context, value);
                Ok(self.harness_value_vm(handle))
            }
            None => {
                let value = self.call_context.store_string(value);
                Ok(self.harness_value(value))
            }
        }
    }

    /// Build one backend-specific byte-slice value.
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
                let bytes = NativeSlice {
                    data: bytes.as_ptr() as *mut u8,
                    len: bytes.len() as u32,
                };
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Build one mutable backend-specific byte-slice value.
    pub(crate) fn mutable_bytes_value(
        &mut self,
        bytes: &mut [u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = VmSlice::from_bytes(&mut context.write(), bytes)?;
                Ok(self.harness_value_vm(bytes))
            }
            None => {
                let bytes = NativeSlice {
                    data: bytes.as_mut_ptr(),
                    len: bytes.len() as u32,
                };
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Decode one backend-specific byte-slice value into owned bytes.
    pub(crate) fn bytes_from_value(
        &mut self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        match value {
            HarnessValue::Native(value) => {
                let bytes = unsafe { value.as_slice()? };
                Ok(bytes.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm byte-slice values",
                    ))
                    .boxed()
                })?;
                value.read_bytes(&context.read())
            }
        }
    }

    /// Build one backend-specific transferred-handle slice value.
    pub(crate) fn transferred_handles_value(
        &mut self,
        handles: &[resource::TransferredHandle],
    ) -> RuntimeResult<
        HarnessValue<
            NativeSlice<resource::TransferredHandle>,
            VmSlice<resource::TransferredHandle>,
        >,
    > {
        match self.vm_context_mut() {
            Some(context) => {
                let handles = VmSlice::from_values(&mut context.write(), handles)?;
                Ok(self.harness_value_vm(handles))
            }
            None => {
                let handles = NativeSlice {
                    data: handles.as_ptr() as *mut resource::TransferredHandle,
                    len: handles.len() as u32,
                };
                Ok(self.harness_value(handles))
            }
        }
    }

    /// Decode one message-queue receive payload to the native view.
    #[cfg(target_os = "linux")]
    pub(crate) fn message_queue_receive_value(
        &self,
        value: HarnessValue<MessageQueueReceive, MessageQueueReceiveVm>,
    ) -> MessageQueueReceive {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one unix ancillary payload to concrete values.
    pub(crate) fn unix_receive_value(
        &mut self,
        value: HarnessValue<UnixReceiveAncillary, UnixReceiveAncillaryVm>,
    ) -> RuntimeResult<(
        u64,
        Vec<resource::TransferredHandle>,
        Option<UnixPeerCredentials>,
    )> {
        match value {
            HarnessValue::Native(value) => {
                let handles = unsafe { value.handles.as_slice()? };
                Ok((value.bytes, handles.to_vec(), value.credentials))
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm unix ancillary values",
                    ))
                    .boxed()
                })?;
                let handles = value.handles.read_values(&context.read())?;
                Ok((value.bytes, handles, value.credentials))
            }
        }
    }
}

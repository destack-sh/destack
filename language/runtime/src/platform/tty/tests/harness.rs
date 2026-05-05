use super::TtyHarnessContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::tty::{TtyMode, TtyModeVm, TtySize, TtySizeVm};
use crate::platform::{PlatformError, VmSlice};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> TtyHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut destack_vm::BindingContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut destack_vm::BindingContext<'_>) })
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

    /// Decode one backend-specific byte-slice value into bytes.
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

    /// Build one native tty mode wrapper.
    pub(crate) fn tty_mode_value(&mut self, mode: TtyMode) -> HarnessValue<TtyMode, TtyModeVm> {
        self.harness_value_from(mode)
            .expect("tty mode should encode")
    }

    /// Build one native tty size wrapper.
    pub(crate) fn tty_size_value(&mut self, size: TtySize) -> HarnessValue<TtySize, TtySizeVm> {
        self.harness_value_from(size)
            .expect("tty size should encode")
    }
}

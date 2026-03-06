use destack_vm as vm;

use super::ThreadHarnessContext;
use crate::platform::thread::{ThreadOptions, ThreadOptionsVm};
use crate::runtime::NativeStringRef;

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> ThreadHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &mut self,
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

    /// Build one backend-specific thread options value.
    pub(crate) fn thread_options_value(
        &mut self,
        options: ThreadOptions,
    ) -> HarnessValue<ThreadOptions, ThreadOptionsVm> {
        match self.vm_context_mut() {
            Some(_) => {
                let options = ThreadOptionsVm {
                    stack_bytes: options.stack_bytes,
                    flags: options.flags,
                };
                self.harness_value_vm(options)
            }
            None => self.harness_value(options),
        }
    }
}

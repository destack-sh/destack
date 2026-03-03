use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ResourceKindVm;
use crate::platform::{PlatformError, ResourceKind};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> ResourceHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Decode one vm resource kind payload into a stable label.
    fn vm_kind_label(&mut self, kind: ResourceKindVm) -> RuntimeResult<String> {
        let context = self.vm_context_mut().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "kind",
                "missing vm context for vm resource kind",
            ))
            .boxed()
        })?;
        let value = context
            .string_ref(kind.0)
            .map_err(|error| RuntimeError::from(error).boxed())?;

        Ok(value.as_str().to_string())
    }

    /// Return one stable resource kind label for either backend.
    pub(crate) fn resource_kind_label_from_value(
        &mut self,
        value: HarnessValue<ResourceKind, ResourceKindVm>,
    ) -> RuntimeResult<String> {
        match value {
            HarnessValue::Native(kind) => Ok(kind.label().to_string()),
            HarnessValue::Vm(kind) => self.vm_kind_label(kind),
        }
    }
}

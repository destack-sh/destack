use crate::runtime::binding::BindingRegistry;
use destack_vm::{Isolate, Result as VmResult};

/// VM binding set for a platform domain.
#[derive(Debug, Clone, Copy)]
pub struct VmBindingSet {
    /// Domain name for diagnostics and registration.
    pub name: &'static str,
    /// Registration entrypoint for this binding set.
    pub install: fn(&mut BindingRegistry, &mut Isolate) -> VmResult<()>,
}

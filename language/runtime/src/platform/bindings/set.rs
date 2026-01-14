use crate::platform::host::HostContext;

use super::BindingRegistry;
use destack_vm::Isolate;

/// Binding set for a host domain.
#[derive(Debug, Clone, Copy)]
pub struct BindingSet {
    /// Domain name for diagnostics and registration.
    pub name: &'static str,
    /// Registration entrypoint for this binding set.
    pub install: fn(&mut BindingRegistry, &mut Isolate, &HostContext),
}

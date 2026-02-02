use std::sync::Arc;

use super::abi::PlatformStringRef;

/// Shared platform state for binding handlers.
#[derive(Debug)]
struct PlatformState {
    /// Process arguments available to bindings.
    args: Vec<String>,
    /// Precomputed string references for native bindings.
    args_refs: Vec<PlatformStringRef>,
}

/// Platform context available to binding installers.
#[derive(Debug, Clone)]
pub struct PlatformContext {
    /// Shared platform state for this runtime.
    state: Arc<PlatformState>,
}

impl PlatformContext {
    /// Create platform context with explicit process arguments.
    pub fn new(args: Vec<String>) -> Self {
        // build string references for native bindings
        let args_refs = args.iter().map(PlatformStringRef::from).collect();

        // store shared platform state
        Self {
            state: Arc::new(PlatformState { args, args_refs }),
        }
    }

    /// Return the process arguments.
    pub fn args(&self) -> &[String] {
        self.state.args.as_slice()
    }

    /// Return process arguments as native string references.
    pub fn args_refs(&self) -> &[PlatformStringRef] {
        self.state.args_refs.as_slice()
    }
}

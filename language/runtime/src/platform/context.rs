use std::sync::Arc;

use crate::runtime::NativeStringRef;

/// Platform context available to binding installers.
#[derive(Debug, Clone)]
pub struct PlatformContext {
    /// Process arguments available to bindings.
    args: Arc<[String]>,
    /// Precomputed string references for native bindings.
    args_refs: Arc<[NativeStringRef]>,
}

impl PlatformContext {
    /// Create platform context with explicit process arguments.
    pub fn new(args: Vec<String>) -> Self {
        // build string references for native bindings
        let args_refs: Vec<NativeStringRef> = args.iter().map(NativeStringRef::from).collect();

        // store shared platform data
        Self {
            args: Arc::from(args),
            args_refs: Arc::from(args_refs),
        }
    }

    /// Return the process arguments.
    pub fn args(&self) -> &[String] {
        self.args.as_ref()
    }

    /// Return process arguments as native string references.
    pub fn args_refs(&self) -> &[NativeStringRef] {
        self.args_refs.as_ref()
    }
}

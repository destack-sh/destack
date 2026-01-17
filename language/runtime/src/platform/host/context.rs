use std::sync::Arc;

use super::abi::HostStringRef;
use super::io::HostIo;

/// Shared host state for binding handlers.
#[derive(Debug)]
struct HostState {
    /// Process arguments available to bindings.
    args: Vec<String>,
    /// Precomputed string references for native bindings.
    args_refs: Vec<HostStringRef>,
}

/// Host context available to binding installers.
#[derive(Debug, Clone)]
pub struct HostContext {
    /// Shared host state for this runtime.
    state: Arc<HostState>,
    /// Host I/O streams.
    io: Arc<HostIo>,
}

impl HostContext {
    /// Create host context with explicit process arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self::new_with_io(args, HostIo::default())
    }

    /// Create host context with explicit I/O streams.
    pub fn new_with_io(args: Vec<String>, io: HostIo) -> Self {
        // build string references for native bindings
        let args_refs = args.iter().map(HostStringRef::from).collect();

        // store shared host state
        Self {
            state: Arc::new(HostState { args, args_refs }),
            io: Arc::new(io),
        }
    }

    /// Return a copy of this context with I/O streams applied.
    pub fn with_io(mut self, io: HostIo) -> Self {
        self.io = Arc::new(io);
        self
    }

    /// Return the process arguments.
    pub fn args(&self) -> &[String] {
        self.state.args.as_slice()
    }

    /// Return process arguments as native string references.
    pub fn args_refs(&self) -> &[HostStringRef] {
        self.state.args_refs.as_slice()
    }

    /// Return host I/O streams.
    pub fn io(&self) -> &HostIo {
        &self.io
    }
}

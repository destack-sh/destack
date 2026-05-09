use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{Decorator, GlobalNodeId, GlobalNodeIdAny, GlobalSymbolId, StaticExpression};

/// Macro expansion state for one DIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroTable {
    /// The module id of the macro table.
    pub module_id: ModuleId,
    /// Expanded macro invocations.
    pub invocations: Vec<MacroInvocation>,
}

impl MacroTable {
    /// Create an empty macro table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            invocations: Vec::new(),
        }
    }

    /// Store one expanded macro invocation.
    pub fn push(&mut self, invocation: MacroInvocation) {
        self.invocations.push(invocation);
    }

    /// Iterate expanded macro invocations.
    pub fn iter(&self) -> impl Iterator<Item = &MacroInvocation> + '_ {
        self.invocations.iter()
    }

    /// Return whether this table has no expanded macro invocations.
    pub fn is_empty(&self) -> bool {
        self.invocations.is_empty()
    }
}

/// One macro invocation completed during fixed-point expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroInvocation {
    /// The decorated target node.
    pub target_node: GlobalNodeIdAny,
    /// What caused this macro invocation.
    pub trigger: MacroTrigger,
    /// The resolved `Macro` implementation.
    pub implementation: GlobalSymbolId,
    /// The static macro configuration.
    pub config: StaticExpression,
    /// State carried from expansion to materialization.
    pub state: Option<StaticExpression>,
}

/// What caused one macro invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MacroTrigger {
    /// A decorator on the target node.
    Decorator(GlobalNodeId<Decorator>),
    /// A configured auto-derive provider.
    AutoDerive,
}

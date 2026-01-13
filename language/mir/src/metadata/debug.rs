use std::collections::HashMap;

use destack_base::StringId;
use destack_source::Span;

use crate::{Block, Function, Global, Instruction, Local, LocalNodeId, Type, Value};

/// Identifier for a debug scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DebugScopeId(u32);

impl DebugScopeId {
    /// Create a debug scope id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a debug variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DebugVariableId(u32);

impl DebugVariableId {
    /// Create a debug variable id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Debug scope kind for lexical regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugScopeKind {
    /// Function scope.
    Function,
    /// Lexical scope.
    Lexical,
    /// Inlined callsite scope.
    Inline,
}

/// Debug scope metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugScope {
    /// The kind of scope.
    pub kind: DebugScopeKind,
    /// Optional name for the scope.
    pub name: Option<StringId>,
    /// Source span for the scope.
    pub span: Span,
    /// Parent scope for nesting.
    pub parent: Option<DebugScopeId>,
}

/// Debug variable metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugVariable {
    /// Variable name.
    pub name: StringId,
    /// Variable type.
    pub ty: LocalNodeId<Type>,
    /// Scope containing the variable.
    pub scope: DebugScopeId,
    /// True when the variable is a parameter.
    pub is_parameter: bool,
    /// True when the variable is compiler synthesized.
    pub is_artificial: bool,
}

/// Location for a debug variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugValueLocation {
    /// The variable is stored in an SSA value.
    Value(Value),
    /// The variable is stored in a local slot.
    Local(LocalNodeId<Local>),
    /// The variable is stored in a global location.
    Global(LocalNodeId<Global>),
    /// The variable has no concrete location.
    Undefined,
}

/// Debug location for an instruction or block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugLocation {
    /// The source span for this location.
    pub span: Span,
    /// The scope containing this location.
    pub scope: DebugScopeId,
    /// Inline scope for inlined callsites.
    pub inlined_at: Option<DebugScopeId>,
}

/// Table of debug information for MIR nodes.
#[derive(Debug, Clone, Default)]
pub struct DebugInfoTable {
    /// Debug scopes indexed by id.
    pub scopes: Vec<DebugScope>,
    /// Debug variables indexed by id.
    pub variables: Vec<DebugVariable>,
    /// Function scopes keyed by function id.
    pub function_scopes: HashMap<LocalNodeId<Function>, DebugScopeId>,
    /// Block scopes keyed by block id.
    pub block_scopes: HashMap<LocalNodeId<Block>, DebugScopeId>,
    /// Instruction locations keyed by instruction id.
    pub instruction_locations: HashMap<LocalNodeId<Instruction>, DebugLocation>,
    /// Variable locations keyed by debug variable id.
    pub variable_locations: HashMap<DebugVariableId, DebugValueLocation>,
}

impl DebugInfoTable {
    /// Create a new empty debug info table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new debug scope entry.
    pub fn create_scope(
        &mut self,
        kind: DebugScopeKind,
        name: Option<StringId>,
        span: Span,
        parent: Option<DebugScopeId>,
    ) -> DebugScopeId {
        let id = DebugScopeId::new(self.scopes.len() as u32);
        self.scopes.push(DebugScope {
            kind,
            name,
            span,
            parent,
        });
        id
    }

    /// Create a new debug variable entry.
    pub fn create_variable(
        &mut self,
        name: StringId,
        ty: LocalNodeId<Type>,
        scope: DebugScopeId,
        is_parameter: bool,
        is_artificial: bool,
    ) -> DebugVariableId {
        let id = DebugVariableId::new(self.variables.len() as u32);
        self.variables.push(DebugVariable {
            name,
            ty,
            scope,
            is_parameter,
            is_artificial,
        });
        id
    }

    /// Return the debug scope for an id.
    pub fn scope(&self, id: DebugScopeId) -> &DebugScope {
        &self.scopes[id.index()]
    }

    /// Return the debug variable for an id.
    pub fn variable(&self, id: DebugVariableId) -> &DebugVariable {
        &self.variables[id.index()]
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Block, Constant, Function, Global, Instruction, Local, LocalNodeId, ProvenanceId, Type, Value,
};
use destack_core::StringId;

/// Table of debug information for MIR nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugMetadata {
    /// Debug scopes indexed by id.
    pub scopes: Vec<DebugScope>,
    /// Debug bindings indexed by id.
    pub bindings: Vec<DebugBinding>,
    /// Inlined call sites indexed by id.
    pub inline_calls: Vec<DebugInlineCall>,
    /// Function scopes keyed by function id.
    pub function_scopes: HashMap<LocalNodeId<Function>, DebugScopeId>,
    /// Block scopes keyed by block id.
    pub block_scopes: HashMap<LocalNodeId<Block>, DebugScopeId>,
    /// Instruction locations keyed by instruction id.
    pub instruction_locations: HashMap<LocalNodeId<Instruction>, DebugLocation>,
    /// Binding location ranges keyed by debug binding id.
    pub binding_ranges: HashMap<DebugBindingId, Vec<DebugBindingRange>>,
}

impl DebugMetadata {
    /// Create a new empty debug info table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new debug scope entry.
    pub fn create_scope(
        &mut self,
        name: Option<StringId>,
        provenance: Option<ProvenanceId>,
        parent: Option<DebugScopeId>,
    ) -> DebugScopeId {
        let id = DebugScopeId::new(self.scopes.len() as u32);
        self.scopes.push(DebugScope {
            name,
            provenance,
            parent,
        });
        id
    }

    /// Create a new debug binding entry.
    pub fn create_binding(
        &mut self,
        name: StringId,
        ty: LocalNodeId<Type>,
        scope: DebugScopeId,
        provenance: Option<ProvenanceId>,
        kind: DebugBindingKind,
    ) -> DebugBindingId {
        let id = DebugBindingId::new(self.bindings.len() as u32);
        self.bindings.push(DebugBinding {
            name,
            ty,
            scope,
            provenance,
            kind,
        });
        id
    }

    /// Create a new inline call entry.
    pub fn create_inline_call(
        &mut self,
        callee_scope: DebugScopeId,
        call_location: DebugLocation,
    ) -> DebugInlineCallId {
        let id = DebugInlineCallId::new(self.inline_calls.len() as u32);
        self.inline_calls.push(DebugInlineCall {
            callee_scope,
            call_location,
        });
        id
    }

    /// Record the function scope for one function.
    pub fn set_function_scope(
        &mut self,
        function_id: LocalNodeId<Function>,
        scope_id: DebugScopeId,
    ) {
        self.function_scopes.insert(function_id, scope_id);
    }

    /// Record the lexical scope for one block.
    pub fn set_block_scope(&mut self, block_id: LocalNodeId<Block>, scope_id: DebugScopeId) {
        self.block_scopes.insert(block_id, scope_id);
    }

    /// Record the debug location for one instruction.
    pub fn set_instruction_location(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        location: DebugLocation,
    ) {
        self.instruction_locations.insert(instruction_id, location);
    }

    /// Append one binding location range.
    pub fn add_binding_range(
        &mut self,
        binding: DebugBindingId,
        location: DebugValueLocation,
        start: Option<LocalNodeId<Instruction>>,
        end: Option<LocalNodeId<Instruction>>,
    ) {
        self.binding_ranges
            .entry(binding)
            .or_default()
            .push(DebugBindingRange {
                location,
                start,
                end,
            });
    }

    /// Return the debug scope for an id.
    pub fn scope(&self, id: DebugScopeId) -> &DebugScope {
        &self.scopes[id.index()]
    }

    /// Return the debug binding for an id.
    pub fn binding(&self, id: DebugBindingId) -> &DebugBinding {
        &self.bindings[id.index()]
    }

    /// Return the inline call for an id.
    pub fn inline_call(&self, id: DebugInlineCallId) -> &DebugInlineCall {
        &self.inline_calls[id.index()]
    }
}

/// Identifier for a debug scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugScopeId(u32);

impl DebugScopeId {
    /// Create a debug scope id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw table index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a debug binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugBindingId(u32);

impl DebugBindingId {
    /// Create a debug binding id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw table index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an inlined call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugInlineCallId(u32);

impl DebugInlineCallId {
    /// Create an inline call id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw table index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Debug scope metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugScope {
    /// Optional display name.
    pub name: Option<StringId>,
    /// The provenance record for this scope when one exists.
    pub provenance: Option<ProvenanceId>,
    /// Parent scope for nesting.
    pub parent: Option<DebugScopeId>,
}

/// Debug binding kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugBindingKind {
    /// Function parameter binding.
    Parameter,
    /// Local binding.
    Local,
    /// Captured binding.
    Capture,
    /// Compiler-synthesized binding.
    Synthetic,
}

/// Debug binding metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBinding {
    /// Binding name.
    pub name: StringId,
    /// Binding type.
    pub ty: LocalNodeId<Type>,
    /// Scope containing the binding.
    pub scope: DebugScopeId,
    /// The provenance record for this binding when one exists.
    pub provenance: Option<ProvenanceId>,
    /// Binding category.
    pub kind: DebugBindingKind,
}

/// One inlined call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugInlineCall {
    /// The inlined callee function scope.
    pub callee_scope: DebugScopeId,
    /// The source location of the call expression.
    pub call_location: DebugLocation,
}

/// Debug location for an instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugLocation {
    /// The scope containing this location.
    pub scope: DebugScopeId,
    /// The provenance record for this location when one exists.
    pub provenance: Option<ProvenanceId>,
    /// The inlined call containing this location.
    pub inline_call: Option<DebugInlineCallId>,
}

/// One fragment of a split debug value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugValueFragment {
    /// Byte offset within the logical binding value.
    pub offset_bytes: u32,
    /// Byte size covered by this piece.
    pub size_bytes: u32,
    /// Storage for this piece.
    pub location: Box<DebugValueLocation>,
}

/// Storage location for a debug binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugValueLocation {
    /// The binding is stored in an SSA value.
    Value(Value),
    /// The binding is stored in a local slot.
    Local(LocalNodeId<Local>),
    /// The binding is stored in a global location.
    Global(LocalNodeId<Global>),
    /// The binding is represented by a constant.
    Constant(Constant),
    /// The binding is assembled from multiple fragments.
    Composite(Vec<DebugValueFragment>),
    /// The binding existed semantically but is unavailable here.
    OptimizedOut,
    /// The binding has no meaningful value here.
    Undefined,
}

/// One binding location valid over a half-open instruction range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugBindingRange {
    /// The storage location over the covered range.
    pub location: DebugValueLocation,
    /// The first covered instruction, or function entry.
    pub start: Option<LocalNodeId<Instruction>>,
    /// The first instruction after the covered range, if bounded.
    pub end: Option<LocalNodeId<Instruction>>,
}

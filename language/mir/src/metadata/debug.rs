use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Block, Constant, Function, Global, Instruction, Local, LocalNodeId, ProvenanceId, Type, Value,
};
use destack_core::StringId;

/// Identifier for a debug scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Identifier for a debug binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugBindingId(u32);

impl DebugBindingId {
    /// Create a debug binding id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a debug type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugTypeId(u32);

impl DebugTypeId {
    /// Create a debug type id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an inline site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugInlineSiteId(u32);

impl DebugInlineSiteId {
    /// Create an inline site id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a coroutine state mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugCoroutineStateId(u32);

impl DebugCoroutineStateId {
    /// Create a coroutine state id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Debug scope kind for lexical regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugScopeKind {
    /// Function scope.
    Function,
    /// Lexical scope.
    Lexical,
}

/// Debug scope metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugScope {
    /// Optional display name.
    pub name: Option<StringId>,
    /// The kind of scope.
    pub kind: DebugScopeKind,
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

/// Debug type metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugType {
    /// Optional display name for the type.
    pub name: Option<StringId>,
    /// MIR type represented by this debug type.
    pub ty: LocalNodeId<Type>,
    /// The provenance record for this debug type when one exists.
    pub provenance: Option<ProvenanceId>,
}

/// One explicit inline call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugInlineSite {
    /// Scope of the inlined callee.
    pub callee_scope: DebugScopeId,
    /// Source location of the call site.
    pub call_location: DebugLocation,
    /// The provenance record for this inline site when one exists.
    pub provenance: Option<ProvenanceId>,
    /// Parent inline site for nested inlining.
    pub parent: Option<DebugInlineSiteId>,
}

/// Debug location for an instruction or block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugLocation {
    /// The scope containing this location.
    pub scope: DebugScopeId,
    /// The provenance record for this location when one exists.
    pub provenance: Option<ProvenanceId>,
    /// Inline provenance for this location.
    pub inline_site: Option<DebugInlineSiteId>,
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

/// Availability state for a debug binding value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugValueState {
    /// The binding existed semantically but is unavailable here.
    OptimizedOut,
    /// The binding has no meaningful value here.
    Undefined,
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
    /// The binding has a non-location state here.
    State(DebugValueState),
}

/// The start of one debug binding location range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugRangeStart {
    /// The range begins at function entry.
    FunctionEntry,
    /// The range begins at one specific instruction.
    Instruction(LocalNodeId<Instruction>),
}

impl DebugRangeStart {
    /// Create one function-entry range start.
    pub fn function_entry() -> Self {
        Self::FunctionEntry
    }

    /// Create one instruction range start.
    pub fn instruction(instruction: LocalNodeId<Instruction>) -> Self {
        Self::Instruction(instruction)
    }

    /// Return the instruction for this start, when present.
    pub fn instruction_id(self) -> Option<LocalNodeId<Instruction>> {
        match self {
            Self::FunctionEntry => None,
            Self::Instruction(instruction) => Some(instruction),
        }
    }
}

/// One binding location valid over a half-open instruction range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugBindingLocationRange {
    /// The binding being described.
    pub binding: DebugBindingId,
    /// The storage location over the covered range.
    pub location: DebugValueLocation,
    /// The start of the covered range.
    pub start: DebugRangeStart,
    /// The first instruction after the covered range, if bounded.
    pub end: Option<LocalNodeId<Instruction>>,
}

/// Debug metadata for one lowered coroutine state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugCoroutineState {
    /// Scope for the logical coroutine body.
    pub scope: DebugScopeId,
    /// Source location of the suspend point.
    pub suspend_location: DebugLocation,
    /// The provenance record for this coroutine state when one exists.
    pub provenance: Option<ProvenanceId>,
    /// Bindings lifted into coroutine state.
    pub lifted_bindings: Vec<DebugBindingId>,
}

/// Table of debug information for MIR nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Debug {
    /// Debug scopes indexed by id.
    pub scopes: Vec<DebugScope>,
    /// Debug bindings indexed by id.
    pub bindings: Vec<DebugBinding>,
    /// Debug types indexed by id.
    pub types: Vec<DebugType>,
    /// Inline sites indexed by id.
    pub inline_sites: Vec<DebugInlineSite>,
    /// Coroutine state mappings indexed by id.
    pub coroutine_states: Vec<DebugCoroutineState>,
    /// Function scopes keyed by function id.
    pub function_scopes: HashMap<LocalNodeId<Function>, DebugScopeId>,
    /// Block scopes keyed by block id.
    pub block_scopes: HashMap<LocalNodeId<Block>, DebugScopeId>,
    /// Instruction locations keyed by instruction id.
    pub instruction_locations: HashMap<LocalNodeId<Instruction>, DebugLocation>,
    /// Binding location histories keyed by debug binding id.
    pub binding_location_ranges: HashMap<DebugBindingId, Vec<DebugBindingLocationRange>>,
}

impl Debug {
    /// Create a new empty debug info table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new debug scope entry.
    pub fn create_scope(
        &mut self,
        kind: DebugScopeKind,
        name: Option<StringId>,
        provenance: Option<ProvenanceId>,
        parent: Option<DebugScopeId>,
    ) -> DebugScopeId {
        let id = DebugScopeId::new(self.scopes.len() as u32);
        self.scopes.push(DebugScope {
            name,
            kind,
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

    /// Create a new debug type entry.
    pub fn create_type(
        &mut self,
        name: Option<StringId>,
        ty: LocalNodeId<Type>,
        provenance: Option<ProvenanceId>,
    ) -> DebugTypeId {
        let id = DebugTypeId::new(self.types.len() as u32);
        self.types.push(DebugType {
            name,
            ty,
            provenance,
        });
        id
    }

    /// Create a new inline site entry.
    pub fn create_inline_site(
        &mut self,
        callee_scope: DebugScopeId,
        call_location: DebugLocation,
        provenance: Option<ProvenanceId>,
        parent: Option<DebugInlineSiteId>,
    ) -> DebugInlineSiteId {
        let id = DebugInlineSiteId::new(self.inline_sites.len() as u32);
        self.inline_sites.push(DebugInlineSite {
            callee_scope,
            call_location,
            provenance,
            parent,
        });
        id
    }

    /// Create a new coroutine state entry.
    pub fn create_coroutine_state(
        &mut self,
        scope: DebugScopeId,
        suspend_location: DebugLocation,
        provenance: Option<ProvenanceId>,
        lifted_bindings: Vec<DebugBindingId>,
    ) -> DebugCoroutineStateId {
        let id = DebugCoroutineStateId::new(self.coroutine_states.len() as u32);
        self.coroutine_states.push(DebugCoroutineState {
            scope,
            suspend_location,
            provenance,
            lifted_bindings,
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
    pub fn add_binding_location_range(
        &mut self,
        binding: DebugBindingId,
        location: DebugValueLocation,
        start: DebugRangeStart,
        end: Option<LocalNodeId<Instruction>>,
    ) {
        self.binding_location_ranges
            .entry(binding)
            .or_default()
            .push(DebugBindingLocationRange {
                binding,
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

    /// Return the debug type for an id.
    pub fn debug_type(&self, id: DebugTypeId) -> &DebugType {
        &self.types[id.index()]
    }

    /// Return the inline site for an id.
    pub fn inline_site(&self, id: DebugInlineSiteId) -> &DebugInlineSite {
        &self.inline_sites[id.index()]
    }

    /// Return the coroutine state for an id.
    pub fn coroutine_state(&self, id: DebugCoroutineStateId) -> &DebugCoroutineState {
        &self.coroutine_states[id.index()]
    }
}

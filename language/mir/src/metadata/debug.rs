use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Block, Constant, Function, Global, Local, LocalNodeId, ProvenanceId, Type, Value};
use destack_core::StringId;

/// Table of debug information for MIR nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugMetadata {
    /// Debug scopes indexed by id.
    pub scopes: Vec<DebugScope>,
    /// Debug bindings indexed by id.
    pub bindings: Vec<DebugBinding>,
    /// Inline frames indexed by id.
    pub inline_frames: Vec<InlineFrame>,
    /// Function scopes keyed by function id.
    pub function_scopes: HashMap<LocalNodeId<Function>, DebugScopeId>,
    /// Block scopes keyed by block id.
    pub block_scopes: HashMap<LocalNodeId<Block>, DebugScopeId>,
    /// Debug locations keyed by source-level execution point.
    pub locations: HashMap<DebugPoint, DebugLocation>,
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

    /// Create a new inline frame entry.
    pub fn create_inline_frame(
        &mut self,
        callee_scope: DebugScopeId,
        call_location: DebugLocation,
    ) -> InlineFrameId {
        let id = InlineFrameId::new(self.inline_frames.len() as u32);
        self.inline_frames.push(InlineFrame {
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

    /// Record the debug location for one source-level execution point.
    pub fn set_location(&mut self, point: DebugPoint, location: DebugLocation) {
        self.locations.insert(point, location);
    }

    /// Append one binding location range.
    pub fn add_binding_range(
        &mut self,
        binding: DebugBindingId,
        value: DebugValue,
        range: DebugRange,
    ) {
        self.binding_ranges
            .entry(binding)
            .or_default()
            .push(DebugBindingRange { value, range });
    }

    /// Return the debug scope for an id.
    pub fn scope(&self, id: DebugScopeId) -> &DebugScope {
        &self.scopes[id.index()]
    }

    /// Return the debug binding for an id.
    pub fn binding(&self, id: DebugBindingId) -> &DebugBinding {
        &self.bindings[id.index()]
    }

    /// Return the inline frame for an id.
    pub fn inline_frame(&self, id: InlineFrameId) -> &InlineFrame {
        &self.inline_frames[id.index()]
    }
}

/// Source-level execution point inside one MIR function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugPoint {
    /// The owning function.
    pub function: LocalNodeId<Function>,
    /// The owning block.
    pub block: LocalNodeId<Block>,
    /// The MIR program point ordinal inside the block.
    ///
    /// Points are counted at block entry, after each instruction, and after the terminator.
    pub point: u32,
}

impl DebugPoint {
    /// Create one source-level execution point.
    pub const fn new(
        function: LocalNodeId<Function>,
        block: LocalNodeId<Block>,
        point: u32,
    ) -> Self {
        Self {
            function,
            block,
            point,
        }
    }
}

/// Half-open range over source-level execution points in one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugRange {
    /// The first covered point.
    pub start: DebugPoint,
    /// The first point after the covered range in the same block.
    pub end: DebugPoint,
}

impl DebugRange {
    /// Create one half-open same-block source-level range.
    pub const fn new(start: DebugPoint, end: DebugPoint) -> Self {
        Self { start, end }
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

/// Identifier for an inline frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InlineFrameId(u32);

impl InlineFrameId {
    /// Create an inline frame id from a raw index.
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

/// One inline frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineFrame {
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
    /// The inline frame containing this location.
    pub inline_frame: Option<InlineFrameId>,
}

/// One fragment of a split debug value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugValueFragment {
    /// Byte offset within the logical binding value.
    pub offset_bytes: u32,
    /// Byte size covered by this piece.
    pub size_bytes: u32,
    /// Storage for this piece.
    pub value: Box<DebugValue>,
}

/// Storage location for a debug binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugValue {
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
    /// The binding is not available at this point.
    Unavailable,
}

/// One binding value valid over a half-open debug range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugBindingRange {
    /// The value over the covered range.
    pub value: DebugValue,
    /// The covered debug range.
    pub range: DebugRange,
}

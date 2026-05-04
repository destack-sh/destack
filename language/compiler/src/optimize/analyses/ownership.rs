use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;
use mir::{Instruction, ReferenceKind, Type, Value};

use super::{ControlFlowGraph, DataflowResult, Lattice};
use crate::optimize::common::ValueTypeMap;
use crate::optimize::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

/// Location where a move occurred (for diagnostics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveLocation {
    /// Move occurred at an instruction.
    Instruction(mir::LocalNodeId<Instruction>),
    /// Move occurred at a block terminator.
    Terminator(mir::LocalNodeId<mir::Block>),
}

/// Ownership state for a single value.
///
/// Forms a lattice:
/// ```text
///        Owned (top - definitely usable)
///       /     \
///      /       \
/// MaybeMoved    |   (moved on some paths)
///      \       /
///       \     /
///        Moved (bottom - definitely moved)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    /// Value is owned and valid for use on all paths.
    Owned,
    /// Value is moved on some paths but not others.
    MaybeMoved { first_move: MoveLocation },
    /// Value is definitely moved on all paths.
    Moved { at: MoveLocation },
}

impl OwnershipState {
    /// Check if this state represents a moved value (definitely or maybe).
    pub fn is_moved(&self) -> bool {
        !matches!(self, OwnershipState::Owned)
    }

    /// Check if this is a "maybe moved" state.
    pub fn is_maybe_moved(&self) -> bool {
        matches!(self, OwnershipState::MaybeMoved { .. })
    }

    /// Get the move location if moved.
    pub fn move_location(&self) -> Option<&MoveLocation> {
        match self {
            OwnershipState::Owned => None,
            OwnershipState::MaybeMoved { first_move } => Some(first_move),
            OwnershipState::Moved { at } => Some(at),
        }
    }
}

/// Ownership state for all values and locals at a test point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnershipMap {
    /// Ownership state for values.
    values: HashMap<Value, OwnershipState>,
    /// Ownership state for locals.
    locals: HashMap<mir::LocalNodeId<mir::Local>, OwnershipState>,
    /// Origins for values.
    origins: HashMap<Value, mir::LocalNodeId<mir::Local>>,
}

impl OwnershipMap {
    /// Create an empty ownership map.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            locals: HashMap::new(),
            origins: HashMap::new(),
        }
    }

    /// Get the ownership state for a value.
    pub fn get(&self, value: impl Into<mir::ValueReference>) -> Option<&OwnershipState> {
        let value = value.into().value()?;
        self.values.get(&value)
    }

    /// Check if a value is owned (usable).
    pub fn is_owned(&self, value: impl Into<mir::ValueReference>) -> bool {
        self.get(value)
            .map(|s| matches!(s, OwnershipState::Owned))
            .unwrap_or(true) // unknown values are assumed owned
    }

    /// Check if a value is moved.
    pub fn is_moved(&self, value: impl Into<mir::ValueReference>) -> bool {
        self.get(value).is_some_and(|s| s.is_moved())
    }

    /// Mark a value as owned.
    pub fn mark_owned(&mut self, value: impl Into<mir::ValueReference>) {
        let Some(value) = value.into().value() else {
            return;
        };

        self.values.insert(value, OwnershipState::Owned);
    }

    /// Mark a value as moved.
    pub fn mark_moved(&mut self, value: impl Into<mir::ValueReference>, at: MoveLocation) {
        let Some(value) = value.into().value() else {
            return;
        };

        self.values.insert(value, OwnershipState::Moved { at });
    }

    /// Mark a value as moved and propagate to the source local when tracked.
    pub fn mark_moved_with_source(
        &mut self,
        value: impl Into<mir::ValueReference>,
        at: MoveLocation,
    ) {
        let value = value.into();
        self.mark_moved(value, at.clone());
        if let Some(local) = self.origin_for_value(value) {
            self.mark_local_moved(local, at);
        }
    }

    /// Mark a value as moved when it is not copy and propagate to locals.
    pub fn mark_moved_if_not_copy_with_source(
        &mut self,
        value: impl Into<mir::ValueReference>,
        at: MoveLocation,
        tree: &mir::Tree,
        value_types: &ValueTypeMap,
    ) {
        let value = value.into();

        if !value_is_copy(value, tree, value_types) {
            self.mark_moved_with_source(value, at);
        }
    }

    /// Get the origin local for a value, if known.
    pub fn origin_for_value(
        &self,
        value: impl Into<mir::ValueReference>,
    ) -> Option<mir::LocalReference> {
        let value = value.into().value()?;
        self.origins.get(&value).copied().map(Into::into)
    }

    /// Set the origin local for a value.
    pub fn set_origin(
        &mut self,
        value: impl Into<mir::ValueReference>,
        origin: Option<mir::LocalReference>,
    ) {
        let Some(value) = value.into().value() else {
            return;
        };

        let origin = origin.and_then(|origin| origin.local());

        if let Some(local) = origin {
            self.origins.insert(value, local);
        } else {
            self.origins.remove(&value);
        }
    }

    /// Get the ownership state for a local.
    pub fn local_state(&self, local: impl Into<mir::LocalReference>) -> Option<&OwnershipState> {
        let local = local.into().local()?;
        self.locals.get(&local)
    }

    /// Check if a local is owned (usable).
    pub fn local_is_owned(&self, local: impl Into<mir::LocalReference>) -> bool {
        self.locals
            .get(&match local.into().local() {
                Some(local) => local,
                None => return true,
            })
            .map(|s| matches!(s, OwnershipState::Owned))
            .unwrap_or(true) // unknown locals are assumed owned
    }

    /// Check if a local is moved.
    pub fn local_is_moved(&self, local: impl Into<mir::LocalReference>) -> bool {
        self.locals
            .get(&match local.into().local() {
                Some(local) => local,
                None => return false,
            })
            .map(|s| s.is_moved())
            .unwrap_or(false)
    }

    /// Mark a local as owned.
    pub fn mark_local_owned(&mut self, local: impl Into<mir::LocalReference>) {
        let Some(local) = local.into().local() else {
            return;
        };

        self.locals.insert(local, OwnershipState::Owned);
    }

    /// Mark a local as moved.
    pub fn mark_local_moved(&mut self, local: impl Into<mir::LocalReference>, at: MoveLocation) {
        let Some(local) = local.into().local() else {
            return;
        };

        self.locals
            .insert(local, OwnershipState::Moved { at: at.clone() });

        // propagate local moves to values derived from the same origin
        let moved_values: Vec<Value> = self
            .origins
            .iter()
            .filter_map(
                |(value, origin)| {
                    if *origin == local { Some(*value) } else { None }
                },
            )
            .collect();

        for value in moved_values {
            self.values
                .insert(value, OwnershipState::Moved { at: at.clone() });
        }
    }
}

impl Lattice for OwnershipMap {
    fn meet(&self, other: &Self) -> Self {
        // merge value ownership
        let mut result_values = self.values.clone();
        for (value, state_b) in &other.values {
            result_values
                .entry(*value)
                .and_modify(|state_a| {
                    *state_a = match (&*state_a, state_b) {
                        (OwnershipState::Owned, OwnershipState::Owned) => OwnershipState::Owned,
                        (OwnershipState::Moved { at }, OwnershipState::Moved { .. }) => {
                            OwnershipState::Moved { at: at.clone() }
                        }
                        (OwnershipState::MaybeMoved { first_move }, _)
                        | (_, OwnershipState::MaybeMoved { first_move }) => {
                            OwnershipState::MaybeMoved {
                                first_move: first_move.clone(),
                            }
                        }
                        (OwnershipState::Owned, OwnershipState::Moved { at })
                        | (OwnershipState::Moved { at }, OwnershipState::Owned) => {
                            OwnershipState::MaybeMoved {
                                first_move: at.clone(),
                            }
                        }
                    };
                })
                .or_insert_with(|| state_b.clone());
        }

        // merge local ownership
        let mut result_locals = self.locals.clone();
        for (local, state_b) in &other.locals {
            result_locals
                .entry(*local)
                .and_modify(|state_a| {
                    *state_a = match (&*state_a, state_b) {
                        (OwnershipState::Owned, OwnershipState::Owned) => OwnershipState::Owned,
                        (OwnershipState::Moved { at }, OwnershipState::Moved { .. }) => {
                            OwnershipState::Moved { at: at.clone() }
                        }
                        (OwnershipState::MaybeMoved { first_move }, _)
                        | (_, OwnershipState::MaybeMoved { first_move }) => {
                            OwnershipState::MaybeMoved {
                                first_move: first_move.clone(),
                            }
                        }
                        (OwnershipState::Owned, OwnershipState::Moved { at })
                        | (OwnershipState::Moved { at }, OwnershipState::Owned) => {
                            OwnershipState::MaybeMoved {
                                first_move: at.clone(),
                            }
                        }
                    };
                })
                .or_insert_with(|| state_b.clone());
        }

        // merge value origins (only keep when both agree)
        let mut result_origins = HashMap::new();
        for (value, local) in &self.origins {
            if let Some(other_local) = other.origins.get(value)
                && other_local == local
            {
                result_origins.insert(*value, *local);
            }
        }

        OwnershipMap {
            values: result_values,
            locals: result_locals,
            origins: result_origins,
        }
    }
}

/// Ownership analysis computes ownership state for each value at every test point.
///
/// This analysis uses forward dataflow to track which values have been moved
/// and where. At control flow join points, states are merged using a lattice
/// meet operation that produces "maybe moved" when a value is moved on some
/// paths but not others.
#[derive(Debug)]
pub struct OwnershipAnalysis {
    /// Ownership state at entry to each block.
    block_entry: HashMap<mir::LocalNodeId<mir::Block>, OwnershipMap>,
    /// Ownership state at exit of each block.
    block_exit: HashMap<mir::LocalNodeId<mir::Block>, OwnershipMap>,
    /// Type information for values (for determining copy vs move semantics).
    value_types: ValueTypeMap,
    /// Values allocated on the stack (from StackAlloc).
    stack_allocated: HashSet<Value>,
    /// Values allocated with heap allocation instructions.
    heap_allocated: HashSet<Value>,
}

impl OwnershipAnalysis {
    /// Get ownership state at entry to a block.
    pub fn state_at_entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&OwnershipMap> {
        self.block_entry.get(&block)
    }

    /// Get ownership state at exit of a block.
    pub fn state_at_exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&OwnershipMap> {
        self.block_exit.get(&block)
    }

    /// Get the type of a value.
    pub fn value_type(&self, value: impl Into<mir::ValueReference>) -> mir::LocalNodeId<Type> {
        self.value_types.require_value_type(value)
    }

    /// Get the type of a value.
    pub fn require_value_type(
        &self,
        value: impl Into<mir::ValueReference>,
    ) -> mir::LocalNodeId<Type> {
        self.value_type(value)
    }

    /// Return the value type map for this function.
    pub fn value_types(&self) -> &ValueTypeMap {
        &self.value_types
    }

    /// Return the pointee type for a pointer value when known.
    pub fn pointee_type(
        &self,
        pointer: impl Into<mir::ValueReference>,
        tree: &mir::Tree,
    ) -> Option<mir::LocalNodeId<Type>> {
        let type_id = self.require_value_type(pointer);
        let ty = tree.get(type_id);
        match ty {
            Type::Reference { pointee, .. } => pointee.ty(),
            Type::TensorView { element, .. } => element.ty(),
            _ => None,
        }
    }

    /// Check if a type has copy semantics.
    pub fn is_copy_type(&self, ty_id: mir::LocalNodeId<Type>, tree: &mir::Tree) -> bool {
        let ty = tree.get(ty_id);
        match ty {
            // primitives are always copy
            Type::Void
            | Type::Boolean
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => true,
            // callable metadata and values are copy
            Type::FunctionSignature { .. }
            | Type::FunctionPointer { .. }
            | Type::Callable { .. } => true,
            // raw and borrowed references are copy
            Type::Reference {
                kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
                ..
            } => true,
            // tensor views follow reference copy semantics
            Type::TensorView {
                kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
                ..
            } => true,
            // owned and managed references are not copy
            Type::Reference {
                kind: ReferenceKind::Owned | ReferenceKind::Managed,
                ..
            } => false,
            // managed tensor views are not copy
            Type::TensorView {
                kind: ReferenceKind::Owned | ReferenceKind::Managed,
                ..
            } => false,
            // slices follow reference copy semantics
            Type::Slice {
                kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
                ..
            } => true,
            Type::Slice {
                kind: ReferenceKind::Owned | ReferenceKind::Managed,
                ..
            } => false,
            Type::Array { copy, .. }
            | Type::Tuple { copy, .. }
            | Type::Struct { copy, .. }
            | Type::Newtype { copy, .. }
            | Type::Vector { copy, .. }
            | Type::Tensor { copy, .. } => *copy == mir::Copy::Yes,
        }
    }

    /// Check if a value has copy semantics.
    pub fn value_is_copy(&self, value: impl Into<mir::ValueReference>, tree: &mir::Tree) -> bool {
        // resolve the value type
        let ty_id = self.require_value_type(value);

        self.is_copy_type(ty_id, tree)
    }

    /// Set the origin for a destination value when it is move-only.
    fn set_origin_for_destination(
        &self,
        state: &mut OwnershipMap,
        destination: impl Into<mir::ValueReference>,
        origin: Option<mir::LocalReference>,
        tree: &mir::Tree,
    ) {
        let destination = destination.into();

        if !self.value_is_copy(destination, tree) {
            state.set_origin(destination, origin);
        } else {
            state.set_origin(destination, None);
        }
    }

    /// Check if a value was allocated on the stack.
    pub fn is_stack_allocated(&self, value: Value) -> bool {
        self.stack_allocated.contains(&value)
    }

    /// Check if a value was allocated by a heap allocator.
    pub fn is_heap_allocated(&self, value: Value) -> bool {
        self.heap_allocated.contains(&value)
    }

    /// Apply the effects of an instruction on ownership state.
    ///
    /// This is the canonical implementation of instruction move semantics.
    /// Both the analysis and the move checker use this to ensure consistency.
    pub fn apply_instruction_effects(
        &self,
        state: &mut OwnershipMap,
        inst_id: mir::LocalNodeId<Instruction>,
        inst: &Instruction,
        tree: &mir::Tree,
    ) {
        let at = MoveLocation::Instruction(inst_id);

        match inst {
            Instruction::Error => {
                panic!("recovered MIR instruction reached optimizer");
            }

            // pin moves ownership into the pinned carrier
            Instruction::Pin {
                destination, value, ..
            } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*value);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::Unpin { .. } => {}

            // drop marks ownership end and raw.free consumes explicit raw storage
            Instruction::Drop { value } => {
                state.mark_moved_with_source(*value, at);
            }
            Instruction::RawFree { pointer } => {
                state.mark_moved_with_source(*pointer, at);
            }

            // store/local.set move the value (if non-copy)
            Instruction::Store { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
            }
            Instruction::AtomicStore { value, .. } | Instruction::AtomicRmw { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
            }
            Instruction::AtomicCompareExchange {
                expected,
                new_value,
                ..
            } => {
                if !self.value_is_copy(*expected, tree) {
                    state.mark_moved_with_source(*expected, at.clone());
                }
                if !self.value_is_copy(*new_value, tree) {
                    state.mark_moved_with_source(*new_value, at);
                }
            }
            Instruction::LocalSet { value, local } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_local_owned(*local);
            }

            // local.get reads the local into an owned value
            Instruction::LocalGet { destination, local } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, Some(*local), tree);
            }
            // callable.environment reads the hidden environment pointer
            Instruction::CallableEnvironment { destination } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }

            // call moves arguments (if non-copy)
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. } => {
                let args = tree.get_arguments(call.arguments);
                for &arg in args {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                    self.set_origin_for_destination(state, dest, None, tree);
                }
            }
            Instruction::CallIndirect { call, .. } => {
                let args = tree.get_arguments(call.arguments);
                for &arg in args {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                    self.set_origin_for_destination(state, dest, None, tree);
                }
            }

            // field.set/element.set move the value (if non-copy)
            Instruction::FieldSet {
                destination,
                aggregate,
                value,
                ..
            } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*aggregate);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::ElementSet {
                destination,
                array,
                value,
                ..
            } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*array);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }

            // aggregate construction moves fields (if non-copy)
            Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let field_values = tree.get_arguments(*fields);
                for &field in field_values {
                    if !self.value_is_copy(field, tree) {
                        state.mark_moved_with_source(field, at.clone());
                    }
                }
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::CallableBind {
                destination,
                environment,
                ..
            } => {
                if !self.value_is_copy(*environment, tree) {
                    state.mark_moved_with_source(*environment, at);
                }
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::Tuple {
                destination,
                elements,
                ..
            }
            | Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let elem_values = tree.get_arguments(*elements);
                for &elem in elem_values {
                    if !self.value_is_copy(elem, tree) {
                        state.mark_moved_with_source(elem, at.clone());
                    }
                }
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }

            // vector operations
            Instruction::VectorSplat { destination, value } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*value);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::VectorExtract {
                destination,
                vector,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*vector);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::VectorInsert {
                destination,
                vector,
                value,
                ..
            } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*vector);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::VectorShuffle { destination, .. }
            | Instruction::VectorReduce { destination, .. }
            | Instruction::VectorCompare { destination, .. }
            | Instruction::VectorConvert { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }

            // tensor ops that produce owned values
            Instruction::TensorSplat { destination, value } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*value);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::TensorExtract {
                destination,
                tensor,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*tensor);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::TensorLoad { destination, .. }
            | Instruction::TensorReshape { destination, .. }
            | Instruction::TensorBroadcast { destination, .. }
            | Instruction::TensorTranspose { destination, .. }
            | Instruction::TensorSlice { destination, .. }
            | Instruction::TensorPad { destination, .. }
            | Instruction::TensorConcat { destination, .. }
            | Instruction::TensorDot { destination, .. }
            | Instruction::TensorConvolution { destination, .. }
            | Instruction::TensorGather { destination, .. }
            | Instruction::TensorScatter { destination, .. }
            | Instruction::TensorCompare { destination, .. }
            | Instruction::TensorConvert { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::TensorReduce { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::TensorCast { destination, .. }
            | Instruction::TensorView { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }

            // tensor stores move values when needed
            Instruction::TensorStore { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
            }
            Instruction::TensorFill { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at);
                }
            }
            Instruction::TensorCopy { .. } => {}

            // intrinsics: some consume their arguments
            Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                let args = tree.get_arguments(*arguments);
                for &idx in intrinsic.consumed_arguments() {
                    if let Some(&arg) = args.get(idx as usize)
                        && !self.value_is_copy(arg, tree)
                    {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                    self.set_origin_for_destination(state, dest, None, tree);
                }
            }

            // instructions that produce owned values
            Instruction::Binary { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::Unary {
                destination,
                argument,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*argument);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::Cast {
                destination,
                argument,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*argument);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = match (
                    state.origin_for_value(*then_value),
                    state.origin_for_value(*else_value),
                ) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    _ => None,
                };
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::VectorSelect {
                destination,
                then_value,
                else_value,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = match (
                    state.origin_for_value(*then_value),
                    state.origin_for_value(*else_value),
                ) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    _ => None,
                };
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::TensorSelect {
                destination,
                then_value,
                else_value,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = match (
                    state.origin_for_value(*then_value),
                    state.origin_for_value(*else_value),
                ) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    _ => None,
                };
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::Load { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*aggregate);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::FieldAddr { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::ElementGet {
                destination, array, ..
            } => {
                state.mark_owned(*destination);
                let origin = state.origin_for_value(*array);
                self.set_origin_for_destination(state, *destination, origin, tree);
            }
            Instruction::ElementAddr { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }
            Instruction::Const { destination, .. }
            | Instruction::GlobalAddr { destination, .. }
            | Instruction::LocalAddr { destination, .. }
            | Instruction::FunctionAddr { destination, .. }
            | Instruction::New { destination, .. }
            | Instruction::RawAlloc { destination, .. }
            | Instruction::StackAlloc { destination, .. }
            | Instruction::NewSlice { destination, .. } => {
                state.mark_owned(*destination);
                self.set_origin_for_destination(state, *destination, None, tree);
            }

            Instruction::Assume { .. } => {}
            Instruction::AtomicLoad { .. }
            | Instruction::AtomicFence { .. }
            | Instruction::BarrierWrite { .. } => {}
        }
    }

    /// Apply the effects of a terminator on ownership state.
    ///
    /// This is the canonical implementation of terminator move semantics.
    pub fn apply_terminator_effects(
        &self,
        state: &mut OwnershipMap,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        tree: &mir::Tree,
    ) {
        let at = MoveLocation::Terminator(block_id);

        match terminator {
            mir::Terminator::Error => {
                panic!("recovered MIR terminator reached optimizer");
            }

            mir::Terminator::Return { value } => {
                if let Some(v) = value
                    && !self.value_is_copy(*v, tree)
                {
                    state.mark_moved_with_source(*v, at);
                }
            }
            mir::Terminator::Jump { target } => {
                for &arg in &target.arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::Yield { value, resume } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved_with_source(*value, at.clone());
                }

                for &arg in &resume.arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::Invoke {
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                for &arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                if !self.value_is_copy(*callee, tree) {
                    state.mark_moved_with_source(*callee, at.clone());
                }
                for &arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::InvokeVirtual {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeInterface {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                if !self.value_is_copy(*receiver, tree) {
                    state.mark_moved_with_source(*receiver, at.clone());
                }
                for &arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::Throw { .. }
            | mir::Terminator::Unreachable => {}
            mir::Terminator::Trap { payload, .. } => {
                if let Some(payload) = payload
                    && !self.value_is_copy(*payload, tree)
                {
                    state.mark_moved_with_source(*payload, at.clone());
                }
            }
            mir::Terminator::TailCall { call, .. } => {
                for &arg in &call.arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::TailCallVirtual { receiver, call, .. }
            | mir::Terminator::TailCallInterface { receiver, call, .. } => {
                if !self.value_is_copy(*receiver, tree) {
                    state.mark_moved_with_source(*receiver, at.clone());
                }
                for &arg in &call.arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
            mir::Terminator::TailCallIndirect { callee, call, .. } => {
                if !self.value_is_copy(*callee, tree) {
                    state.mark_moved_with_source(*callee, at.clone());
                }
                for &arg in &call.arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved_with_source(arg, at.clone());
                    }
                }
            }
        }
    }

    /// Build the analysis.
    fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlFlowGraph) -> Self {
        if function.entry.is_none() {
            let value_types = ValueTypeMap::new(function, tree);
            return Self {
                block_entry: HashMap::new(),
                block_exit: HashMap::new(),
                value_types,
                stack_allocated: HashSet::new(),
                heap_allocated: HashSet::new(),
            };
        }

        // unwrap entry block now that we know it exists
        let entry = function.entry.unwrap();

        // collect value types for all SSA values
        let value_types = ValueTypeMap::new(function, tree);

        // collect allocation sites for drop decisions
        let (stack_allocated, heap_allocated) = collect_allocation_kinds(function, tree);

        // initial state: function parameters are owned
        let mut entry_state = OwnershipMap::new();
        for param in &function.parameters {
            entry_state.mark_owned(param.value);
        }

        // clone for the closure
        let value_types_clone = value_types.clone();

        // run forward dataflow with block parameter origin tracking
        let mut result = DataflowResult::new();
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

        // initialize entry block
        result.block_entry.insert(entry, entry_state.clone());
        worklist.push_back(entry);
        in_worklist.insert(entry);

        while let Some(block_id) = worklist.pop_front() {
            in_worklist.remove(&block_id);

            // compute entry state by merging predecessor exits
            let mut new_entry = if block_id == entry {
                result
                    .block_entry
                    .get(&entry)
                    .cloned()
                    .unwrap_or_else(|| entry_state.clone())
            } else {
                let predecessors = cfg.predecessors(block_id);
                if predecessors.is_empty() {
                    continue;
                }

                let mut merged: Option<OwnershipMap> = None;
                for &pred in predecessors {
                    if let Some(pred_exit) = result.block_exit.get(&pred) {
                        merged = Some(match merged {
                            Some(state) => state.meet(pred_exit),
                            None => pred_exit.clone(),
                        });
                    }
                }

                let Some(merged) = merged else {
                    continue;
                };

                merged
            };

            // refresh block parameters and origins
            let block = tree.get(block_id);
            for (index, param) in block.parameters.iter().enumerate() {
                new_entry.mark_owned(param.value);

                // determine origin from all predecessor edges
                let mut origin: Option<mir::LocalReference> = None;
                let mut initialized = false;

                for &pred in cfg.predecessors(block_id) {
                    let pred_exit = match result.block_exit.get(&pred) {
                        Some(state) => state,
                        None => continue,
                    };

                    for args in predecessor_arguments(pred, block_id, tree) {
                        let Some(arg_value) = args.get(index) else {
                            origin = None;
                            initialized = true;
                            continue;
                        };

                        let arg_origin = pred_exit.origin_for_value(*arg_value);
                        if !initialized {
                            origin = arg_origin;
                            initialized = true;
                        } else if origin != arg_origin {
                            origin = None;
                        }
                    }
                }

                set_origin_if_move_only(
                    &mut new_entry,
                    param.value,
                    origin,
                    tree,
                    &value_types_clone,
                );
            }

            // check if entry state changed
            let entry_changed = result
                .block_entry
                .get(&block_id)
                .map(|old| old != &new_entry)
                .unwrap_or(true);

            if entry_changed || block_id == entry {
                result.block_entry.insert(block_id, new_entry.clone());

                // apply transfer function
                let mut state = new_entry;
                for &inst_id in &block.instructions {
                    let inst = tree.get(inst_id);
                    process_instruction(&mut state, inst_id, inst, tree, &value_types_clone);
                }

                process_terminator(
                    &mut state,
                    block_id,
                    tree.get(block.terminator),
                    tree,
                    &value_types_clone,
                );

                let exit_changed = result
                    .block_exit
                    .get(&block_id)
                    .map(|old| old != &state)
                    .unwrap_or(true);

                if exit_changed {
                    result.block_exit.insert(block_id, state);

                    let terminator = tree.get(block.terminator);
                    for succ in terminator.successors() {
                        let Some(succ) = succ.block() else {
                            continue;
                        };

                        if !in_worklist.contains(&succ) {
                            worklist.push_back(succ);
                            in_worklist.insert(succ);
                        }
                    }
                }
            }
        }

        Self {
            block_entry: result.block_entry,
            block_exit: result.block_exit,
            value_types,
            stack_allocated,
            heap_allocated,
        }
    }
}

impl Analysis for OwnershipAnalysis {
    const ID: AnalysisId = AnalysisId("ownership");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for OwnershipAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        Self::build(function, tree, &cfg)
    }
}

/// Collect allocation sites for stack and heap values.
fn collect_allocation_kinds(
    function: &mir::Function,
    tree: &mir::Tree,
) -> (HashSet<Value>, HashSet<Value>) {
    // seed allocation sets
    let mut stack_allocated = HashSet::new();
    let mut heap_allocated = HashSet::new();

    // scan instructions for allocation results
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);
            match inst {
                Instruction::StackAlloc { destination, .. } => {
                    if let Some(destination) = destination.value() {
                        stack_allocated.insert(destination);
                    }
                }
                Instruction::New { destination, .. }
                | Instruction::NewSlice { destination, .. } => {
                    if let Some(destination) = destination.value() {
                        heap_allocated.insert(destination);
                    }
                }
                _ => {}
            }
        }
    }

    (stack_allocated, heap_allocated)
}

/// Check if a value has copy semantics.
fn value_is_copy(
    value: impl Into<mir::ValueReference>,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) -> bool {
    let type_id = value_types.require_value_type(value);
    let ty = tree.get(type_id);
    match ty {
        Type::Void
        | Type::Boolean
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::TypeDescriptor
        | Type::TypeId => true,
        Type::FunctionSignature { .. } | Type::FunctionPointer { .. } | Type::Callable { .. } => {
            true
        }
        Type::Reference {
            kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
            ..
        } => true,
        Type::TensorView {
            kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
            ..
        } => true,
        Type::Reference {
            kind: ReferenceKind::Owned | ReferenceKind::Managed,
            ..
        } => false,
        Type::TensorView {
            kind: ReferenceKind::Owned | ReferenceKind::Managed,
            ..
        } => false,
        Type::Slice {
            kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
            ..
        } => true,
        Type::Slice {
            kind: ReferenceKind::Owned | ReferenceKind::Managed,
            ..
        } => false,
        Type::Array { copy, .. }
        | Type::Tuple { copy, .. }
        | Type::Struct { copy, .. }
        | Type::Newtype { copy, .. }
        | Type::Vector { copy, .. }
        | Type::Tensor { copy, .. } => *copy == mir::Copy::Yes,
    }
}

/// Set the origin for a destination value when it is move-only.
fn set_origin_if_move_only(
    state: &mut OwnershipMap,
    destination: impl Into<mir::ValueReference>,
    origin: Option<mir::LocalReference>,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) {
    let destination = destination.into();

    if !value_is_copy(destination, tree, value_types) {
        state.set_origin(destination, origin);
    } else {
        state.set_origin(destination, None);
    }
}

/// Process an instruction, updating ownership state.
fn process_instruction(
    state: &mut OwnershipMap,
    inst_id: mir::LocalNodeId<Instruction>,
    inst: &Instruction,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) {
    let at = MoveLocation::Instruction(inst_id);

    match inst {
        Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // pin moves ownership into the pinned carrier
        Instruction::Pin {
            destination, value, ..
        } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*value);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::Unpin { .. } => {}

        // drop marks ownership end and raw.free consumes explicit raw storage
        Instruction::Drop { value } => {
            state.mark_moved_with_source(*value, at);
        }
        Instruction::RawFree { pointer } => {
            state.mark_moved_with_source(*pointer, at);
        }

        // store moves the value (if non-copy)
        Instruction::Store { value, .. } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
        }
        Instruction::AtomicStore { value, .. } | Instruction::AtomicRmw { value, .. } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
        }
        Instruction::AtomicCompareExchange {
            expected,
            new_value,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*expected, at.clone(), tree, value_types);
            state.mark_moved_if_not_copy_with_source(*new_value, at, tree, value_types);
        }

        // local.set moves the value (if non-copy)
        Instruction::LocalSet { value, .. } => {
            // move the assigned value when required
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);

            // local receives a fresh owned value
            if let Instruction::LocalSet { local, .. } = inst {
                state.mark_local_owned(*local);
            }
        }

        // atomic reads and barriers do not consume ownership
        Instruction::AtomicLoad { .. }
        | Instruction::AtomicFence { .. }
        | Instruction::BarrierWrite { .. } => {}

        // callable.environment reads the hidden environment pointer
        Instruction::CallableEnvironment { destination } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }

        // local.get reads the local into an owned value
        Instruction::LocalGet { destination, local } => {
            // the read produces a new owned value
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, Some(*local), tree, value_types);
        }

        // call moves arguments (if non-copy)
        Instruction::Call {
            destination, call, ..
        }
        | Instruction::CallVirtual {
            destination, call, ..
        }
        | Instruction::CallInterface {
            destination, call, ..
        }
        | Instruction::CallIndirect {
            destination, call, ..
        } => {
            let args = tree.get_arguments(call.arguments);
            for &arg in args {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
            if let Some(dest) = destination {
                state.mark_owned(*dest);
                set_origin_if_move_only(state, *dest, None, tree, value_types);
            }
        }

        // field.set/element.set move the new value (if non-copy)
        Instruction::FieldSet {
            destination,
            aggregate,
            value,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*aggregate);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::ElementSet {
            destination,
            array,
            value,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*array);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }

        // aggregate construction moves all fields (if non-copy)
        Instruction::Struct {
            destination,
            fields,
            ..
        } => {
            let field_values = tree.get_arguments(*fields);
            for &field in field_values {
                state.mark_moved_if_not_copy_with_source(field, at.clone(), tree, value_types);
            }
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::CallableBind {
            destination,
            environment,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*environment, at, tree, value_types);
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::Tuple {
            destination,
            elements,
            ..
        } => {
            let elem_values = tree.get_arguments(*elements);
            for &elem in elem_values {
                state.mark_moved_if_not_copy_with_source(elem, at.clone(), tree, value_types);
            }
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::Array {
            destination,
            elements,
            ..
        } => {
            let elem_values = tree.get_arguments(*elements);
            for &elem in elem_values {
                state.mark_moved_if_not_copy_with_source(elem, at.clone(), tree, value_types);
            }
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }

        // vector operations
        Instruction::VectorSplat { destination, value } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*value);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::VectorExtract {
            destination,
            vector,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*vector);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::VectorInsert {
            destination,
            vector,
            value,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*vector);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::VectorShuffle { destination, .. }
        | Instruction::VectorReduce { destination, .. }
        | Instruction::VectorCompare { destination, .. }
        | Instruction::VectorConvert { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }

        // tensor ops that produce owned values
        Instruction::TensorSplat { destination, value } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*value);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::TensorExtract {
            destination,
            tensor,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*tensor);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::TensorLoad { destination, .. }
        | Instruction::TensorReshape { destination, .. }
        | Instruction::TensorBroadcast { destination, .. }
        | Instruction::TensorTranspose { destination, .. }
        | Instruction::TensorSlice { destination, .. }
        | Instruction::TensorPad { destination, .. }
        | Instruction::TensorConcat { destination, .. }
        | Instruction::TensorReduce { destination, .. }
        | Instruction::TensorDot { destination, .. }
        | Instruction::TensorConvolution { destination, .. }
        | Instruction::TensorGather { destination, .. }
        | Instruction::TensorScatter { destination, .. }
        | Instruction::TensorCompare { destination, .. }
        | Instruction::TensorConvert { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::TensorCast { destination, .. }
        | Instruction::TensorView { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }

        // tensor stores move values when required
        Instruction::TensorStore { value, .. } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
        }
        Instruction::TensorFill { value, .. } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
        }
        Instruction::TensorCopy { .. } => {}

        // intrinsics: some consume arguments
        Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            for &idx in intrinsic.consumed_arguments() {
                if let Some(&arg) = args.get(idx as usize) {
                    state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
                }
            }
            if let Some(dest) = destination {
                state.mark_owned(*dest);
                set_origin_if_move_only(state, *dest, None, tree, value_types);
            }
        }

        // assume has no ownership effects
        Instruction::Assume { .. } => {}

        // instructions that produce new values
        Instruction::Binary { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::Unary {
            destination,
            argument,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*argument);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::Cast {
            destination,
            argument,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*argument);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::Select {
            destination,
            then_value,
            else_value,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = match (
                state.origin_for_value(*then_value),
                state.origin_for_value(*else_value),
            ) {
                (Some(left), Some(right)) if left == right => Some(left),
                _ => None,
            };
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::VectorSelect {
            destination,
            then_value,
            else_value,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = match (
                state.origin_for_value(*then_value),
                state.origin_for_value(*else_value),
            ) {
                (Some(left), Some(right)) if left == right => Some(left),
                _ => None,
            };
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::TensorSelect {
            destination,
            then_value,
            else_value,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = match (
                state.origin_for_value(*then_value),
                state.origin_for_value(*else_value),
            ) {
                (Some(left), Some(right)) if left == right => Some(left),
                _ => None,
            };
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::Load { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::FieldGet {
            destination,
            aggregate,
            ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*aggregate);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::FieldAddr { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::ElementGet {
            destination, array, ..
        } => {
            state.mark_owned(*destination);
            let origin = state.origin_for_value(*array);
            set_origin_if_move_only(state, *destination, origin, tree, value_types);
        }
        Instruction::ElementAddr { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
        Instruction::Const { destination, .. }
        | Instruction::GlobalAddr { destination, .. }
        | Instruction::LocalAddr { destination, .. }
        | Instruction::FunctionAddr { destination, .. }
        | Instruction::New { destination, .. }
        | Instruction::RawAlloc { destination, .. }
        | Instruction::StackAlloc { destination, .. }
        | Instruction::NewSlice { destination, .. } => {
            state.mark_owned(*destination);
            set_origin_if_move_only(state, *destination, None, tree, value_types);
        }
    }
}

/// Process a terminator, updating ownership state.
fn process_terminator(
    state: &mut OwnershipMap,
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) {
    let at = MoveLocation::Terminator(block_id);

    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }

        mir::Terminator::Return { value } => {
            if let Some(v) = value {
                state.mark_moved_if_not_copy_with_source(*v, at, tree, value_types);
            }
        }
        mir::Terminator::Jump { target } => {
            for &arg in &target.arguments {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::Yield { value, resume } => {
            state.mark_moved_if_not_copy_with_source(*value, at.clone(), tree, value_types);

            for &arg in &resume.arguments {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::Invoke {
            call,
            normal_target,
            unwind_target,
            ..
        } => {
            for &arg in call
                .arguments
                .iter()
                .chain(normal_target.arguments.iter())
                .chain(unwind_target.arguments.iter())
            {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::InvokeIndirect {
            callee,
            call,
            normal_target,
            unwind_target,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*callee, at.clone(), tree, value_types);
            for &arg in call
                .arguments
                .iter()
                .chain(normal_target.arguments.iter())
                .chain(unwind_target.arguments.iter())
            {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::InvokeVirtual {
            receiver,
            call,
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeInterface {
            receiver,
            call,
            normal_target,
            unwind_target,
            ..
        } => {
            state.mark_moved_if_not_copy_with_source(*receiver, at.clone(), tree, value_types);
            for &arg in call
                .arguments
                .iter()
                .chain(normal_target.arguments.iter())
                .chain(unwind_target.arguments.iter())
            {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::Throw { value } => {
            state.mark_moved_if_not_copy_with_source(*value, at, tree, value_types);
        }
        mir::Terminator::Trap { payload, .. } => {
            if let Some(payload) = payload {
                state.mark_moved_if_not_copy_with_source(*payload, at, tree, value_types);
            }
        }
        mir::Terminator::Branch { .. }
        | mir::Terminator::Check { .. }
        | mir::Terminator::Switch { .. }
        | mir::Terminator::Unreachable => {}
        mir::Terminator::TailCall { call, .. } => {
            for &arg in &call.arguments {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::TailCallVirtual { receiver, call, .. }
        | mir::Terminator::TailCallInterface { receiver, call, .. } => {
            state.mark_moved_if_not_copy_with_source(*receiver, at.clone(), tree, value_types);
            for &arg in &call.arguments {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
        mir::Terminator::TailCallIndirect { callee, call, .. } => {
            state.mark_moved_if_not_copy_with_source(*callee, at.clone(), tree, value_types);
            for &arg in &call.arguments {
                state.mark_moved_if_not_copy_with_source(arg, at.clone(), tree, value_types);
            }
        }
    }
}

/// Collect argument lists that flow from a predecessor to a target block.
fn predecessor_arguments(
    predecessor: mir::LocalNodeId<mir::Block>,
    target: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
) -> Vec<&[mir::ValueReference]> {
    let block = tree.get(predecessor);
    let terminator = tree.get(block.terminator);
    let mut arguments = Vec::new();

    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }

        mir::Terminator::Jump { target: edge } => {
            if edge.block.block() == Some(target) {
                arguments.push(edge.arguments.as_slice());
            }
        }
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            if then_target.block.block() == Some(target) {
                arguments.push(then_target.arguments.as_slice());
            }

            if else_target.block.block() == Some(target) {
                arguments.push(else_target.arguments.as_slice());
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if success.block.block() == Some(target) {
                arguments.push(success.arguments.as_slice());
            }

            if failure.block.block() == Some(target) {
                arguments.push(failure.arguments.as_slice());
            }
        }
        mir::Terminator::Switch { default, cases, .. } => {
            if default.block.block() == Some(target) {
                arguments.push(default.arguments.as_slice());
            }

            for case in cases {
                if case.target.block.block() == Some(target) {
                    arguments.push(case.target.arguments.as_slice());
                }
            }
        }
        mir::Terminator::Yield { resume, .. } => {
            if resume.block.block() == Some(target) {
                arguments.push(resume.arguments.as_slice());
            }
        }
        mir::Terminator::Invoke {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeIndirect {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeVirtual {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeInterface {
            normal_target,
            unwind_target,
            ..
        } => {
            if normal_target.block.block() == Some(target) {
                arguments.push(normal_target.arguments.as_slice());
            }

            if unwind_target.block.block() == Some(target) {
                arguments.push(unwind_target.arguments.as_slice());
            }
        }
        mir::Terminator::Return { .. }
        | mir::Terminator::Throw { .. }
        | mir::Terminator::Trap { .. }
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallVirtual { .. }
        | mir::Terminator::TailCallInterface { .. }
        | mir::Terminator::TailCallIndirect { .. }
        | mir::Terminator::Unreachable => {}
    }

    arguments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_ownership_simple() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        let entry = function.entry.unwrap();

        // v0 is defined in this block, so it should be owned at exit
        let exit_state = ownership.state_at_exit(entry).unwrap();
        assert!(exit_state.is_owned(mir::Value::new(0)));
    }

    #[test]
    fn test_ownership_drop() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    drop v0
    v1: int32 = 0int32
    return v1
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        let entry = function.entry.unwrap();
        let exit_state = ownership.state_at_exit(entry).unwrap();

        // v0 was dropped, should be moved
        assert!(exit_state.is_moved(mir::Value::new(0)));
        // v1 is still owned
        assert!(exit_state.is_owned(mir::Value::new(1)));
    }

    #[test]
    fn test_ownership_diamond_maybe_moved() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1, b2
b1:
    drop v1
    jump b3
b2:
    jump b3
b3:
    v2: int32 = 0int32
    return v2
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        // block3 is the merge point (index 3 in blocks list)
        let block3 = function.blocks[3];
        let entry_state = ownership.state_at_entry(block3).unwrap();

        // v1 should be MaybeMoved at block3 entry (moved on one path, not the other)
        let v1_state = entry_state.get(mir::Value::new(1));
        assert!(v1_state.is_some());
        assert!(v1_state.unwrap().is_maybe_moved());
    }

    /// Moving one local-derived value moves other values from the same origin.
    #[test]
    fn test_ownership_local_get_propagates_move() {
        let test = TestProgram::new(
            r#"
function test(): void {
    local local0: ref<int32, managed>, owned
b0:
    v0: ref<int32, managed> = new int32
    local.set local0, v0
    v1: ref<int32, managed> = local.get local0
    v2: ref<int32, managed> = local.get local0
    drop v1
    return
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        let entry = function.entry.unwrap();
        let exit_state = ownership.state_at_exit(entry).unwrap();

        assert!(exit_state.is_moved(mir::Value::new(1)));
        assert!(exit_state.is_moved(mir::Value::new(2)));
    }
}

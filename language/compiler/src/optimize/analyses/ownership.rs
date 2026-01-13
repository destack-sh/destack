use std::collections::{HashMap, HashSet};

use destack_mir as mir;
use mir::{Instruction, ReferenceKind, Type, Value};

use super::{ControlFlowGraph, Lattice, forward_dataflow};
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

/// Ownership state for all values at a program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnershipMap(pub HashMap<Value, OwnershipState>);

impl OwnershipMap {
    /// Create an empty ownership map.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Get the ownership state for a value.
    pub fn get(&self, value: Value) -> Option<&OwnershipState> {
        self.0.get(&value)
    }

    /// Check if a value is owned (usable).
    pub fn is_owned(&self, value: Value) -> bool {
        self.0
            .get(&value)
            .map(|s| matches!(s, OwnershipState::Owned))
            .unwrap_or(true) // unknown values are assumed owned
    }

    /// Check if a value is moved.
    pub fn is_moved(&self, value: Value) -> bool {
        self.0.get(&value).map(|s| s.is_moved()).unwrap_or(false)
    }

    /// Mark a value as owned.
    pub fn mark_owned(&mut self, value: Value) {
        self.0.insert(value, OwnershipState::Owned);
    }

    /// Mark a value as moved.
    pub fn mark_moved(&mut self, value: Value, at: MoveLocation) {
        self.0.insert(value, OwnershipState::Moved { at });
    }

    /// Mark a value as moved if it's not a copy type.
    pub fn mark_moved_if_not_copy(
        &mut self,
        value: Value,
        at: MoveLocation,
        tree: &mir::NodeTree,
        value_types: &HashMap<Value, mir::LocalNodeId<Type>>,
        copy_values: &HashSet<Value>,
    ) {
        if !value_is_copy(value, tree, value_types, copy_values) {
            self.mark_moved(value, at);
        }
    }
}

impl Lattice for OwnershipMap {
    fn meet(&self, other: &Self) -> Self {
        let mut result = self.0.clone();
        for (value, state_b) in &other.0 {
            result
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
        OwnershipMap(result)
    }
}

/// Ownership analysis computes ownership state for each value at every program point.
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
    value_types: HashMap<Value, mir::LocalNodeId<Type>>,
    /// Values known to be copy types (constants, etc.).
    copy_values: HashSet<Value>,
    /// Values allocated on the stack (from StackAlloc).
    /// Used to determine whether to emit StackDrop vs RawDrop.
    stack_allocated: HashSet<Value>,
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

    /// Get the type of a value (if known).
    pub fn value_type(&self, value: Value) -> Option<mir::LocalNodeId<Type>> {
        self.value_types.get(&value).copied()
    }

    /// Return the value type map for this function.
    pub fn value_types(&self) -> &HashMap<Value, mir::LocalNodeId<Type>> {
        &self.value_types
    }

    /// Check if a type has copy semantics.
    pub fn is_copy_type(&self, ty_id: mir::LocalNodeId<Type>, tree: &mir::NodeTree) -> bool {
        let ty = tree.get(ty_id);
        match ty {
            // primitives are always copy
            Type::Void | Type::Boolean | Type::Int { .. } | Type::Float { .. } => true,
            // function pointers are copy
            Type::FunctionPointer { .. } => true,
            // raw and borrowed references are copy
            Type::Reference {
                kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
                ..
            } => true,
            // owned and managed references are not copy
            Type::Reference {
                kind: ReferenceKind::Owned | ReferenceKind::Managed,
                ..
            } => false,
            // aggregates: check copyability field
            Type::Array { copyability, .. }
            | Type::Tuple { copyability, .. }
            | Type::Struct { copyability, .. } => *copyability == mir::Copyability::Trivial,
        }
    }

    /// Check if a value has copy semantics.
    pub fn value_is_copy(&self, value: Value, tree: &mir::NodeTree) -> bool {
        // check if this value is known to be copy (e.g., from a constant)
        if self.copy_values.contains(&value) {
            return true;
        }

        // otherwise check the type
        match self.value_types.get(&value) {
            Some(&ty_id) => self.is_copy_type(ty_id, tree),
            None => false, // unknown type, be conservative
        }
    }

    /// Check if a value was allocated on the stack.
    ///
    /// Stack-allocated values use StackDrop (no-op, frame handles cleanup)
    /// instead of RawDrop (explicit deallocation).
    pub fn is_stack_allocated(&self, value: Value) -> bool {
        self.stack_allocated.contains(&value)
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
        tree: &mir::NodeTree,
    ) {
        let at = MoveLocation::Instruction(inst_id);

        match inst {
            // raw.drop/stack.drop/raw.free always consume
            Instruction::RawDrop { value } => {
                state.mark_moved(*value, at);
            }
            Instruction::StackDrop { value } => {
                state.mark_moved(*value, at);
            }
            Instruction::RawFree { pointer } => {
                state.mark_moved(*pointer, at);
            }

            // store/local.set move the value (if non-copy)
            Instruction::Store { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved(*value, at);
                }
            }
            Instruction::LocalSet { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved(*value, at);
                }
            }

            // call moves arguments (if non-copy)
            Instruction::Call { arguments, .. } | Instruction::CallIndirect { arguments, .. } => {
                let args = tree.get_arguments(*arguments);
                for &arg in args {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved(arg, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }

            // field.set/element.set move the value (if non-copy)
            Instruction::FieldSet { value, .. } | Instruction::ElementSet { value, .. } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved(*value, at);
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }

            // aggregate construction moves fields (if non-copy)
            Instruction::Struct { fields, .. } => {
                let field_values = tree.get_arguments(*fields);
                for &field in field_values {
                    if !self.value_is_copy(field, tree) {
                        state.mark_moved(field, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }
            Instruction::Tuple { elements, .. } | Instruction::Array { elements, .. } => {
                let elem_values = tree.get_arguments(*elements);
                for &elem in elem_values {
                    if !self.value_is_copy(elem, tree) {
                        state.mark_moved(elem, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }

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
                        state.mark_moved(arg, at.clone());
                    }
                }
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }

            // instructions that produce owned values
            _ => {
                if let Some(dest) = inst.destination() {
                    state.mark_owned(dest);
                }
            }
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
        tree: &mir::NodeTree,
    ) {
        let at = MoveLocation::Terminator(block_id);

        match terminator {
            mir::Terminator::Return { value } => {
                if let Some(v) = value
                    && !self.value_is_copy(*v, tree)
                {
                    state.mark_moved(*v, at);
                }
            }
            mir::Terminator::Jump { arguments, .. } => {
                for &arg in arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved(arg, at.clone());
                    }
                }
            }
            mir::Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                if !self.value_is_copy(*value, tree) {
                    state.mark_moved(*value, at.clone());
                }
                for &arg in resume_arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved(arg, at.clone());
                    }
                }
            }
            mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::Unreachable => {}
            mir::Terminator::TailCall { arguments, .. } => {
                for &arg in arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved(arg, at.clone());
                    }
                }
            }
            mir::Terminator::TailCallIndirect { callee, arguments } => {
                if !self.value_is_copy(*callee, tree) {
                    state.mark_moved(*callee, at.clone());
                }
                for &arg in arguments {
                    if !self.value_is_copy(arg, tree) {
                        state.mark_moved(arg, at.clone());
                    }
                }
            }
        }
    }

    /// Build the analysis.
    fn build(function: &mir::Function, tree: &mir::NodeTree, cfg: &ControlFlowGraph) -> Self {
        if function.entry.is_none() {
            return Self {
                block_entry: HashMap::new(),
                block_exit: HashMap::new(),
                value_types: HashMap::new(),
                copy_values: HashSet::new(),
                stack_allocated: HashSet::new(),
            };
        }

        // collect type information, copy values, and stack allocations
        let mut value_types = HashMap::new();
        let mut copy_values = HashSet::new();
        let mut stack_allocated = HashSet::new();

        // function parameters
        for param in &function.parameters {
            value_types.insert(param.value, param.ty);
        }

        // block parameters and instruction results
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for param in &block.parameters {
                value_types.insert(param.value, param.ty);
            }

            // collect types from instructions that have explicit types
            for &inst_id in &block.instructions {
                let inst = tree.get(inst_id);
                instruction_collect_types(
                    inst,
                    tree,
                    &mut value_types,
                    &mut copy_values,
                    &mut stack_allocated,
                );
            }
        }

        // initial state: function parameters are owned
        let mut entry_state = OwnershipMap::new();
        for param in &function.parameters {
            entry_state.mark_owned(param.value);
        }

        // clone for the closure
        let value_types_clone = value_types.clone();
        let copy_values_clone = copy_values.clone();

        // run forward dataflow
        let result = forward_dataflow(
            function,
            tree,
            cfg,
            entry_state,
            |block_id, mut state, tree| {
                let block = tree.get(block_id);

                // block parameters are fresh definitions (owned)
                for param in &block.parameters {
                    state.mark_owned(param.value);
                }

                // process each instruction
                for &inst_id in &block.instructions {
                    let inst = tree.get(inst_id);
                    process_instruction(
                        &mut state,
                        inst_id,
                        inst,
                        tree,
                        &value_types_clone,
                        &copy_values_clone,
                    );
                }

                // process terminator
                process_terminator(
                    &mut state,
                    block_id,
                    &block.terminator,
                    tree,
                    &value_types_clone,
                    &copy_values_clone,
                );

                state
            },
        );

        Self {
            block_entry: result.block_entry,
            block_exit: result.block_exit,
            value_types,
            copy_values,
            stack_allocated,
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
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        Self::build(function, tree, &cfg)
    }
}

/// Collect type information, copy values, and stack allocations from an instruction.
fn instruction_collect_types(
    instruction: &Instruction,
    tree: &mir::NodeTree,
    value_types: &mut HashMap<Value, mir::LocalNodeId<Type>>,
    copy_values: &mut HashSet<Value>,
    stack_allocated: &mut HashSet<Value>,
) {
    match instruction {
        // instructions with explicit result types
        Instruction::Cast {
            destination,
            to_type,
            ..
        } => {
            value_types.insert(*destination, *to_type);
        }
        Instruction::Struct {
            destination, ty, ..
        }
        | Instruction::Tuple {
            destination, ty, ..
        }
        | Instruction::Array {
            destination, ty, ..
        } => {
            value_types.insert(*destination, *ty);
        }
        // raw allocs produce raw pointers (copy semantics)
        Instruction::RawAlloc { destination, .. } => {
            copy_values.insert(*destination);
        }
        // stack allocs produce raw pointers (copy semantics) and track allocation kind
        Instruction::StackAlloc { destination, .. } => {
            copy_values.insert(*destination);
            stack_allocated.insert(*destination);
        }

        // managed allocs produce owned/managed refs (non-copy)
        // (don't add to value_types or copy_values)
        Instruction::ManagedAlloc { .. } | Instruction::ManagedAllocArray { .. } => {}
        Instruction::LocalGet { destination, local } => {
            let local_decl = tree.get(*local);
            value_types.insert(*destination, local_decl.ty);
        }
        Instruction::Call {
            destination,
            function,
            ..
        } => {
            if let Some(dest) = destination {
                let func = tree.get(*function);
                value_types.insert(*dest, func.return_type);
            }
        }

        // for Load, get pointee type from pointer
        Instruction::Load {
            destination,
            pointer,
        } => {
            if let Some(&ptr_ty) = value_types.get(pointer)
                && let Type::Reference { pointee, .. } = tree.get(ptr_ty)
            {
                value_types.insert(*destination, *pointee);
            }
        }

        // constants are always primitives (copy types)
        Instruction::Const { destination, .. } => {
            copy_values.insert(*destination);
        }

        // binary/unary produce primitives (always copy)
        Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => {
            copy_values.insert(*destination);
            if !binary_operator_is_comparison(*operator) {
                let inferred = value_types.get(left).or_else(|| value_types.get(right));
                if let Some(&ty) = inferred {
                    value_types.insert(*destination, ty);
                }
            }
        }
        Instruction::Unary {
            destination,
            argument,
            ..
        } => {
            copy_values.insert(*destination);
            if let Some(&ty) = value_types.get(argument) {
                value_types.insert(*destination, ty);
            }
        }

        // field/element access: derive type from aggregate
        Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => {
            if let Some(&agg_ty) = value_types.get(aggregate) {
                match tree.get(agg_ty) {
                    Type::Struct { fields, .. } => {
                        if let Some(&field_id) = fields.get(*index as usize) {
                            let field_def = tree.get(field_id);
                            value_types.insert(*destination, field_def.ty);
                        }
                    }
                    Type::Tuple { elements, .. } => {
                        if let Some(&elem_ty) = elements.get(*index as usize) {
                            value_types.insert(*destination, elem_ty);
                        }
                    }
                    _ => {}
                }
            }
        }
        Instruction::ElementGet {
            destination, array, ..
        } => {
            if let Some(&arr_ty) = value_types.get(array)
                && let Type::Array { element, .. } = tree.get(arr_ty)
            {
                value_types.insert(*destination, *element);
            }
        }

        // address instructions keep reference type
        Instruction::FieldAddr {
            destination,
            aggregate,
            ..
        }
        | Instruction::ElementAddr {
            destination,
            array: aggregate,
            ..
        } => {
            if let Some(&ty) = value_types.get(aggregate) {
                value_types.insert(*destination, ty);
            }
        }

        // field.set/element.set result has same type as aggregate
        Instruction::FieldSet {
            destination,
            aggregate,
            ..
        }
        | Instruction::ElementSet {
            destination,
            array: aggregate,
            ..
        } => {
            if let Some(&ty) = value_types.get(aggregate) {
                value_types.insert(*destination, ty);
            }
        }

        // other instructions: no explicit type to collect
        _ => {}
    }
}

/// Check whether a binary operator yields a boolean result.
fn binary_operator_is_comparison(operator: mir::BinaryOperator) -> bool {
    use mir::BinaryOperator::*;
    matches!(
        operator,
        Equal
            | NotEqual
            | SignedLessThan
            | SignedLessEqual
            | SignedGreaterThan
            | SignedGreaterEqual
            | UnsignedLessThan
            | UnsignedLessEqual
            | UnsignedGreaterThan
            | UnsignedGreaterEqual
            | FloatEqual
            | FloatNotEqual
            | FloatLessThan
            | FloatLessEqual
            | FloatGreaterThan
            | FloatGreaterEqual
    )
}

/// Check if a value has copy semantics.
fn value_is_copy(
    value: Value,
    tree: &mir::NodeTree,
    value_types: &HashMap<Value, mir::LocalNodeId<Type>>,
    copy_values: &HashSet<Value>,
) -> bool {
    // check if known to be copy (constants, binary/unary results)
    if copy_values.contains(&value) {
        return true;
    }

    // otherwise check the type
    match value_types.get(&value) {
        Some(&ty_id) => {
            let ty = tree.get(ty_id);
            match ty {
                Type::Void | Type::Boolean | Type::Int { .. } | Type::Float { .. } => true,
                Type::FunctionPointer { .. } => true,
                Type::Reference {
                    kind: ReferenceKind::Raw | ReferenceKind::Borrowed,
                    ..
                } => true,
                Type::Reference {
                    kind: ReferenceKind::Owned | ReferenceKind::Managed,
                    ..
                } => false,
                Type::Array { copyability, .. }
                | Type::Tuple { copyability, .. }
                | Type::Struct { copyability, .. } => *copyability == mir::Copyability::Trivial,
            }
        }
        None => false,
    }
}

/// Process an instruction, updating ownership state.
fn process_instruction(
    state: &mut OwnershipMap,
    inst_id: mir::LocalNodeId<Instruction>,
    inst: &Instruction,
    tree: &mir::NodeTree,
    value_types: &HashMap<Value, mir::LocalNodeId<Type>>,
    copy_values: &HashSet<Value>,
) {
    let at = MoveLocation::Instruction(inst_id);

    match inst {
        // raw.drop/stack.drop/raw.free always consume
        Instruction::RawDrop { value } => {
            state.mark_moved(*value, at);
        }
        Instruction::StackDrop { value } => {
            state.mark_moved(*value, at);
        }
        Instruction::RawFree { pointer } => {
            state.mark_moved(*pointer, at);
        }

        // store moves the value (if non-copy)
        Instruction::Store { value, .. } => {
            state.mark_moved_if_not_copy(*value, at, tree, value_types, copy_values);
        }

        // local.set moves the value (if non-copy)
        Instruction::LocalSet { value, .. } => {
            state.mark_moved_if_not_copy(*value, at, tree, value_types, copy_values);
        }

        // call moves arguments (if non-copy)
        Instruction::Call {
            destination,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            for &arg in args {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
            if let Some(dest) = destination {
                state.mark_owned(*dest);
            }
        }
        Instruction::CallIndirect {
            destination,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            for &arg in args {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
            if let Some(dest) = destination {
                state.mark_owned(*dest);
            }
        }

        // field.set/element.set move the new value (if non-copy)
        Instruction::FieldSet {
            destination, value, ..
        } => {
            state.mark_moved_if_not_copy(*value, at, tree, value_types, copy_values);
            state.mark_owned(*destination);
        }
        Instruction::ElementSet {
            destination, value, ..
        } => {
            state.mark_moved_if_not_copy(*value, at, tree, value_types, copy_values);
            state.mark_owned(*destination);
        }

        // aggregate construction moves all fields (if non-copy)
        Instruction::Struct {
            destination,
            fields,
            ..
        } => {
            let field_values = tree.get_arguments(*fields);
            for &field in field_values {
                state.mark_moved_if_not_copy(field, at.clone(), tree, value_types, copy_values);
            }
            state.mark_owned(*destination);
        }
        Instruction::Tuple {
            destination,
            elements,
            ..
        } => {
            let elem_values = tree.get_arguments(*elements);
            for &elem in elem_values {
                state.mark_moved_if_not_copy(elem, at.clone(), tree, value_types, copy_values);
            }
            state.mark_owned(*destination);
        }
        Instruction::Array {
            destination,
            elements,
            ..
        } => {
            let elem_values = tree.get_arguments(*elements);
            for &elem in elem_values {
                state.mark_moved_if_not_copy(elem, at.clone(), tree, value_types, copy_values);
            }
            state.mark_owned(*destination);
        }

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
                    state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
                }
            }
            if let Some(dest) = destination {
                state.mark_owned(*dest);
            }
        }

        // assume has no ownership effects
        Instruction::Assume { .. } => {}

        // instructions that produce new values (all mark destination as owned)
        Instruction::Binary { destination, .. }
        | Instruction::Unary { destination, .. }
        | Instruction::Cast { destination, .. }
        | Instruction::Select { destination, .. }
        | Instruction::Load { destination, .. }
        | Instruction::FieldGet { destination, .. }
        | Instruction::FieldAddr { destination, .. }
        | Instruction::ElementGet { destination, .. }
        | Instruction::ElementAddr { destination, .. }
        | Instruction::Const { destination, .. }
        | Instruction::LocalGet { destination, .. }
        | Instruction::GlobalAddr { destination, .. }
        | Instruction::GlobalConst { destination, .. }
        | Instruction::ManagedAlloc { destination, .. }
        | Instruction::RawAlloc { destination, .. }
        | Instruction::StackAlloc { destination, .. }
        | Instruction::ManagedAllocArray { destination, .. } => {
            state.mark_owned(*destination);
        }
    }
}

/// Process a terminator, updating ownership state.
fn process_terminator(
    state: &mut OwnershipMap,
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    tree: &mir::NodeTree,
    value_types: &HashMap<Value, mir::LocalNodeId<Type>>,
    copy_values: &HashSet<Value>,
) {
    let at = MoveLocation::Terminator(block_id);

    match terminator {
        mir::Terminator::Return { value } => {
            if let Some(v) = value {
                state.mark_moved_if_not_copy(*v, at, tree, value_types, copy_values);
            }
        }
        mir::Terminator::Jump { arguments, .. } => {
            for &arg in arguments {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
        }
        mir::Terminator::Yield {
            value,
            resume_arguments,
            ..
        } => {
            state.mark_moved_if_not_copy(*value, at.clone(), tree, value_types, copy_values);
            for &arg in resume_arguments {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
        }
        mir::Terminator::Branch { .. }
        | mir::Terminator::Check { .. }
        | mir::Terminator::Switch { .. }
        | mir::Terminator::Unreachable => {}
        mir::Terminator::TailCall { arguments, .. } => {
            for &arg in arguments {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
        }
        mir::Terminator::TailCallIndirect { callee, arguments } => {
            state.mark_moved_if_not_copy(*callee, at.clone(), tree, value_types, copy_values);
            for &arg in arguments {
                state.mark_moved_if_not_copy(arg, at.clone(), tree, value_types, copy_values);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_ownership_simple() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        let entry = function.entry.unwrap();

        // v0 is defined in this block, so it should be owned at exit
        let exit_state = ownership.state_at_exit(entry).unwrap();
        assert!(exit_state.is_owned(mir::Value::new(0)));
    }

    #[test]
    fn test_ownership_drop() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    raw.drop v0
    v1 = iconst 0i32
    return v1
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
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
        let program = TestProgram::new(
            r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1, block2
block1:
    raw.drop v1
    jump block3
block2:
    jump block3
block3:
    v2 = iconst 0i32
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let ownership = analyses.get::<OwnershipAnalysis>();

        // block3 is the merge point (index 3 in blocks list)
        let block3 = function.blocks[3];
        let entry_state = ownership.state_at_entry(block3).unwrap();

        // v1 should be MaybeMoved at block3 entry (moved on one path, not the other)
        let v1_state = entry_state.get(mir::Value::new(1));
        assert!(v1_state.is_some());
        assert!(v1_state.unwrap().is_maybe_moved());
    }
}

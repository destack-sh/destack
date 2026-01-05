use std::collections::HashMap;

use destack_mir as mir;

use crate::memory::Value;

use super::dispatch;
use super::threaded::{
    ArgumentRange, SwitchCase, SwitchRange, ThreadedBlock, ThreadedFunction, ThreadedInstruction,
    ThreadedInstructionData, pack_optional_value,
};

/// Scalar and aggregate kinds used for typed dispatch selection.
#[derive(Clone, Copy, Debug)]
enum ValueKind {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed or unsigned integer with width.
    Int { width: u8, signed: bool },
    /// Floating point value with width.
    Float { width: u8 },
    /// Unicode character value.
    Char,
    /// Pointer-like value with pointee type.
    Pointer {
        pointee: mir::LocalNodeId<mir::Type>,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: mir::LocalNodeId<mir::Type> },
    /// Heap aggregate value with concrete type.
    Aggregate { ty: mir::LocalNodeId<mir::Type> },
    /// Managed array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
    },
    /// Unknown or unsupported type.
    Unknown,
}

/// Table mapping SSA value ids to their inferred kind.
struct ValueKinds {
    /// Kind for each SSA value id.
    kinds: Vec<Option<ValueKind>>,
}

impl ValueKinds {
    /// Create a new value kind table.
    fn new(value_count: usize) -> Self {
        Self {
            kinds: vec![None; value_count],
        }
    }

    /// Get the kind for a value.
    fn get(&self, value: mir::Value) -> Option<ValueKind> {
        self.kinds.get(value.0 as usize).and_then(|kind| *kind)
    }

    /// Set the kind for a value.
    fn set(&mut self, value: mir::Value, kind: ValueKind) {
        if let Some(slot) = self.kinds.get_mut(value.0 as usize) {
            *slot = Some(kind);
        }
    }
}

/// Pick a binary handler based on inferred operand kind.
fn select_binary_handler(
    value_kinds: &ValueKinds,
    left: mir::Value,
    operator: mir::BinaryOperator,
) -> super::threaded::ThreadedHandler {
    // resolve operand kind
    let kind = value_kinds.get(left);

    // select handler by kind
    match kind {
        Some(ValueKind::Int { signed: true, .. }) => dispatch::handle_binary_int,
        Some(ValueKind::Int { signed: false, .. }) => dispatch::handle_binary_uint,
        Some(ValueKind::Float { width: 32 }) => dispatch::handle_binary_float32,
        Some(ValueKind::Float { width: 64 }) => dispatch::handle_binary_float64,
        Some(ValueKind::Bool)
            if matches!(
                operator,
                mir::BinaryOperator::And | mir::BinaryOperator::Or | mir::BinaryOperator::Xor
            ) =>
        {
            dispatch::handle_binary_bool
        }
        _ => dispatch::handle_binary,
    }
}

/// Pick a unary handler based on inferred operand kind.
fn select_unary_handler(
    value_kinds: &ValueKinds,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> super::threaded::ThreadedHandler {
    // resolve operand kind
    let kind = value_kinds.get(argument);

    // select handler by kind
    match (kind, operator) {
        (Some(ValueKind::Int { signed: true, .. }), _) => dispatch::handle_unary_int,
        (Some(ValueKind::Int { signed: false, .. }), _) => dispatch::handle_unary_uint,
        (Some(ValueKind::Float { width: 32 }), mir::UnaryOperator::FloatNegate) => {
            dispatch::handle_unary_float32
        }
        (Some(ValueKind::Float { width: 64 }), mir::UnaryOperator::FloatNegate) => {
            dispatch::handle_unary_float64
        }
        (Some(ValueKind::Bool), mir::UnaryOperator::Not) => dispatch::handle_unary_bool,
        _ => dispatch::handle_unary,
    }
}

/// Pick a branch handler based on inferred condition kind.
fn select_branch_handler(
    value_kinds: &ValueKinds,
    condition: mir::Value,
) -> super::threaded::ThreadedHandler {
    // resolve condition kind
    match value_kinds.get(condition) {
        Some(ValueKind::Bool) => dispatch::handle_branch_bool,
        _ => dispatch::handle_branch,
    }
}

/// Pick a switch handler based on inferred value kind.
fn select_switch_handler(
    value_kinds: &ValueKinds,
    value: mir::Value,
) -> super::threaded::ThreadedHandler {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => dispatch::handle_switch_int,
        _ => dispatch::handle_switch,
    }
}

/// Convert a MIR function to threaded form for fast execution.
pub(super) fn thread_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
) -> Option<ThreadedFunction> {
    // load function
    let func = tree.get(func_id);

    // imported functions can't be threaded
    if func.is_import() {
        return None;
    }

    // read entry block
    let entry_block = func.entry?;

    // prepare block index mapping
    let mut block_index_map: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut mir_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // seed traversal queue
    let mut queue = vec![entry_block];
    let mut visited = std::collections::HashSet::new();

    while let Some(block_id) = queue.pop() {
        // skip visited blocks
        if visited.contains(&block_id) {
            continue;
        }

        // record block index
        visited.insert(block_id);
        let idx = mir_blocks.len();
        block_index_map.insert(block_id, idx);
        mir_blocks.push(block_id);

        // enqueue successor blocks
        let block = tree.get(block_id);
        match &block.terminator {
            mir::Terminator::Jump { target, .. } => {
                queue.push(*target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                queue.push(*then_target);
                queue.push(*else_target);
            }
            mir::Terminator::Switch { cases, default, .. } => {
                for case in cases {
                    queue.push(case.target);
                }
                queue.push(*default);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::Yield { .. } => {}
        }
    }

    // compute value kinds for typed dispatch
    let value_count = compute_value_count_from_mir(tree, func, &mir_blocks);
    let value_kinds = build_value_kinds(tree, func, &mir_blocks, value_count);

    // validate threaded indices
    debug_assert!(
        mir_blocks.len() <= u32::MAX as usize,
        "too many blocks for threaded indices"
    );

    // allocate argument pools
    let mut argument_pool = Vec::new();
    let mut switch_case_pool = Vec::new();

    // gather function parameters
    let parameter_values: Vec<mir::Value> = func.parameters.iter().map(|p| p.value).collect();
    let parameters = push_argument_range(&mut argument_pool, &parameter_values);

    // allocate threaded blocks
    let mut threaded_blocks = Vec::with_capacity(mir_blocks.len());

    // thread each mir block
    for mir_block_id in &mir_blocks {
        let block = tree.get(*mir_block_id);
        let threaded = thread_block(
            tree,
            *mir_block_id,
            block,
            &block_index_map,
            &value_kinds,
            &mut argument_pool,
            &mut switch_case_pool,
        );
        threaded_blocks.push(threaded);
    }

    // compute storage sizes
    let local_count = func.locals.len();

    // assemble threaded function
    Some(ThreadedFunction {
        parameters,
        entry: block_index_map[&entry_block] as u32,
        blocks: threaded_blocks,
        argument_pool,
        switch_case_pool,
        value_count,
        local_count,
    })
}

/// Convert a MIR block to threaded form.
fn thread_block(
    tree: &mir::NodeTree,
    mir_block: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    value_kinds: &ValueKinds,
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
) -> ThreadedBlock {
    // preallocate instruction list
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);

    // convert regular instructions
    for &inst_id in &block.instructions {
        let inst = tree.get(inst_id);
        let threaded = thread_instruction(tree, inst, value_kinds, argument_pool);
        instructions.push(threaded);
    }

    // append threaded terminator
    let terminator = thread_terminator(
        &block.terminator,
        block_index_map,
        value_kinds,
        argument_pool,
        switch_case_pool,
    );
    instructions.push(terminator);

    // gather block parameters
    let parameter_values: Vec<mir::Value> = block.parameters.iter().map(|p| p.value).collect();
    let parameters = push_argument_range(argument_pool, &parameter_values);

    // assemble block
    ThreadedBlock {
        mir_block,
        parameters,
        instructions,
    }
}

/// Convert a MIR instruction to threaded form.
fn thread_instruction(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
    argument_pool: &mut Vec<mir::Value>,
) -> ThreadedInstruction {
    // map instruction opcode to threaded form
    match inst {
        mir::Instruction::Const { destination, value } => ThreadedInstruction {
            handler: dispatch::handle_const,
            data: ThreadedInstructionData::Const {
                dest: *destination,
                value: Value::from(value),
            },
        },

        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => ThreadedInstruction {
            handler: select_binary_handler(value_kinds, *left, *operator),
            data: ThreadedInstructionData::Binary {
                dest: *destination,
                op: *operator,
                left: *left,
                right: *right,
            },
        },

        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => ThreadedInstruction {
            handler: select_unary_handler(value_kinds, *argument, *operator),
            data: ThreadedInstructionData::Unary {
                dest: *destination,
                op: *operator,
                arg: *argument,
            },
        },

        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => ThreadedInstruction {
            handler: dispatch::handle_cast,
            data: ThreadedInstructionData::Cast {
                dest: *destination,
                op: *operator,
                arg: *argument,
                to_type: to_type.id,
            },
        },

        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            ThreadedInstruction {
                handler: dispatch::handle_call,
                data: ThreadedInstructionData::Call {
                    dest: pack_optional_value(*destination),
                    function: function.id,
                    arguments: args,
                },
            }
        }

        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            ThreadedInstruction {
                handler: dispatch::handle_call_indirect,
                data: ThreadedInstructionData::CallIndirect {
                    dest: pack_optional_value(*destination),
                    callee: *callee,
                    arguments: args,
                },
            }
        }

        mir::Instruction::LocalGet { destination, local } => ThreadedInstruction {
            handler: dispatch::handle_local_get,
            data: ThreadedInstructionData::LocalGet {
                dest: *destination,
                local: local.id,
            },
        },

        mir::Instruction::LocalSet { local, value } => ThreadedInstruction {
            handler: dispatch::handle_local_set,
            data: ThreadedInstructionData::LocalSet {
                local: local.id,
                value: *value,
            },
        },

        mir::Instruction::GlobalAddr {
            destination,
            global,
        } => ThreadedInstruction {
            handler: dispatch::handle_global_addr,
            data: ThreadedInstructionData::GlobalAddr {
                dest: *destination,
                global: global.id,
            },
        },

        mir::Instruction::GlobalConst {
            destination,
            global,
        } => ThreadedInstruction {
            handler: dispatch::handle_global_const,
            data: ThreadedInstructionData::GlobalConst {
                dest: *destination,
                global: global.id,
            },
        },

        mir::Instruction::Load {
            destination,
            pointer,
        } => ThreadedInstruction {
            handler: dispatch::handle_load,
            data: ThreadedInstructionData::Load {
                dest: *destination,
                pointer: *pointer,
            },
        },

        mir::Instruction::Store { pointer, value } => ThreadedInstruction {
            handler: dispatch::handle_store,
            data: ThreadedInstructionData::Store {
                pointer: *pointer,
                value: *value,
            },
        },

        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => ThreadedInstruction {
            handler: dispatch::handle_field_get,
            data: ThreadedInstructionData::FieldGet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
            },
        },

        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => ThreadedInstruction {
            handler: dispatch::handle_field_set,
            data: ThreadedInstructionData::FieldSet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => ThreadedInstruction {
            handler: dispatch::handle_element_get,
            data: ThreadedInstructionData::ElementGet {
                dest: *destination,
                array: *array,
                index: *index,
            },
        },

        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => ThreadedInstruction {
            handler: dispatch::handle_element_set,
            data: ThreadedInstructionData::ElementSet {
                dest: *destination,
                array: *array,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ManagedAlloc { destination, .. } => ThreadedInstruction {
            handler: dispatch::handle_managed_alloc,
            data: ThreadedInstructionData::ManagedAlloc { dest: *destination },
        },

        mir::Instruction::ManagedAllocArray {
            destination,
            length,
            ..
        } => ThreadedInstruction {
            handler: dispatch::handle_managed_alloc_array,
            data: ThreadedInstructionData::ManagedAllocArray {
                dest: *destination,
                length: *length,
            },
        },

        mir::Instruction::RawAlloc { destination, .. } => ThreadedInstruction {
            handler: dispatch::handle_raw_alloc,
            data: ThreadedInstructionData::RawAlloc { dest: *destination },
        },

        mir::Instruction::RawFree { pointer } => ThreadedInstruction {
            handler: dispatch::handle_raw_free,
            data: ThreadedInstructionData::RawFree { pointer: *pointer },
        },

        mir::Instruction::StackAlloc { destination, .. } => ThreadedInstruction {
            handler: dispatch::handle_stack_alloc,
            data: ThreadedInstructionData::StackAlloc { dest: *destination },
        },

        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            ThreadedInstruction {
                handler: dispatch::handle_intrinsic,
                data: ThreadedInstructionData::Intrinsic {
                    dest: pack_optional_value(*destination),
                    intrinsic: *intrinsic,
                    arguments: args,
                    ordering: *ordering,
                },
            }
        }
    }
}

/// Build the value kind table for typed dispatch.
fn build_value_kinds(
    tree: &mir::NodeTree,
    func: &mir::Function,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_count: usize,
) -> ValueKinds {
    // allocate value kinds
    let mut value_kinds = ValueKinds::new(value_count);

    // seed function parameter kinds
    for param in &func.parameters {
        value_kinds.set(param.value, kind_from_type(tree, param.ty));
    }

    // seed block parameter kinds
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        for param in &block.parameters {
            value_kinds.set(param.value, kind_from_type(tree, param.ty));
        }
    }

    // iteratively infer instruction results
    let mut changed = true;
    while changed {
        // reset iteration flag
        changed = false;

        // scan instructions for new kinds
        for block_id in mir_blocks {
            let block = tree.get(*block_id);
            // scan block instructions
            for inst_id in &block.instructions {
                let inst = tree.get(*inst_id);
                let Some(dest) = inst.destination() else {
                    continue;
                };
                if value_kinds.get(dest).is_some() {
                    continue;
                }
                let Some(kind) = infer_instruction_kind(tree, inst, &value_kinds) else {
                    continue;
                };
                value_kinds.set(dest, kind);
                changed = true;
            }
        }
    }

    // return inferred kinds
    value_kinds
}

/// Infer the value kind for a MIR instruction.
fn infer_instruction_kind(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
) -> Option<ValueKind> {
    // resolve instruction kind
    match inst {
        mir::Instruction::Const { value, .. } => Some(kind_from_constant(value)),
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            // comparisons always produce bool
            if operator.is_comparison() {
                return Some(ValueKind::Bool);
            }

            // resolve operand kind
            let lhs_kind = value_kinds.get(*left);
            let rhs_kind = value_kinds.get(*right);
            let operand_kind = lhs_kind.or(rhs_kind);

            // select result kind by operand and operator
            match (operator.is_float(), operand_kind) {
                (true, Some(ValueKind::Float { width })) => Some(ValueKind::Float { width }),
                (false, Some(ValueKind::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueKind::Bool)
                }
                (false, Some(ValueKind::Int { width, signed })) => {
                    Some(ValueKind::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_kinds.get(*argument),
        mir::Instruction::Cast { to_type, .. } => Some(kind_from_type(tree, *to_type)),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            destination.as_ref()?;
            let func = tree.get(*function);
            Some(kind_from_type(tree, func.return_type))
        }
        mir::Instruction::CallIndirect {
            destination,
            callee,
            ..
        } => {
            destination.as_ref()?;
            let callee_kind = value_kinds.get(*callee)?;
            match callee_kind {
                ValueKind::FunctionPointer { result } => Some(kind_from_type(tree, result)),
                _ => None,
            }
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(*local);
            Some(kind_from_type(tree, local.ty))
        }
        mir::Instruction::GlobalAddr { global, .. } => {
            let global = tree.get(*global);
            Some(ValueKind::Pointer { pointee: global.ty })
        }
        mir::Instruction::GlobalConst { global, .. } => {
            let global = tree.get(*global);
            Some(kind_from_type(tree, global.ty))
        }
        mir::Instruction::Load { pointer, .. } => {
            let pointer_kind = value_kinds.get(*pointer)?;
            kind_from_pointer(tree, pointer_kind)
        }
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kinds.get(*aggregate)?;
            kind_from_field(tree, aggregate_kind, *index)
        }
        mir::Instruction::FieldSet { aggregate, .. } => value_kinds.get(*aggregate),
        mir::Instruction::ElementGet { array, .. } => {
            let array_kind = value_kinds.get(*array)?;
            kind_from_element(tree, array_kind)
        }
        mir::Instruction::ElementSet { array, .. } => value_kinds.get(*array),
        mir::Instruction::ManagedAlloc { layout, .. } => {
            Some(ValueKind::Pointer { pointee: *layout })
        }
        mir::Instruction::ManagedAllocArray { element, .. } => {
            Some(ValueKind::Array { element: *element })
        }
        mir::Instruction::RawAlloc { layout, .. } | mir::Instruction::StackAlloc { layout, .. } => {
            Some(ValueKind::Pointer { pointee: *layout })
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_kind(tree, *intrinsic, *arguments, value_kinds),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::RawFree { .. } => None,
    }
}

/// Infer the value kind for an intrinsic call.
fn infer_intrinsic_kind(
    tree: &mir::NodeTree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_kinds: &ValueKinds,
) -> Option<ValueKind> {
    // load argument values
    let args = tree.get_arguments(arguments);

    // resolve result kind from intrinsic metadata
    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Bool => Some(ValueKind::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueKind::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueKind::Int {
            width: 64,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueKind::Int {
            width: 64,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let arg = args.get(index as usize)?;
            value_kinds.get(*arg)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let arg = args.get(index as usize)?;
            let pointer_kind = value_kinds.get(*arg)?;
            kind_from_pointer(tree, pointer_kind)
        }
        mir::IntrinsicResultType::CheckedArithmetic => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::TypeDescriptor => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::Explicit => Some(ValueKind::Unknown),
    }
}

/// Get the kind for a MIR type.
fn kind_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> ValueKind {
    // map mir type to value kind
    match tree.get(ty) {
        mir::Type::Void => ValueKind::Void,
        mir::Type::Boolean => ValueKind::Bool,
        mir::Type::Int { width, signed } => ValueKind::Int {
            width: *width as u8,
            signed: *signed,
        },
        mir::Type::Float { width } => ValueKind::Float {
            width: *width as u8,
        },
        mir::Type::RawPointer { pointee } | mir::Type::ManagedReference { pointee, .. } => {
            ValueKind::Pointer { pointee: *pointee }
        }
        mir::Type::FunctionPointer { result, .. } => ValueKind::FunctionPointer { result: *result },
        mir::Type::Array { .. } | mir::Type::Tuple { .. } | mir::Type::Struct { .. } => {
            ValueKind::Aggregate { ty }
        }
    }
}

/// Get the kind for a constant value.
fn kind_from_constant(constant: &mir::Constant) -> ValueKind {
    // map constant to value kind
    match constant {
        mir::Constant::Boolean { .. } => ValueKind::Bool,
        mir::Constant::Int {
            width, is_signed, ..
        } => ValueKind::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ValueKind::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { width, .. } => ValueKind::Float { width: *width },
        mir::Constant::String { .. } => ValueKind::Unknown,
        mir::Constant::Char { .. } => ValueKind::Char,
    }
}

/// Resolve the pointee kind from a pointer-like value.
fn kind_from_pointer(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    // resolve pointee kind when available
    match kind {
        ValueKind::Pointer { pointee } => Some(kind_from_type(tree, pointee)),
        _ => None,
    }
}

/// Resolve the field kind for an aggregate value.
fn kind_from_field(tree: &mir::NodeTree, kind: ValueKind, index: u32) -> Option<ValueKind> {
    let ValueKind::Aggregate { ty } = kind else {
        return None;
    };

    // resolve field type from aggregate layout
    match tree.get(ty) {
        mir::Type::Struct { fields } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(kind_from_type(tree, field.ty))
        }
        mir::Type::Tuple { elements } => {
            let field = elements.get(index as usize)?;
            Some(kind_from_type(tree, *field))
        }
        _ => None,
    }
}

/// Resolve the element kind for an array value.
fn kind_from_element(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    // resolve element kind for array layouts
    match kind {
        ValueKind::Array { element } => Some(kind_from_type(tree, element)),
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(kind_from_type(tree, *element)),
            _ => None,
        },
        _ => None,
    }
}

/// Compute the number of SSA values required by a function.
fn compute_value_count_from_mir(
    tree: &mir::NodeTree,
    func: &mir::Function,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
) -> usize {
    // start with no max value id
    let mut max_value: Option<u32> = None;

    // scan function parameters
    for param in &func.parameters {
        update_max_value(&mut max_value, param.value);
    }

    // scan blocks and instructions
    for block_id in mir_blocks {
        let block = tree.get(*block_id);

        // scan block parameters
        for param in &block.parameters {
            update_max_value(&mut max_value, param.value);
        }

        // scan block instructions
        for inst_id in &block.instructions {
            let inst = tree.get(*inst_id);

            // scan instruction destination
            if let Some(dest) = inst.destination() {
                update_max_value(&mut max_value, dest);
            }

            // scan inline instruction uses
            for value in inst.uses() {
                update_max_value(&mut max_value, value);
            }

            // scan externalized argument lists
            match inst {
                mir::Instruction::Call { arguments, .. }
                | mir::Instruction::CallIndirect { arguments, .. }
                | mir::Instruction::Intrinsic { arguments, .. } => {
                    for value in tree.get_arguments(*arguments) {
                        update_max_value(&mut max_value, *value);
                    }
                }
                _ => {}
            }
        }

        // scan terminator uses
        for value in block.terminator.uses() {
            update_max_value(&mut max_value, value);
        }
    }

    max_value.map(|id| id as usize + 1).unwrap_or(0)
}

/// Update the tracked maximum SSA value id.
fn update_max_value(max_value: &mut Option<u32>, value: mir::Value) {
    // grab the raw value id
    let id = value.0;

    // update max tracking
    match max_value {
        Some(current) => {
            if id > *current {
                *current = id;
            }
        }
        None => {
            *max_value = Some(id);
        }
    }
}

/// Append arguments to the pool and return their range.
fn push_argument_range(pool: &mut Vec<mir::Value>, arguments: &[mir::Value]) -> ArgumentRange {
    // fast path: no arguments
    if arguments.is_empty() {
        return ArgumentRange::empty();
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + arguments.len() <= u32::MAX as usize,
        "argument pool overflow"
    );

    // append arguments
    pool.extend_from_slice(arguments);

    // return range
    ArgumentRange {
        start: start as u32,
        len: arguments.len() as u32,
    }
}

/// Append switch cases to the pool and return their range.
fn push_switch_case_range(
    switch_case_pool: &mut Vec<SwitchCase>,
    argument_pool: &mut Vec<mir::Value>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    cases: &[mir::SwitchCase],
) -> SwitchRange {
    // fast path: no cases
    if cases.is_empty() {
        return SwitchRange::empty();
    }

    // compute range start
    let start = switch_case_pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + cases.len() <= u32::MAX as usize,
        "switch case pool overflow"
    );

    // append cases
    for case in cases {
        let arguments = push_argument_range(argument_pool, &case.arguments);
        switch_case_pool.push(SwitchCase {
            value: case.value,
            target: block_index_map[&case.target] as u32,
            arguments,
        });
    }

    // return range
    SwitchRange {
        start: start as u32,
        len: cases.len() as u32,
    }
}

/// Convert a MIR terminator to threaded form.
fn thread_terminator(
    term: &mir::Terminator,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    value_kinds: &ValueKinds,
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
) -> ThreadedInstruction {
    // map terminator opcode to threaded form
    match term {
        mir::Terminator::Return { value } => ThreadedInstruction {
            handler: dispatch::handle_return,
            data: ThreadedInstructionData::Return {
                value: pack_optional_value(*value),
            },
        },

        mir::Terminator::Jump { target, arguments } => ThreadedInstruction {
            handler: dispatch::handle_jump,
            data: ThreadedInstructionData::Jump {
                target: block_index_map[target] as u32,
                arguments: push_argument_range(argument_pool, arguments),
            },
        },

        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => ThreadedInstruction {
            handler: select_branch_handler(value_kinds, *condition),
            data: ThreadedInstructionData::Branch {
                condition: *condition,
                then_target: block_index_map[then_target] as u32,
                then_arguments: push_argument_range(argument_pool, then_arguments),
                else_target: block_index_map[else_target] as u32,
                else_arguments: push_argument_range(argument_pool, else_arguments),
            },
        },

        mir::Terminator::Switch {
            value,
            cases,
            default,
            default_arguments,
        } => {
            let cases =
                push_switch_case_range(switch_case_pool, argument_pool, block_index_map, cases);

            ThreadedInstruction {
                handler: select_switch_handler(value_kinds, *value),
                data: ThreadedInstructionData::Switch {
                    value: *value,
                    cases,
                    default_target: block_index_map[default] as u32,
                    default_arguments: push_argument_range(argument_pool, default_arguments),
                },
            }
        }

        mir::Terminator::Unreachable => ThreadedInstruction {
            handler: dispatch::handle_unreachable,
            data: ThreadedInstructionData::Unreachable,
        },

        mir::Terminator::Yield { .. } => ThreadedInstruction {
            handler: dispatch::handle_unsupported,
            data: ThreadedInstructionData::Unsupported { name: "yield" },
        },
    }
}

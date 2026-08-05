use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use destack_core::StringPool;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use destack_mir::{Mutation, clone_instruction_tables};

declare_pass! {
    /// Eliminates tail-recursive calls by converting them to jumps.
    ///
    /// A call is in tail position when it is the last instruction before a return,
    /// and the return value is exactly the call's result (or void for void functions).
    /// This pass transforms such patterns into jumps to the entry block, converting
    /// recursion into iteration and eliminating stack growth.
    ///
    /// Also performs accumulator transformation to convert near-tail-recursive
    /// functions into fully tail-recursive form. The pattern `return x OP call(...)`
    /// where OP is associative (int.add, int.mul, int.and, int.or, int.xor) is transformed by adding
    /// an accumulator parameter.
    #[pass(id = "eliminate-tail-calls")]
    pub EliminateTailCalls,
    "Eliminate tail-recursive calls"
}

impl ModulePass for EliminateTailCalls {
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mut mir::ModuleAnalyses,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let layouts = &mut optimized.layouts;
        let memory = &mut optimized.memory;

        // eliminate tail calls across the module
        let changed = eliminate_tail_calls(tree, layouts, memory, ctx.strings);
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "EliminateTailCalls"
    }

    fn id(&self) -> &'static str {
        "eliminate-tail-calls"
    }
}

/// Eliminate tail calls across one MIR tree.
fn eliminate_tail_calls(
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    memory: &mut mir::MemoryTable,
    strings: &StringPool,
) -> bool {
    let mut changed = false;

    // collect function ids first to avoid borrow issues
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    for function_id in function_ids {
        let function = tree.get(function_id);

        // skip imported functions
        let Some(entry_block) = function.entry() else {
            continue;
        };

        // clone function for mutation
        let mut function = function.clone();

        // phase 1: try accumulator transformation to enable more tail calls
        // (this may modify call sites in other functions, or create wrapper for exported)
        if try_accumulator_transform(
            &mut function,
            tree,
            layouts,
            memory,
            function_id,
            entry_block,
            strings,
        ) {
            changed = true;
        }

        // phase 2: transform self-recursive tail calls to jumps
        for &block_id in function.blocks() {
            if transform_self_recursive_tail_call(block_id, function_id, entry_block, tree) {
                changed = true;
            }
        }

        // phase 3: transform sibling tail calls (calls to OTHER functions)
        // These become TailCall terminators for codegen optimization
        for &block_id in function.blocks() {
            if transform_sibling_tail_call(block_id, function_id, tree) {
                changed = true;
            }
        }

        // write function back
        *tree.get_mut(function_id) = function;
    }

    changed
}

/// Information about a block with the accumulator pattern.
#[derive(Debug)]
struct AccumulatorPattern {
    /// The block containing the pattern.
    block_id: mir::LocalNodeId<mir::Block>,
    /// The binary operator used.
    operator: mir::BinaryOperator,
    /// The "other" operand (not the call result).
    other_operand: mir::Value,
    /// Index of the call instruction in the block.
    call_index: usize,
    /// Index of the binary instruction in the block.
    binary_index: usize,
    /// The call arguments.
    call_arguments: mir::ValueSlice,
    /// The call signature type.
    call_signature: mir::TypeId,
}

/// Try to transform a non-tail-recursive function into tail-recursive form.
///
/// Pattern: `v1 = call self(...); v2 = OP v1, x; return v2` (or OP x, v1)
/// Transform: add accumulator parameter, accumulate before recursing.
///
/// For local functions: modifies in place and updates all call sites.
/// For exported functions: creates internal `func_impl` with accumulator,
/// rewrites original as a thin wrapper that calls impl with identity.
fn try_accumulator_transform(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    memory: &mut mir::MemoryTable,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    strings: &StringPool,
) -> bool {
    // find all blocks with the accumulator pattern
    let patterns = find_accumulator_patterns(function, tree, current_function_id);
    if patterns.is_empty() {
        return false;
    }

    // all patterns must use the same associative operator
    let operator = patterns[0].operator;
    if !is_associative_operator(operator) {
        return false;
    }
    if !patterns.iter().all(|p| p.operator == operator) {
        return false;
    }

    // get the return type to determine the identity constant
    let return_type = function.return_type;
    let Some(identity) = identity_constant_for_operator(operator, return_type, tree) else {
        return false;
    };

    // exported functions need special handling: create impl + wrapper
    if function.linkage.is_exported() {
        return try_accumulator_transform_exported(
            function,
            tree,
            layouts,
            memory,
            current_function_id,
            entry_block,
            &patterns,
            operator,
            &identity,
            strings,
        );
    }

    // local function: transform in place and update call sites
    let base_cases = find_base_case_blocks(function, tree, current_function_id, &identity);
    let recursive_call_blocks: Vec<_> = patterns.iter().map(|p| p.block_id).collect();
    let call_sites = find_external_call_sites(current_function_id, &recursive_call_blocks, tree);

    // ensure we allocate fresh values with types recorded
    function.recompute_next_value_id(tree);

    // add accumulator parameter to entry block
    let acc_value = function.next_typed_value(return_type);

    let entry = tree.get(entry_block);
    let mut new_entry = entry.clone();
    new_entry.parameters.push(mir::BlockParameter {
        value: acc_value,
        ty: return_type,
    });
    tree.set(entry_block, new_entry);

    // also add to function parameters
    function
        .parameters
        .push(mir::FunctionParameter::new(acc_value, return_type));

    // insert the expanded signature once for every rewritten callsite
    let signature = tree.intern_type(function.signature());
    layouts.copy_type_entries(patterns[0].call_signature, signature);

    // update all external call sites to pass the identity constant
    let mut call_sites_by_function: HashMap<_, Vec<_>> = HashMap::new();
    for call_site in call_sites {
        call_sites_by_function
            .entry(call_site.function_id)
            .or_default()
            .push(call_site);
    }

    for (function_id, sites) in call_sites_by_function {
        let mut function = tree.get(function_id).clone();
        function.recompute_next_value_id(tree);

        for call_site in sites {
            let identity_value = function.next_typed_value(return_type);
            update_call_site(&call_site, identity_value, &identity, signature, tree);
        }

        *tree.get_mut(function_id) = function;
    }

    // transform each accumulator pattern block
    for pattern in &patterns {
        transform_accumulator_block(pattern, entry_block, acc_value, function, tree);
    }

    // transform base case blocks
    for &(block_id, is_identity) in &base_cases {
        transform_base_case_block(block_id, acc_value, operator, is_identity, function, tree);
    }

    true
}

/// Transform an exported function using the wrapper approach.
///
/// Creates an internal `func_impl` with the accumulator parameter,
/// and rewrites the original exported function as a thin wrapper.
fn try_accumulator_transform_exported(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    memory: &mut mir::MemoryTable,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    patterns: &[AccumulatorPattern],
    operator: mir::BinaryOperator,
    identity: &mir::Constant,
    strings: &StringPool,
) -> bool {
    let return_type = function.return_type;

    // create the impl function name
    let impl_name_str = format!("{}_impl", strings.get(function.name));
    let impl_name = strings.intern(&impl_name_str);

    // clone the function to create the impl version
    let (impl_function_id, impl_entry_block, block_map) =
        clone_function_as_impl(function, tree, memory, impl_name);

    // find base cases in the IMPL function (using mapped block IDs)
    let impl_function = tree.get(impl_function_id).clone();
    let impl_base_cases =
        find_base_case_blocks(&impl_function, tree, current_function_id, identity);
    let impl_patterns: Vec<AccumulatorPattern> = patterns
        .iter()
        .map(|p| AccumulatorPattern {
            block_id: block_map[&p.block_id],
            operator: p.operator,
            other_operand: p.other_operand,
            call_index: p.call_index,
            binary_index: p.binary_index,
            call_arguments: p.call_arguments,
            call_signature: p.call_signature,
        })
        .collect();

    let mut impl_function = tree.get(impl_function_id).clone();
    impl_function.recompute_next_value_id(tree);

    // add accumulator parameter to impl entry block
    let acc_value = impl_function.next_typed_value(return_type);

    let impl_entry = tree.get(impl_entry_block);
    let mut new_impl_entry = impl_entry.clone();
    new_impl_entry.parameters.push(mir::BlockParameter {
        value: acc_value,
        ty: return_type,
    });
    tree.set(impl_entry_block, new_impl_entry);

    // add to impl function parameters
    impl_function
        .parameters
        .push(mir::FunctionParameter::new(acc_value, return_type));

    // insert the expanded implementation signature
    let impl_signature = tree.intern_type(impl_function.signature());
    layouts.copy_type_entries(patterns[0].call_signature, impl_signature);

    // transform impl function's accumulator blocks
    for pattern in &impl_patterns {
        transform_accumulator_block(
            pattern,
            impl_entry_block,
            acc_value,
            &mut impl_function,
            tree,
        );
    }

    // transform impl function's base case blocks
    for &(block_id, is_identity) in &impl_base_cases {
        transform_base_case_block(
            block_id,
            acc_value,
            operator,
            is_identity,
            &mut impl_function,
            tree,
        );
    }

    *tree.get_mut(impl_function_id) = impl_function;

    // rewrite original function as wrapper: call impl with identity
    rewrite_as_wrapper(
        function,
        tree,
        entry_block,
        impl_function_id,
        impl_signature,
        identity,
    );

    true
}

/// Clone a function to create an internal impl version.
///
/// Returns (impl_function_id, impl_entry_block, block_mapping).
#[allow(clippy::type_complexity)]
fn clone_function_as_impl(
    original: &mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    impl_name: destack_core::StringId,
) -> (
    mir::LocalNodeId<mir::Function>,
    mir::LocalNodeId<mir::Block>,
    std::collections::HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
) {
    use std::collections::HashMap;

    let mut block_map: HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>> =
        HashMap::new();

    // clone all blocks
    for &old_block_id in original.blocks() {
        // clone block first to release borrow on tree
        let old_block = tree.get(old_block_id).clone();

        // clone instructions
        let mut new_instructions = Vec::new();
        for &old_instr_id in &old_block.instructions {
            let old_instr = tree.get(old_instr_id).clone();
            let new_instr_id = tree.insert(old_instr);
            clone_instruction_tables(tree, memory, old_instr_id, new_instr_id, &HashMap::new());
            new_instructions.push(new_instr_id);
        }

        // create new block (terminator block refs fixed up later)
        let new_terminator = tree.get(old_block.terminator).clone();
        let new_terminator_id = tree.insert(new_terminator);
        let new_block = mir::Block {
            parameters: old_block.parameters.clone(),
            instructions: new_instructions,
            terminator: new_terminator_id,
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(old_block_id, new_block_id);
    }

    // fix up terminators to use new block IDs
    for &new_block_id in block_map.values() {
        let block = tree.get(new_block_id).clone();
        let terminator = tree.get(block.terminator).clone();
        let fixed_terminator = remap_terminator_blocks(tree, &terminator, &block_map);
        tree.set(block.terminator, fixed_terminator);
    }

    // create impl function
    let impl_entry = block_map[&original.entry().unwrap()];
    let impl_blocks: Vec<_> = original.blocks().iter().map(|id| block_map[id]).collect();
    let mut impl_function = mir::Function::define(
        impl_name,
        original.lifetimes.clone(),
        original.parameters.clone(),
        original.return_type,
        impl_entry,
    );
    impl_function.linkage = mir::Linkage::Local;
    impl_function.allocation = original.allocation;
    impl_function.replace_locals(original.locals().to_vec());
    impl_function.replace_blocks(impl_blocks, tree);
    impl_function.replace_value_types(original.value_types().to_vec());

    // recompute next_value_id after cloning
    impl_function.recompute_next_value_id(tree);

    let impl_function_id = tree.insert(impl_function);

    (impl_function_id, impl_entry, block_map)
}

/// Remap block references in a terminator.
fn remap_terminator_blocks(
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    block_map: &std::collections::HashMap<
        mir::LocalNodeId<mir::Block>,
        mir::LocalNodeId<mir::Block>,
    >,
) -> mir::Terminator {
    let remap_block = |block| block_map.get(&block).copied().unwrap_or(block);
    let clone_target = |target: &mir::BlockTarget| {
        mir::BlockTarget::new(remap_block(target.block), target.arguments)
    };

    match terminator {
        mir::Terminator::Error => {
            panic!("invalid MIR terminator reached optimizer");
        }
        mir::Terminator::Jump { target } => mir::Terminator::Jump {
            target: clone_target(target),
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => mir::Terminator::Branch {
            condition: *condition,
            then_target: clone_target(then_target),
            else_target: clone_target(else_target),
        },
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => mir::Terminator::Check {
            constraint: constraint.clone(),
            success: clone_target(success),
            failure: clone_target(failure),
        },
        mir::Terminator::NewZeroedTry {
            storage_type,
            success,
            failure,
        } => mir::Terminator::NewZeroedTry {
            storage_type: *storage_type,
            success: clone_target(success),
            failure: clone_target(failure),
        },
        mir::Terminator::NewUninitTry {
            storage_type,
            success,
            failure,
        } => mir::Terminator::NewUninitTry {
            storage_type: *storage_type,
            success: clone_target(success),
            failure: clone_target(failure),
        },
        mir::Terminator::NewSliceZeroedTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceZeroedTry {
            element: *element,
            length: *length,
            success: clone_target(success),
            failure: clone_target(failure),
        },
        mir::Terminator::NewSliceUninitTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceUninitTry {
            element: *element,
            length: *length,
            success: clone_target(success),
            failure: clone_target(failure),
        },
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let cases = tree
                .get_switch_cases(*cases)
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: clone_target(&case.target),
                })
                .collect::<Vec<_>>();
            let cases = tree.add_switch_cases(&cases);

            mir::Terminator::Switch {
                value: *value,
                default: clone_target(default),
                cases,
            }
        }
        mir::Terminator::VariantSwitch {
            value,
            default,
            cases,
        } => {
            let cases = tree
                .get_switch_cases(*cases)
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: clone_target(&case.target),
                })
                .collect::<Vec<_>>();
            let cases = tree.add_switch_cases(&cases);

            mir::Terminator::VariantSwitch {
                value: *value,
                default: default.as_ref().map(clone_target),
                cases,
            }
        }
        mir::Terminator::Invoke {
            call,
            target,
            unwind,
        } => mir::Terminator::Invoke {
            call: call.clone(),
            target: clone_target(target),
            unwind: clone_target(unwind),
        },
        // return, unreachable, tailcall don't reference blocks that need remapping
        mir::Terminator::Return { .. }
        | mir::Terminator::Abort { .. }
        | mir::Terminator::Panic { .. }
        | mir::Terminator::UnwindResume
        | mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. } => terminator.clone(),
    }
}

/// Rewrite a function as a thin wrapper that calls impl with identity.
fn rewrite_as_wrapper(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    entry_block: mir::LocalNodeId<mir::Block>,
    impl_function_id: mir::LocalNodeId<mir::Function>,
    signature: mir::TypeId,
    identity: &mir::Constant,
) {
    // ensure new values get typed ids
    function.recompute_next_value_id(tree);

    // resolve the return type for wrapper values
    let return_type = function.return_type;

    // create identity constant
    let identity_value = function.next_typed_value(return_type);
    let const_instr = mir::Instruction::Const {
        destination: identity_value,
        value: identity.clone(),
    };
    let const_id = tree.insert(const_instr);

    // build call arguments: original params + identity
    let mut call_args: Vec<mir::Value> = function
        .parameters
        .iter()
        .map(|parameter| parameter.value)
        .collect();
    call_args.push(identity_value);
    let call_arguments = tree.add_values(&call_args);

    // create call to impl
    let result_value = function.next_typed_value(return_type);
    let call_instr = mir::Instruction::Call {
        destination: Some(result_value),
        call: mir::Call::new(
            mir::Callee::Direct {
                function: impl_function_id,
            },
            call_arguments,
            signature,
        ),
    };
    let call_id = tree.insert(call_instr);

    // create new entry block with just: const, call, return
    let entry = tree.get(entry_block).clone();
    let entry_terminator = tree.insert(mir::Terminator::Return {
        value: Some(result_value),
    });
    let new_entry = mir::Block {
        parameters: entry.parameters.clone(),
        instructions: vec![const_id, call_id],
        terminator: entry_terminator,
    };
    tree.set(entry_block, new_entry);

    // clear other blocks from function (they're now orphaned, DCE will clean up)
    function.replace_blocks(vec![entry_block], tree);
}

/// Information about a call site that needs to be updated.
#[derive(Debug)]
struct CallSite {
    /// The function containing the call.
    function_id: mir::LocalNodeId<mir::Function>,
    /// The block containing the call.
    block_id: mir::LocalNodeId<mir::Block>,
    /// Index of the call instruction in the block.
    instruction_index: usize,
    /// The call instruction id.
    instruction_id: mir::LocalNodeId<mir::Instruction>,
}

/// Find all call sites to the target function, excluding recursive calls.
///
/// These are the call sites we need to update to pass the identity constant.
fn find_external_call_sites(
    target_function_id: mir::LocalNodeId<mir::Function>,
    recursive_call_blocks: &[mir::LocalNodeId<mir::Block>],
    tree: &mir::Tree,
) -> Vec<CallSite> {
    let mut call_sites = Vec::new();

    // scan all functions in the module
    for (func_id, func) in tree.iter_nodes::<mir::Function>() {
        // skip imported functions (no body)
        let Some(_entry) = func.entry() else {
            continue;
        };

        // check all blocks in this function
        for &block_id in func.blocks() {
            // skip the recursive call blocks (they become jumps)
            if func_id == target_function_id && recursive_call_blocks.contains(&block_id) {
                continue;
            }

            let block = tree.get(block_id);

            // check all instructions for calls to target
            for (idx, &instr_id) in block.instructions.iter().enumerate() {
                let instr = tree.get(instr_id);
                if let mir::Instruction::Call { call, .. } = instr
                    && call.callee.function() == Some(target_function_id)
                {
                    call_sites.push(CallSite {
                        function_id: func_id,
                        block_id,
                        instruction_index: idx,
                        instruction_id: instr_id,
                    });
                }
            }
        }
    }

    call_sites
}

/// Update a call site to pass the identity constant as the accumulator argument.
fn update_call_site(
    call_site: &CallSite,
    identity_value: mir::Value,
    identity: &mir::Constant,
    signature: mir::TypeId,
    tree: &mut mir::Tree,
) {
    // get the existing call instruction
    let call_instr = tree.get(call_site.instruction_id).clone();
    let mir::Instruction::Call {
        destination,
        mut call,
    } = call_instr
    else {
        return;
    };
    // create a value for the identity constant
    // create the const instruction
    let const_instr = mir::Instruction::Const {
        destination: identity_value,
        value: identity.clone(),
    };
    let const_id = tree.insert(const_instr);

    // get the existing arguments and append the identity
    let mut new_args: Vec<mir::Value> = tree.get_values(call.arguments).to_vec();
    new_args.push(identity_value);

    // create new call with extended arguments
    let new_arguments = tree.add_values(&new_args);
    call.arguments = new_arguments;
    call.signature = signature;
    let new_call = mir::Instruction::Call { destination, call };
    let new_call_id = tree.insert(new_call);

    // update the block: insert const before call, replace call
    let block = tree.get(call_site.block_id).clone();
    let mut new_instructions = Vec::with_capacity(block.instructions.len() + 1);

    for (idx, &instr_id) in block.instructions.iter().enumerate() {
        if idx == call_site.instruction_index {
            // insert const before the call
            new_instructions.push(const_id);
            // replace old call with new call
            new_instructions.push(new_call_id);
        } else {
            new_instructions.push(instr_id);
        }
    }

    tree.replace_block_instructions(call_site.function_id, call_site.block_id, new_instructions);
}

/// Check if an operator is associative (and commutative for safety).
fn is_associative_operator(op: mir::BinaryOperator) -> bool {
    matches!(
        op,
        mir::BinaryOperator::Add
            | mir::BinaryOperator::Multiply
            | mir::BinaryOperator::And
            | mir::BinaryOperator::Or
            | mir::BinaryOperator::Xor
    )
}

/// Get the identity constant for an operator and type.
fn identity_constant_for_operator(
    op: mir::BinaryOperator,
    type_id: mir::LocalNodeId<mir::Type>,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    let ty = tree.get(type_id);

    // require integer identity constants
    let mir::Type::Int {
        width,
        is_signed: signed,
    } = ty
    else {
        return None;
    };

    let identity_value: i64 = match op {
        mir::BinaryOperator::Add | mir::BinaryOperator::Or | mir::BinaryOperator::Xor => 0,
        mir::BinaryOperator::Multiply => 1,
        mir::BinaryOperator::And => {
            // all ones: -1 for signed, max value for unsigned
            if *signed {
                -1
            } else {
                match width {
                    8 => u8::MAX as i64,
                    16 => u16::MAX as i64,
                    32 => u32::MAX as i64,
                    64 => -1, // wraps to all ones
                    _ => return None,
                }
            }
        }
        _ => return None,
    };

    Some(mir::Constant::Int {
        value: i128::from(identity_value),
        width: *width,
        is_signed: *signed,
    })
}

/// Find all blocks with the accumulator pattern.
fn find_accumulator_patterns(
    function: &mir::Function,
    tree: &mir::Tree,
    current_function_id: mir::LocalNodeId<mir::Function>,
) -> Vec<AccumulatorPattern> {
    let mut patterns = Vec::new();

    for &block_id in function.blocks() {
        if let Some(pattern) = detect_accumulator_pattern(block_id, tree, current_function_id) {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Detect the accumulator pattern in a single block.
///
/// Pattern: call self => binary op using call result -> return binary result
fn detect_accumulator_pattern(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    current_function_id: mir::LocalNodeId<mir::Function>,
) -> Option<AccumulatorPattern> {
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);

    // must end with return of a value
    let mir::Terminator::Return {
        value: Some(returned_value),
    } = terminator
    else {
        return None;
    };

    // need at least two instructions (call and binary op)
    if block.instructions.len() < 2 {
        return None;
    }

    // find the binary instruction that produces the return value
    let mut binary_index = None;
    let mut binary_info = None;

    for (idx, &instr_id) in block.instructions.iter().enumerate() {
        let instr = tree.get(instr_id);
        if let mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } = instr
            && *destination == *returned_value
        {
            binary_index = Some(idx);
            binary_info = Some((*operator, *left, *right));
            break;
        }
    }

    let (binary_idx, (operator, left, right)) = binary_index.zip(binary_info)?;

    // collect recursive call results and definition indices
    let mut recursive_call_results: HashSet<mir::Value> = HashSet::new();
    let mut definition_indices: HashMap<mir::Value, usize> = HashMap::new();
    for (idx, &instr_id) in block.instructions.iter().enumerate() {
        let instr = tree.get(instr_id);
        if let Some(destination) = instr.destination() {
            definition_indices.insert(destination, idx);
        }
        if let mir::Instruction::Call {
            destination: Some(destination),
            call,
        } = instr
            && call.callee.function() == Some(current_function_id)
        {
            recursive_call_results.insert(*destination);
        }
    }

    // find the call instruction that produces one of the binary operands
    for (idx, &instr_id) in block.instructions.iter().enumerate() {
        let instr = tree.get(instr_id);
        if let mir::Instruction::Call {
            destination: Some(call_dest),
            call,
        } = instr
        {
            // must be calling ourselves
            if call.callee.function() != Some(current_function_id) {
                continue;
            }

            // call result must be used by the binary op
            let other_operand = if *call_dest == left {
                right
            } else if *call_dest == right {
                left
            } else {
                continue;
            };

            // skip when the other operand is another recursive call result
            if recursive_call_results.contains(&other_operand) {
                continue;
            }

            // the call must come before the binary op
            if idx >= binary_idx {
                continue;
            }

            // the other operand must be available before the call
            if let Some(def_index) = definition_indices.get(&other_operand)
                && *def_index > idx
            {
                continue;
            }

            // the binary op must be the last instruction using the call result
            // (no other uses between call and return)
            let call_result_used_elsewhere =
                block.instructions[idx + 1..binary_idx]
                    .iter()
                    .any(|&other_id| {
                        let other = tree.get(other_id);
                        other.uses().contains(call_dest)
                    });

            if call_result_used_elsewhere {
                continue;
            }

            return Some(AccumulatorPattern {
                block_id,
                operator,
                other_operand,
                call_index: idx,
                binary_index: binary_idx,
                call_arguments: call.arguments,
                call_signature: call.signature,
            });
        }
    }

    None
}

/// Find blocks that return without recursion (base cases).
///
/// Returns (block_id, is_identity) pairs where is_identity indicates
/// whether the returned value is the identity constant.
fn find_base_case_blocks(
    function: &mir::Function,
    tree: &mir::Tree,
    current_function_id: mir::LocalNodeId<mir::Function>,
    identity: &mir::Constant,
) -> Vec<(mir::LocalNodeId<mir::Block>, bool)> {
    let mut base_cases = Vec::new();

    for &block_id in function.blocks() {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // must end with return
        let mir::Terminator::Return { value } = terminator else {
            continue;
        };

        // check if block contains a recursive call
        let has_recursive_call = block.instructions.iter().any(|&instr_id| {
            let instr = tree.get(instr_id);
            matches!(instr, mir::Instruction::Call { call, .. } if call.callee.function() == Some(current_function_id))
        });
        if has_recursive_call {
            continue;
        }

        // check if return value is the identity
        let is_identity = if let Some(ret_val) = value {
            is_value_identity(*ret_val, identity, function, tree)
        } else {
            false
        };

        base_cases.push((block_id, is_identity));
    }

    base_cases
}

/// Check if a value is the identity constant.
///
/// Searches all blocks in the function to find where the value is defined.
fn is_value_identity(
    value: mir::Value,
    identity: &mir::Constant,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // search all blocks for the defining instruction
    for &block_id in function.blocks() {
        let block = tree.get(block_id);
        for &instr_id in &block.instructions {
            let instr = tree.get(instr_id);
            if let mir::Instruction::Const {
                destination,
                value: const_val,
            } = instr
                && *destination == value
            {
                return const_val == identity;
            }
        }
    }

    false
}

/// Transform an accumulator pattern block.
///
/// Replaces: `v1 = call self(args); v2 = OP v1, x; return v2`
/// With: `v_new = OP acc, x; jump entry(args..., v_new)`
fn transform_accumulator_block(
    pattern: &AccumulatorPattern,
    entry_block: mir::LocalNodeId<mir::Block>,
    acc_value: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) {
    // get the call arguments before any mutations
    let call_args: Vec<mir::Value> = tree.get_values(pattern.call_arguments).to_vec();

    // create new accumulator value
    let acc_type = function.return_type;
    let new_acc = function.next_typed_value(acc_type);

    // create new binary instruction: new_acc = OP acc, other
    let new_binary = mir::Instruction::Binary {
        destination: new_acc,
        operator: pattern.operator,
        left: acc_value,
        right: pattern.other_operand,
    };

    // build new instructions list: keep everything except call and old binary
    let block = tree.get(pattern.block_id);
    let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = block
        .instructions
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != pattern.call_index && i != pattern.binary_index)
        .map(|(_, &id)| id)
        .collect();

    // add the new binary instruction
    let new_binary_id = tree.insert(new_binary);
    new_instructions.push(new_binary_id);

    // create jump with accumulated value
    let mut jump_args = call_args;
    jump_args.push(new_acc);
    let jump_arguments: Vec<_> = jump_args.into_iter().collect();
    let jump_arguments = tree.add_values(&jump_arguments);

    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget::new(entry_block, jump_arguments),
    };

    // replace the block
    let terminator_id = tree.get(pattern.block_id).terminator;
    function.replace_block_instructions(pattern.block_id, new_instructions, tree);
    tree.set(terminator_id, new_terminator);

    // old instructions become orphaned (not referenced by any block)
    // they will be cleaned up by DCE or tree compaction
}

/// Transform a base case block to return the accumulator.
///
/// If the base case returns the identity, simply return acc.
/// Otherwise, return OP(acc, original_value).
fn transform_base_case_block(
    block_id: mir::LocalNodeId<mir::Block>,
    acc_value: mir::Value,
    operator: mir::BinaryOperator,
    is_identity: bool,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) {
    // clone block first to avoid borrow conflicts
    let block = tree.get(block_id).clone();
    let terminator = tree.get(block.terminator);
    let mir::Terminator::Return {
        value: Some(original_value),
    } = terminator
    else {
        return;
    };
    let original_value = *original_value;

    if is_identity {
        // just return the accumulator
        let new_terminator = mir::Terminator::Return {
            value: Some(acc_value),
        };
        tree.set(block.terminator, new_terminator);
    } else {
        // return OP(acc, original_value)
        let acc_type = function.return_type;
        let result_val = function.next_typed_value(acc_type);

        let combine_instr = mir::Instruction::Binary {
            destination: result_val,
            operator,
            left: acc_value,
            right: original_value,
        };
        let combine_id = tree.insert(combine_instr);

        let mut instructions = block.instructions.clone();
        instructions.push(combine_id);
        let new_terminator = mir::Terminator::Return {
            value: Some(result_val),
        };
        let terminator_id = block.terminator;
        function.replace_block_instructions(block_id, instructions, tree);
        tree.set(terminator_id, new_terminator);
    }
}

/// Checks if a block ends with a self-recursive tail call and transforms it to a jump.
///
/// The pattern is: last instruction is `v = call self(args...)`, terminator is `return v`.
/// For void functions: last instruction is `call self(args...)`, terminator is `return`.
fn transform_self_recursive_tail_call(
    block_id: mir::LocalNodeId<mir::Block>,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::Tree,
) -> bool {
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);

    // must end with a return
    let returned_value = match terminator {
        mir::Terminator::Return { value } => *value,
        _ => return false,
    };

    // need at least one instruction
    let Some(&last_instruction_id) = block.instructions.last() else {
        return false;
    };

    // last instruction must be a call
    let last_instruction = tree.get(last_instruction_id);
    let mir::Instruction::Call { destination, call } = last_instruction else {
        return false;
    };

    // must be calling ourselves
    if call.callee.function() != Some(current_function_id) {
        return false;
    }

    // return value must match call result
    let is_tail_position = match (destination, returned_value) {
        (Some(call_result), Some(return_val)) => *call_result == return_val,
        (None, None) => true,
        _ => false,
    };

    if !is_tail_position {
        return false;
    }

    // extract call arguments before mutating
    let call_args: Vec<mir::Value> = tree.get_values(call.arguments).to_vec();
    let jump_arguments: Vec<_> = call_args.into_iter().collect();

    // rewrite the block: remove call, replace return with jump to entry
    let (terminator_id, mut new_instructions) = {
        let block = tree.get(block_id);
        (block.terminator, block.instructions.clone())
    };
    new_instructions.pop();

    let jump_arguments = tree.add_values(&jump_arguments);
    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget::new(entry_block, jump_arguments),
    };

    tree.set(terminator_id, new_terminator);
    tree.replace_block_instructions(current_function_id, block_id, new_instructions);

    true
}

/// Checks if a block ends with a sibling tail call (call to ANOTHER function) and transforms it.
///
/// The pattern is: last instruction is `v = call other(args...)`, terminator is `return v`.
/// For void functions: last instruction is `call other(args...)`, terminator is `return`.
///
/// Transforms to: `tail.call other(args...)` (or `tail.call.indirect` for indirect calls).
fn transform_sibling_tail_call(
    block_id: mir::LocalNodeId<mir::Block>,
    current_function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
) -> bool {
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);

    // must end with a return
    let returned_value = match terminator {
        mir::Terminator::Return { value } => *value,
        _ => return false,
    };

    // need at least one instruction
    let Some(&last_instruction_id) = block.instructions.last() else {
        return false;
    };

    // last instruction must be a call
    let last_instruction = tree.get(last_instruction_id).clone();

    match &last_instruction {
        mir::Instruction::Call { destination, call } => {
            // skip self-recursive calls (handled by transform_self_recursive_tail_call)
            if call.callee.function() == Some(current_function_id) {
                return false;
            }

            // return value must match call result
            let is_tail_position = match (destination, returned_value) {
                (Some(call_result), Some(return_val)) => *call_result == return_val,
                (None, None) => true,
                _ => false,
            };

            if !is_tail_position {
                return false;
            }

            // extract call arguments before mutating
            let call_args: Vec<mir::Value> = tree.get_values(call.arguments).to_vec();

            // rewrite the block: remove call, replace return with TailCall
            let (terminator_id, mut new_instructions) = {
                let block = tree.get(block_id);
                (block.terminator, block.instructions.clone())
            };
            new_instructions.pop();

            let call_args = tree.add_values(&call_args);
            let new_terminator = mir::Terminator::TailCall {
                call: call.remap(call.callee.clone(), call_args),
            };

            tree.set(terminator_id, new_terminator);
            tree.replace_block_instructions(current_function_id, block_id, new_instructions);

            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_eliminate_basic_tail_recursion() {
        // factorial(n, acc) with accumulator style
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.eq v0, v2
    branch v3 => b1 | b2

b1:
    return v1

b2:
    v4: int32 = int.mul v0, v1
    v5: int32 = 1
    v6: int32 = int.sub v0, v5
    v7: int32 = call test(v6, v4): (int32, int32) => int32
    return v7
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.eq v0, v2
    branch v3 => b1 | b2

b1:
    return v1

b2:
    v4: int32 = int.mul v0, v1
    v5: int32 = 1
    v6: int32 = int.sub v0, v5
    jump entry(v6, v4)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_eliminate_void_tail_recursion() {
        // countdown to zero
        let input = r#"
function test(v0: int32): void {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    call test(v4): (int32) => void
    return
}
"#;
        let expected = r#"
function test(v0: int32): void {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    jump entry(v4)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_factorial_with_accumulator() {
        // classic factorial: n * factorial(n-1), transformed via accumulator
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call test(v3): (int32) => int32
    v5: int32 = int.mul v0, v4
    return v5
}
"#;
        // after accumulator transform: adds v6 param, base returns v6, recurse accumulates
        let expected = r#"
function test(v0: int32, v6: int32): int32 {
entry(v0: int32, v6: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v6

b2:
    v3: int32 = int.sub v0, v1
    v7: int32 = int.mul v6, v0
    jump entry(v3, v7)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_preserve_non_associative_operation() {
        // subtraction is not associative, cannot transform
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v5: int32 = call test(v4): (int32) => int32
    v6: int32 = int.sub v0, v5
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_unchanged(input);
    }

    #[test]
    fn test_transform_sibling_tail_call() {
        // sibling call (to different function) in tail position becomes tailcall
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call other(v0): (int32) => int32
    return v1
}

function other(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    tail.call other(v0): (int32) => int32
}

function other(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_eliminate_gcd_recursion() {
        // euclidean gcd is naturally tail recursive
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.eq v1, v2
    branch v3 => b1 | b2

b1:
    return v0

b2:
    v4: int32 = int.rem.s v0, v1
    v5: int32 = call test(v1, v4): (int32, int32) => int32
    return v5
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.eq v1, v2
    branch v3 => b1 | b2

b1:
    return v0

b2:
    v4: int32 = int.rem.s v0, v1
    jump entry(v1, v4)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_convert_infinite_recursion_to_loop() {
        // infinite recursion becomes infinite loop
        let input = r#"
function test(): void {
entry:
    call test(): () => void
    return
}
"#;
        let expected = r#"
function test(): void {
entry:
    jump entry
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_preserve_return_value_mismatch() {
        // returning different value than call result
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = call test(v0): (int32) => int32
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_unchanged(input);
    }

    #[test]
    fn test_eliminate_fibonacci_recursion() {
        // fib(n, a, b) where a and b are accumulators
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: int32 = 0
    v4: boolean = int.eq v0, v3
    branch v4 => b1 | b2

b1:
    return v1

b2:
    v5: int32 = 1
    v6: int32 = int.sub v0, v5
    v7: int32 = int.add v1, v2
    v8: int32 = call test(v6, v2, v7): (int32, int32, int32) => int32
    return v8
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: int32 = 0
    v4: boolean = int.eq v0, v3
    branch v4 => b1 | b2

b1:
    return v1

b2:
    v5: int32 = 1
    v6: int32 = int.sub v0, v5
    v7: int32 = int.add v1, v2
    jump entry(v6, v2, v7)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_eliminate_multiple_tail_calls() {
        // function with multiple blocks that have tail calls
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.lt.s v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = int.negate v0
    v4: int32 = call test(v3): (int32) => int32
    return v4

b2:
    v5: int32 = 10
    v6: boolean = int.gt.s v0, v5
    branch v6 => b3 | b4

b3:
    v7: int32 = int.sub v0, v5
    v8: int32 = call test(v7): (int32) => int32
    return v8

b4:
    return v0
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.lt.s v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = int.negate v0
    jump entry(v3)

b2:
    v5: int32 = 10
    v6: boolean = int.gt.s v0, v5
    branch v6 => b3 | b4

b3:
    v7: int32 = int.sub v0, v5
    jump entry(v7)

b4:
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_eliminate_reordered_args_call() {
        // swap(a, b) calls swap(b, a)
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: boolean = int.gt.s v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = call test(v1, v0): (int32, int32) => int32
    return v3

b2:
    return v0
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: boolean = int.gt.s v0, v1
    branch v2 => b1 | b2

b1:
    jump entry(v1, v0)

b2:
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_handle_empty_block() {
        // block with only terminator, no instructions
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_non_final_call() {
        // call followed by other instruction before return
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = int.sub v0, v1
    v3: int32 = call test(v2): (int32) => int32
    v4: int32 = 0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_unchanged(input);
    }

    #[test]
    fn test_transform_mutual_recursion() {
        // even/odd mutual recursion becomes sibling tail calls
        let input = r#"
function even(v0: int32): boolean {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: boolean = true
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v6: boolean = call odd(v5): (int32) => boolean
    return v6
}

function odd(v0: int32): boolean {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: boolean = false
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v6: boolean = call even(v5): (int32) => boolean
    return v6
}
"#;
        let expected = r#"
function even(v0: int32): boolean {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: boolean = true
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    tail.call odd(v5): (int32) => boolean
}

function odd(v0: int32): boolean {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: boolean = false
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    tail.call even(v5): (int32) => boolean
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_sum_with_accumulator() {
        // sum(n) = n + sum(n-1), identity for add is 0
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v5: int32 = call test(v4): (int32) => int32
    v6: int32 = int.add v0, v5
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v7: int32): int32 {
entry(v0: int32, v7: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v7

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v8: int32 = int.add v7, v0
    jump entry(v4, v8)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_bitwise_or_accumulator() {
        // or_bits(n) = n | or_bits(n-1), identity for or is 0
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v5: int32 = call test(v4): (int32) => int32
    v6: int32 = int.or v0, v5
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v7: int32): int32 {
entry(v0: int32, v7: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v7

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v8: int32 = int.or v7, v0
    jump entry(v4, v8)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_non_identity_base_case() {
        // sum with non-zero base: returns 5 when n=0
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = 5
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v6: int32 = call test(v5): (int32) => int32
    v7: int32 = int.add v0, v6
    return v7
}
"#;
        // base case returns OP(acc, 5) since 5 is not the identity
        let expected = r#"
function test(v0: int32, v8: int32): int32 {
entry(v0: int32, v8: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    v3: int32 = 5
    v10: int32 = int.add v8, v3
    return v10

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v9: int32 = int.add v8, v0
    jump entry(v5, v9)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_preserve_call_result_used_twice() {
        // call result used in multiple places, not just the binary op
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call test(v3): (int32) => int32
    v5: int32 = int.mul v0, v4
    v6: int32 = int.add v5, v4
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_mixed_operators() {
        // multiple recursive sites with different operators
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
entry(v0: int32, v1: boolean):
    v2: int32 = 0
    v3: boolean = int.eq v0, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = 1
    return v4

b2:
    v5: int32 = 1
    v6: int32 = int.sub v0, v5
    branch v1 => b3 | b4

b3:
    v7: int32 = call test(v6, v1): (int32, boolean) => int32
    v8: int32 = int.mul v0, v7
    return v8

b4:
    v9: int32 = call test(v6, v1): (int32, boolean) => int32
    v10: int32 = int.add v0, v9
    return v10
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        // should not transform: different operators in different paths
        test.assert_unchanged(input);
    }

    #[test]
    fn test_transform_with_external_caller() {
        // factorial with accumulator pattern, called from main
        // should transform and update the call site in main to pass identity
        let input = r#"
function factorial(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call factorial(v3): (int32) => int32
    v5: int32 = int.mul v0, v4
    return v5
}

function main(): int32 {
entry:
    v0: int32 = 5
    v1: int32 = call factorial(v0): (int32) => int32
    return v1
}
"#;
        // after transform: factorial gets accumulator param, main's tail call becomes tailcall
        let expected = r#"
function factorial(v0: int32, v6: int32): int32 {
entry(v0: int32, v6: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v6

b2:
    v3: int32 = int.sub v0, v1
    v7: int32 = int.mul v6, v0
    jump entry(v3, v7)
}

function main(): int32 {
entry:
    v0: int32 = 5
    v2: int32 = 1
    tail.call factorial(v0, v2): (int32, int32) => int32
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_exported_with_wrapper() {
        // exported factorial: should create impl + wrapper
        let input = r#"
export function factorial(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call factorial(v3): (int32) => int32
    v5: int32 = int.mul v0, v4
    return v5
}
"#;
        // exported wrapper tail-calls internal impl with identity
        // impl has tail-recursive structure
        let expected = r#"
export function factorial(v0: int32): int32 {
entry(v0: int32):
    v6: int32 = 1
    tail.call factorial_impl(v0, v6): (int32, int32) => int32
}

function factorial_impl(v0: int32, v6: int32): int32 {
entry(v0: int32, v6: int32):
    v1: int32 = 1
    v2: boolean = int.le.s v0, v1
    branch v2 => b1 | b2

b1:
    return v6

b2:
    v3: int32 = int.sub v0, v1
    v7: int32 = int.mul v6, v0
    jump entry(v3, v7)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_indirect_tail_call() {
        // indirect call in tail position becomes tail.call.indirect
        let input = r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}
"#;
        let expected = r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    tail.call.indirect v0(v1): (int32) => int32
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_void_sibling_tail_call() {
        // void sibling tail call
        let input = r#"
function test(v0: int32): void {
entry(v0: int32):
    call other(v0): (int32) => void
    return
}

function other(v0: int32): void {
entry(v0: int32):
    return
}
"#;
        let expected = r#"
function test(v0: int32): void {
entry(v0: int32):
    tail.call other(v0): (int32) => void
}

function other(v0: int32): void {
entry(v0: int32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_bitwise_and_accumulator() {
        // and_bits(n) = n & and_bits(n-1), identity for int.and is all-ones (-1)
        // base case returns v1 (defined in b0) to avoid leftover instruction
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    v3: int32 = -1
    branch v2 => b1 | b2

b1:
    return v3

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v6: int32 = call test(v5): (int32) => int32
    v7: int32 = int.and v0, v6
    return v7
}
"#;
        let expected = r#"
function test(v0: int32, v8: int32): int32 {
entry(v0: int32, v8: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    v3: int32 = -1
    branch v2 => b1 | b2

b1:
    return v8

b2:
    v4: int32 = 1
    v5: int32 = int.sub v0, v4
    v9: int32 = int.and v8, v0
    jump entry(v5, v9)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }

    #[test]
    fn test_transform_bitwise_xor_accumulator() {
        // xor_bits(n) = n ^ xor_bits(n-1), identity for bxor is 0
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v1

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v5: int32 = call test(v4): (int32) => int32
    v6: int32 = int.xor v0, v5
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v7: int32): int32 {
entry(v0: int32, v7: int32):
    v1: int32 = 0
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

b1:
    return v7

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v8: int32 = int.xor v7, v0
    jump entry(v4, v8)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateTailCalls);
        test.assert_output(expected);
    }
}

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{BinaryOperator, Constant, Instruction};

use crate::optimize::{AnalysisPreservation, ModulePass, OptimizationContext, Pass, PassMetadata};

// FUGU: add tailcall MIR instruction for non-recursive tail calls (codegen optimization)

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
    /// where OP is associative (iadd, imul, band, bor, bxor) is transformed by adding
    /// an accumulator parameter.
    #[pass(id = "tail-call-elim")]
    pub TailCallElim,
    "Eliminate tail-recursive calls"
}

impl Pass for TailCallElim {
    fn metadata(&self) -> &'static PassMetadata {
        TailCallElim::metadata()
    }
}

impl ModulePass for TailCallElim {
    fn run_on_module(
        &self,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let mut changed = false;

        // collect function ids first to avoid borrow issues
        let function_ids: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in function_ids {
            let function = tree.get(function_id);

            // skip imported functions
            let Some(entry_block) = function.entry else {
                continue;
            };

            // clone function for mutation
            let mut function = function.clone();

            // phase 1: try accumulator transformation to enable more tail calls
            // (this may modify call sites in other functions, or create wrapper for exported)
            if try_accumulator_transform(&mut function, tree, function_id, entry_block, context) {
                changed = true;
            }

            // phase 2: transform tail calls in each block
            for &block_id in &function.blocks {
                if transform_tail_call(block_id, function_id, entry_block, tree) {
                    changed = true;
                }
            }

            // write function back
            *tree.get_mut(function_id) = function;
        }

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Information about a block with the accumulator pattern.
#[derive(Debug)]
struct AccumulatorPattern {
    /// The block containing the pattern.
    block_id: mir::LocalNodeId<mir::Block>,
    /// The binary operator used.
    operator: BinaryOperator,
    /// The "other" operand (not the call result).
    other_operand: mir::Value,
    /// Index of the call instruction in the block.
    call_index: usize,
    /// Index of the binary instruction in the block.
    binary_index: usize,
    /// The call arguments.
    call_arguments: mir::ArgumentSlice,
}

/// Try to transform a non-tail-recursive function into tail-recursive form.
///
/// Pattern: `v1 = call @self(...); v2 = OP v1, x; return v2` (or OP x, v1)
/// Transform: add accumulator parameter, accumulate before recursing.
///
/// For local functions: modifies in place and updates all call sites.
/// For exported functions: creates internal `@func$impl` with accumulator,
/// rewrites original as a thin wrapper that calls impl with identity.
fn try_accumulator_transform(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    context: &OptimizationContext<'_>,
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
            current_function_id,
            entry_block,
            &patterns,
            operator,
            &identity,
            context,
        );
    }

    // local function: transform in place and update call sites
    let base_cases = find_base_case_blocks(function, tree, current_function_id, &identity);
    let recursive_call_blocks: Vec<_> = patterns.iter().map(|p| p.block_id).collect();
    let call_sites = find_external_call_sites(current_function_id, &recursive_call_blocks, tree);

    let mut next_value = find_max_value_in_module(tree) + 1;

    // add accumulator parameter to entry block
    let acc_value = mir::Value::new(next_value);
    next_value += 1;

    let entry = tree.get(entry_block);
    let mut new_entry = entry.clone();
    new_entry.parameters.push(mir::TypedValue {
        value: acc_value,
        ty: return_type,
    });
    tree.replace(entry_block, new_entry);

    // also add to function parameters
    function.parameters.push(mir::TypedValue {
        value: acc_value,
        ty: return_type,
    });

    // update all external call sites to pass the identity constant
    for call_site in &call_sites {
        update_call_site(call_site, &identity, &mut next_value, tree);
    }

    // transform each accumulator pattern block
    for pattern in &patterns {
        transform_accumulator_block(pattern, entry_block, acc_value, &mut next_value, tree);
    }

    // transform base case blocks
    for &(block_id, is_identity) in &base_cases {
        transform_base_case_block(
            block_id,
            acc_value,
            operator,
            is_identity,
            &mut next_value,
            tree,
        );
    }

    true
}

/// Transform an exported function using the wrapper approach.
///
/// Creates an internal `@func$impl` with the accumulator parameter,
/// and rewrites the original exported function as a thin wrapper.
#[allow(clippy::too_many_arguments)]
fn try_accumulator_transform_exported(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    patterns: &[AccumulatorPattern],
    operator: BinaryOperator,
    identity: &Constant,
    context: &OptimizationContext<'_>,
) -> bool {
    let return_type = function.return_type;

    // create the impl function name: "func" -> "func$impl"
    let impl_name_str = format!("{}$impl", &*context.strings.get(function.name));
    let impl_name = context.strings.intern(&impl_name_str);

    // clone the function to create the impl version
    let (impl_function_id, impl_entry_block, block_map) =
        clone_function_as_impl(function, tree, impl_name);

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
        })
        .collect();

    // update the impl patterns to call impl instead of original
    // (the cloned call instructions still reference current_function_id)
    for pattern in &impl_patterns {
        update_recursive_calls_to_impl(
            pattern.block_id,
            current_function_id,
            impl_function_id,
            tree,
        );
    }

    let mut next_value = find_max_value_in_module(tree) + 1;

    // add accumulator parameter to impl entry block
    let acc_value = mir::Value::new(next_value);
    next_value += 1;

    let impl_entry = tree.get(impl_entry_block);
    let mut new_impl_entry = impl_entry.clone();
    new_impl_entry.parameters.push(mir::TypedValue {
        value: acc_value,
        ty: return_type,
    });
    tree.replace(impl_entry_block, new_impl_entry);

    // add to impl function parameters
    {
        let impl_func = tree.get_mut(impl_function_id);
        impl_func.parameters.push(mir::TypedValue {
            value: acc_value,
            ty: return_type,
        });
    }

    // transform impl function's accumulator blocks
    for pattern in &impl_patterns {
        transform_accumulator_block(pattern, impl_entry_block, acc_value, &mut next_value, tree);
    }

    // transform impl function's base case blocks
    for &(block_id, is_identity) in &impl_base_cases {
        transform_base_case_block(
            block_id,
            acc_value,
            operator,
            is_identity,
            &mut next_value,
            tree,
        );
    }

    // rewrite original function as wrapper: call impl with identity
    rewrite_as_wrapper(
        function,
        tree,
        entry_block,
        impl_function_id,
        identity,
        &mut next_value,
    );

    true
}

/// Clone a function to create an internal impl version.
///
/// Returns (impl_function_id, impl_entry_block, block_mapping).
#[allow(clippy::type_complexity)]
fn clone_function_as_impl(
    original: &mir::Function,
    tree: &mut mir::NodeTree,
    impl_name: destack_base::StringId,
) -> (
    mir::LocalNodeId<mir::Function>,
    mir::LocalNodeId<mir::Block>,
    std::collections::HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
) {
    use std::collections::HashMap;

    let mut block_map: HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>> =
        HashMap::new();

    // clone all blocks
    for &old_block_id in &original.blocks {
        // clone block first to release borrow on tree
        let old_block = tree.get(old_block_id).clone();

        // clone instructions
        let mut new_instructions = Vec::new();
        for &old_instr_id in &old_block.instructions {
            let old_instr = tree.get(old_instr_id).clone();
            let new_instr_id = tree.insert(old_instr);
            new_instructions.push(new_instr_id);
        }

        // create new block (terminator block refs fixed up later)
        let new_block = mir::Block {
            parameters: old_block.parameters.clone(),
            instructions: new_instructions,
            terminator: old_block.terminator.clone(),
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(old_block_id, new_block_id);
    }

    // fix up terminators to use new block IDs
    for &new_block_id in block_map.values() {
        let block = tree.get(new_block_id).clone();
        let fixed_terminator = remap_terminator_blocks(&block.terminator, &block_map);
        let mut updated = block;
        updated.terminator = fixed_terminator;
        tree.replace(new_block_id, updated);
    }

    // create impl function
    let impl_entry = block_map[&original.entry.unwrap()];
    let impl_blocks: Vec<_> = original.blocks.iter().map(|id| block_map[id]).collect();
    let mut impl_function = mir::Function::new(
        impl_name,
        original.parameters.clone(),
        original.return_type,
        impl_entry,
    );
    impl_function.return_lifetime = original.return_lifetime.clone();
    impl_function.linkage = mir::Linkage::Local;
    impl_function.allocation = original.allocation;
    impl_function.coroutine = original.coroutine;
    impl_function.locals = original.locals.clone();
    impl_function.blocks = impl_blocks;

    // recompute next_value_id after cloning
    impl_function.recompute_next_value_id(tree);

    let impl_function_id = tree.insert(impl_function);

    (impl_function_id, impl_entry, block_map)
}

/// Remap block references in a terminator.
fn remap_terminator_blocks(
    terminator: &mir::Terminator,
    block_map: &std::collections::HashMap<
        mir::LocalNodeId<mir::Block>,
        mir::LocalNodeId<mir::Block>,
    >,
) -> mir::Terminator {
    match terminator {
        mir::Terminator::Jump { target, arguments } => mir::Terminator::Jump {
            target: block_map.get(target).copied().unwrap_or(*target),
            arguments: arguments.clone(),
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => mir::Terminator::Branch {
            condition: *condition,
            then_target: block_map.get(then_target).copied().unwrap_or(*then_target),
            then_arguments: then_arguments.clone(),
            else_target: block_map.get(else_target).copied().unwrap_or(*else_target),
            else_arguments: else_arguments.clone(),
        },
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => mir::Terminator::Switch {
            value: *value,
            default: block_map.get(default).copied().unwrap_or(*default),
            default_arguments: default_arguments.clone(),
            cases: cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: block_map.get(&case.target).copied().unwrap_or(case.target),
                    arguments: case.arguments.clone(),
                })
                .collect(),
        },
        // return, unreachable, unwind don't reference blocks
        other => other.clone(),
    }
}

/// Update recursive calls in a block to call the impl function instead.
fn update_recursive_calls_to_impl(
    block_id: mir::LocalNodeId<mir::Block>,
    original_function_id: mir::LocalNodeId<mir::Function>,
    impl_function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::NodeTree,
) {
    let block = tree.get(block_id).clone();

    for &instr_id in &block.instructions {
        let instr = tree.get(instr_id).clone();
        if let Instruction::Call {
            destination,
            function,
            arguments,
        } = instr
            && function == original_function_id
        {
            let new_instr = Instruction::Call {
                destination,
                function: impl_function_id,
                arguments,
            };
            tree.replace(instr_id, new_instr);
        }
    }
}

/// Rewrite a function as a thin wrapper that calls impl with identity.
fn rewrite_as_wrapper(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    entry_block: mir::LocalNodeId<mir::Block>,
    impl_function_id: mir::LocalNodeId<mir::Function>,
    identity: &Constant,
    next_value: &mut u32,
) {
    // create identity constant
    let identity_value = mir::Value::new(*next_value);
    *next_value += 1;
    let const_instr = Instruction::Const {
        destination: identity_value,
        value: identity.clone(),
    };
    let const_id = tree.insert(const_instr);

    // build call arguments: original params + identity
    let mut call_args: Vec<mir::Value> = function.parameters.iter().map(|p| p.value).collect();
    call_args.push(identity_value);
    let call_arguments = tree.add_arguments(&call_args);

    // create call to impl
    let result_value = mir::Value::new(*next_value);
    *next_value += 1;
    let call_instr = Instruction::Call {
        destination: Some(result_value),
        function: impl_function_id,
        arguments: call_arguments,
    };
    let call_id = tree.insert(call_instr);

    // create new entry block with just: const, call, return
    let entry = tree.get(entry_block);
    let new_entry = mir::Block {
        parameters: entry.parameters.clone(),
        instructions: vec![const_id, call_id],
        terminator: mir::Terminator::Return {
            value: Some(result_value),
        },
    };
    tree.replace(entry_block, new_entry);

    // clear other blocks from function (they're now orphaned, DCE will clean up)
    function.blocks = vec![entry_block];
}

/// Information about a call site that needs to be updated.
#[derive(Debug)]
struct CallSite {
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
    tree: &mir::NodeTree,
) -> Vec<CallSite> {
    let mut call_sites = Vec::new();

    // scan all functions in the module
    for (func_id, func) in tree.iter_nodes::<mir::Function>() {
        // skip imported functions (no body)
        let Some(_entry) = func.entry else {
            continue;
        };

        // check all blocks in this function
        for &block_id in &func.blocks {
            // skip the recursive call blocks (they become jumps)
            if func_id == target_function_id && recursive_call_blocks.contains(&block_id) {
                continue;
            }

            let block = tree.get(block_id);

            // check all instructions for calls to target
            for (idx, &instr_id) in block.instructions.iter().enumerate() {
                let instr = tree.get(instr_id);
                if let Instruction::Call { function, .. } = instr
                    && *function == target_function_id
                {
                    call_sites.push(CallSite {
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
    identity: &Constant,
    next_value: &mut u32,
    tree: &mut mir::NodeTree,
) {
    // get the existing call instruction
    let call_instr = tree.get(call_site.instruction_id).clone();
    let Instruction::Call {
        destination,
        function,
        arguments,
    } = call_instr
    else {
        return;
    };

    // create a value for the identity constant
    let identity_value = mir::Value::new(*next_value);
    *next_value += 1;

    // create the const instruction
    let const_instr = Instruction::Const {
        destination: identity_value,
        value: identity.clone(),
    };
    let const_id = tree.insert(const_instr);

    // get the existing arguments and append the identity
    let mut new_args: Vec<mir::Value> = tree.get_arguments(arguments).to_vec();
    new_args.push(identity_value);

    // create new call with extended arguments
    let new_arguments = tree.add_arguments(&new_args);
    let new_call = Instruction::Call {
        destination,
        function,
        arguments: new_arguments,
    };
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

    let mut new_block = block;
    new_block.instructions = new_instructions;
    tree.replace(call_site.block_id, new_block);
}

/// Check if an operator is associative (and commutative for safety).
fn is_associative_operator(op: BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Add
            | BinaryOperator::Multiply
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Xor
    )
}

/// Get the identity constant for an operator and type.
fn identity_constant_for_operator(
    op: BinaryOperator,
    type_id: mir::LocalNodeId<mir::Type>,
    tree: &mir::NodeTree,
) -> Option<Constant> {
    let ty = tree.get(type_id);

    // only handle integer types for now
    let mir::Type::Int { width, signed } = ty else {
        return None;
    };

    let identity_value: i64 = match op {
        BinaryOperator::Add | BinaryOperator::Or | BinaryOperator::Xor => 0,
        BinaryOperator::Multiply => 1,
        BinaryOperator::And => {
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

    // convert width from u16 to u8 for Constant
    let width_u8 = (*width).try_into().ok()?;

    Some(Constant::Int {
        value: identity_value,
        width: width_u8,
        is_signed: *signed,
    })
}

/// Find all blocks with the accumulator pattern.
fn find_accumulator_patterns(
    function: &mir::Function,
    tree: &mir::NodeTree,
    current_function_id: mir::LocalNodeId<mir::Function>,
) -> Vec<AccumulatorPattern> {
    let mut patterns = Vec::new();

    for &block_id in &function.blocks {
        if let Some(pattern) = detect_accumulator_pattern(block_id, tree, current_function_id) {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Detect the accumulator pattern in a single block.
///
/// Pattern: call @self -> binary op using call result -> return binary result
fn detect_accumulator_pattern(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    current_function_id: mir::LocalNodeId<mir::Function>,
) -> Option<AccumulatorPattern> {
    let block = tree.get(block_id);

    // must end with return of a value
    let mir::Terminator::Return {
        value: Some(returned_value),
    } = &block.terminator
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
        if let Instruction::Binary {
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

    // find the call instruction that produces one of the binary operands
    for (idx, &instr_id) in block.instructions.iter().enumerate() {
        let instr = tree.get(instr_id);
        if let Instruction::Call {
            destination: Some(call_dest),
            function: called_func,
            arguments,
        } = instr
        {
            // must be calling ourselves
            if *called_func != current_function_id {
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

            // the call must come before the binary op
            if idx >= binary_idx {
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
                call_arguments: *arguments,
            });
        }
    }

    None
}

/// Find the maximum value number used across all functions in the module.
///
/// Needed when we're updating call sites in other functions.
fn find_max_value_in_module(tree: &mir::NodeTree) -> u32 {
    let mut max_val = 0u32;

    for (_, func) in tree.iter_nodes::<mir::Function>() {
        // check function parameters
        for param in &func.parameters {
            max_val = max_val.max(param.value.0);
        }

        // check all blocks
        for &block_id in &func.blocks {
            let block = tree.get(block_id);

            // block parameters
            for param in &block.parameters {
                max_val = max_val.max(param.value.0);
            }

            // instructions
            for &instr_id in &block.instructions {
                let instr = tree.get(instr_id);
                if let Some(dest) = instr.destination() {
                    max_val = max_val.max(dest.0);
                }
            }
        }
    }

    max_val
}

/// Find blocks that return without recursion (base cases).
///
/// Returns (block_id, is_identity) pairs where is_identity indicates
/// whether the returned value is the identity constant.
fn find_base_case_blocks(
    function: &mir::Function,
    tree: &mir::NodeTree,
    current_function_id: mir::LocalNodeId<mir::Function>,
    identity: &Constant,
) -> Vec<(mir::LocalNodeId<mir::Block>, bool)> {
    let mut base_cases = Vec::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // must end with return
        let mir::Terminator::Return { value } = &block.terminator else {
            continue;
        };

        // check if block contains a recursive call
        let has_recursive_call = block.instructions.iter().any(|&instr_id| {
            let instr = tree.get(instr_id);
            matches!(instr, Instruction::Call { function, .. } if *function == current_function_id)
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
    identity: &Constant,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> bool {
    // search all blocks for the defining instruction
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instr_id in &block.instructions {
            let instr = tree.get(instr_id);
            if let Instruction::Const {
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
/// Replaces: `v1 = call @self(args); v2 = OP v1, x; return v2`
/// With: `v_new = OP acc, x; jump entry(args..., v_new)`
fn transform_accumulator_block(
    pattern: &AccumulatorPattern,
    entry_block: mir::LocalNodeId<mir::Block>,
    acc_value: mir::Value,
    next_value: &mut u32,
    tree: &mut mir::NodeTree,
) {
    // get the call arguments before any mutations
    let call_args: Vec<mir::Value> = tree.get_arguments(pattern.call_arguments).to_vec();

    // create new accumulator value
    let new_acc = mir::Value::new(*next_value);
    *next_value += 1;

    // create new binary instruction: new_acc = OP acc, other
    let new_binary = Instruction::Binary {
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

    let new_terminator = mir::Terminator::Jump {
        target: entry_block,
        arguments: jump_args,
    };

    // replace the block
    let block = tree.get(pattern.block_id);
    let mut new_block = block.clone();
    new_block.instructions = new_instructions;
    new_block.terminator = new_terminator;
    tree.replace(pattern.block_id, new_block);

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
    operator: BinaryOperator,
    is_identity: bool,
    next_value: &mut u32,
    tree: &mut mir::NodeTree,
) {
    // clone block first to avoid borrow conflicts
    let block = tree.get(block_id).clone();
    let mir::Terminator::Return {
        value: Some(original_value),
    } = &block.terminator
    else {
        return;
    };
    let original_value = *original_value;

    if is_identity {
        // just return the accumulator
        let mut new_block = block;
        new_block.terminator = mir::Terminator::Return {
            value: Some(acc_value),
        };
        tree.replace(block_id, new_block);
    } else {
        // return OP(acc, original_value)
        let result_val = mir::Value::new(*next_value);
        *next_value += 1;

        let combine_instr = Instruction::Binary {
            destination: result_val,
            operator,
            left: acc_value,
            right: original_value,
        };
        let combine_id = tree.insert(combine_instr);

        let mut new_block = block;
        new_block.instructions.push(combine_id);
        new_block.terminator = mir::Terminator::Return {
            value: Some(result_val),
        };
        tree.replace(block_id, new_block);
    }
}

/// Checks if a block ends with a tail-recursive call and transforms it.
///
/// The pattern is: last instruction is `v = call @self(args...)`, terminator is `return v`.
/// For void functions: last instruction is `call @self(args...)`, terminator is `return`.
fn transform_tail_call(
    block_id: mir::LocalNodeId<mir::Block>,
    current_function_id: mir::LocalNodeId<mir::Function>,
    entry_block: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::NodeTree,
) -> bool {
    let block = tree.get(block_id);

    // must end with a return
    let returned_value = match &block.terminator {
        mir::Terminator::Return { value } => *value,
        _ => return false,
    };

    // need at least one instruction
    let Some(&last_instruction_id) = block.instructions.last() else {
        return false;
    };

    // last instruction must be a call
    let last_instruction = tree.get(last_instruction_id);
    let mir::Instruction::Call {
        destination,
        function: called_function,
        arguments,
    } = last_instruction
    else {
        return false;
    };

    // must be calling ourselves
    if *called_function != current_function_id {
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
    let call_args: Vec<mir::Value> = tree.get_arguments(*arguments).to_vec();

    // rewrite the block: remove call, replace return with jump to entry
    let block = tree.get(block_id);
    let mut new_instructions = block.instructions.clone();
    new_instructions.pop();

    let new_terminator = mir::Terminator::Jump {
        target: entry_block,
        arguments: call_args,
    };

    let mut new_block = block.clone();
    new_block.instructions = new_instructions;
    new_block.terminator = new_terminator;
    tree.replace(block_id, new_block);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_eliminate_basic_tail_recursion() {
        // factorial(n, acc) with accumulator style
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_eq v0, v2
    branch v3, block1, block2
block1:
    return v1
block2:
    v4 = imul v0, v1
    v5 = iconst 1i32
    v6 = isub v0, v5
    v7 = call @test(v6, v4)
    return v7
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_eq v0, v2
    branch v3, block1, block2
block1:
    return v1
block2:
    v4 = imul v0, v1
    v5 = iconst 1i32
    v6 = isub v0, v5
    jump block0(v6, v4)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_eliminate_void_tail_recursion() {
        // countdown to zero
        let input = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    call @test(v4)
    return
}"#;
        let expected = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    jump block0(v4)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_transform_factorial_with_accumulator() {
        // classic factorial: n * factorial(n-1), transformed via accumulator
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = isub v0, v1
    v4 = call @test(v3)
    v5 = imul v0, v4
    return v5
}"#;
        // after accumulator transform: adds v6 param, base returns v6, recurse accumulates
        let expected = r#"function @test(v0: i32, v6: i32) -> i32 {
block0(v0: i32, v6: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v6
block2:
    v3 = isub v0, v1
    v7 = imul v6, v0
    jump block0(v3, v7)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_preserve_non_associative_operation() {
        // subtraction is not associative, cannot transform
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v5 = call @test(v4)
    v6 = isub v0, v5
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_call_to_other_function() {
        // call to different function is not transformed
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @other(v0)
    return v1
}
function @other(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_eliminate_gcd_recursion() {
        // euclidean gcd is naturally tail recursive
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_eq v1, v2
    branch v3, block1, block2
block1:
    return v0
block2:
    v4 = srem v0, v1
    v5 = call @test(v1, v4)
    return v5
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_eq v1, v2
    branch v3, block1, block2
block1:
    return v0
block2:
    v4 = srem v0, v1
    jump block0(v1, v4)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_convert_infinite_recursion_to_loop() {
        // infinite recursion becomes infinite loop
        let input = r#"function @test() -> void {
block0:
    call @test()
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    jump block0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_preserve_return_value_mismatch() {
        // returning different value than call result
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = call @test(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_eliminate_fibonacci_recursion() {
        // fib(n, a, b) where a and b are accumulators
        let input = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = iconst 0i32
    v4 = icmp_eq v0, v3
    branch v4, block1, block2
block1:
    return v1
block2:
    v5 = iconst 1i32
    v6 = isub v0, v5
    v7 = iadd v1, v2
    v8 = call @test(v6, v2, v7)
    return v8
}"#;
        let expected = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = iconst 0i32
    v4 = icmp_eq v0, v3
    branch v4, block1, block2
block1:
    return v1
block2:
    v5 = iconst 1i32
    v6 = isub v0, v5
    v7 = iadd v1, v2
    jump block0(v6, v2, v7)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_eliminate_multiple_tail_calls() {
        // function with multiple blocks that have tail calls
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_slt v0, v1
    branch v2, block1, block2
block1:
    v3 = ineg v0
    v4 = call @test(v3)
    return v4
block2:
    v5 = iconst 10i32
    v6 = icmp_sgt v0, v5
    branch v6, block3, block4
block3:
    v7 = isub v0, v5
    v8 = call @test(v7)
    return v8
block4:
    return v0
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_slt v0, v1
    branch v2, block1, block2
block1:
    v3 = ineg v0
    jump block0(v3)
block2:
    v5 = iconst 10i32
    v6 = icmp_sgt v0, v5
    branch v6, block3, block4
block3:
    v7 = isub v0, v5
    jump block0(v7)
block4:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_eliminate_reordered_args_call() {
        // swap(a, b) calls swap(b, a)
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_sgt v0, v1
    branch v2, block1, block2
block1:
    v3 = call @test(v1, v0)
    return v3
block2:
    return v0
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_sgt v0, v1
    branch v2, block1, block2
block1:
    jump block0(v1, v0)
block2:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_handle_empty_block() {
        // block with only terminator, no instructions
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_non_final_call() {
        // call followed by other instruction before return
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = isub v0, v1
    v3 = call @test(v2)
    v4 = iconst 0i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_mutual_recursion() {
        // even/odd mutual recursion: not self-recursion
        let input = r#"function @even(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iconst true
    return v3
block2:
    v4 = iconst 1i32
    v5 = isub v0, v4
    v6 = call @odd(v5)
    return v6
}
function @odd(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iconst false
    return v3
block2:
    v4 = iconst 1i32
    v5 = isub v0, v4
    v6 = call @even(v5)
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_transform_sum_with_accumulator() {
        // sum(n) = n + sum(n-1), identity for add is 0
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v5 = call @test(v4)
    v6 = iadd v0, v5
    return v6
}"#;
        let expected = r#"function @test(v0: i32, v7: i32) -> i32 {
block0(v0: i32, v7: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return v7
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v8 = iadd v7, v0
    jump block0(v4, v8)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_transform_bitwise_or_accumulator() {
        // or_bits(n) = n | or_bits(n-1), identity for or is 0
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v5 = call @test(v4)
    v6 = bor v0, v5
    return v6
}"#;
        let expected = r#"function @test(v0: i32, v7: i32) -> i32 {
block0(v0: i32, v7: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    return v7
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v8 = bor v7, v0
    jump block0(v4, v8)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_transform_non_identity_base_case() {
        // sum with non-zero base: returns 5 when n=0
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iconst 5i32
    return v3
block2:
    v4 = iconst 1i32
    v5 = isub v0, v4
    v6 = call @test(v5)
    v7 = iadd v0, v6
    return v7
}"#;
        // base case returns OP(acc, 5) since 5 is not the identity
        let expected = r#"function @test(v0: i32, v8: i32) -> i32 {
block0(v0: i32, v8: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iconst 5i32
    v10 = iadd v8, v3
    return v10
block2:
    v4 = iconst 1i32
    v5 = isub v0, v4
    v9 = iadd v8, v0
    jump block0(v5, v9)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_preserve_call_result_used_twice() {
        // call result used in multiple places, not just the binary op
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = isub v0, v1
    v4 = call @test(v3)
    v5 = imul v0, v4
    v6 = iadd v5, v4
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_preserve_mixed_operators() {
        // multiple recursive sites with different operators
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 0i32
    v3 = icmp_eq v0, v2
    branch v3, block1, block2
block1:
    v4 = iconst 1i32
    return v4
block2:
    v5 = iconst 1i32
    v6 = isub v0, v5
    branch v1, block3, block4
block3:
    v7 = call @test(v6, v1)
    v8 = imul v0, v7
    return v8
block4:
    v9 = call @test(v6, v1)
    v10 = iadd v0, v9
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        // should not transform: different operators in different paths
        program.assert_unchanged(input);
    }

    #[test]
    fn test_transform_with_external_caller() {
        // factorial with accumulator pattern, called from main
        // should transform and update the call site in main to pass identity
        let input = r#"function @factorial(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = isub v0, v1
    v4 = call @factorial(v3)
    v5 = imul v0, v4
    return v5
}
function @main() -> i32 {
block0:
    v0 = iconst 5i32
    v1 = call @factorial(v0)
    return v1
}"#;
        // after transform: factorial gets accumulator param, main passes identity (1)
        let expected = r#"function @factorial(v0: i32, v6: i32) -> i32 {
block0(v0: i32, v6: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v6
block2:
    v3 = isub v0, v1
    v8 = imul v6, v0
    jump block0(v3, v8)
}
function @main() -> i32 {
block0:
    v0 = iconst 5i32
    v7 = iconst 1i32
    v1 = call @factorial(v0, v7)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }

    #[test]
    fn test_transform_exported_with_wrapper() {
        // exported factorial: should create impl + wrapper
        let input = r#"export function @factorial(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = isub v0, v1
    v4 = call @factorial(v3)
    v5 = imul v0, v4
    return v5
}"#;
        // exported wrapper calls internal impl with identity
        // impl has tail-recursive structure
        let expected = r#"export function @factorial(v0: i32) -> i32 {
block0(v0: i32):
    v8 = iconst 1i32
    v9 = call @factorial$impl(v0, v8)
    return v9
}
function @factorial$impl(v0: i32, v6: i32) -> i32 {
block0(v0: i32, v6: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v6
block2:
    v3 = isub v0, v1
    v7 = imul v6, v0
    jump block0(v3, v7)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&TailCallElim);
        program.assert_output(expected);
    }
}

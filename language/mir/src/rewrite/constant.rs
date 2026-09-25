use tspp_core::FxIndexMap;

use crate as mir;
use crate::{instruction_substitute_uses_in_tree, terminator_substitute_uses};

/// Apply constant parameters inside a function body.
pub fn apply_constant_parameters(
    function_id: mir::LocalNodeId<mir::Function>,
    constants: &[Option<mir::Constant>],
    tree: &mut mir::Tree,
) -> bool {
    // prepare the substitution map and new instructions
    let mut substitutions: FxIndexMap<mir::Value, mir::Value> = FxIndexMap::default();
    let mut new_instructions = Vec::new();

    // allocate new constants at the entry block
    let mut function = tree.get(function_id).clone();
    let parameters = function.parameters.clone();
    let Some(body) = function.body_mut() else {
        unreachable!("cannot apply constants to a function without a body");
    };
    body.recompute_next_value_id(&parameters, tree);

    let entry_id = body.entry();

    for (param, constant) in parameters.iter().zip(constants.iter()) {
        // skip non constant parameters
        let Some(constant) = constant else {
            continue;
        };

        // allocate a new constant value
        let parameter = param.typed_value();

        let destination = body.next_typed_value(parameter.ty);
        let parameter_value = parameter.value;
        substitutions.insert(parameter_value, destination);
        new_instructions.push((destination, constant.clone()));
    }

    // insert constant instructions before the entry block body
    if !new_instructions.is_empty() {
        let mut new_instruction_ids = Vec::new();

        for (destination, constant) in &new_instructions {
            let instruction_id = tree.insert(mir::Instruction::Const {
                destination: (*destination),
                value: constant.clone(),
            });
            new_instruction_ids.push(instruction_id);
        }

        let mut instructions = tree.get(entry_id).instructions.clone();
        instructions.splice(0..0, new_instruction_ids);
        function.replace_block_instructions(entry_id, instructions, tree);
        *tree.get_mut(function_id) = function.clone();
    }

    // stop if no substitutions were created
    if substitutions.is_empty() {
        return false;
    }

    // substitute uses across all blocks
    let function = tree.get(function_id).clone();
    for &block_id in function.blocks() {
        let instruction_ids = tree.get(block_id).instructions.clone();

        // rewrite instruction operands
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
            if instruction != updated {
                *tree.get_mut(instruction_id) = updated;
            }
        }

        // rewrite terminator operands
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();
        let updated = terminator_substitute_uses(tree, &terminator, &substitutions);
        if terminator != updated {
            tree.set(terminator_id, updated);
        }
    }

    true
}

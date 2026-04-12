use destack_core::{StringId, StringPool};

use crate::{Function, LocalNodeId, NodeTree, Value, ValueReference};

/// Finalize missing block and SSA value names for one function.
pub(crate) fn finalize_function_names(
    tree: &mut NodeTree,
    strings: &mut StringPool,
    function_id: LocalNodeId<Function>,
) {
    let function = tree.get(function_id).clone();

    // block names
    for (block_index, block_id) in function.blocks.iter().enumerate() {
        let block = tree.get(*block_id);
        if block.name.is_some() {
            continue;
        }

        let prefix = if block_index == 0 { "entry" } else { "block" };
        let name = strings.intern(&format!("{prefix}{block_index}"));
        let block = tree.get_mut(*block_id);
        block.name = Some(name);
    }

    // function parameters
    for (parameter_index, parameter) in function.parameters.iter().enumerate() {
        let ValueReference::Value(value) = parameter.value else {
            continue;
        };

        if tree.get(function_id).value_name(value).is_some() {
            continue;
        }

        // derive the generated-name prefix from the authored parameter name
        let prefix = function
            .parameter_names
            .get(parameter_index)
            .and_then(|name| *name)
            .map(|name| strings.get(name).to_string())
            .map(|name| identifier_prefix(&name))
            .unwrap_or_else(|| "value".to_string());

        let name = strings.intern(&format!("{prefix}{}", value.0));
        let function = tree.get_mut(function_id);
        set_value_name(function, value, name);
    }

    // block parameters and instruction destinations
    for block_id in &function.blocks {
        let block = tree.get(*block_id).clone();

        for parameter in &block.parameters {
            let ValueReference::Value(value) = parameter.value else {
                continue;
            };

            if tree.get(function_id).value_name(value).is_some() {
                continue;
            }

            let name = strings.intern(&format!("value{}", value.0));
            let function = tree.get_mut(function_id);
            set_value_name(function, value, name);
        }

        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            let Some(ValueReference::Value(destination)) = instruction.destination() else {
                continue;
            };

            if tree.get(function_id).value_name(destination).is_some() {
                continue;
            }

            let name = strings.intern(&format!("value{}", destination.0));
            let function = tree.get_mut(function_id);
            set_value_name(function, destination, name);
        }
    }
}

/// Set the explicit name for one SSA value.
fn set_value_name(function: &mut Function, value: Value, name: StringId) {
    let index = value.0 as usize;
    if index >= function.value_names.len() {
        function.value_names.resize(index + 1, None);
    }

    function.value_names[index] = Some(name);
}

/// Normalize one authored identifier into a generated-name prefix.
fn identifier_prefix(name: &str) -> String {
    let mut prefix = String::new();

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            prefix.push(ch);
        }
    }

    while prefix.ends_with(|ch: char| ch.is_ascii_digit()) {
        prefix.pop();
    }

    if prefix.is_empty() {
        return "value".to_string();
    }

    if !prefix
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
    {
        return "value".to_string();
    }

    prefix
}

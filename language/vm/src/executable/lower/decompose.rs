use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use super::kind::value_type_for_value;
use super::pool::Pool;
use super::tree::{
    BlockParameterMap, ValueDecomposition, ValueTree, can_decompose_value_type, seed_value_tree,
};
use crate::executable::{Instruction, InstructionData, InstructionOperation};

/// One deferred dynamic element.get plan.
struct IndexSelectPlan {
    /// The destination slot to write.
    dest: mir::Value,
    /// The candidate values in source order.
    child: Vec<mir::Value>,
}

/// One deferred dynamic element.set plan for one slot.
struct SelectByIndexPlan {
    /// The destination slot to write.
    dest: mir::Value,
    /// The matching array index for this destination.
    match_index: u64,
    /// The replacement value when the index matches.
    then_value: mir::Value,
    /// The original value when the index does not match.
    else_value: mir::Value,
}

/// One block-local decomposition lowerer.
pub(super) struct DecompositionLowerer<'a> {
    /// The MIR node tree.
    tree: &'a mir::NodeTree,
    /// The lowered value type by SSA value id.
    value_type: &'a [mir::LocalNodeId<mir::Type>],
    /// The block parameter decomposition metadata.
    block_parameter_map: &'a BlockParameterMap,
    /// The block-local decomposition state by root value.
    decomposition_by_value: HashMap<mir::Value, ValueDecomposition>,
}

impl<'a> DecompositionLowerer<'a> {
    /// Create one decomposition lowerer.
    pub(super) fn new(
        tree: &'a mir::NodeTree,
        value_type: &'a [mir::LocalNodeId<mir::Type>],
        block_parameter_map: &'a BlockParameterMap,
    ) -> Self {
        Self {
            tree,
            value_type,
            block_parameter_map,
            decomposition_by_value: HashMap::new(),
        }
    }

    /// Return whether the block-local decomposition state is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.decomposition_by_value.is_empty()
    }

    /// Return the current decomposition state.
    pub(super) fn map(&self) -> &HashMap<mir::Value, ValueDecomposition> {
        &self.decomposition_by_value
    }

    /// Seed one block entry from decomposed block parameters.
    pub(super) fn seed_block(&mut self, block: mir::LocalNodeId<mir::Block>) {
        let Some(parameter_tree) = self.block_parameter_map.tree_by_block.get(&block) else {
            return;
        };

        for tree in parameter_tree.values() {
            seed_value_tree(tree, &mut self.decomposition_by_value);
        }
    }

    /// Try to lower one instruction through the decomposition model.
    pub(super) fn lower_instruction(
        &mut self,
        inst: &mir::Instruction,
        pool: &mut Pool,
        instructions: &mut Vec<Instruction>,
    ) -> bool {
        match inst {
            // pure construction
            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let child = self
                    .tree
                    .get_arguments(*fields)
                    .iter()
                    .map(|field| field.value())
                    .collect::<Option<Vec<_>>>();
                let Some(child) = child else {
                    return false;
                };
                self.decomposition_by_value
                    .insert(destination, ValueDecomposition { child });

                true
            }
            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let child = self
                    .tree
                    .get_arguments(*elements)
                    .iter()
                    .map(|element| element.value())
                    .collect::<Option<Vec<_>>>();
                let Some(child) = child else {
                    return false;
                };
                self.decomposition_by_value
                    .insert(destination, ValueDecomposition { child });

                true
            }
            mir::Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let child = self
                    .tree
                    .get_arguments(*elements)
                    .iter()
                    .map(|element| element.value())
                    .collect::<Option<Vec<_>>>();
                let Some(child) = child else {
                    return false;
                };
                self.decomposition_by_value
                    .insert(destination, ValueDecomposition { child });

                true
            }

            // pure projection
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let Some(aggregate) = aggregate.value() else {
                    return false;
                };
                let Some(aggregate) = self.decomposition_by_value.get(&aggregate) else {
                    return false;
                };
                let Some(source) = aggregate.child.get(*index as usize).copied() else {
                    return false;
                };
                let Some(destination_type) = value_type_for_value(destination, self.value_type)
                else {
                    return false;
                };

                if can_decompose_value_type(self.tree, destination_type)
                    && let Some(source_composite) =
                        self.decomposition_by_value.get(&source).cloned()
                {
                    self.decomposition_by_value
                        .insert(destination, source_composite);
                    return true;
                }

                instructions.push(Instruction {
                    operation: InstructionOperation::Copy,
                    data: InstructionData::Copy {
                        dest: destination,
                        source,
                    },
                });

                true
            }

            // pure updates
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let Some(aggregate) = aggregate.value() else {
                    return false;
                };
                let Some(value) = value.value() else {
                    return false;
                };
                let Some(aggregate) = self.decomposition_by_value.get(&aggregate).cloned() else {
                    return false;
                };
                if aggregate.child.get(*index as usize).is_none() {
                    return false;
                }

                let mut child = aggregate.child;
                child[*index as usize] = value;

                self.decomposition_by_value
                    .insert(destination, ValueDecomposition { child });

                true
            }
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let Some(array) = array.value() else {
                    return false;
                };
                let Some(index) = index.value() else {
                    return false;
                };
                let Some(array) = self.decomposition_by_value.get(&array) else {
                    return false;
                };

                let Some(destination_type) = value_type_for_value(destination, self.value_type)
                else {
                    return false;
                };

                if can_decompose_value_type(self.tree, destination_type) {
                    let Some(destination_tree) =
                        self.block_parameter_map.tree_by_value.get(&destination)
                    else {
                        return false;
                    };

                    let mut plan = Vec::new();
                    let mut path = Vec::new();
                    let is_valid = self.collect_index_select_plan(
                        destination_tree,
                        &array.child,
                        &mut path,
                        &mut plan,
                    );
                    if !is_valid {
                        return false;
                    }

                    self.emit_index_select_plan(instructions, pool, index, plan);
                    seed_value_tree(destination_tree, &mut self.decomposition_by_value);
                    return true;
                }

                let elements = pool.argument_range(&array.child);
                instructions.push(Instruction {
                    operation: InstructionOperation::IndexSelect,
                    data: InstructionData::IndexSelect {
                        dest: destination,
                        index,
                        elements,
                    },
                });

                true
            }
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let Some(destination) = destination.value() else {
                    return false;
                };
                let Some(array) = array.value() else {
                    return false;
                };
                let Some(index) = index.value() else {
                    return false;
                };
                let Some(value) = value.value() else {
                    return false;
                };
                let Some(array) = self.decomposition_by_value.get(&array) else {
                    return false;
                };
                let Some(destination_tree) =
                    self.block_parameter_map.tree_by_value.get(&destination)
                else {
                    return false;
                };

                if destination_tree.child.len() != array.child.len() {
                    return false;
                }

                let mut plan = Vec::new();

                for (match_index, (destination_element, source_element)) in
                    destination_tree.child.iter().zip(&array.child).enumerate()
                {
                    let mut path = Vec::new();
                    let is_valid = self.collect_select_by_index_plan(
                        destination_element,
                        *source_element,
                        value,
                        match_index as u64,
                        &mut path,
                        &mut plan,
                    );
                    if !is_valid {
                        return false;
                    }
                }

                self.emit_select_by_index_plan(instructions, index, plan);
                seed_value_tree(destination_tree, &mut self.decomposition_by_value);
                true
            }

            _ => false,
        }
    }

    /// Flush all decomposed values into explicit composite instructions.
    pub(super) fn flush(&mut self, instructions: &mut Vec<Instruction>, pool: &mut Pool) {
        // collect roots before recursive emission mutates the map
        let root = self
            .decomposition_by_value
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut emitted = HashSet::new();

        // emit each decomposed value in dependency order
        for value in root {
            self.emit_value(value, instructions, pool, &mut emitted);
        }

        // clear the block-local decomposition state after emission
        self.decomposition_by_value.clear();
    }

    /// Project one decomposed value through a constant child path.
    fn project_value_path(&self, value: mir::Value, path: &[usize]) -> Option<mir::Value> {
        let mut current = value;

        for index in path {
            let decomposition = self.decomposition_by_value.get(&current)?;
            current = *decomposition.child.get(*index)?;
        }

        Some(current)
    }

    /// Collect one dynamic index-select plan for a destination component tree.
    fn collect_index_select_plan(
        &self,
        target: &ValueTree,
        source_child: &[mir::Value],
        path: &mut Vec<usize>,
        plan: &mut Vec<IndexSelectPlan>,
    ) -> bool {
        if target.child.is_empty() {
            let mut child = Vec::with_capacity(source_child.len());

            for source in source_child {
                let Some(projected) = self.project_value_path(*source, path) else {
                    return false;
                };
                child.push(projected);
            }

            plan.push(IndexSelectPlan {
                dest: target.slot,
                child,
            });
            return true;
        }

        for (index, component) in target.child.iter().enumerate() {
            path.push(index);

            let is_valid = self.collect_index_select_plan(component, source_child, path, plan);

            path.pop();

            if !is_valid {
                return false;
            }
        }

        true
    }

    /// Collect one dynamic element-set plan for a destination component tree.
    fn collect_select_by_index_plan(
        &self,
        target: &ValueTree,
        source_value: mir::Value,
        replacement_value: mir::Value,
        match_index: u64,
        path: &mut Vec<usize>,
        plan: &mut Vec<SelectByIndexPlan>,
    ) -> bool {
        if target.child.is_empty() {
            let Some(then_value) = self.project_value_path(replacement_value, path) else {
                return false;
            };
            let Some(else_value) = self.project_value_path(source_value, path) else {
                return false;
            };

            plan.push(SelectByIndexPlan {
                dest: target.slot,
                match_index,
                then_value,
                else_value,
            });
            return true;
        }

        for (index, component) in target.child.iter().enumerate() {
            path.push(index);

            let is_valid = self.collect_select_by_index_plan(
                component,
                source_value,
                replacement_value,
                match_index,
                path,
                plan,
            );

            path.pop();

            if !is_valid {
                return false;
            }
        }

        true
    }

    /// Emit one collected dynamic element.get plan.
    fn emit_index_select_plan(
        &self,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool,
        index: mir::Value,
        plan: Vec<IndexSelectPlan>,
    ) {
        for plan in plan {
            let elements = pool.argument_range(&plan.child);

            instructions.push(Instruction {
                operation: InstructionOperation::IndexSelect,
                data: InstructionData::IndexSelect {
                    dest: plan.dest,
                    index,
                    elements,
                },
            });
        }
    }

    /// Emit one collected dynamic element.set plan.
    fn emit_select_by_index_plan(
        &self,
        instructions: &mut Vec<Instruction>,
        index: mir::Value,
        plan: Vec<SelectByIndexPlan>,
    ) {
        for plan in plan {
            instructions.push(Instruction {
                operation: InstructionOperation::SelectByIndex,
                data: InstructionData::SelectByIndex {
                    dest: plan.dest,
                    index,
                    match_index: plan.match_index,
                    then_value: plan.then_value,
                    else_value: plan.else_value,
                },
            });
        }
    }

    /// Emit one decomposed value and all decomposed dependencies it references.
    fn emit_value(
        &self,
        value: mir::Value,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool,
        emitted: &mut HashSet<mir::Value>,
    ) {
        // skip already emitted or already materialized values
        if emitted.contains(&value) {
            return;
        }
        let Some(composite) = self.decomposition_by_value.get(&value) else {
            return;
        };

        // materialize decomposed children before this parent
        for child in &composite.child {
            self.emit_value(*child, instructions, pool, emitted);
        }

        // emit one explicit materialization at the boundary
        let elements = pool.argument_range(&composite.child);
        instructions.push(Instruction {
            operation: InstructionOperation::Composite,
            data: InstructionData::Composite {
                dest: value,
                elements,
            },
        });
        emitted.insert(value);
    }
}
